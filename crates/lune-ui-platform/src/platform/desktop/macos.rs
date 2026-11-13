//! macOS (not run on the development host: compiled only). Desktop units are points from the
//! top-left of the primary display (`NSScreen.screens[0]`), y down. winit's own "physical" values
//! are points times each window's or monitor's own backing scale, so they are not one space on
//! monitors of different scales: the frame is divided by its window's scale, and the screens and
//! the pointer are read from AppKit. Every function needs the main thread and answers `None` or an
//! empty list elsewhere.

use lune_ui_core::desktop::{DesktopPoint, DesktopRect, Monitor};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSEvent, NSScreen, NSWindow};
use winit::dpi::LogicalPosition;
use winit::window::Window;

use crate::desktop::{flip_rect, flip_y, round_units};
use crate::platform::macos::ns_window;

/// The height of the primary display in points: the line the y axis is flipped about.
fn primary_height(mtm: MainThreadMarker) -> Option<f64> {
    let screens = NSScreen::screens(mtm);
    Some(screens.iter().next()?.frame().size.height)
}

pub(crate) fn monitors(_window: &Window) -> Vec<Monitor> {
    let Some(mtm) = MainThreadMarker::new() else {
        return Vec::new();
    };
    let screens = NSScreen::screens(mtm);
    let Some(height) = screens.iter().next().map(|s| s.frame().size.height) else {
        return Vec::new();
    };
    screens
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let (f, v) = (s.frame(), s.visibleFrame());
            Monitor {
                bounds: flip_rect(f.origin.x, f.origin.y, f.size.width, f.size.height, height),
                work: flip_rect(v.origin.x, v.origin.y, v.size.width, v.size.height, height),
                units_per_logical: 1.0,
                primary: i == 0,
            }
        })
        .collect()
}

pub(crate) fn pointer() -> Option<DesktopPoint> {
    let mtm = MainThreadMarker::new()?;
    let height = primary_height(mtm)?;
    let p = NSEvent::mouseLocation();
    Some(DesktopPoint::new(round_units(p.x), flip_y(p.y, height)))
}

pub(crate) fn window_frame(window: &Window) -> Option<DesktopRect> {
    let scale = window.scale_factor();
    let p = window.outer_position().ok()?;
    let s = window.outer_size();
    Some(DesktopRect::new(
        round_units(f64::from(p.x) / scale),
        round_units(f64::from(p.y) / scale),
        round_units(f64::from(s.width) / scale),
        round_units(f64::from(s.height) / scale),
    ))
}

pub(crate) fn set_window_origin(window: &Window, at: DesktopPoint) {
    window.set_outer_position(LogicalPosition::new(f64::from(at.x), f64::from(at.y)));
}

pub(crate) fn move_windows(moves: &[(&Window, DesktopPoint)]) -> Result<(), String> {
    for (window, at) in moves {
        set_window_origin(window, *at);
    }
    Ok(())
}

pub(crate) fn units_per_logical(_window: &Window) -> f32 {
    1.0
}

pub(crate) fn window_at(window: &Window, at: DesktopPoint) -> Option<bool> {
    let mtm = MainThreadMarker::new()?;
    let height = primary_height(mtm)?;
    let own = ns_window(window)?;
    // A point of the right type, made by changing the pointer's own (the type is AppKit's).
    let mut point = NSEvent::mouseLocation();
    point.x = f64::from(at.x);
    point.y = height - f64::from(at.y);
    let number = NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(point, 0, mtm);
    Some(number == own.windowNumber())
}
