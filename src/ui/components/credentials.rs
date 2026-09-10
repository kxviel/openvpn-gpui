use crate::app::VpnApp;
use crate::ui::components::{column, field, muted, row, secondary, text};
use crate::ui::theme::GREEN;
use gpui::{prelude::*, *};
use gpui_component::Disableable;
use gpui_component::button::ButtonVariants;
use gpui_component::{Sizable, input::Input};

impl VpnApp {
    pub(in crate::ui) fn login_fields(&self, saved: bool, cx: &mut Context<Self>) -> Div {
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
}
