mod assets;
mod components;
mod screens;
mod theme;

pub(crate) use assets::Assets;
pub(crate) use theme::init_theme;

use crate::app::{Back, ImportProfile, Quit, Screen, ShowProfiles, VpnApp};
use components::{column, glyph, icon, primary, row, secondary, text};
use gpui::{prelude::*, *};
use gpui_component::{Disableable, button::ButtonVariants};
use theme::{BG, BLUE, BLUE_BG, INK, LINE, MUTED, PAD};

impl VpnApp {
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
