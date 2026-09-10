mod app;
mod backend;
mod profile;
mod ui;

use gpui::*;
use gpui_component::{Root, Theme, ThemeMode};
use std::{
    borrow::Cow,
    fs::OpenOptions,
    sync::{Arc, atomic::AtomicBool},
};

const WINDOW_WIDTH: f32 = 460.0;
const WINDOW_HEIGHT: f32 = 780.0;
const MIN_WINDOW_WIDTH: f32 = 420.0;
const MIN_WINDOW_HEIGHT: f32 = 700.0;

struct Assets;
impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        let bytes: &'static [u8] = match path {
            "shield.svg" => include_bytes!("../assets/shield.svg"),
            "power.svg" => include_bytes!("../assets/power.svg"),
            "plus.svg" => include_bytes!("../assets/plus.svg"),
            "chevron.svg" => include_bytes!("../assets/chevron.svg"),
            "file.svg" => include_bytes!("../assets/file.svg"),
            "close.svg" => include_bytes!("../assets/close.svg"),
            "back.svg" => include_bytes!("../assets/back.svg"),
            "menu.svg" => include_bytes!("../assets/menu.svg"),
            "trash.svg" => include_bytes!("../assets/trash.svg"),
            "lock.svg" => include_bytes!("../assets/lock.svg"),
            "settings.svg" => include_bytes!("../assets/settings.svg"),
            _ => return Ok(None),
        };
        Ok(Some(Cow::Borrowed(bytes)))
    }
    fn list(&self, _: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(vec![])
    }
}

fn main() -> anyhow::Result<()> {
    let root = profile::data_dir()?;
    profile::private_dir(&root)?;
    // Prevent two windows from racing profile saves or connection operations.
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("app.lock"))?;
    if fs2::FileExt::try_lock_exclusive(&lock).is_err() {
        eprintln!("OpenVPN is already running. Use the existing window.");
        return Ok(());
    }
    let exit_requested = Arc::new(AtomicBool::new(false));
    for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
        signal_hook::flag::register(signal, exit_requested.clone())?;
    }
    Application::new().with_assets(Assets).run(move |cx| {
        gpui_component::init(cx);
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
        cx.bind_keys([
            KeyBinding::new("ctrl-o", app::ImportProfile, Some("OpenVpn")),
            KeyBinding::new("ctrl-p", app::ShowProfiles, Some("OpenVpn")),
            KeyBinding::new("escape", app::Back, Some("OpenVpn")),
            KeyBinding::new("ctrl-q", app::Quit, Some("OpenVpn")),
        ]);
        let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("OpenVPN".into()),
                    ..Default::default()
                }),
                window_min_size: Some(size(px(MIN_WINDOW_WIDTH), px(MIN_WINDOW_HEIGHT))),
                app_id: Some("dev.kxviel.OpenVPN".into()),
                ..Default::default()
            },
            |window, cx| {
                window.set_window_title("OpenVPN");
                let view = cx.new(|cx| app::VpnApp::new(root, exit_requested, window, cx));
                let closing = view.downgrade();
                window.on_window_should_close(cx, move |_, cx| {
                    closing
                        .update(cx, |view, cx| view.request_quit(cx))
                        .is_err()
                });
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("Could not open a GPUI window. Check your Wayland/X11 session and Vulkan driver.");
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
    drop(lock);
    Ok(())
}
