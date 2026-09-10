use super::{
    VpnEvent,
    nmcli::{parse_import_uuid, run_nmcli},
};
use crate::profile::{self, Profile, Profiles};
use anyhow::{Context, Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};
use uuid::Uuid;

pub(super) fn import(
    path: PathBuf,
    name: String,
    credentials: Option<(String, String)>,
    root: &Path,
    store: &mut Profiles,
) -> Result<()> {
    let name = store.validate_name(&name)?;
    let credentials = credentials
        .map(|(username, password)| {
            profile::validate_credentials(&username, &password, false).map(
                |(username, password)| {
                    (
                        username,
                        password.expect("new credentials include a password"),
                    )
                },
            )
        })
        .transpose()?;
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
        let (username, has_password) = if let Some((username, password)) = credentials {
            run_nmcli(
                &[
                    "connection",
                    "modify",
                    "uuid",
                    &uuid,
                    "+vpn.data",
                    &format!("username={username}"),
                ],
                10,
                None,
            )?;
            profile::save_password(root, id, &password)?;
            (username, true)
        } else {
            (String::new(), false)
        };
        let mut next = store.clone();
        next.profiles.push(Profile {
            id,
            nm_uuid,
            name,
            remote,
            protocol,
            username,
            has_password,
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
    Ok(())
}

pub(super) fn select(id: Uuid, root: &Path, store: &mut Profiles) -> Result<()> {
    if !store.profiles.iter().any(|p| p.id == id) {
        bail!("Profile no longer exists.");
    }
    let mut next = store.clone();
    next.selected = Some(id);
    profile::save(root, &next)?;
    *store = next;
    Ok(())
}

pub(super) fn save_credentials(
    id: Uuid,
    username: String,
    password: Option<String>,
    root: &Path,
    store: &mut Profiles,
    tx: &Sender<VpnEvent>,
) -> Result<()> {
    let selected = store
        .profiles
        .iter()
        .find(|profile| profile.id == id)
        .context("This profile no longer exists.")?
        .clone();
    let (username, password) = profile::validate_credentials(
        &username,
        password.as_deref().unwrap_or_default(),
        selected.has_password,
    )?;
    let uuid = selected.nm_uuid.to_string();
    run_nmcli(
        &[
            "connection",
            "modify",
            "uuid",
            &uuid,
            "+vpn.data",
            &format!("username={username}"),
        ],
        10,
        None,
    )?;
    if let Some(password) = password {
        profile::save_password(root, selected.id, &password)?;
    }
    let mut next = store.clone();
    let profile = next
        .profiles
        .iter_mut()
        .find(|profile| profile.id == selected.id)
        .context("Profile no longer exists.")?;
    profile.username = username;
    profile.has_password = true;
    profile::save(root, &next)?;
    *store = next;
    let _ = tx.send(VpnEvent::Notice(
        "Saved login details for this profile.".into(),
    ));
    Ok(())
}

pub(super) fn remove(id: Uuid, root: &Path, store: &mut Profiles) -> Result<()> {
    let profile = store
        .profiles
        .iter()
        .find(|profile| profile.id == id)
        .context("This profile no longer exists.")?
        .clone();
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
    if next.selected == Some(id) {
        next.selected = next.profiles.first().map(|p| p.id);
    }
    profile::save(root, &next)?;
    *store = next;
    fs::remove_dir_all(root.join(profile.id.to_string())).or_else(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Ok(())
        } else {
            Err(e)
        }
    })?;
    Ok(())
}
