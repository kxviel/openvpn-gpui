use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Profile {
    pub(crate) id: Uuid,
    pub(crate) nm_uuid: Uuid,
    pub(crate) name: String,
    pub(crate) remote: String,
    pub(crate) protocol: String,
    #[serde(default)]
    pub(crate) username: String,
    #[serde(default)]
    pub(crate) has_password: bool,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct Profiles {
    pub(crate) profiles: Vec<Profile>,
    pub(crate) selected: Option<Uuid>,
}

impl Profiles {
    pub(crate) fn selected(&self) -> Option<&Profile> {
        self.profiles.iter().find(|p| Some(p.id) == self.selected)
    }

    pub(crate) fn validate_name(&self, name: &str) -> Result<String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 60 || name.chars().any(char::is_control) {
            bail!("Choose a profile name between 1 and 60 characters.");
        }
        if self
            .profiles
            .iter()
            .any(|p| p.name.to_lowercase() == name.to_lowercase())
        {
            bail!("That profile name is already in use.");
        }
        Ok(name.to_owned())
    }
}

pub(crate) fn data_dir() -> Result<PathBuf> {
    Ok(dirs::data_local_dir()
        .context("Cannot locate your data directory")?
        .join("openvpn-gpui"))
}

pub(crate) fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

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

pub(crate) fn credentials_path(root: &Path, id: Uuid) -> PathBuf {
    root.join(id.to_string()).join("credentials")
}

pub(crate) fn validate_credentials(
    username: &str,
    password: &str,
    has_saved_password: bool,
) -> Result<(String, Option<String>)> {
    let username = username.trim();
    if username.is_empty()
        || username.chars().count() > 255
        || username
            .chars()
            .any(|character| character.is_control() || character == ',')
    {
        bail!("Enter a username between 1 and 255 characters without commas.");
    }
    if password.is_empty() {
        if has_saved_password {
            return Ok((username.to_owned(), None));
        }
        bail!("Enter a password to save with this profile.");
    }
    if password.chars().count() > 4096 || password.chars().any(char::is_control) {
        bail!("The password is too long or contains unsupported control characters.");
    }
    Ok((username.to_owned(), Some(password.to_owned())))
}

pub(crate) fn save_password(root: &Path, id: Uuid, password: &str) -> Result<()> {
    let directory = root.join(id.to_string());
    private_dir(&directory)?;
    let mut file = tempfile::NamedTempFile::new_in(&directory)?;
    file.as_file_mut()
        .set_permissions(fs::Permissions::from_mode(0o600))?;
    // nmcli's passwd-file format keeps the secret out of process arguments.
    writeln!(file, "vpn.secrets.password:{password}")?;
    file.as_file().sync_all()?;
    file.persist(credentials_path(root, id))?;
    fs::File::open(directory)?.sync_all()?;
    Ok(())
}

pub(crate) fn load(root: &Path) -> Result<Profiles> {
    let path = root.join("profiles.json");
    if !path.exists() {
        return Ok(Profiles::default());
    }
    let mut profiles: Profiles = serde_json::from_slice(&fs::read(&path)?)
        .context("Cannot read saved profiles. The existing file has been preserved.")?;
    for profile in &mut profiles.profiles {
        profile.has_password = credentials_path(root, profile.id).is_file();
    }
    if profiles.selected().is_none() {
        profiles.selected = profiles.profiles.first().map(|p| p.id);
    }
    Ok(profiles)
}

pub(crate) fn save(root: &Path, profiles: &Profiles) -> Result<()> {
    private_dir(root)?;
    let mut file = tempfile::NamedTempFile::new_in(root)?;
    file.write_all(&serde_json::to_vec_pretty(profiles)?)?;
    file.as_file().sync_all()?;
    file.persist(root.join("profiles.json"))?;
    fs::File::open(root)?.sync_all()?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundles_relative_files_and_preserves_inline_blocks() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("client key.pem"), "private key fixture").unwrap();
        fs::write(tmp.path().join("test.ovpn"), "client\nremote vpn.example.test 443 tcp-client\nkey \"client key.pem\"\n<ca>\ncertificate\n</ca>\n").unwrap();
        let dest = tmp.path().join("saved profile");
        let (remote, proto) = bundle(&tmp.path().join("test.ovpn"), &dest).unwrap();
        assert_eq!(remote, "vpn.example.test");
        assert_eq!(proto, "TCP · 443");
        fs::remove_file(tmp.path().join("client key.pem")).unwrap();
        assert_eq!(
            fs::read_to_string(dest.join("2-key")).unwrap(),
            "private key fixture"
        );
        assert_eq!(
            fs::metadata(dest.join("2-key"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert!(
            fs::read_to_string(config_path(&dest))
                .unwrap()
                .contains("<ca>\ncertificate\n</ca>")
        );
    }
    #[test]
    fn rejects_executable_configs_and_missing_assets() {
        let tmp = tempfile::tempdir().unwrap();
        for config in [
            "remote host\nup /tmp/script",
            "remote host\nkey missing.pem",
            "remote host\n<ca>\nunfinished",
            "garbage",
        ] {
            fs::write(tmp.path().join("test.ovpn"), config).unwrap();
            assert!(bundle(&tmp.path().join("test.ovpn"), &tmp.path().join("out")).is_err());
        }
    }
    #[test]
    fn store_roundtrip_and_corruption_preservation() {
        let tmp = tempfile::tempdir().unwrap();
        let id = Uuid::new_v4();
        let store = Profiles {
            profiles: vec![Profile {
                id,
                nm_uuid: Uuid::new_v4(),
                name: "DFKI".into(),
                remote: "vpn.example.test".into(),
                protocol: "UDP · 1194".into(),
                username: String::new(),
                has_password: false,
            }],
            selected: Some(id),
        };
        save(tmp.path(), &store).unwrap();
        let loaded = load(tmp.path()).unwrap();
        assert_eq!(loaded.selected().unwrap().name, "DFKI");
        assert!(loaded.validate_name(" dfki ").is_err());
        assert!(loaded.validate_name(" ").is_err());
        fs::write(tmp.path().join("profiles.json"), "broken").unwrap();
        assert!(load(tmp.path()).is_err());
        assert_eq!(
            fs::read_to_string(tmp.path().join("profiles.json")).unwrap(),
            "broken"
        );
    }

    #[test]
    fn credentials_are_validated_and_saved_privately() {
        let tmp = tempfile::tempdir().unwrap();
        let id = Uuid::new_v4();
        let (username, password) =
            validate_credentials("  sam.samples  ", "secret phrase", false).unwrap();
        assert_eq!(username, "sam.samples");
        save_password(tmp.path(), id, password.as_deref().unwrap()).unwrap();
        let path = credentials_path(tmp.path(), id);
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "vpn.secrets.password:secret phrase\n"
        );
        let profiles = Profiles {
            profiles: vec![Profile {
                id,
                nm_uuid: Uuid::new_v4(),
                name: "Office".into(),
                remote: "vpn.example.test".into(),
                protocol: "UDP · 1194".into(),
                username: username.clone(),
                has_password: false,
            }],
            selected: Some(id),
        };
        save(tmp.path(), &profiles).unwrap();
        assert!(load(tmp.path()).unwrap().selected().unwrap().has_password);
        assert!(validate_credentials("sam", "", false).is_err());
        assert_eq!(
            validate_credentials("sam", "", true).unwrap(),
            ("sam".into(), None)
        );
        assert!(validate_credentials("samples,sam", "secret", false).is_err());
        assert!(validate_credentials("sam", "line\nbreak", false).is_err());
    }
}
