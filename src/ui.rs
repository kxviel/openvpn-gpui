use crate::app::{Back, ImportProfile, Quit, Screen, ShowProfiles, VpnApp};
use crate::backend::{ConnectionState, VpnRequest};
use gpui::{prelude::*, *};
use gpui_component::{
    Disableable, Icon,
    button::{Button, ButtonVariants},
    input::Input,
};

const BG: u32 = 0x0d1117;
const PANEL: u32 = 0x161c25;
const INK: u32 = 0xf1f4f8;
const MUTED: u32 = 0x929caa;
const LINE: u32 = 0x29323e;
const BLUE: u32 = 0x4c7dff;
const BLUE_SURFACE: u32 = 0x17294d;
const HOVER_LINE: u32 = 0x48586c;
const INACTIVE: u32 = 0x596473;
const SHIELD_BG: u32 = 0x202833;
const SECONDARY_BUTTON: u32 = 0x28313d;
const ERROR_BG: u32 = 0x25191b;
const ERROR_LINE: u32 = 0x6b3e43;

const HEADER_HEIGHT: f32 = 60.0;
const FOOTER_HEIGHT: f32 = 48.0;
const PAGE_PADDING: f32 = 24.0;
const PAGE_TOP: f32 = 20.0;
const PAGE_BOTTOM: f32 = 20.0;
const CARD_PADDING: f32 = 14.0;
const CARD_RADIUS: f32 = 12.0;
const CONTROL_HEIGHT: f32 = 46.0;

fn icon(path: &'static str) -> Icon {
    Icon::default().path(path)
}
fn caption(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(10.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(MUTED))
        .child(text.into())
}

fn page_heading(title: &'static str, description: &'static str) -> Div {
    div()
        .child(caption("NEW CONNECTION"))
        .child(
            div()
                .mt(px(10.))
                .text_size(px(27.))
                .font_weight(FontWeight::MEDIUM)
                .child(title),
        )
        .child(
            div()
                .mt(px(7.))
                .text_size(px(12.))
                .line_height(px(19.))
                .text_color(rgb(MUTED))
                .child(description),
        )
}

fn secondary(id: &'static str, label: &'static str) -> Button {
    Button::new(id)
        .label(label)
        .ghost()
        .h(px(36.))
        .rounded(px(8.))
        .text_size(px(12.))
}
fn primary(id: &'static str, label: &'static str) -> Button {
    Button::new(id)
        .label(label)
        .primary()
        .text_color(rgb(0xffffff))
        .w_full()
        .h(px(CONTROL_HEIGHT))
        .rounded(px(10.))
        .text_size(px(13.))
}

impl VpnApp {
    fn connection(&self, cx: &mut Context<Self>) -> AnyElement {
        let active = self.state == ConnectionState::Connected;
        let has_profile = self.profiles.selected().is_some();
        let connecting = self.state == ConnectionState::Connecting;
        let accent = if active || connecting { BLUE } else { LINE };
        let (status, title, subtitle) = match self.state {
            ConnectionState::Connected => (
                "TUNNEL ACTIVE",
                "You're connected",
                "Your work network is within reach.",
            ),
            ConnectionState::Connecting => (
                "CONNECTING",
                "Making a connection",
                "Complete any desktop sign-in prompt.",
            ),
            ConnectionState::Disconnecting => (
                "DISCONNECTING",
                "Closing the tunnel",
                "Your connection will end in a moment.",
            ),
            ConnectionState::Unavailable => (
                "CHECK CONNECTION",
                "Let's try again",
                "Review the message below to continue.",
            ),
            ConnectionState::Idle if has_profile => (
                "NOT CONNECTED",
                "Ready to connect",
                "A secure connection to your work network.",
            ),
            _ => (
                "NOT CONNECTED",
                "Ready when you are",
                "Add a VPN profile to get started.",
            ),
        };
        let profile_name = self
            .profiles
            .selected()
            .map(|p| p.name.clone())
            .unwrap_or("Add your first profile".into());
        let remote = self
            .profiles
            .selected()
            .map(|p| p.remote.clone())
            .unwrap_or("Import an OpenVPN configuration".into());
        let protocol = self
            .profiles
            .selected()
            .map(|p| p.protocol.clone())
            .unwrap_or("—".into());
        let seconds = self.since.map(|s| s.elapsed().as_secs());
        let duration = seconds
            .map(|s| format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60))
            .unwrap_or("—".into());
        let action = if active {
            "Disconnect"
        } else if connecting {
            "Cancel connection"
        } else if self.state == ConnectionState::Disconnecting {
            "Disconnecting…"
        } else if has_profile {
            "Connect"
        } else {
            "Import configuration"
        };
        div()
            .flex()
            .flex_col()
            .w_full()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .size(px(6.))
                                    .rounded_full()
                                    .bg(rgb(if active || connecting { BLUE } else { INACTIVE })),
                            )
                            .child(caption(status)),
                    )
                    .child(caption("OPENVPN")),
            )
            .child(
                div().flex().justify_center().pt(px(22.)).pb(px(18.)).child(
                    div()
                        .size(px(148.))
                        .rounded_full()
                        .border_1()
                        .border_color(rgb(accent))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .size(px(110.))
                                .rounded_full()
                                .border_1()
                                .border_color(rgb(accent))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .size(px(76.))
                                        .rounded_full()
                                        .border_1()
                                        .border_color(rgb(accent))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            div()
                                                .size(px(50.))
                                                .rounded_full()
                                                .bg(rgb(if active { BLUE } else { SHIELD_BG }))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .child(
                                                    svg()
                                                        .path("shield.svg")
                                                        .size(px(22.))
                                                        .text_color(rgb(if active {
                                                            0xffffff
                                                        } else {
                                                            MUTED
                                                        })),
                                                ),
                                        ),
                                ),
                        ),
                ),
            )
            .child(
                div()
                    .text_center()
                    .text_size(px(24.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(title),
            )
            .child(
                div()
                    .mt(px(6.))
                    .text_center()
                    .text_size(px(12.))
                    .text_color(rgb(MUTED))
                    .child(subtitle),
            )
            .child(
                div()
                    .id("profile-picker")
                    .mt(px(22.))
                    .p(px(CARD_PADDING))
                    .rounded(px(CARD_RADIUS))
                    .bg(rgb(PANEL))
                    .border_1()
                    .border_color(rgb(LINE))
                    .when(!self.locked(), |d| {
                        d.cursor_pointer()
                            .hover(|s| s.border_color(rgb(HOVER_LINE)))
                    })
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.locked() {
                            return;
                        }
                        if this.profiles.profiles.is_empty() {
                            this.begin_import(window, cx);
                        } else {
                            this.page = Screen::Profiles;
                            this.confirm_remove = false;
                            cx.notify();
                        }
                    }))
                    .child(caption("CONNECTION PROFILE"))
                    .child(
                        div()
                            .mt(px(7.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .truncate()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_size(px(13.))
                                            .child(profile_name),
                                    )
                                    .child(
                                        div()
                                            .mt(px(3.))
                                            .truncate()
                                            .font_family("DejaVu Sans Mono")
                                            .text_size(px(10.))
                                            .text_color(rgb(MUTED))
                                            .child(remote),
                                    ),
                            )
                            .child(
                                svg()
                                    .path("chevron.svg")
                                    .size(px(16.))
                                    .text_color(rgb(MUTED)),
                            ),
                    ),
            )
            .child(
                div().mt(px(10.)).child(
                    primary("connect", action)
                        .icon(icon(if has_profile { "power.svg" } else { "plus.svg" }))
                        .when(active, |b| b.bg(rgb(SECONDARY_BUTTON)).text_color(rgb(INK)))
                        .disabled(
                            !self.ready
                                || self.state == ConnectionState::Disconnecting
                                || (self.busy && !connecting),
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            if this.profiles.selected().is_none() {
                                this.begin_import(window, cx);
                            } else {
                                this.toggle(cx);
                            }
                        })),
                ),
            )
            .child(
                div()
                    .mt(px(18.))
                    .pt(px(16.))
                    .border_t_1()
                    .border_color(rgb(LINE))
                    .flex()
                    .gap_3()
                    .child(Self::stat("PROTOCOL", protocol))
                    .child(Self::stat(
                        "VPN ADDRESS",
                        if self.address.is_empty() {
                            "—".into()
                        } else {
                            self.address.clone()
                        },
                    ))
                    .child(Self::stat("DURATION", duration)),
            )
            .into_any_element()
    }

    fn stat(label: &'static str, value: String) -> Div {
        div().flex_1().min_w_0().child(caption(label)).child(
            div()
                .mt(px(7.))
                .text_size(px(10.))
                .font_family("DejaVu Sans Mono")
                .overflow_hidden()
                .child(value),
        )
    }

    fn profiles_page(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .child(caption("YOUR CONNECTIONS"))
            .child(
                div()
                    .mt(px(10.))
                    .text_size(px(27.))
                    .font_weight(FontWeight::MEDIUM)
                    .child("Your saved profiles"),
            )
            .child(
                div()
                    .mt(px(7.))
                    .text_size(px(12.))
                    .line_height(px(19.))
                    .text_color(rgb(MUTED))
                    .child("Choose a saved profile, or add another."),
            )
            .child(div().mt(px(22.)).flex().flex_col().gap_2().children(
                self.profiles.profiles.iter().map(|profile| {
                    let id = profile.id;
                    let selected = Some(id) == self.profiles.selected;
                    Button::new(SharedString::from(format!("profile-{id}")))
                        .w_full()
                        .h(px(62.))
                        .justify_start()
                        .rounded(px(10.))
                        .label(profile.name.clone())
                        .icon(icon("shield.svg"))
                        .when(selected, |b| {
                            b.border_color(rgb(BLUE)).bg(rgb(BLUE_SURFACE))
                        })
                        .disabled(self.busy)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.page = Screen::Connection;
                            this.send(VpnRequest::Select(id), cx);
                        }))
                }),
            ))
            .child(
                primary("add-profile", "Add a profile")
                    .icon(icon("plus.svg"))
                    .mt(px(10.))
                    .disabled(self.busy)
                    .on_click(cx.listener(|this, _, window, cx| this.begin_import(window, cx))),
            )
            .when(self.profiles.selected().is_some(), |d| {
                d.child(
                    div()
                        .mt(px(22.))
                        .border_t_1()
                        .border_color(rgb(LINE))
                        .pt(px(14.))
                        .child(
                            secondary(
                                "remove-profile",
                                if self.confirm_remove {
                                    "Confirm removal of selected profile"
                                } else {
                                    "Remove selected profile"
                                },
                            )
                            .text_color(rgb(0xa05247))
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.confirm_remove {
                                    this.confirm_remove = false;
                                    this.send(VpnRequest::Remove, cx);
                                } else {
                                    this.confirm_remove = true;
                                    cx.notify();
                                }
                            })),
                        )
                        .when(self.confirm_remove, |d| {
                            d.child(
                                div()
                                    .mt_2()
                                    .text_size(px(12.))
                                    .text_color(rgb(MUTED))
                                    .child(format!(
                                        "Remove “{}” and its saved configuration?",
                                        self.profiles
                                            .selected()
                                            .map(|p| p.name.as_str())
                                            .unwrap_or("")
                                    )),
                            )
                        }),
                )
            })
            .into_any_element()
    }

    fn import_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let filename = self
            .file
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned());
        div().flex().flex_col()
            .child(page_heading(
                "Add a connection",
                "Import the configuration from your workplace, then give it a name you'll recognize.",
            ))
            .child(div().mt(px(22.)).p(px(20.)).rounded(px(CARD_RADIUS)).border_1().border_color(rgb(LINE)).bg(rgb(PANEL)).flex().flex_col().items_center().gap_2()
                .child(svg().path("file.svg").size(px(30.)).text_color(rgb(BLUE)))
                .child(div().max_w_full().truncate().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(filename.unwrap_or("Your OpenVPN configuration".into())))
                .child(div().text_size(px(11.)).text_color(rgb(MUTED)).child(".ovpn or .conf"))
                .child(secondary("choose-file", if self.picking { "Opening…" } else if self.file.is_some() { "Choose a different file" } else { "Choose file" })
                    .disabled(self.busy || self.picking).on_click(cx.listener(|this, _, _, cx| this.pick_file(cx)))))
            .child(div().mt(px(22.)).child(caption("PROFILE NAME")).child(div().mt(px(7.)).child(Input::new(&self.name).h(px(CONTROL_HEIGHT)).disabled(self.busy))))
            .child(div().mt(px(16.)).child(primary("save-profile", if self.busy { "Saving profile…" } else { "Save profile" })
                .disabled(self.file.is_none() || self.busy || self.picking)
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(path) = this.file.clone() {
                        let name = this.name.read(cx).value().to_string();
                        match this.profiles.validate_name(&name) {
                            Ok(name) => this.send(VpnRequest::Import { path, name }, cx),
                            Err(e) => { this.error = Some(e.to_string()); cx.notify(); },
                        }
                    }
                }))))
            .child(div().mt(px(12.)).text_size(px(11.)).line_height(px(17.)).text_color(rgb(MUTED)).child("Your profile is saved on this device. Sign-in is handled by your desktop when you connect."))
            .into_any_element()
    }
}

impl Render for VpnApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.page {
            Screen::Connection => self.connection(cx),
            Screen::Profiles => self.profiles_page(cx),
            Screen::Import => self.import_page(cx),
        };
        div()
            .size_full()
            .bg(rgb(BG))
            .text_color(rgb(INK))
            .font_family("Noto Sans")
            .flex()
            .flex_col()
            .track_focus(&self.focus)
            .key_context("OpenVpn")
            .on_action(cx.listener(|this, _: &ShowProfiles, _, cx| {
                if !this.locked() {
                    this.page = Screen::Profiles;
                    cx.notify();
                }
            }))
            .on_action(
                cx.listener(|this, _: &ImportProfile, window, cx| this.begin_import(window, cx)),
            )
            .on_action(cx.listener(|this, _: &Back, window, cx| {
                if !this.busy {
                    this.page = Screen::Connection;
                    this.confirm_remove = false;
                    window.focus(&this.focus);
                    cx.notify();
                }
            }))
            .on_action(|_: &Quit, _, cx| cx.quit())
            .child(
                div()
                    .h(px(HEADER_HEIGHT))
                    .flex_shrink_0()
                    .px(px(PAGE_PADDING))
                    .border_b_1()
                    .border_color(rgb(LINE))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when(self.page != Screen::Connection, |d| {
                                d.child(
                                    secondary("back", "Back")
                                        .icon(icon("back.svg"))
                                        .disabled(self.busy)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.page = Screen::Connection;
                                            this.confirm_remove = false;
                                            window.focus(&this.focus);
                                            cx.notify();
                                        })),
                                )
                            })
                            .when(self.page == Screen::Connection, |d| {
                                d.child(
                                    div()
                                        .size(px(28.))
                                        .rounded(px(7.))
                                        .bg(rgb(BLUE))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            svg()
                                                .path("shield.svg")
                                                .size(px(18.))
                                                .text_color(rgb(0xffffff)),
                                        ),
                                )
                            })
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("OpenVPN"),
                            ),
                    )
                    .child(
                        secondary("import", "Add")
                            .icon(icon("plus.svg"))
                            .disabled(self.locked() || !self.ready)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.begin_import(window, cx)),
                            ),
                    ),
            )
            .child(
                div()
                    .id("content-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(PAGE_PADDING))
                    .pt(px(PAGE_TOP))
                    .pb(px(PAGE_BOTTOM))
                    .child(content)
                    .when_some(self.error.clone(), |d, error| {
                        d.child(
                            div()
                                .mt(px(16.))
                                .p(px(12.))
                                .rounded(px(10.))
                                .border_1()
                                .border_color(rgb(ERROR_LINE))
                                .bg(rgb(ERROR_BG))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_size(px(12.))
                                                .child("Needs attention"),
                                        )
                                        .child(
                                            secondary("dismiss-error", "")
                                                .icon(icon("close.svg"))
                                                .tooltip("Dismiss message")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.error = None;
                                                    cx.notify();
                                                })),
                                        ),
                                )
                                .child(div().text_size(px(12.)).line_height(px(18.)).child(error)),
                        )
                    }),
            )
            .child(
                div()
                    .h(px(FOOTER_HEIGHT))
                    .flex_shrink_0()
                    .px(px(PAGE_PADDING))
                    .border_t_1()
                    .border_color(rgb(LINE))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        svg()
                            .path("shield.svg")
                            .size(px(13.))
                            .text_color(rgb(MUTED)),
                    )
                    .child(div().text_size(px(10.)).text_color(rgb(MUTED)).child(
                        if self.state == ConnectionState::Connected {
                            "VPN stays connected when you close this window."
                        } else {
                            "Your profiles are saved on this device."
                        },
                    )),
            )
    }
}
