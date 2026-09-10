use super::{ConnectionState, VpnEvent, nmcli::run_nmcli};
use crate::profile::{self, Profiles};
use anyhow::{Context, Result, bail};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    thread,
    time::{Duration, Instant},
};
use uuid::Uuid;

pub(super) fn active_uuids() -> Result<Vec<Uuid>> {
    let output = run_nmcli(&["-g", "UUID", "connection", "show", "--active"], 8, None)?;
    Ok(output
        .lines()
        .filter_map(|line| Uuid::parse_str(line).ok())
        .collect())
}

fn disconnect_uuid(uuid: Uuid) -> Result<()> {
    disconnect_uuid_with(uuid, |args, limit| run_nmcli(args, limit, None))
}

pub(super) fn disconnect_uuid_with(
    uuid: Uuid,
    mut run: impl FnMut(&[&str], u64) -> Result<String>,
) -> Result<()> {
    let uuid = uuid.to_string();
    let query = ["-g", "UUID", "connection", "show", "--active"];
    if !run(&query, 8)?.lines().any(|line| line == uuid) {
        return Ok(());
    }
    // Deactivating the VPN connection lets NM remove its tunnel, routes and
    // DNS contributions. The physical Wi-Fi/Ethernet connection stays active.
    let result = run(&["--wait", "15", "connection", "down", "uuid", &uuid], 20);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if !run(&query, 8)?.lines().any(|line| line == uuid) {
            return Ok(()); // Also handles a concurrent desktop disconnect.
        }
        if result.is_err() || Instant::now() >= deadline {
            result.context("NetworkManager could not disconnect the VPN")?;
            bail!("The VPN is still active. Try Disconnect again.");
        }
        thread::sleep(Duration::from_millis(200));
    }
}

pub(super) fn disconnect_all(store: &Profiles) -> Result<()> {
    if store.profiles.is_empty() {
        return Ok(());
    }
    let active = active_uuids()?;
    let mut failures = Vec::new();
    for profile in &store.profiles {
        if active.contains(&profile.nm_uuid)
            && let Err(error) = disconnect_uuid(profile.nm_uuid)
        {
            failures.push(format!("{}: {error:#}", profile.name));
        }
    }
    // Another desktop client may activate a managed profile during cleanup.
    // Never report completion if any owned UUID is still active.
    let remaining = active_uuids()?;
    for profile in &store.profiles {
        if remaining.contains(&profile.nm_uuid) {
            failures.push(format!(
                "{} is still active. Try disconnecting again.",
                profile.name
            ));
        }
    }
    if !failures.is_empty() {
        bail!("{}", failures.join("\n"));
    }
    Ok(())
}

pub(super) fn connect(
    root: &Path,
    store: &Profiles,
    tx: &Sender<VpnEvent>,
    cancel: &AtomicBool,
) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        return Ok(());
    }
    let profile = store.selected().context("Import a profile first.")?;
    let uuid = profile.nm_uuid.to_string();
    let _ = tx.send(VpnEvent::Status {
        state: ConnectionState::Connecting,
        address: String::new(),
    });
    let credentials = profile::credentials_path(root, profile.id);
    let result = if profile.has_password && credentials.is_file() {
        run_nmcli(
            &[
                "--wait",
                "90",
                "connection",
                "up",
                "uuid",
                &uuid,
                "passwd-file",
                credentials
                    .to_str()
                    .context("Invalid saved credentials path")?,
            ],
            95,
            Some(cancel),
        )
    } else {
        run_nmcli(
            &["--wait", "90", "connection", "up", "uuid", &uuid],
            95,
            Some(cancel),
        )
    };
    if result.is_err() {
        // Killing nmcli doesn't stop NM activation; explicitly tear down this exact profile.
        disconnect_uuid(profile.nm_uuid)
            .context("The connection failed and VPN cleanup needs another attempt")?;
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
    }
    result.context("Connection failed. If credentials are required, check your desktop's VPN authentication prompt.")?;
    Ok(())
}

pub(super) fn disconnect(store: &Profiles, tx: &Sender<VpnEvent>) -> Result<()> {
    let _ = tx.send(VpnEvent::Status {
        state: ConnectionState::Disconnecting,
        address: String::new(),
    });
    disconnect_all(store)?;
    let _ = tx.send(VpnEvent::Status {
        state: ConnectionState::Idle,
        address: String::new(),
    });
    let _ = tx.send(VpnEvent::Notice(
        "VPN disconnected. Your regular network connection is in use.".into(),
    ));
    Ok(())
}
