mod commands;
mod events;
mod navigation;

use crate::backend::{ConnectionState, VpnBackend};
use crate::profile::{Profile, Profiles};
use gpui::*;
use gpui_component::input::InputState;
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
    time::Instant,
};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Screen {
    Connection,
    Profiles,
    Import,
    ProfileSettings,
}

actions!(openvpn, [ImportProfile, ShowProfiles, Back, Quit]);

pub(crate) struct VpnApp {
    pub(crate) backend: VpnBackend,
    pub(crate) profiles: Profiles,
    pub(crate) state: ConnectionState,
    pub(crate) address: String,
    pub(crate) since: Option<Instant>,
    pub(crate) page: Screen,
    pub(crate) name: Entity<InputState>,
    pub(crate) username: Entity<InputState>,
    pub(crate) password: Entity<InputState>,
    pub(crate) file: Option<PathBuf>,
    pub(crate) error: Option<String>,
    pub(crate) notice: Option<String>,
    pub(crate) ready: bool,
    pub(crate) busy: bool,
    pub(crate) picking: bool,
    pub(crate) confirm_remove: bool,
    pub(crate) editing: Option<Uuid>,
    pub(crate) quitting: bool,
    pub(crate) show_password: bool,
    settings_return: Screen,
    backend_stopped: bool,
    pub(crate) focus: FocusHandle,
    _ticker: Task<()>,
}

impl VpnApp {
    pub(crate) fn new(
        root: PathBuf,
        exit_requested: Arc<AtomicBool>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("e.g. DFKI — Saarbrücken"));
        let username = cx.new(|cx| InputState::new(window, cx).placeholder("Username"));
        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Password")
                .masked(true)
        });
        let focus = cx.focus_handle();
        window.focus(&focus);
        let ticker = Self::start_ticker(exit_requested, window, cx);
        let page = Screen::Connection;
        // Development-only screen selection for visual checks without desktop input automation.
        #[cfg(debug_assertions)]
        let page = match std::env::var("OPENVPN_GPUI_SCREEN").as_deref() {
            Ok("import") => Screen::Import,
            Ok("profiles") => Screen::Profiles,
            Ok("settings") => Screen::ProfileSettings,
            _ => page,
        };

        Self {
            backend: VpnBackend::start(root),
            profiles: Profiles::default(),
            state: ConnectionState::Idle,
            address: String::new(),
            since: None,
            page,
            name,
            username,
            password,
            file: None,
            error: None,
            notice: None,
            ready: false,
            busy: false,
            picking: false,
            confirm_remove: false,
            editing: None,
            quitting: false,
            show_password: false,
            settings_return: Screen::Connection,
            backend_stopped: false,
            focus,
            _ticker: ticker,
        }
    }

    pub(crate) fn locked(&self) -> bool {
        self.quitting
            || self.busy
            || matches!(
                self.state,
                ConnectionState::Connected
                    | ConnectionState::Connecting
                    | ConnectionState::Disconnecting
            )
    }

    pub(crate) fn editing_profile(&self) -> Option<&Profile> {
        self.profiles
            .profiles
            .iter()
            .find(|profile| Some(profile.id) == self.editing)
    }
}
