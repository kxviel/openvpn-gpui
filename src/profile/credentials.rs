use super::private_dir;
use anyhow::{Result, bail};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use uuid::Uuid;

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
