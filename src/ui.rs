use crate::app::{Back, ImportProfile, Quit, Screen, ShowProfiles, VpnApp};
use crate::backend::{ConnectionState, VpnRequest};
use crate::profile::Profile;
use gpui::{prelude::*, *};
use gpui_component::{
    Disableable, Icon, Sizable,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
};

const BG: u32 = 0x0d1117;
const PANEL: u32 = 0x161d27;
const INK: u32 = 0xf1f4f8;
const MUTED: u32 = 0x9ba8ba;
const LINE: u32 = 0x293443;
const BLUE: u32 = 0x739bff;
const BLUE_BG: u32 = 0x182b4d;
const GREEN: u32 = 0x63d6ad;
const GREEN_BG: u32 = 0x142c29;
const RED: u32 = 0xffb1ae;
const RED_BG: u32 = 0x302024;
const PAD: f32 = 24.;

fn row() -> Div {
    div().flex().items_center()
}
fn column() -> Div {
    div().flex().flex_col()
}
fn icon(path: &'static str) -> Icon {
    Icon::default().path(path)
}
fn glyph(path: &'static str, color: u32, size: f32) -> Svg {
    svg()
        .path(path)
        .size(px(size))
        .flex_shrink_0()
        .text_color(rgb(color))
}
fn text(value: impl Into<SharedString>, size: f32) -> Div {
    div()
        .text_size(px(size))
        .line_height(px(size * 1.5))
        .child(value.into())
}
fn muted(value: impl Into<SharedString>) -> Div {
    text(value, 12.).text_color(rgb(MUTED))
}
fn section(title: &'static str, description: &'static str) -> Div {
    column()
        .gap(px(5.))
        .child(text(title, 25.).font_weight(FontWeight::SEMIBOLD))
        .child(muted(description))
}
fn panel() -> Div {
    column()
        .p(px(20.))
        .rounded(px(18.))
        .bg(rgb(PANEL))
        .border_1()
        .border_color(rgb(LINE))
}
fn secondary(id: impl Into<ElementId>, label: &'static str) -> Button {
    Button::new(id)
        .label(label)
        .small()
        .h(px(38.))
        .px(px(12.))
        .rounded(px(9.))
}
fn primary(id: &'static str, label: &'static str) -> Button {
    Button::new(id)
        .label(label)
        .primary()
        .w_full()
        .h(px(48.))
        .rounded(px(11.))
}
fn badge(label: &'static str, color: u32, background: u32) -> Div {
    row()
        .gap(px(6.))
        .px(px(10.))
        .py(px(5.))
        .rounded_full()
        .bg(rgb(background))
        .child(div().size(px(5.)).rounded_full().bg(rgb(color)))
        .child(
            text(label, 11.)
                .text_color(rgb(color))
                .font_weight(FontWeight::MEDIUM),
        )
}
fn field(label: &'static str, state: &Entity<InputState>, disabled: bool) -> Div {
    column()
        .gap(px(7.))
        .child(text(label, 13.).font_weight(FontWeight::MEDIUM))
        .child(
            Input::new(state)
                .large()
                .min_h(px(44.))
                .flex_shrink_0()
                .rounded(px(9.))
                .text_size(px(14.))
                .disabled(disabled),
        )
}
fn identity(profile: &Profile, active: bool) -> Div {
    row()
        .gap(px(13.))
        .child(
            row()
                .size(px(48.))
                .justify_center()
                .rounded(px(13.))
                .bg(rgb(if active { GREEN_BG } else { BLUE_BG }))
                .child(glyph("shield.svg", if active { GREEN } else { BLUE }, 25.)),
        )
        .child(
            column()
                .flex_1()
                .min_w_0()
                .child(
                    text(profile.name.clone(), 17.)
                        .truncate()
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(muted(profile.remote.clone()).truncate()),
        )
}
fn detail(label: &'static str, value: impl Into<SharedString>) -> Div {
    row()
        .justify_between()
        .gap(px(16.))
        .child(muted(label))
        .child(
            text(value, 12.)
                .min_w_0()
                .truncate()
                .font_weight(FontWeight::MEDIUM),
        )
}

impl VpnApp {
    fn connection(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(profile) = self.profiles.selected() else {
            return column()
                .pt(px(34.))
                .gap(px(28.))
                .child(
                    column()
                        .items_center()
                        .gap(px(14.))
                        .child(
                            row()
                                .size(px(72.))
                                .justify_center()
                                .rounded(px(21.))
                                .bg(rgb(BLUE_BG))
                                .child(glyph("shield.svg", BLUE, 36.)),
                        )
                        .child(
                            text("Your network. One connection.", 23.)
                                .text_center()
                                .font_weight(FontWeight::SEMIBOLD),
                        )
                        .child(
                            muted(
                                "Add the VPN configuration from your administrator to get started.",
                            )
                            .text_center(),
                        ),
                )
                .child(
                    panel()
                        .items_center()
                        .gap(px(12.))
                        .child(
                            text("Have an OpenVPN profile?", 15.).font_weight(FontWeight::MEDIUM),
                        )
                        .child(muted("Import a .ovpn or .conf file."))
                        .child(
                            primary("first-profile", "Add profile")
                                .icon(icon("plus.svg"))
                                .disabled(!self.ready || self.quitting)
                                .on_click(
                                    cx.listener(|this, _, window, cx| {
                                        this.begin_import(window, cx)
                                    }),
                                ),
                        ),
                )
                .into_any_element();
        };
        let active = self.state == ConnectionState::Connected;
        let connecting = self.state == ConnectionState::Connecting;
        let (label, title, subtitle, color, background) = match self.state {
            ConnectionState::Connected => (
                "Connected",
                "You're connected",
                "Your VPN connection is active.",
                GREEN,
                GREEN_BG,
            ),
            ConnectionState::Connecting => (
                "Connecting",
                "Opening your connection",
                "Complete any sign-in prompt from your desktop.",
                BLUE,
                BLUE_BG,
            ),
            ConnectionState::Disconnecting => (
                "Disconnecting",
                "Closing your connection",
                "Restoring your regular network connection.",
                BLUE,
                BLUE_BG,
            ),
            ConnectionState::Unavailable => (
                "Needs attention",
                "Connection unavailable",
                "Check the message below and try again.",
                RED,
                RED_BG,
            ),
            ConnectionState::Idle => (
                "Not connected",
                "Ready when you are",
                "Connect to your work network in one click.",
                MUTED,
                PANEL,
            ),
        };
        let elapsed = self
            .since
            .map(|since| since.elapsed().as_secs())
            .unwrap_or(0);
        let duration = format!(
            "{:02}:{:02}:{:02}",
            elapsed / 3600,
            elapsed / 60 % 60,
            elapsed % 60
        );
        let action = match self.state {
            ConnectionState::Connected => "Disconnect",
            ConnectionState::Connecting => "Cancel connection",
            ConnectionState::Disconnecting => "Disconnecting…",
            _ => "Connect",
        };
        let id = profile.id;
        column()
            .gap(px(24.))
            .child(
                column()
                    .items_center()
                    .gap(px(8.))
                    .pt(px(18.))
                    .pb(px(2.))
                    .child(badge(label, color, background))
                    .child(
                        text(title, 26.)
                            .text_center()
                            .font_weight(FontWeight::SEMIBOLD)
                            .mt(px(5.)),
                    )
                    .child(muted(subtitle).text_center())
                    .when(active, |view| {
                        view.child(
                            text(duration, 14.)
                                .font_family("DejaVu Sans Mono")
                                .text_color(rgb(MUTED)),
                        )
                    }),
            )
            .child(
                panel()
                    .gap(px(22.))
                    .when(active, |view| view.border_color(rgb(0x306657)))
                    .child(identity(profile, active))
                    .child(
                        primary("connect", action)
                            .icon(icon("power.svg"))
                            .when(active, |button| {
                                button.bg(rgb(0x2b3848)).text_color(rgb(INK))
                            })
                            .disabled(
                                !self.ready
                                    || self.quitting
                                    || self.state == ConnectionState::Disconnecting
                                    || (self.busy && !connecting),
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.toggle(cx))),
                    )
                    .child(
                        column()
                            .gap(px(12.))
                            .pt(px(17.))
                            .border_t_1()
                            .border_color(rgb(LINE))
                            .child(detail("Protocol", profile.protocol.clone()))
                            .child(detail(
                                "VPN address",
                                if self.address.is_empty() {
                                    "—".into()
                                } else {
                                    self.address.clone()
                                },
                            ))
                            .child(detail(
                                "Sign-in",
                                if profile.has_password {
                                    "Login saved"
                                } else {
                                    "Ask when connecting"
                                },
                            )),
                    )
                    .child(
                        secondary("card-settings", "Profile settings")
                            .icon(icon("settings.svg"))
                            .w_full()
                            .disabled(self.busy || self.quitting)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.begin_settings(id, window, cx)
                            })),
                    ),
            )
            .child(
                row()
                    .justify_between()
                    .gap(px(10.))
                    .child(muted(format!(
                        "{} saved {}",
                        self.profiles.profiles.len(),
                        if self.profiles.profiles.len() == 1 {
                            "profile"
                        } else {
                            "profiles"
                        }
                    )))
                    .child(
                        secondary("switch-profile", "Switch profile")
                            .ghost()
                            .disabled(self.busy || self.quitting)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.begin_profiles(window, cx)),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn profiles_page(&self, cx: &mut Context<Self>) -> AnyElement {
        column()
            .gap(px(22.))
            .child(section(
                "Your profiles",
                "Choose a connection or manage its saved login.",
            ))
            .when(self.profiles.profiles.is_empty(), |view| {
                view.child(
                    panel()
                        .gap(px(10.))
                        .child(glyph("file.svg", BLUE, 28.))
                        .child(text("No profiles yet", 16.).font_weight(FontWeight::MEDIUM))
                        .child(muted(
                            "Add a configuration file to create your first connection.",
                        )),
                )
            })
            .children(self.profiles.profiles.iter().map(|profile| {
                let id = profile.id;
                let selected = Some(id) == self.profiles.selected;
                panel()
                    .gap(px(16.))
                    .when(selected, |view| view.border_color(rgb(0x426398)))
                    .child(identity(
                        profile,
                        selected && self.state == ConnectionState::Connected,
                    ))
                    .child(
                        row()
                            .justify_between()
                            .gap(px(8.))
                            .child(if selected {
                                badge("Selected", BLUE, BLUE_BG)
                            } else {
                                badge("Available", MUTED, BG)
                            })
                            .child(
                                text(
                                    if profile.has_password {
                                        "Login saved"
                                    } else {
                                        "No saved login"
                                    },
                                    11.,
                                )
                                .text_color(rgb(MUTED)),
                            ),
                    )
                    .child(
                        row()
                            .gap(px(10.))
                            .child(
                                secondary(
                                    SharedString::from(format!("use-{id}")),
                                    if selected {
                                        "View connection"
                                    } else {
                                        "Use profile"
                                    },
                                )
                                .flex_1()
                                .disabled(
                                    self.quitting || self.busy || (!selected && self.locked()),
                                )
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        if !selected && this.locked() {
                                            return;
                                        }
                                        this.page = Screen::Connection;
                                        window.focus(&this.focus);
                                        if !selected {
                                            this.send(VpnRequest::Select(id), cx);
                                        } else {
                                            cx.notify();
                                        }
                                    },
                                )),
                            )
                            .child(
                                secondary(SharedString::from(format!("settings-{id}")), "Settings")
                                    .icon(icon("settings.svg"))
                                    .flex_1()
                                    .disabled(self.busy || self.quitting)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.begin_settings(id, window, cx)
                                    })),
                            ),
                    )
            }))
            .when(self.locked() && !self.busy, |view| {
                view.child(muted("Disconnect before switching or editing profiles."))
            })
            .into_any_element()
    }

    fn login_fields(&self, saved: bool, cx: &mut Context<Self>) -> Div {
        column()
            .gap(px(16.))
            .child(field("Username", &self.username, self.locked()))
            .child(
                column()
                    .gap(px(7.))
                    .child(
                        row()
                            .justify_between()
                            .child(text("Password", 13.).font_weight(FontWeight::MEDIUM))
                            .when(saved, |view| {
                                view.child(text("Saved on this device", 11.).text_color(rgb(GREEN)))
                            }),
                    )
                    .child(
                        Input::new(&self.password)
                            .large()
                            .min_h(px(44.))
                            .flex_shrink_0()
                            .rounded(px(9.))
                            .text_size(px(14.))
                            .disabled(self.locked())
                            .suffix(
                                secondary(
                                    "show-password",
                                    if self.show_password { "Hide" } else { "Show" },
                                )
                                .ghost()
                                .h(px(28.))
                                .disabled(self.locked())
                                .on_click(cx.listener(
                                    |this, _, window, cx| {
                                        this.show_password = !this.show_password;
                                        this.password.update(cx, |password, cx| {
                                            password.set_masked(!this.show_password, window, cx)
                                        });
                                        cx.notify();
                                    },
                                )),
                            ),
                    ),
            )
            .child(muted(if saved {
                "Leave the password blank to keep your saved login."
            } else {
                "Saved login details are reused whenever you connect."
            }))
    }

    fn settings_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(profile) = self.editing_profile() else {
            return muted("Loading profile…").into_any_element();
        };
        column()
            .gap(px(24.))
            .child(section(
                "Profile settings",
                "Manage the login for this connection.",
            ))
            .child(panel().p(px(16.)).child(identity(profile, false)))
            .child(
                column()
                    .gap(px(16.))
                    .child(text("Sign-in details", 15.).font_weight(FontWeight::SEMIBOLD))
                    .child(self.login_fields(profile.has_password, cx)),
            )
            .when(self.locked() && !self.busy, |view| {
                view.child(muted(
                    "Your VPN is active. Disconnect to edit these details.",
                ))
            })
            .child(
                column()
                    .gap(px(12.))
                    .border_t_1()
                    .border_color(rgb(LINE))
                    .pt(px(16.))
                    .child(
                        row()
                            .gap(px(8.))
                            .child(glyph("lock.svg", MUTED, 14.))
                            .child(muted("Stored only on this device.")),
                    )
                    .when(!self.confirm_remove, |view| {
                        view.child(
                            secondary("delete-profile", "Remove profile")
                                .icon(icon("trash.svg"))
                                .ghost()
                                .text_color(rgb(RED))
                                .justify_start()
                                .disabled(self.locked())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.confirm_remove = true;
                                    cx.notify();
                                })),
                        )
                    })
                    .when(self.confirm_remove, |view| {
                        view.child(
                            panel()
                                .p(px(14.))
                                .bg(rgb(RED_BG))
                                .gap(px(10.))
                                .child(
                                    text(format!("Remove “{}”?", profile.name), 13.)
                                        .font_weight(FontWeight::SEMIBOLD),
                                )
                                .child(muted(
                                    "This removes its configuration and saved login from this app.",
                                ))
                                .child(
                                    row()
                                        .gap(px(8.))
                                        .child(
                                            secondary("keep-profile", "Keep profile")
                                                .flex_1()
                                                .disabled(self.busy)
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.confirm_remove = false;
                                                    cx.notify();
                                                })),
                                        )
                                        .child(
                                            secondary("confirm-delete", "Remove")
                                                .flex_1()
                                                .text_color(rgb(RED))
                                                .disabled(self.locked())
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.remove_profile(cx)
                                                })),
                                        ),
                                ),
                        )
                    }),
            )
            .into_any_element()
    }

    fn import_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let filename = self
            .file
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned());
        column()
            .gap(px(22.))
            .child(section(
                "Add a profile",
                "Import the configuration from your administrator.",
            ))
            .child(
                panel()
                    .p(px(16.))
                    .gap(px(12.))
                    .child(
                        row()
                            .gap(px(12.))
                            .child(glyph("file.svg", BLUE, 25.))
                            .child(
                                column()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        text(
                                            filename
                                                .unwrap_or_else(|| "Choose a configuration".into()),
                                            13.,
                                        )
                                        .truncate()
                                        .font_weight(FontWeight::MEDIUM),
                                    )
                                    .child(muted(".ovpn or .conf")),
                            ),
                    )
                    .child(
                        secondary(
                            "choose-file",
                            if self.picking {
                                "Opening…"
                            } else if self.file.is_some() {
                                "Change file"
                            } else {
                                "Choose file"
                            },
                        )
                        .w_full()
                        .disabled(self.busy || self.picking || self.quitting)
                        .on_click(cx.listener(|this, _, _, cx| this.pick_file(cx))),
                    ),
            )
            .child(field(
                "Profile name",
                &self.name,
                self.busy || self.quitting,
            ))
            .child(
                column()
                    .gap(px(14.))
                    .pt(px(19.))
                    .border_t_1()
                    .border_color(rgb(LINE))
                    .child(
                        row()
                            .justify_between()
                            .child(text("Sign-in details", 15.).font_weight(FontWeight::SEMIBOLD))
                            .child(text("Optional", 11.).text_color(rgb(MUTED))),
                    )
                    .child(self.login_fields(false, cx))
                    .child(muted(
                        "Leave both fields blank to sign in through your desktop.",
                    )),
            )
            .into_any_element()
    }

    fn messages(&self, cx: &mut Context<Self>) -> Div {
        column()
            .gap(px(12.))
            .when_some(self.error.clone(), |view, error| {
                view.child(
                    panel()
                        .p(px(14.))
                        .gap(px(5.))
                        .bg(rgb(RED_BG))
                        .border_color(rgb(0x624149))
                        .child(
                            row()
                                .justify_between()
                                .child(
                                    text("Needs attention", 13.)
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(RED)),
                                )
                                .child(
                                    secondary("dismiss-error", "")
                                        .ghost()
                                        .icon(icon("close.svg"))
                                        .w(px(28.))
                                        .h(px(28.))
                                        .tooltip("Dismiss message")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.error = None;
                                            cx.notify();
                                        })),
                                ),
                        )
                        .child(text(error, 12.)),
                )
            })
            .when_some(self.notice.clone(), |view, notice| {
                view.child(
                    panel()
                        .p(px(13.))
                        .bg(rgb(GREEN_BG))
                        .border_color(rgb(0x306657))
                        .child(text(notice, 12.).text_color(rgb(GREEN))),
                )
            })
    }

    fn footer(&self, cx: &mut Context<Self>) -> AnyElement {
        let action = match self.page {
            Screen::ProfileSettings => primary(
                "save-login",
                if self.busy && !self.quitting {
                    "Saving…"
                } else {
                    "Save login"
                },
            )
            .disabled(self.locked() || self.editing_profile().is_none())
            .on_click(cx.listener(|this, _, window, cx| this.save_login(window, cx)))
            .into_any_element(),
            Screen::Import => primary(
                "import-profile",
                if self.busy && !self.quitting {
                    "Importing…"
                } else {
                    "Import profile"
                },
            )
            .disabled(self.locked() || self.picking || self.file.is_none() || !self.ready)
            .on_click(cx.listener(|this, _, window, cx| this.import_profile(window, cx)))
            .into_any_element(),
            Screen::Profiles => primary("add-profile", "Add profile")
                .icon(icon("plus.svg"))
                .disabled(self.locked() || !self.ready)
                .on_click(cx.listener(|this, _, window, cx| this.begin_import(window, cx)))
                .into_any_element(),
            Screen::Connection => row()
                .justify_center()
                .gap(px(7.))
                .child(glyph("lock.svg", MUTED, 13.))
                .child(
                    text("Disconnects automatically when you close the app.", 11.)
                        .text_color(rgb(MUTED)),
                )
                .into_any_element(),
        };
        column()
            .flex_shrink_0()
            .px(px(PAD))
            .py(px(18.))
            .border_t_1()
            .border_color(rgb(LINE))
            .child(action)
            .into_any_element()
    }
}

impl Render for VpnApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.page {
            Screen::Connection => self.connection(cx),
            Screen::Profiles => self.profiles_page(cx),
            Screen::Import => self.import_page(cx),
            Screen::ProfileSettings => self.settings_page(cx),
        };
        let page_id = match self.page {
            Screen::Connection => "connection-scroll",
            Screen::Profiles => "profiles-scroll",
            Screen::Import => "import-scroll",
            Screen::ProfileSettings => "settings-scroll",
        };
        column()
            .size_full()
            .bg(rgb(BG))
            .text_color(rgb(INK))
            .font_family("Noto Sans")
            .track_focus(&self.focus)
            .key_context("OpenVpn")
            .on_action(
                cx.listener(|this, _: &ShowProfiles, window, cx| this.begin_profiles(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ImportProfile, window, cx| this.begin_import(window, cx)),
            )
            .on_action(cx.listener(|this, _: &Back, window, cx| this.go_back(window, cx)))
            .on_action(cx.listener(|this, _: &Quit, _, cx| this.request_quit(cx)))
            .child(
                row()
                    .h(px(76.))
                    .flex_shrink_0()
                    .px(px(PAD))
                    .justify_between()
                    .gap(px(12.))
                    .child(row().w(px(110.)).flex_shrink_0().child(
                        if self.page == Screen::Connection {
                            secondary("nav-profiles", "Profiles")
                                .ghost()
                                .icon(icon("menu.svg"))
                                .disabled(self.busy || self.quitting)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.begin_profiles(window, cx)
                                }))
                        } else {
                            secondary("nav-back", "Back")
                                .ghost()
                                .icon(icon("back.svg"))
                                .disabled(self.busy || self.quitting)
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.go_back(window, cx)),
                                )
                        },
                    ))
                    .child(
                        text("OpenVPN", 13.)
                            .flex_1()
                            .text_center()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(MUTED)),
                    )
                    .child(
                        row().w(px(110.)).flex_shrink_0().justify_end().child(
                            secondary("nav-add", "")
                                .icon(icon("plus.svg"))
                                .ghost()
                                .w(px(38.))
                                .tooltip("Add profile")
                                .disabled(
                                    self.locked() || !self.ready || self.page == Screen::Import,
                                )
                                .on_click(
                                    cx.listener(|this, _, window, cx| {
                                        this.begin_import(window, cx)
                                    }),
                                ),
                        ),
                    ),
            )
            .when(self.quitting, |view| {
                view.child(
                    row()
                        .px(px(PAD))
                        .py(px(12.))
                        .bg(rgb(BLUE_BG))
                        .flex_shrink_0()
                        .child(text("Disconnecting before closing…", 13.).text_color(rgb(BLUE))),
                )
            })
            .child(
                column()
                    .id(page_id)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(PAD))
                    .pb(px(24.))
                    .gap(px(18.))
                    .when(self.error.is_some() || self.notice.is_some(), |view| {
                        view.child(self.messages(cx))
                    })
                    .child(content),
            )
            .child(self.footer(cx))
    }
}
