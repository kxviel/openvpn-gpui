//! Deterministic disconnect tests. Every command is intercepted by the closure;
//! these tests never invoke nmcli or change the host's network configuration.

use super::super::connection::disconnect_uuid_with;
use anyhow::{Result, anyhow};
use std::collections::VecDeque;
use uuid::Uuid;

const VPN: &str = "11111111-1111-4111-8111-111111111111";
const WIFI: &str = "22222222-2222-4222-8222-222222222222";
const OTHER_VPN: &str = "33333333-3333-4333-8333-333333333333";

fn query() -> Vec<String> {
    ["-g", "UUID", "connection", "show", "--active"]
        .map(str::to_owned)
        .to_vec()
}

fn down() -> Vec<String> {
    ["--wait", "15", "connection", "down", "uuid", VPN]
        .map(str::to_owned)
        .to_vec()
}

fn disconnect_with_replies(
    replies: impl IntoIterator<Item = Result<String>>,
) -> (Result<()>, Vec<Vec<String>>) {
    let mut replies: VecDeque<_> = replies.into_iter().collect();
    let mut commands = Vec::new();
    let result = disconnect_uuid_with(Uuid::parse_str(VPN).unwrap(), |args, limit| {
        assert!(limit > 0 && limit <= 20, "each command must be bounded");
        commands.push(args.iter().map(|arg| (*arg).to_owned()).collect());
        replies.pop_front().expect("unexpected extra nmcli command")
    });
    assert!(
        replies.is_empty(),
        "disconnect returned before verification"
    );
    (result, commands)
}

#[test]
fn already_inactive_disconnect_only_queries_active_connections() {
    for active in [String::new(), format!("{WIFI}\n{OTHER_VPN}\n")] {
        let (result, commands) = disconnect_with_replies([Ok(active)]);
        result.unwrap();
        assert_eq!(commands, [query()]);
    }
}

#[test]
fn disconnect_targets_exact_vpn_and_is_idempotent() {
    let remaining = format!("{WIFI}\n{OTHER_VPN}\n");
    let (result, commands) = disconnect_with_replies([
        Ok(format!("{WIFI}\n{VPN}\n{OTHER_VPN}\n")),
        Ok("Connection successfully deactivated".into()),
        Ok(remaining.clone()),
    ]);
    result.unwrap();
    // The only mutation targets this VPN's exact UUID. Wi-Fi and another VPN
    // remain in the active listing and must never receive a down/delete call.
    assert_eq!(commands, [query(), down(), query()]);

    let (result, commands) = disconnect_with_replies([Ok(remaining)]);
    result.unwrap();
    assert_eq!(commands, [query()]);
}

#[test]
fn active_listing_requires_a_complete_uuid_line() {
    let (result, commands) =
        disconnect_with_replies([Ok(format!("{WIFI}\n{VPN}0\nprefix-{VPN}\n"))]);
    result.unwrap();
    assert_eq!(commands, [query()]);
}

#[test]
fn successful_down_waits_until_the_vpn_disappears() {
    let still_active = format!("{VPN}\n{WIFI}");
    let (result, commands) = disconnect_with_replies([
        Ok(still_active.clone()),
        Ok(String::new()),
        Ok(still_active),
        Ok(WIFI.into()),
    ]);
    result.unwrap();
    // A successful command alone is insufficient; deactivation can lag behind.
    assert_eq!(commands, [query(), down(), query(), query()]);
}

#[test]
fn failed_down_with_vpn_still_active_returns_the_failure() {
    let active = format!("{VPN}\n{WIFI}");
    let (result, commands) = disconnect_with_replies([
        Ok(active.clone()),
        Err(anyhow!("permission denied for test VPN")),
        Ok(active),
    ]);
    let error = format!("{:#}", result.unwrap_err());
    assert!(error.contains("permission denied for test VPN"), "{error}");
    assert_eq!(commands, [query(), down(), query()]);
}

#[test]
fn concurrent_deactivation_is_success_even_if_down_reports_an_error() {
    let (result, commands) = disconnect_with_replies([
        Ok(format!("{VPN}\n{WIFI}")),
        Err(anyhow!("connection is not active")),
        Ok(WIFI.into()),
    ]);
    result.unwrap();
    assert_eq!(commands, [query(), down(), query()]);
}

#[test]
fn failed_initial_active_query_never_attempts_a_mutation() {
    let (result, commands) =
        disconnect_with_replies([Err(anyhow!("NetworkManager is unavailable"))]);
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("NetworkManager is unavailable")
    );
    assert_eq!(commands, [query()]);
}

#[test]
fn failed_verification_is_not_reported_as_success() {
    let (result, commands) = disconnect_with_replies([
        Ok(format!("{VPN}\n{WIFI}")),
        Ok(String::new()),
        Err(anyhow!("active connection query timed out")),
    ]);
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("active connection query timed out")
    );
    assert_eq!(commands, [query(), down(), query()]);
}
