use super::{Screen, VpnApp};
use gpui::*;
use gpui_component::input::InputState;
use uuid::Uuid;

impl VpnApp {
    pub(crate) fn begin_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() || !self.ready {
            return;
        }
        self.page = Screen::Import;
        self.error = None;
        self.notice = None;
        self.confirm_remove = false;
        self.editing = None;
        self.file = None;
        self.name
            .update(cx, |name, cx| name.set_value("", window, cx));
        self.username
            .update(cx, |username, cx| username.set_value("", window, cx));
        self.reset_password(window, cx);
        self.name.update(cx, |name, cx| name.focus(window, cx));
        cx.notify();
    }

    pub(crate) fn begin_profiles(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.quitting {
            return;
        }
        let username = self
            .profiles
            .selected()
            .map(|profile| profile.username.clone())
            .unwrap_or_default();
        self.page = Screen::Profiles;
        self.editing = None;
        self.error = None;
        self.notice = None;
        self.confirm_remove = false;
        self.username
            .update(cx, |state, cx| state.set_value(username, window, cx));
        self.reset_password(window, cx);
        window.focus(&self.focus);
        cx.notify();
    }

    pub(super) fn reset_password(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_password = false;
        // A fresh entity also drops the old field's undo history.
        self.password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter password")
                .masked(true)
        });
    }

    pub(crate) fn begin_settings(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.quitting {
            return;
        }
        let Some(profile) = self
            .profiles
            .profiles
            .iter()
            .find(|profile| profile.id == id)
        else {
            return;
        };
        let username = profile.username.clone();
        self.editing = Some(id);
        self.settings_return = if self.page == Screen::Profiles {
            Screen::Profiles
        } else {
            Screen::Connection
        };
        self.page = Screen::ProfileSettings;
        self.error = None;
        self.notice = None;
        self.confirm_remove = false;
        self.username
            .update(cx, |input, cx| input.set_value(username, window, cx));
        self.reset_password(window, cx);
        window.focus(&self.focus);
        cx.notify();
    }

    pub(crate) fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.quitting {
            return;
        }
        self.page = if self.page == Screen::ProfileSettings {
            self.settings_return
        } else {
            Screen::Connection
        };
        self.editing = None;
        self.confirm_remove = false;
        self.error = None;
        self.notice = None;
        self.reset_password(window, cx);
        window.focus(&self.focus);
        cx.notify();
    }
}
