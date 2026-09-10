use crate::app::VpnApp;
use crate::ui::components::{column, field, glyph, muted, panel, row, secondary, section, text};
use crate::ui::theme::{BLUE, LINE, MUTED};
use gpui::{prelude::*, *};
use gpui_component::Disableable;

impl VpnApp {
    pub(in crate::ui) fn import_page(&self, cx: &mut Context<Self>) -> AnyElement {
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
}
