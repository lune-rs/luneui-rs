use lune_ui_core::input::Keymap;
use winit::platform::macos::WindowAttributesExtMacOS;
use winit::window::WindowAttributes;

pub(crate) fn glass(attrs: WindowAttributes) -> WindowAttributes {
    attrs
        .with_titlebar_transparent(true)
        .with_fullsize_content_view(true)
        .with_title_hidden(true)
}

pub(crate) fn popover(attrs: WindowAttributes) -> WindowAttributes {
    attrs.with_has_shadow(true).with_accepts_first_mouse(true)
}

pub(crate) fn panel(attrs: WindowAttributes) -> WindowAttributes {
    attrs.with_has_shadow(false).with_accepts_first_mouse(true)
}

/// The keyboard conventions of this platform.
pub(crate) fn keymap() -> Keymap {
    Keymap::MAC
}

#[cfg(test)]
mod tests {
    use super::keymap;
    use crate::input::Translator;
    use lune_ui_core::input::Keymap;

    #[test]
    fn the_keymap_is_the_mac_one() {
        assert_eq!(keymap(), Keymap::MAC);
    }

    /// A path that is not UTF-8 passes through the file events unchanged: the core carries the
    /// `PathBuf` as the OS gave it and never converts it to text.
    #[test]
    fn a_path_that_is_not_utf8_passes_through_a_file_event() {
        use lune_ui_core::input::Event;
        use std::os::unix::ffi::OsStrExt;
        use std::path::PathBuf;
        use winit::dpi::PhysicalSize;
        use winit::event::WindowEvent;

        let bytes = b"/tmp/caf\xe9-\xff.txt";
        let path = PathBuf::from(std::ffi::OsStr::from_bytes(bytes));
        assert!(path.to_str().is_none(), "the fixture is not UTF-8");
        let mut t = Translator::new(PhysicalSize::new(100, 100), 1.0);
        t.window_event(&WindowEvent::DroppedFile(path.clone()));
        let input = t.take(std::time::Duration::ZERO);
        match input.events.as_slice() {
            [Event::FileDrop { path: got, .. }] => assert_eq!(got.as_os_str().as_bytes(), bytes),
            other => panic!("expected one file drop, got {other:?}"),
        }
    }
}
