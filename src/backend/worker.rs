use super::{
    ConnectionState, VpnEvent, VpnRequest,
    connection::{self, disconnect_all},
    nmcli::{parse_status, run_nmcli},
    profiles,
};
use crate::profile::{self, Profiles};
use anyhow::{Result, bail};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

pub(crate) struct VpnBackend {
    pub(crate) requests: Sender<VpnRequest>,
    pub(crate) events: Receiver<VpnEvent>,
    pub(crate) cancel: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl VpnBackend {
    pub(crate) fn start(root: PathBuf) -> Self {
        let (requests, rx) = mpsc::channel();
        let (tx, events) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let worker = thread::spawn(move || {
            let mut store = match profile::load(&root) {
                Ok(store) => store,
                Err(e) => {
                    let _ = tx.send(VpnEvent::Error(format!("{e:#}")));
                    return;
                }
            };
            let _ = tx.send(VpnEvent::Profiles(store.clone()));
            let _ = tx.send(VpnEvent::Ready);
            let mut previous = ConnectionState::Idle;
            let mut last_error = String::new();
            loop {
                let request = match rx.recv_timeout(Duration::from_secs(2)) {
                    Ok(request) => Some(request),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(_) => break,
                };
                if let Some(request) = request {
                    let shutting_down = matches!(request, VpnRequest::Shutdown | VpnRequest::Stop);
                    let forced = matches!(request, VpnRequest::Stop);
                    let _ = tx.send(VpnEvent::Busy(true));
                    let result = handle_request(request, &root, &mut store, &tx, &worker_cancel);
                    if shutting_down {
                        match result {
                            Ok(()) => {
                                let _ = tx.send(VpnEvent::ShutdownComplete);
                                return;
                            }
                            Err(error) => {
                                let _ = tx.send(VpnEvent::ShutdownFailed(format!(
                                    "Could not finish disconnecting. The app is still open; try closing again. {error:#}"
                                )));
                                if forced {
                                    eprintln!("VPN exit cleanup failed: {error:#}");
                                    return;
                                }
                            }
                        }
                    } else if let Err(e) = result {
                        let _ = tx.send(VpnEvent::Error(format!("{e:#}")));
                    }
                    let _ = tx.send(VpnEvent::Profiles(store.clone()));
                    let _ = tx.send(VpnEvent::Busy(false));
                    // The command itself reports errors; don't also report an intentional disconnect.
                    previous = ConnectionState::Idle;
                }
                let result = if let Some(profile) = store.selected() {
                    run_nmcli(
                        &[
                            "-g",
                            "GENERAL.STATE,IP4.ADDRESS,IP6.ADDRESS",
                            "connection",
                            "show",
                            "uuid",
                            &profile.nm_uuid.to_string(),
                        ],
                        8,
                        None,
                    )
                    .map(|text| parse_status(&text))
                } else {
                    run_nmcli(&["-t", "-f", "RUNNING", "general"], 8, None).and_then(|s| {
                        if s == "running" {
                            Ok((ConnectionState::Idle, String::new()))
                        } else {
                            bail!("NetworkManager is not running.")
                        }
                    })
                };
                match result {
                    Ok((state, address)) => {
                        if previous == ConnectionState::Connected && state == ConnectionState::Idle
                        {
                            let _ = tx.send(VpnEvent::Error(
                                "The VPN connection ended. Connect again when you are ready."
                                    .into(),
                            ));
                        }
                        previous = state;
                        last_error.clear();
                        if tx.send(VpnEvent::Status { state, address }).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let error = format!("{e:#}");
                        if last_error != error {
                            let _ = tx.send(VpnEvent::Error(error.clone()));
                            last_error = error;
                        }
                        previous = ConnectionState::Unavailable;
                        if tx
                            .send(VpnEvent::Status {
                                state: ConnectionState::Unavailable,
                                address: String::new(),
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
            // A lost UI/channel still has to release tunnels owned by this app.
            if let Err(error) = disconnect_all(&store) {
                eprintln!("VPN exit cleanup failed: {error:#}");
            }
        });
        Self {
            requests,
            events,
            cancel,
            worker: Some(worker),
        }
    }
}

impl Drop for VpnBackend {
    fn drop(&mut self) {
        // Fallback for application-level quit/unwind. Normal window closing
        // waits asynchronously for ShutdownComplete before dropping this handle.
        self.cancel.store(true, Ordering::Relaxed);
        let _ = self.requests.send(VpnRequest::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Dispatch requests without mixing worker lifecycle with operation details.
pub(super) fn handle_request(
    request: VpnRequest,
    root: &Path,
    store: &mut Profiles,
    tx: &Sender<VpnEvent>,
    cancel: &AtomicBool,
) -> Result<()> {
    match request {
        VpnRequest::Import {
            path,
            name,
            credentials,
        } => profiles::import(path, name, credentials, root, store),
        VpnRequest::Select(id) => profiles::select(id, root, store),
        VpnRequest::SaveCredentials {
            id,
            username,
            password,
        } => profiles::save_credentials(id, username, password, root, store, tx),
        VpnRequest::Connect => connection::connect(root, store, tx, cancel),
        VpnRequest::Disconnect => connection::disconnect(store, tx),
        VpnRequest::Shutdown | VpnRequest::Stop => disconnect_all(store),
        VpnRequest::Remove(id) => profiles::remove(id, root, store),
    }
}
