mod connection;
mod nmcli;
mod profiles;
mod worker;

#[cfg(test)]
mod tests;

pub(crate) use worker::VpnBackend;

use crate::profile::Profiles;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum ConnectionState {
    #[default]
    Idle,
    Connecting,
    Connected,
    Disconnecting,
    Unavailable,
}

pub(crate) enum VpnRequest {
    Import {
        path: PathBuf,
        name: String,
        credentials: Option<(String, String)>,
    },
    Select(Uuid),
    SaveCredentials {
        id: Uuid,
        username: String,
        password: Option<String>,
    },
    Connect,
    Disconnect,
    Remove(Uuid),
    Shutdown,
    Stop,
}

pub(crate) enum VpnEvent {
    Profiles(Profiles),
    Status {
        state: ConnectionState,
        address: String,
    },
    Error(String),
    Notice(String),
    Busy(bool),
    Ready,
    ShutdownComplete,
    ShutdownFailed(String),
}
