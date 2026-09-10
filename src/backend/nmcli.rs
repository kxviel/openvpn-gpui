use super::ConnectionState;
use anyhow::{Context, Result, bail};
use std::{
    io::{Read, Seek},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};
use uuid::Uuid;

/// Bound every subprocess and keep pipes off the UI thread. Secrets are never
/// passed on argv; nmcli reads a private password file or asks the desktop agent.
pub(super) fn run_nmcli(args: &[&str], limit: u64, cancel: Option<&AtomicBool>) -> Result<String> {
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

pub(super) fn parse_import_uuid(output: &str) -> Result<Uuid> {
    output
        .rsplit_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .and_then(|(id, _)| Uuid::parse_str(id).ok())
        .context("NetworkManager did not return an imported connection ID")
}

pub(super) fn parse_status(output: &str) -> (ConnectionState, String) {
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
