//! Windows: the popover window is a plain borderless window. It takes focus when shown, because a
//! window that does not gets no keys and no focus loss, so the surface's keys and its focus-loss
//! dismiss could not work; Windows' own tray menus take the foreground for the same reason.

use tray_icon::TrayIcon;
use winit::window::Window;

#[derive(Debug)]
pub(crate) struct Panel;

impl Panel {
    pub(crate) fn wrap(_window: &Window) -> Result<Self, String> {
        Ok(Self)
    }

    pub(crate) fn show(&self, window: &Window) {
        window.set_visible(true);
        window.focus_window();
    }

    pub(crate) fn hide(&self, window: &Window) {
        window.set_visible(false);
    }

    pub(crate) fn restore(&self, window: &Window) {
        window.set_visible(false);
    }
}

pub(crate) fn frontmost_app() -> Option<String> {
    None
}

pub(crate) fn bar_is_dark(_item: &TrayIcon) -> Option<bool> {
    None
}
