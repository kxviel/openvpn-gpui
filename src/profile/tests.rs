use super::*;
use std::{fs, os::unix::fs::PermissionsExt};

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
