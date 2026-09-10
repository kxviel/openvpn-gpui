use crate::app::VpnApp;
use crate::ui::components::{column, icon, panel, row, secondary, text};
use crate::ui::theme::{GREEN, GREEN_BG, RED, RED_BG};
use gpui::{prelude::*, *};
use gpui_component::button::ButtonVariants;

impl VpnApp {
    pub(in crate::ui) fn messages(&self, cx: &mut Context<Self>) -> Div {
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
}
