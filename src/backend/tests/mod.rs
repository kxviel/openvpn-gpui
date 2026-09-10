mod lifecycle;
mod networkmanager;

use super::{
    ConnectionState,
    nmcli::{parse_import_uuid, parse_status},
};
use uuid::Uuid;

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
