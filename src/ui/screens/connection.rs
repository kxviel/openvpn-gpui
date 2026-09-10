use crate::app::VpnApp;
use crate::backend::ConnectionState;
use crate::ui::components::{
    badge, column, detail, glyph, icon, identity, muted, panel, primary, row, secondary, text,
};
use crate::ui::theme::{BLUE, BLUE_BG, GREEN, GREEN_BG, INK, LINE, MUTED, PANEL, RED, RED_BG};
use gpui::{prelude::*, *};
use gpui_component::Disableable;
use gpui_component::button::ButtonVariants;

impl VpnApp {
    pub(in crate::ui) fn connection(&self, cx: &mut Context<Self>) -> AnyElement {
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
}
