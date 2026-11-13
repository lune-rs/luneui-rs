//! Window helpers on winit: the glass window attributes and the resize edge → winit mapping.

use winit::dpi::LogicalSize;
use winit::window::{ResizeDirection, Window, WindowAttributes, WindowLevel};

use lune_ui_core::window::ResizeEdge;

use crate::platform;

/// Window attributes for a transparent glass window. Sizes are logical pixels. macOS gets a
/// transparent, full-size title bar; Windows gets a borderless window (see `caption_buttons` in `lune_ui_core::window`).
pub fn glass_attributes(
    title: &str,
    size: (f64, f64),
    min_size: (f64, f64),
    visible: bool,
) -> WindowAttributes {
    let attrs = Window::default_attributes()
        .with_title(title)
        .with_inner_size(LogicalSize::new(size.0, size.1))
        .with_min_inner_size(LogicalSize::new(min_size.0, min_size.1))
        .with_transparent(true)
        .with_visible(visible);
    platform::glass(attrs)
}

/// Window attributes for a tray surface's popover window: hidden, borderless, transparent, above
/// other windows and of a fixed size (logical pixels). Pass the window to
/// `lune_ui_platform::popover::PopoverWindow::wrap` before showing it.
pub fn popover_attributes(size: (f64, f64)) -> WindowAttributes {
    let attrs = Window::default_attributes()
        .with_inner_size(LogicalSize::new(size.0, size.1))
        .with_visible(false)
        .with_decorations(false)
        .with_transparent(true)
        .with_window_level(WindowLevel::AlwaysOnTop)
        .with_resizable(false);
    platform::popover(attrs)
}

/// Window attributes for a panel: a borderless, transparent window of a fixed size (logical
/// pixels) with no shadow and no rounded corners, so the app's own drawing and window shape are
/// all that shows. The level is left at its default; a group of panels is raised and kept on top
/// through the platform crate's window group.
pub fn panel_attributes(title: &str, size: (f64, f64), visible: bool) -> WindowAttributes {
    let attrs = Window::default_attributes()
        .with_title(title)
        .with_inner_size(LogicalSize::new(size.0, size.1))
        .with_visible(visible)
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false);
    platform::panel(attrs)
}

/// The winit resize direction of `edge`.
pub fn resize_direction(edge: ResizeEdge) -> ResizeDirection {
    match edge {
        ResizeEdge::North => ResizeDirection::North,
        ResizeEdge::South => ResizeDirection::South,
        ResizeEdge::East => ResizeDirection::East,
        ResizeEdge::West => ResizeDirection::West,
        ResizeEdge::NorthEast => ResizeDirection::NorthEast,
        ResizeEdge::NorthWest => ResizeDirection::NorthWest,
        ResizeEdge::SouthEast => ResizeDirection::SouthEast,
        ResizeEdge::SouthWest => ResizeDirection::SouthWest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::dpi::Size;

    #[test]
    fn panel_attributes_are_a_borderless_transparent_fixed_window() {
        let attrs = panel_attributes("Panel", (275.0, 116.0), true);
        assert_eq!(attrs.title, "Panel");
        assert!(attrs.visible);
        assert!(!attrs.decorations);
        assert!(attrs.transparent);
        assert!(!attrs.resizable);
        assert_eq!(attrs.window_level, WindowLevel::Normal);
        assert_eq!(
            attrs.inner_size,
            Some(Size::Logical(LogicalSize::new(275.0, 116.0)))
        );
        assert!(!panel_attributes("Panel", (275.0, 116.0), false).visible);
    }

    #[test]
    fn popover_attributes_are_a_hidden_borderless_fixed_window_on_top() {
        let attrs = popover_attributes((320.0, 240.0));
        assert!(!attrs.visible);
        assert!(!attrs.decorations);
        assert!(attrs.transparent);
        assert!(!attrs.resizable);
        assert_eq!(attrs.window_level, WindowLevel::AlwaysOnTop);
        assert_eq!(
            attrs.inner_size,
            Some(Size::Logical(LogicalSize::new(320.0, 240.0)))
        );
    }
}
