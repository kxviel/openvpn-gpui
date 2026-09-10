mod config;
mod credentials;
mod storage;

#[cfg(test)]
mod tests;

pub(crate) use config::{bundle, config_path};
pub(crate) use credentials::{credentials_path, save_password, validate_credentials};
pub(crate) use storage::{data_dir, load, private_dir, save};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
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
