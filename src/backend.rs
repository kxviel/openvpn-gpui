use crate::profile::{self, Profile, Profiles};
use anyhow::{Context, Result, bail};
use std::{
    fs,
    io::{Read, Seek},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum ConnectionState {
    #[default]
    Idle,
    Connecting,
    Connected,
    Disconnecting,
    Unavailable,
}

pub(crate) enum VpnRequest {
    Import { path: PathBuf, name: String },
    Select(Uuid),
    Connect,
    Disconnect,
    Remove,
}

pub(crate) enum VpnEvent {
    Profiles(Profiles),
    Status {
        state: ConnectionState,
        address: String,
    },
    Error(String),
    Busy(bool),
    Ready,
}

pub(crate) struct VpnBackend {
    pub(crate) requests: Sender<VpnRequest>,
    pub(crate) events: Receiver<VpnEvent>,
    pub(crate) cancel: Arc<AtomicBool>,
}

/// Bound every subprocess and keep pipes off the UI thread. Secrets are never
/// passed on argv; the desktop's NetworkManager secret agent handles them.
fn run_nmcli(args: &[&str], limit: u64, cancel: Option<&AtomicBool>) -> Result<String> {
    let mut stdout = tempfile::tempfile()?;
    let mut stderr = tempfile::tempfile()?;
    let mut child = Command::new("nmcli")
        .env("LC_ALL", "C")
        .env("NO_COLOR", "1")
        .args(["--escape", "no", "--colors", "no"])
        .args(args)
        .stdin(Stdio::null())
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?)
        .spawn()
        .context("Cannot run nmcli. Install NetworkManager and its OpenVPN plugin.")?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed))
            || start.elapsed() > Duration::from_secs(limit)
        {
            let _ = child.kill();
            let _ = child.wait();
            bail!("The operation was cancelled or timed out.");
        }
        thread::sleep(Duration::from_millis(100));
    };
    stdout.rewind()?;
    stderr.rewind()?;
    let mut output = String::new();
    let mut error = String::new();
    stdout.take(64 * 1024).read_to_string(&mut output)?;
    stderr.take(64 * 1024).read_to_string(&mut error)?;
    if !status.success() {
        let message = if error.trim().is_empty() {
            output.trim()
        } else {
            error.trim()
        };
        bail!(
            "{}",
            if message.is_empty() {
                "NetworkManager could not complete the operation."
            } else {
                message
            }
        );
    }
    Ok(output.trim().to_owned())
}

pub(crate) fn parse_import_uuid(output: &str) -> Result<Uuid> {
    output
        .rsplit_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .and_then(|(id, _)| Uuid::parse_str(id).ok())
        .context("NetworkManager did not return an imported connection ID")
}

pub(crate) fn parse_status(output: &str) -> (ConnectionState, String) {
    let mut lines = output.lines();
    let state = match lines.next().unwrap_or("") {
        "activated" => ConnectionState::Connected,
        "activating" => ConnectionState::Connecting,
        "deactivating" => ConnectionState::Disconnecting,
        _ => ConnectionState::Idle,
    };
    let address = if state == ConnectionState::Connected {
        lines
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .split('/')
            .next()
            .unwrap_or("")
            .to_owned()
    } else {
        String::new()
    };
    (state, address)
}

impl VpnBackend {
    pub(crate) fn start(root: PathBuf) -> Self {
        let (requests, rx) = mpsc::channel();
        let (tx, events) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        thread::spawn(move || {
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
                    let _ = tx.send(VpnEvent::Busy(true));
                    let result = handle_request(request, &root, &mut store, &tx, &worker_cancel);
                    if let Err(e) = result {
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
        });
        Self {
            requests,
            events,
            cancel,
        }
    }
}

fn handle_request(
    request: VpnRequest,
    root: &std::path::Path,
    store: &mut Profiles,
    tx: &Sender<VpnEvent>,
    cancel: &AtomicBool,
) -> Result<()> {
    match request {
        VpnRequest::Import { path, name } => {
            let name = store.validate_name(&name)?;
            let id = Uuid::new_v4();
            let destination = root.join(id.to_string());
            let mut imported = None;
            let result = (|| -> Result<()> {
                let (remote, protocol) = profile::bundle(&path, &destination)?;
                let config = profile::config_path(&destination);
                let output = run_nmcli(
                    &[
                        "--wait",
                        "20",
                        "connection",
                        "import",
                        "type",
                        "openvpn",
                        "file",
                        config.to_str().context("Invalid configuration path")?,
                    ],
                    25,
                    None,
                )?;
                let nm_uuid = parse_import_uuid(&output)?;
                imported = Some(nm_uuid);
                let uuid = nm_uuid.to_string();
                run_nmcli(
                    &[
                        "connection",
                        "modify",
                        "uuid",
                        &uuid,
                        "connection.id",
                        &format!("OpenVPN · {name}"),
                        "connection.autoconnect",
                        "no",
                    ],
                    10,
                    None,
                )?;
                if let Ok(user) = std::env::var("USER") {
                    run_nmcli(
                        &[
                            "connection",
                            "modify",
                            "uuid",
                            &uuid,
                            "connection.permissions",
                            &format!("user:{user}"),
                        ],
                        10,
                        None,
                    )?;
                }
                let mut next = store.clone();
                next.profiles.push(Profile {
                    id,
                    nm_uuid,
                    name,
                    remote,
                    protocol,
                });
                next.selected = Some(id);
                profile::save(root, &next)?;
                *store = next;
                Ok(())
            })();
            if result.is_err() {
                let cleaned = imported.is_none_or(|uuid| {
                    run_nmcli(
                        &["connection", "delete", "uuid", &uuid.to_string()],
                        10,
                        None,
                    )
                    .is_ok()
                });
                if cleaned {
                    let _ = fs::remove_dir_all(destination);
                }
            }
            result?;
        }
        VpnRequest::Select(id) => {
            if !store.profiles.iter().any(|p| p.id == id) {
                bail!("Profile no longer exists.");
            }
            let mut next = store.clone();
            next.selected = Some(id);
            profile::save(root, &next)?;
            *store = next;
        }
        VpnRequest::Connect => {
            let profile = store.selected().context("Import a profile first.")?;
            let uuid = profile.nm_uuid.to_string();
            let _ = tx.send(VpnEvent::Status {
                state: ConnectionState::Connecting,
                address: String::new(),
            });
            let result = run_nmcli(
                &["--wait", "90", "connection", "up", "uuid", &uuid],
                95,
                Some(cancel),
            );
            if result.is_err() {
                // Killing nmcli doesn't stop NM activation; explicitly tear down this exact profile.
                let cleanup = run_nmcli(
                    &["--wait", "15", "connection", "down", "uuid", &uuid],
                    20,
                    None,
                );
                if cancel.load(Ordering::Relaxed) {
                    // An already-inactive profile is a successful cancellation too.
                    if cleanup.is_err() {
                        let state = run_nmcli(
                            &["-g", "GENERAL.STATE", "connection", "show", "uuid", &uuid],
                            8,
                            None,
                        )?;
                        if !state.is_empty() && state != "deactivated" {
                            cleanup?;
                        }
                    }
                    return Ok(());
                }
            }
            result.context("Connection failed. If credentials are required, check your desktop's VPN authentication prompt.")?;
        }
        VpnRequest::Disconnect => {
            let profile = store.selected().context("No profile is selected.")?;
            let uuid = profile.nm_uuid.to_string();
            let text = run_nmcli(
                &["-g", "GENERAL.STATE", "connection", "show", "uuid", &uuid],
                8,
                None,
            )?;
            if !text.is_empty() && text != "deactivated" {
                let _ = tx.send(VpnEvent::Status {
                    state: ConnectionState::Disconnecting,
                    address: String::new(),
                });
                run_nmcli(
                    &["--wait", "15", "connection", "down", "uuid", &uuid],
                    20,
                    None,
                )?;
            }
        }
        VpnRequest::Remove => {
            let profile = store.selected().context("No profile is selected.")?.clone();
            let uuid = profile.nm_uuid.to_string();
            // Missing NM connections can still be removed from our library.
            let existing = run_nmcli(&["-g", "UUID", "connection", "show"], 8, None)?;
            if existing.lines().any(|line| line == uuid) {
                let status = run_nmcli(
                    &["-g", "GENERAL.STATE", "connection", "show", "uuid", &uuid],
                    8,
                    None,
                )?;
                if !status.is_empty() && status != "deactivated" {
                    bail!("Disconnect this profile before removing it.");
                }
                run_nmcli(
                    &["--wait", "15", "connection", "delete", "uuid", &uuid],
                    20,
                    None,
                )?;
            }
            let mut next = store.clone();
            next.profiles.retain(|p| p.id != profile.id);
            next.selected = next.profiles.first().map(|p| p.id);
            profile::save(root, &next)?;
            *store = next;
            fs::remove_dir_all(root.join(profile.id.to_string())).or_else(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(e)
                }
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn import_uuid_is_exact() {
        let id = Uuid::new_v4();
        assert_eq!(
            parse_import_uuid(&format!("Connection 'DFKI' ({id}) successfully added.")).unwrap(),
            id
        );
        assert!(parse_import_uuid("Connection failed").is_err());
        assert_eq!(
            parse_import_uuid(&format!(
                "Connection '{}' ({id}) successfully added.",
                Uuid::new_v4()
            ))
            .unwrap(),
            id
        );
    }
    #[test]
    fn only_activated_state_is_connected() {
        assert_eq!(
            parse_status("activated\n10.8.0.6/24"),
            (ConnectionState::Connected, "10.8.0.6".into())
        );
        assert_eq!(
            parse_status("activating\n10.8.0.6/24"),
            (ConnectionState::Connecting, String::new())
        );
        assert_eq!(parse_status(""), (ConnectionState::Idle, String::new()));
    }

    #[test]
    #[ignore = "Creates and removes two disposable NetworkManager profiles; requires nmcli, its OpenVPN plugin, and openssl"]
    fn networkmanager_import_cancel_and_remove() {
        struct Cleanup(Vec<Uuid>);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                for id in &self.0 {
                    let _ = run_nmcli(&["connection", "delete", "uuid", &id.to_string()], 10, None);
                }
            }
        }
        let mut cleanup = Cleanup(Vec::new());
        // NetworkManager has a private /tmp namespace. Use the same filesystem
        // location as real profiles so the VPN service can read the fixture.
        let root = tempfile::Builder::new()
            .prefix("openvpn-gpui-test-")
            .tempdir_in(dirs::data_local_dir().unwrap())
            .unwrap();
        let ca = root.path().join("fixture-ca.pem");
        let key = root.path().join("fixture-key.pem");
        assert!(
            Command::new("openssl")
                .args(["req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout"])
                .arg(&key)
                .args(["-out"])
                .arg(&ca)
                .args(["-days", "1", "-subj", "/CN=Local integration fixture"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success()
        );
        let source = root.path().join("fixture.ovpn");
        // Loopback discard port: this test cannot connect to a real VPN or install routes.
        fs::write(&source, "client\ndev tun\nproto udp\nremote 127.0.0.1 9\nca fixture-ca.pem\nauth-user-pass\nremote-cert-tls server\nconnect-retry-max 1\n").unwrap();
        let data = root.path().join("data");
        let mut store = Profiles::default();
        let (tx, _rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        for name in ["Integration fixture one", "Integration fixture two"] {
            handle_request(
                VpnRequest::Import {
                    path: source.clone(),
                    name: name.into(),
                },
                &data,
                &mut store,
                &tx,
                &cancel,
            )
            .unwrap();
            cleanup.0.push(store.selected().unwrap().nm_uuid);
        }
        assert_eq!(profile::load(&data).unwrap().profiles.len(), 2);
        assert_ne!(store.profiles[0].nm_uuid, store.profiles[1].nm_uuid);
        let first = store.profiles[0].id;
        handle_request(VpnRequest::Select(first), &data, &mut store, &tx, &cancel).unwrap();
        assert_eq!(profile::load(&data).unwrap().selected, Some(first));
        let selected = store.selected().unwrap().nm_uuid.to_string();
        // Avoid a desktop password prompt: use certificate authentication for the cancellation test.
        run_nmcli(
            &[
                "connection",
                "modify",
                "uuid",
                &selected,
                "+vpn.data",
                "connection-type=tls",
            ],
            10,
            None,
        )
        .unwrap();
        run_nmcli(
            &[
                "connection",
                "modify",
                "uuid",
                &selected,
                "+vpn.data",
                &format!("cert={},key={}", ca.display(), key.display()),
            ],
            10,
            None,
        )
        .unwrap();
        let flag = cancel.clone();
        let cancellation = thread::spawn(move || {
            thread::sleep(Duration::from_millis(900));
            flag.store(true, Ordering::Relaxed);
        });
        handle_request(VpnRequest::Connect, &data, &mut store, &tx, &cancel).unwrap();
        cancellation.join().unwrap();
        assert!(cancel.load(Ordering::Relaxed));
        handle_request(VpnRequest::Disconnect, &data, &mut store, &tx, &cancel).unwrap();
        let active = run_nmcli(&["-g", "UUID", "connection", "show", "--active"], 8, None).unwrap();
        assert!(!active.lines().any(|s| s == selected));
        while store.selected().is_some() {
            handle_request(VpnRequest::Remove, &data, &mut store, &tx, &cancel).unwrap();
        }
        assert!(profile::load(&data).unwrap().profiles.is_empty());
        cleanup.0.clear();
    }
}
