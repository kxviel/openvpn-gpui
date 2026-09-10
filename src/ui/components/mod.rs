mod credentials;
mod feedback;

use super::theme::*;
use crate::profile::Profile;
use gpui::{prelude::*, *};
use gpui_component::{
    Icon, Sizable,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
};

pub(in crate::ui) fn row() -> Div {
    div().flex().items_center()
}
pub(in crate::ui) fn column() -> Div {
    div().flex().flex_col()
}
pub(in crate::ui) fn icon(path: &'static str) -> Icon {
    Icon::default().path(path)
}
pub(in crate::ui) fn glyph(path: &'static str, color: u32, size: f32) -> Svg {
    svg()
        .path(path)
        .size(px(size))
        .flex_shrink_0()
        .text_color(rgb(color))
}
pub(in crate::ui) fn text(value: impl Into<SharedString>, size: f32) -> Div {
    div()
        .text_size(px(size))
        .line_height(px(size * 1.5))
        .child(value.into())
}
pub(in crate::ui) fn muted(value: impl Into<SharedString>) -> Div {
    text(value, 12.).text_color(rgb(MUTED))
}
pub(in crate::ui) fn section(title: &'static str, description: &'static str) -> Div {
    column()
        .gap(px(5.))
        .child(text(title, 25.).font_weight(FontWeight::SEMIBOLD))
        .child(muted(description))
}
pub(in crate::ui) fn panel() -> Div {
    column()
        .p(px(20.))
        .rounded(px(18.))
        .bg(rgb(PANEL))
        .border_1()
        .border_color(rgb(LINE))
}
pub(in crate::ui) fn secondary(id: impl Into<ElementId>, label: &'static str) -> Button {
    Button::new(id)
        .label(label)
        .small()
        .h(px(38.))
        .px(px(12.))
        .rounded(px(9.))
}
pub(in crate::ui) fn primary(id: &'static str, label: &'static str) -> Button {
    Button::new(id)
        .label(label)
        .primary()
        .w_full()
        .h(px(48.))
        .rounded(px(11.))
}
pub(in crate::ui) fn badge(label: &'static str, color: u32, background: u32) -> Div {
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
pub(in crate::ui) fn field(label: &'static str, state: &Entity<InputState>, disabled: bool) -> Div {
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
pub(in crate::ui) fn identity(profile: &Profile, active: bool) -> Div {
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
pub(in crate::ui) fn detail(label: &'static str, value: impl Into<SharedString>) -> Div {
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
