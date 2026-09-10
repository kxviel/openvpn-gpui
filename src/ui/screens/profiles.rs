use crate::app::{Screen, VpnApp};
use crate::backend::{ConnectionState, VpnRequest};
use crate::ui::components::{
    badge, column, glyph, icon, identity, muted, panel, row, secondary, section, text,
};
use crate::ui::theme::{BG, BLUE, BLUE_BG, MUTED};
use gpui::{prelude::*, *};
use gpui_component::Disableable;

impl VpnApp {
    pub(in crate::ui) fn profiles_page(&self, cx: &mut Context<Self>) -> AnyElement {
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
}
