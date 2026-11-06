use lune_ui_core::input::Keymap;
use winit::window::WindowAttributes;

pub(crate) fn glass(attrs: WindowAttributes) -> WindowAttributes {
    attrs
}

pub(crate) fn popover(attrs: WindowAttributes) -> WindowAttributes {
    attrs
}

/// The keyboard conventions of this platform.
pub(crate) fn keymap() -> Keymap {
    Keymap::PC
}

#[cfg(test)]
mod tests {
    use super::keymap;
    use lune_ui_core::input::Keymap;

    #[test]
    fn the_keymap_is_the_pc_one() {
        assert_eq!(keymap(), Keymap::PC);
    }
}
