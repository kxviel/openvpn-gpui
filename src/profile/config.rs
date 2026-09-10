use super::private_dir;
use anyhow::{Context, Result, bail};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

pub(crate) fn config_path(destination: &Path) -> PathBuf {
    // NM uses the config basename for extracted inline certificates. A unique
    // name keeps separate profiles from overwriting one another's credentials.
    destination.join(format!(
        "{}.ovpn",
        destination
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    ))
}

/// Bundle external certificate/key references before handing the file to NM.
/// No directives are executed here; NetworkManager's importer validates the config.
pub(crate) fn bundle(source: &Path, destination: &Path) -> Result<(String, String)> {
    let source = source
        .canonicalize()
        .context("The selected configuration no longer exists")?;
    if fs::metadata(&source)?.len() > 8 * 1024 * 1024 {
        bail!("The configuration is too large (maximum 8 MB).");
    }
    let content =
        fs::read_to_string(&source).context("The configuration must be a UTF-8 text file")?;
    private_dir(destination)?;
    let mut base = source
        .parent()
        .context("Configuration has no parent directory")?
        .to_path_buf();
    let mut output = String::new();
    let mut inline: Option<String> = None;
    let mut remote = String::new();
    let mut protocol = "UDP".to_string();
    let mut port = "1194".to_string();
    let mut remote_port = None;
    let mut remote_protocol = None;
    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(tag) = &inline {
            output.push_str(line);
            output.push('\n');
            if trimmed == format!("</{tag}>") {
                inline = None;
            }
            continue;
        }
        if trimmed.starts_with('<')
            && trimmed.ends_with('>')
            && trimmed != "<connection>"
            && trimmed != "</connection>"
        {
            inline = Some(trimmed[1..trimmed.len() - 1].to_owned());
            output.push_str(line);
            output.push('\n');
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            output.push_str(line);
            output.push('\n');
            continue;
        }
        let mut args = shell_words::split(trimmed)
            .with_context(|| format!("Invalid quoting on line {}", index + 1))?;
        let Some(key) = args.first().cloned() else {
            continue;
        };
        let key = key.trim_start_matches("--");
        match key {
            "remote" if remote.is_empty() => {
                remote = args.get(1).cloned().unwrap_or_default();
                remote_port = args.get(2).filter(|v| v.parse::<u16>().is_ok()).cloned();
                remote_protocol = args.get(3).cloned();
            }
            "proto" => protocol = args.get(1).cloned().unwrap_or(protocol),
            "port" | "rport" => port = args.get(1).cloned().unwrap_or(port),
            "cd" => {
                let dir = args.get(1).context("Missing directory after cd")?;
                base = base
                    .join(dir)
                    .canonicalize()
                    .context("Cannot find the configuration's working directory")?;
                continue;
            }
            // Explicitly refuse executable or nested configurations instead of silently changing them.
            "up" | "down" | "route-up" | "route-pre-down" | "ipchange" | "tls-verify"
            | "plugin" | "config" => {
                bail!(
                    "This configuration uses '{key}', which this minimal NetworkManager app does not support."
                );
            }
            _ => {}
        }
        if [
            "ca",
            "cert",
            "key",
            "pkcs12",
            "tls-auth",
            "tls-crypt",
            "tls-crypt-v2",
            "crl-verify",
            "extra-certs",
            "auth-user-pass",
            "askpass",
        ]
        .contains(&key)
            && args
                .get(1)
                .is_some_and(|a| a != "[inline]" && !a.starts_with('#') && !a.starts_with(';'))
        {
            let referenced = base.join(&args[1]);
            let metadata = fs::metadata(&referenced).with_context(|| {
                format!(
                    "Cannot find a file referenced by '{key}' on line {}",
                    index + 1
                )
            })?;
            if !metadata.is_file() || metadata.len() > 8 * 1024 * 1024 {
                bail!("The file referenced by '{key}' is not a supported file (maximum 8 MB).");
            }
            let target = destination.join(format!("{index}-{key}"));
            fs::copy(referenced, &target)?;
            fs::set_permissions(&target, fs::Permissions::from_mode(0o600))?;
            // OpenVPN understands double quotes, not shell single-quote concatenation.
            args[1] = format!(
                "\"{}\"",
                target
                    .to_string_lossy()
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
            );
            output.push_str(&args.join(" "));
        } else {
            output.push_str(line);
        }
        output.push('\n');
    }
    if inline.is_some() {
        bail!("The configuration contains an unclosed certificate or key block.");
    }
    if remote.is_empty() {
        bail!(
            "No VPN server found. Choose an OpenVPN client configuration containing a 'remote' line."
        );
    }
    let target = config_path(destination);
    fs::write(&target, output)?;
    fs::set_permissions(target, fs::Permissions::from_mode(0o600))?;
    let protocol = remote_protocol.unwrap_or(protocol).to_uppercase();
    Ok((
        remote,
        format!(
            "{} · {}",
            if protocol.starts_with("TCP") {
                "TCP"
            } else {
                "UDP"
            },
            remote_port.unwrap_or(port)
        ),
    ))
}
