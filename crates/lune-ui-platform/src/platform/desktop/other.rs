//! Other platforms: winit's physical values; there is no work-area, pointer or hit-test query.

use lune_ui_core::desktop::{DesktopPoint, DesktopRect, Monitor, WindowShape};
use winit::dpi::PhysicalPosition;
use winit::window::Window;

use crate::desktop::ShapeMode;

fn size_units(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

pub(crate) fn monitors(window: &Window) -> Vec<Monitor> {
    let primary = window.primary_monitor();
    window
        .available_monitors()
        .map(|m| {
            let (p, s) = (m.position(), m.size());
            let bounds = DesktopRect::new(p.x, p.y, size_units(s.width), size_units(s.height));
            Monitor {
                bounds,
                work: bounds,
                units_per_logical: m.scale_factor() as f32,
                primary: primary.as_ref() == Some(&m),
            }
        })
        .collect()
}

pub(crate) fn pointer() -> Option<DesktopPoint> {
    None
}

pub(crate) fn window_frame(window: &Window) -> Option<DesktopRect> {
    let p = window.outer_position().ok()?;
    let s = window.outer_size();
    Some(DesktopRect::new(
        p.x,
        p.y,
        size_units(s.width),
        size_units(s.height),
    ))
}

pub(crate) fn set_window_origin(window: &Window, at: DesktopPoint) {
    window.set_outer_position(PhysicalPosition::new(at.x, at.y));
}

pub(crate) fn move_windows(moves: &[(&Window, DesktopPoint)]) -> Result<(), String> {
    for (window, at) in moves {
        set_window_origin(window, *at);
    }
    Ok(())
}

pub(crate) fn units_per_logical(window: &Window) -> f32 {
    window.scale_factor() as f32
}

pub(crate) fn window_at(_window: &Window, _at: DesktopPoint) -> Option<bool> {
    None
}

/// No window shape here: the whole frame takes the pointer.
pub(crate) const SHAPE_MODE: ShapeMode = ShapeMode::Unsupported;

pub(crate) fn set_window_shape(
    _window: &Window,
    _shape: Option<&WindowShape>,
    _scale: f32,
) -> Result<ShapeMode, String> {
    Ok(SHAPE_MODE)
}
