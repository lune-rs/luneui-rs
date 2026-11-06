use lune_ui_core::input::Keymap;
use winit::platform::windows::{CornerPreference, WindowAttributesExtWindows};
use winit::window::WindowAttributes;

pub(crate) fn glass(attrs: WindowAttributes) -> WindowAttributes {
    // Borderless so the glass reaches the top edge; the app draws the caption buttons. No
    // redirection bitmap: wgpu presents through DirectComposition, which keeps alpha.
    attrs
        .with_decorations(false)
        .with_no_redirection_bitmap(true)
        .with_undecorated_shadow(true)
        .with_corner_preference(CornerPreference::Round)
        .with_theme(Some(winit::window::Theme::Dark))
}

pub(crate) fn popover(attrs: WindowAttributes) -> WindowAttributes {
    // No redirection bitmap, for the same reason as `glass`: the transparent card keeps its alpha.
    attrs
        .with_skip_taskbar(true)
        .with_corner_preference(CornerPreference::Round)
        .with_undecorated_shadow(true)
        .with_no_redirection_bitmap(true)
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
