use super::VpnApp;
use crate::{
    backend::{ConnectionState, VpnRequest},
    profile,
};
use gpui::*;
use std::sync::atomic::Ordering;

impl VpnApp {
    pub(crate) fn send(&mut self, request: VpnRequest, cx: &mut Context<Self>) {
        self.error = None;
        self.notice = None;
        self.busy = true;
        if self.backend.requests.send(request).is_err() {
            self.error = Some("The profile service stopped. Close and reopen the app.".into());
            self.busy = false;
        }
        cx.notify();
    }

    pub(crate) fn save_login(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        let Some(profile) = self.editing_profile() else {
            return;
        };
        let id = profile.id;
        let credentials = profile::validate_credentials(
            &self.username.read(cx).value(),
            &self.password.read(cx).value(),
            profile.has_password,
        );
        self.notice = None;
        match credentials {
            Ok((username, password)) => {
                window.focus(&self.focus);
                self.send(
                    VpnRequest::SaveCredentials {
                        id,
                        username,
                        password,
                    },
                    cx,
                );
            }
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
            }
        }
    }

    pub(crate) fn import_profile(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() || self.picking || !self.ready {
            return;
        }
        let Some(path) = self.file.clone() else {
            return;
        };
        let name = self.profiles.validate_name(&self.name.read(cx).value());
        let username = self.username.read(cx).value();
        let password = self.password.read(cx).value();
        let credentials = if username.trim().is_empty() && password.is_empty() {
            Ok(None)
        } else {
            profile::validate_credentials(&username, &password, false)
                .map(|(username, password)| password.map(|password| (username, password)))
        };
        match (name, credentials) {
            (Ok(name), Ok(credentials)) => {
                window.focus(&self.focus);
                self.send(
                    VpnRequest::Import {
                        path,
                        name,
                        credentials,
                    },
                    cx,
                );
            }
            (Err(error), _) | (_, Err(error)) => {
                self.error = Some(error.to_string());
                cx.notify();
            }
        }
    }

    pub(crate) fn remove_profile(&mut self, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        if let Some(id) = self.editing {
            self.confirm_remove = false;
            self.send(VpnRequest::Remove(id), cx);
        }
    }

    pub(crate) fn request_quit(&mut self, cx: &mut Context<Self>) {
        if self.quitting {
            return;
        }
        if self.backend_stopped {
            cx.quit();
            return;
        }
        self.quitting = true;
        self.busy = true;
        self.error = None;
        self.notice = None;
        self.state = ConnectionState::Disconnecting;
        self.backend.cancel.store(true, Ordering::Relaxed);
        if self.backend.requests.send(VpnRequest::Shutdown).is_err() {
            // The worker has already stopped; Drop joins its cleanup.
            cx.quit();
        }
        cx.notify();
    }

    pub(crate) fn pick_file(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.picking {
            return;
        }
        self.picking = true;
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose an OpenVPN configuration (.ovpn or .conf)".into()),
        });
        cx.spawn(async move |this, cx| {
            let result = prompt.await;
            let _ = this.update(cx, |this, cx| {
                this.picking = false;
                match result {
                    Ok(Ok(Some(paths))) => { this.file = paths.into_iter().next(); this.error = None; },
                    Ok(Ok(None)) => {},
                    _ => this.error = Some("Could not open the file picker. Check that your desktop portal is running.".into()),
                }
                cx.notify();
            });
        }).detach();
    }

    pub(crate) fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.quitting {
            return;
        }
        if self.state == ConnectionState::Connecting {
            self.backend.cancel.store(true, Ordering::Relaxed);
            self.state = ConnectionState::Disconnecting;
            self.send(VpnRequest::Disconnect, cx);
        } else if self.state == ConnectionState::Connected && !self.busy {
            self.state = ConnectionState::Disconnecting;
            self.send(VpnRequest::Disconnect, cx);
        } else if !self.busy && self.profiles.selected().is_some() && self.ready {
            self.backend.cancel.store(false, Ordering::Relaxed);
            self.state = ConnectionState::Connecting;
            self.send(VpnRequest::Connect, cx);
        }
    }
}
