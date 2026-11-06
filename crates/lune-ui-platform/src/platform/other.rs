//! Platforms without a tray: the item is refused, the popover window is a plain window.

use winit::window::Window;

use crate::tray::{TrayError, TrayEvent, TrayImage, TrayRect};

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

/// Never built: [`TrayItem::new`] refuses, so the methods have no value to run on.
#[derive(Debug)]
pub(crate) enum TrayItem {}

impl TrayItem {
    pub(crate) fn new(_image: TrayImage, _tooltip: &str) -> Result<Self, TrayError> {
        Err(TrayError::Unsupported)
    }

    pub(crate) fn set_image(&self, _image: TrayImage) -> Result<(), TrayError> {
        match *self {}
    }

    pub(crate) fn set_tooltip(&self, _tooltip: &str) -> Result<(), TrayError> {
        match *self {}
    }

    pub(crate) fn rect(&self) -> Option<TrayRect> {
        match *self {}
    }

    pub(crate) fn bar_is_dark(&self) -> Option<bool> {
        match *self {}
    }
}

pub(crate) fn set_event_handler(_f: impl Fn(TrayEvent) + Send + Sync + 'static) {}

#[cfg(test)]
mod tests {
    use crate::tray::{Tray, TrayError, TrayImage};

    #[test]
    fn a_tray_is_unsupported_here() {
        let image = TrayImage {
            width: 1,
            height: 1,
            rgba: vec![0; 4],
        };
        assert!(matches!(
            Tray::new(image, "tip"),
            Err(TrayError::Unsupported)
        ));
    }

    #[test]
    fn a_bad_image_is_refused_first() {
        let image = TrayImage {
            width: 1,
            height: 1,
            rgba: vec![],
        };
        assert!(matches!(Tray::new(image, "tip"), Err(TrayError::BadImage)));
    }
}
