mod app;
mod backend;
mod profile;
mod ui;

use gpui::*;
use gpui_component::Root;
use std::{
    fs::OpenOptions,
    sync::{Arc, atomic::AtomicBool},
};
use ui::Assets;

const WINDOW_WIDTH: f32 = 460.0;
const WINDOW_HEIGHT: f32 = 780.0;
const MIN_WINDOW_WIDTH: f32 = 420.0;
const MIN_WINDOW_HEIGHT: f32 = 700.0;

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
        ui::init_theme(cx);
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
