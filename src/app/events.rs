use super::{Screen, VpnApp};
use crate::backend::{ConnectionState, VpnEvent};
use gpui::*;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const UI_POLL_INTERVAL: Duration = Duration::from_millis(150);

impl VpnApp {
    pub(super) fn start_ticker(
        exit_requested: Arc<AtomicBool>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<()> {
        cx.spawn_in(window, async move |this, cx| {
            let mut rendered_second = 0;
            loop {
                cx.background_executor().timer(UI_POLL_INTERVAL).await;
                if this
                    .update_in(cx, |this, window, cx| {
                        let mut changed = false;
                        if exit_requested.swap(false, Ordering::Relaxed) {
                            this.request_quit(cx);
                        }
                        while !this.backend_stopped {
                            let event = match this.backend.events.try_recv() {
                                Ok(event) => event,
                                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                    this.backend_stopped = true;
                                    this.ready = false;
                                    this.busy = false;
                                    if this.quitting {
                                        cx.quit();
                                    } else {
                                        this.state = ConnectionState::Unavailable;
                                        this.error.get_or_insert_with(|| "The connection service stopped. Close and reopen the app.".into());
                                    }
                                    cx.notify();
                                    break;
                                }
                            };
                            changed = true;
                            match event {
                                VpnEvent::Profiles(profiles) => {
                                    let imported = this.busy
                                        && profiles.profiles.len() > this.profiles.profiles.len();
                                    let selected_changed =
                                        profiles.selected != this.profiles.selected;
                                    this.profiles = profiles;
                                    #[cfg(debug_assertions)]
                                    if !this.ready && this.page == Screen::ProfileSettings
                                        && let Some(id) = this.profiles.selected
                                    {
                                        this.begin_settings(id, window, cx);
                                    }
                                    if selected_changed {
                                        this.since = None;
                                        this.address.clear();
                                        if !this.quitting {
                                            this.state = ConnectionState::Idle;
                                        }
                                    }
                                    if this.editing.is_some()
                                        && this.editing_profile().is_none()
                                    {
                                        this.editing = None;
                                        this.page = Screen::Profiles;
                                        this.reset_password(window, cx);
                                    }
                                    if imported {
                                        this.page = Screen::Connection;
                                        this.file = None;
                                        this.reset_password(window, cx);
                                    }
                                }
                                VpnEvent::Status { state, address } => {
                                    if state == ConnectionState::Connected {
                                        if this.since.is_none() {
                                            this.since = Some(Instant::now());
                                        }
                                    } else {
                                        this.since = None;
                                    }
                                    this.state = if this.quitting {
                                        ConnectionState::Disconnecting
                                    } else { state };
                                    this.address = address;
                                }
                                VpnEvent::Error(error) => this.error = Some(error),
                                VpnEvent::Notice(notice) => {
                                    this.notice = Some(notice);
                                    this.reset_password(window, cx);
                                }
                                VpnEvent::Busy(busy) => this.busy = busy || this.quitting,
                                VpnEvent::Ready => this.ready = true,
                                VpnEvent::ShutdownComplete => {
                                    cx.quit();
                                    return;
                                }
                                VpnEvent::ShutdownFailed(error) => {
                                    this.quitting = false;
                                    this.busy = false;
                                    this.state = ConnectionState::Unavailable;
                                    this.error = Some(error);
                                }
                            }
                        }
                        let elapsed = this
                            .since
                            .map(|since| since.elapsed().as_secs())
                            .unwrap_or(0);
                        if changed || elapsed != rendered_second {
                            rendered_second = elapsed;
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
    }
}
