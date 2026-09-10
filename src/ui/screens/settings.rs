use crate::app::VpnApp;
use crate::ui::components::{
    column, glyph, icon, identity, muted, panel, row, secondary, section, text,
};
use crate::ui::theme::{LINE, MUTED, RED, RED_BG};
use gpui::{prelude::*, *};
use gpui_component::Disableable;
use gpui_component::button::ButtonVariants;

impl VpnApp {
    pub(in crate::ui) fn settings_page(&self, cx: &mut Context<Self>) -> AnyElement {
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
}
