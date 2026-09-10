use gpui::*;
use gpui_component::{Theme, ThemeMode};

pub(super) const BG: u32 = 0x0d1117;
pub(super) const PANEL: u32 = 0x161d27;
pub(super) const INK: u32 = 0xf1f4f8;
pub(super) const MUTED: u32 = 0x9ba8ba;
pub(super) const LINE: u32 = 0x293443;
pub(super) const BLUE: u32 = 0x739bff;
pub(super) const BLUE_BG: u32 = 0x182b4d;
pub(super) const GREEN: u32 = 0x63d6ad;
pub(super) const GREEN_BG: u32 = 0x142c29;
pub(super) const RED: u32 = 0xffb1ae;
pub(super) const RED_BG: u32 = 0x302024;
pub(super) const PAD: f32 = 24.;

/// Configure the native controls to match the shared UI palette.
pub(crate) fn init_theme(cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);
    let theme = Theme::global_mut(cx);
    theme.colors.background = rgb(0x0d1117).into();
    theme.colors.foreground = rgb(0xf1f4f8).into();
    theme.colors.border = rgb(0x29323e).into();
    theme.colors.input = rgb(0x364252).into();
    theme.colors.primary = rgb(0x3f73f1).into();
    theme.colors.primary_hover = rgb(0x5687ff).into();
    theme.colors.primary_active = rgb(0x3463d2).into();
    theme.colors.primary_foreground = rgb(0xffffff).into();
    theme.colors.secondary = rgb(0x181f29).into();
    theme.colors.secondary_hover = rgb(0x242d39).into();
    theme.colors.secondary_active = rgb(0x2d3846).into();
    theme.colors.secondary_foreground = rgb(0xe8edf4).into();
    theme.colors.muted = rgb(0x202833).into();
    theme.colors.muted_foreground = rgb(0x929caa).into();
    theme.colors.accent = rgb(0x202a38).into();
    theme.colors.accent_foreground = rgb(0xf1f4f8).into();
    theme.colors.popover = rgb(0x161c25).into();
    theme.colors.popover_foreground = rgb(0xf1f4f8).into();
    theme.colors.selection = rgb(0x274b8f).into();
    theme.colors.caret = rgb(0x6f97ff).into();
    theme.colors.ring = rgb(0x4c7dff).into();
    theme.colors.title_bar = rgb(0x0d1117).into();
    theme.colors.title_bar_border = rgb(0x29323e).into();
    theme.colors.window_border = rgb(0x29323e).into();
    theme.font_family = "Noto Sans".into();
}
