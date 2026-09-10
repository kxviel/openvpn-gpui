use super::{Profiles, credentials_path};
use anyhow::{Context, Result};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

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
