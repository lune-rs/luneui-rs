//! Window helpers on winit: the glass window attributes and the resize edge → winit mapping.

use winit::dpi::LogicalSize;
use winit::window::{ResizeDirection, Window, WindowAttributes};

use crate::window::ResizeEdge;

#[cfg(target_os = "macos")]
mod platform {
    use winit::platform::macos::WindowAttributesExtMacOS;
    use winit::window::WindowAttributes;

    pub(super) fn glass(attrs: WindowAttributes) -> WindowAttributes {
        attrs
            .with_titlebar_transparent(true)
            .with_fullsize_content_view(true)
            .with_title_hidden(true)
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use winit::platform::windows::{CornerPreference, WindowAttributesExtWindows};
    use winit::window::WindowAttributes;

    pub(super) fn glass(attrs: WindowAttributes) -> WindowAttributes {
        // Borderless so the glass reaches the top edge; the app draws the caption buttons. No
        // redirection bitmap: wgpu presents through DirectComposition, which keeps alpha.
        attrs
            .with_decorations(false)
            .with_no_redirection_bitmap(true)
            .with_undecorated_shadow(true)
            .with_corner_preference(CornerPreference::Round)
            .with_theme(Some(winit::window::Theme::Dark))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform {
    use winit::window::WindowAttributes;

    pub(super) fn glass(attrs: WindowAttributes) -> WindowAttributes {
        attrs
    }
}

/// Window attributes for a transparent glass window. Sizes are logical pixels. macOS gets a
/// transparent, full-size title bar; Windows gets a borderless window (see [`caption_buttons`]).
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
