use crate::backend::{ConnectionState, VpnBackend, VpnEvent, VpnRequest};
use crate::profile::{self, Profile, Profiles};
use gpui::*;
use gpui_component::input::InputState;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use uuid::Uuid;

const UI_POLL_INTERVAL: Duration = Duration::from_millis(150);

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
        let ticker = cx.spawn_in(window, async move |this, cx| {
            let mut rendered_second = 0;
            loop {
                cx.background_executor().timer(UI_POLL_INTERVAL).await;
                if this
                    .update_in(cx, |this, window, cx| {
                        let mut changed = false;
                        if exit_requested.swap(false, Ordering::Relaxed) {
                            this.request_quit(cx);
                        }
                        while !this.backend_stopped {
                            let event = match this.backend.events.try_recv() {
                                Ok(event) => event,
                                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                    this.backend_stopped = true;
                                    this.ready = false;
                                    this.busy = false;
                                    if this.quitting {
                                        cx.quit();
                                    } else {
                                        this.state = ConnectionState::Unavailable;
                                        this.error.get_or_insert_with(|| "The connection service stopped. Close and reopen the app.".into());
                                    }
                                    cx.notify();
                                    break;
                                }
                            };
                            changed = true;
                            match event {
                                VpnEvent::Profiles(profiles) => {
                                    let imported = this.busy
                                        && profiles.profiles.len() > this.profiles.profiles.len();
                                    let selected_changed =
                                        profiles.selected != this.profiles.selected;
                                    this.profiles = profiles;
                                    #[cfg(debug_assertions)]
                                    if !this.ready && this.page == Screen::ProfileSettings
                                        && let Some(id) = this.profiles.selected
                                    {
                                        this.begin_settings(id, window, cx);
                                    }
                                    if selected_changed {
                                        this.since = None;
                                        this.address.clear();
                                        if !this.quitting {
                                            this.state = ConnectionState::Idle;
                                        }
                                    }
                                    if this.editing.is_some()
                                        && this.editing_profile().is_none()
                                    {
                                        this.editing = None;
                                        this.page = Screen::Profiles;
                                        this.reset_password(window, cx);
                                    }
                                    if imported {
                                        this.page = Screen::Connection;
                                        this.file = None;
                                        this.reset_password(window, cx);
                                    }
                                }
                                VpnEvent::Status { state, address } => {
                                    if state == ConnectionState::Connected {
                                        if this.since.is_none() {
                                            this.since = Some(Instant::now());
                                        }
                                    } else {
                                        this.since = None;
                                    }
                                    this.state = if this.quitting {
                                        ConnectionState::Disconnecting
                                    } else { state };
                                    this.address = address;
                                }
                                VpnEvent::Error(error) => this.error = Some(error),
                                VpnEvent::Notice(notice) => {
                                    this.notice = Some(notice);
                                    this.reset_password(window, cx);
                                }
                                VpnEvent::Busy(busy) => this.busy = busy || this.quitting,
                                VpnEvent::Ready => this.ready = true,
                                VpnEvent::ShutdownComplete => {
                                    cx.quit();
                                    return;
                                }
                                VpnEvent::ShutdownFailed(error) => {
                                    this.quitting = false;
                                    this.busy = false;
                                    this.state = ConnectionState::Unavailable;
                                    this.error = Some(error);
                                }
                            }
                        }
                        let elapsed = this
                            .since
                            .map(|since| since.elapsed().as_secs())
                            .unwrap_or(0);
                        if changed || elapsed != rendered_second {
                            rendered_second = elapsed;
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
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

    pub(crate) fn send(&mut self, request: VpnRequest, cx: &mut Context<Self>) {
        self.error = None;
        self.notice = None;
        self.busy = true;
        if self.backend.requests.send(request).is_err() {
            self.error = Some("The profile service stopped. Close and reopen the app.".into());
            self.busy = false;
        }
        cx.notify();
    }

    pub(crate) fn begin_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() || !self.ready {
            return;
        }
        self.page = Screen::Import;
        self.error = None;
        self.notice = None;
        self.confirm_remove = false;
        self.editing = None;
        self.file = None;
        self.name
            .update(cx, |name, cx| name.set_value("", window, cx));
        self.username
            .update(cx, |username, cx| username.set_value("", window, cx));
        self.reset_password(window, cx);
        self.name.update(cx, |name, cx| name.focus(window, cx));
        cx.notify();
    }

    pub(crate) fn begin_profiles(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.quitting {
            return;
        }
        let username = self
            .profiles
            .selected()
            .map(|profile| profile.username.clone())
            .unwrap_or_default();
        self.page = Screen::Profiles;
        self.editing = None;
        self.error = None;
        self.notice = None;
        self.confirm_remove = false;
        self.username
            .update(cx, |state, cx| state.set_value(username, window, cx));
        self.reset_password(window, cx);
        window.focus(&self.focus);
        cx.notify();
    }

    fn reset_password(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_password = false;
        // A fresh entity also drops the old field's undo history.
        self.password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter password")
                .masked(true)
        });
    }

    pub(crate) fn editing_profile(&self) -> Option<&Profile> {
        self.profiles
            .profiles
            .iter()
            .find(|profile| Some(profile.id) == self.editing)
    }

    pub(crate) fn begin_settings(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.quitting {
            return;
        }
        let Some(profile) = self
            .profiles
            .profiles
            .iter()
            .find(|profile| profile.id == id)
        else {
            return;
        };
        let username = profile.username.clone();
        self.editing = Some(id);
        self.settings_return = if self.page == Screen::Profiles {
            Screen::Profiles
        } else {
            Screen::Connection
        };
        self.page = Screen::ProfileSettings;
        self.error = None;
        self.notice = None;
        self.confirm_remove = false;
        self.username
            .update(cx, |input, cx| input.set_value(username, window, cx));
        self.reset_password(window, cx);
        window.focus(&self.focus);
        cx.notify();
    }

    pub(crate) fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.quitting {
            return;
        }
        self.page = if self.page == Screen::ProfileSettings {
            self.settings_return
        } else {
            Screen::Connection
        };
        self.editing = None;
        self.confirm_remove = false;
        self.error = None;
        self.notice = None;
        self.reset_password(window, cx);
        window.focus(&self.focus);
        cx.notify();
    }

    pub(crate) fn save_login(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        let Some(profile) = self.editing_profile() else {
            return;
        };
        let id = profile.id;
        let credentials = profile::validate_credentials(
            &self.username.read(cx).value(),
            &self.password.read(cx).value(),
            profile.has_password,
        );
        self.notice = None;
        match credentials {
            Ok((username, password)) => {
                window.focus(&self.focus);
                self.send(
                    VpnRequest::SaveCredentials {
                        id,
                        username,
                        password,
                    },
                    cx,
                );
            }
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
            }
        }
    }

    pub(crate) fn import_profile(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() || self.picking || !self.ready {
            return;
        }
        let Some(path) = self.file.clone() else {
            return;
        };
        let name = self.profiles.validate_name(&self.name.read(cx).value());
        let username = self.username.read(cx).value();
        let password = self.password.read(cx).value();
        let credentials = if username.trim().is_empty() && password.is_empty() {
            Ok(None)
        } else {
            profile::validate_credentials(&username, &password, false)
                .map(|(username, password)| password.map(|password| (username, password)))
        };
        match (name, credentials) {
            (Ok(name), Ok(credentials)) => {
                window.focus(&self.focus);
                self.send(
                    VpnRequest::Import {
                        path,
                        name,
                        credentials,
                    },
                    cx,
                );
            }
            (Err(error), _) | (_, Err(error)) => {
                self.error = Some(error.to_string());
                cx.notify();
            }
        }
    }

    pub(crate) fn remove_profile(&mut self, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        if let Some(id) = self.editing {
            self.confirm_remove = false;
            self.send(VpnRequest::Remove(id), cx);
        }
    }

    pub(crate) fn request_quit(&mut self, cx: &mut Context<Self>) {
        if self.quitting {
            return;
        }
        if self.backend_stopped {
            cx.quit();
            return;
        }
        self.quitting = true;
        self.busy = true;
        self.error = None;
        self.notice = None;
        self.state = ConnectionState::Disconnecting;
        self.backend.cancel.store(true, Ordering::Relaxed);
        if self.backend.requests.send(VpnRequest::Shutdown).is_err() {
            // The worker has already stopped; Drop joins its cleanup.
            cx.quit();
        }
        cx.notify();
    }

    pub(crate) fn pick_file(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.picking {
            return;
        }
        self.picking = true;
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose an OpenVPN configuration (.ovpn or .conf)".into()),
        });
        cx.spawn(async move |this, cx| {
            let result = prompt.await;
            let _ = this.update(cx, |this, cx| {
                this.picking = false;
                match result {
                    Ok(Ok(Some(paths))) => { this.file = paths.into_iter().next(); this.error = None; },
                    Ok(Ok(None)) => {},
                    _ => this.error = Some("Could not open the file picker. Check that your desktop portal is running.".into()),
                }
                cx.notify();
            });
        }).detach();
    }

    pub(crate) fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.quitting {
            return;
        }
        if self.state == ConnectionState::Connecting {
            self.backend.cancel.store(true, Ordering::Relaxed);
            self.state = ConnectionState::Disconnecting;
            self.send(VpnRequest::Disconnect, cx);
        } else if self.state == ConnectionState::Connected && !self.busy {
            self.state = ConnectionState::Disconnecting;
            self.send(VpnRequest::Disconnect, cx);
        } else if !self.busy && self.profiles.selected().is_some() && self.ready {
            self.backend.cancel.store(false, Ordering::Relaxed);
            self.state = ConnectionState::Connecting;
            self.send(VpnRequest::Connect, cx);
        }
    }
}
