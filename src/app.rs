use crate::backend::{ConnectionState, VpnBackend, VpnEvent, VpnRequest};
use crate::profile::Profiles;
use gpui::*;
use gpui_component::input::InputState;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

const UI_POLL_INTERVAL: Duration = Duration::from_millis(150);

#[derive(PartialEq)]
pub(crate) enum Screen {
    Connection,
    Profiles,
    Import,
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
    pub(crate) file: Option<PathBuf>,
    pub(crate) error: Option<String>,
    pub(crate) ready: bool,
    pub(crate) busy: bool,
    pub(crate) picking: bool,
    pub(crate) confirm_remove: bool,
    pub(crate) focus: FocusHandle,
    _ticker: Task<()>,
}

impl VpnApp {
    pub(crate) fn new(root: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("e.g. DFKI — Saarbrücken"));
        let focus = cx.focus_handle();
        window.focus(&focus);
        let ticker = cx.spawn(async |this, cx| {
            let mut rendered_second = 0;
            loop {
                cx.background_executor().timer(UI_POLL_INTERVAL).await;
                if this
                    .update(cx, |this, cx| {
                        let mut changed = false;
                        while let Ok(event) = this.backend.events.try_recv() {
                            changed = true;
                            match event {
                                VpnEvent::Profiles(profiles) => {
                                    let imported = this.busy
                                        && profiles.profiles.len() > this.profiles.profiles.len();
                                    let selected_changed =
                                        profiles.selected != this.profiles.selected;
                                    this.profiles = profiles;
                                    if selected_changed {
                                        this.since = None;
                                        this.address.clear();
                                        this.state = ConnectionState::Idle;
                                    }
                                    if imported {
                                        this.page = Screen::Connection;
                                        this.file = None;
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
                                    this.state = state;
                                    this.address = address;
                                }
                                VpnEvent::Error(error) => this.error = Some(error),
                                VpnEvent::Busy(busy) => this.busy = busy,
                                VpnEvent::Ready => this.ready = true,
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
            file: None,
            error: None,
            ready: false,
            busy: false,
            picking: false,
            confirm_remove: false,
            focus,
            _ticker: ticker,
        }
    }

    pub(crate) fn locked(&self) -> bool {
        self.busy
            || matches!(
                self.state,
                ConnectionState::Connected
                    | ConnectionState::Connecting
                    | ConnectionState::Disconnecting
            )
    }

    pub(crate) fn send(&mut self, request: VpnRequest, cx: &mut Context<Self>) {
        self.error = None;
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
        self.confirm_remove = false;
        self.name.update(cx, |name, cx| name.focus(window, cx));
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
