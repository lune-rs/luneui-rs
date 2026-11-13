//! Windows: desktop units are the physical px winit uses (the process is per-monitor DPI aware, so
//! winit, `GetCursorPos` and the work areas share one space).

use std::mem::size_of;

use lune_ui_core::desktop::{DesktopPoint, DesktopRect, Monitor};
use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, HMONITOR, MONITORINFO};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BeginDeferWindowPos, DeferWindowPos, EndDeferWindowPos, GA_ROOT, GetAncestor, GetCursorPos,
    MONITORINFOF_PRIMARY, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER,
    WindowFromPoint,
};
use winit::dpi::PhysicalPosition;
use winit::platform::windows::MonitorHandleExtWindows;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

/// The native handle of `window`.
fn hwnd(window: &Window) -> Option<HWND> {
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(h.hwnd.get() as HWND),
        _ => None,
    }
}

fn size_units(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

pub(crate) fn monitors(window: &Window) -> Vec<Monitor> {
    window
        .available_monitors()
        .map(|m| {
            let (p, s) = (m.position(), m.size());
            let bounds = DesktopRect::new(p.x, p.y, size_units(s.width), size_units(s.height));
            let mut info = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..MONITORINFO::default()
            };
            // SAFETY: `info` is a live MONITORINFO with `cbSize` set, which is all the call
            // writes through; the handle came from winit, and a stale one only makes it fail.
            let ok = unsafe { GetMonitorInfoW(m.hmonitor() as HMONITOR, &mut info) } != 0;
            let work = if ok {
                let r = info.rcWork;
                DesktopRect::new(
                    r.left,
                    r.top,
                    r.right.saturating_sub(r.left),
                    r.bottom.saturating_sub(r.top),
                )
            } else {
                bounds
            };
            Monitor {
                bounds,
                work,
                units_per_logical: m.scale_factor() as f32,
                primary: ok && info.dwFlags & MONITORINFOF_PRIMARY != 0,
            }
        })
        .collect()
}

pub(crate) fn pointer() -> Option<DesktopPoint> {
    let mut p = POINT::default();
    // SAFETY: `p` is a live POINT the call fills in.
    let ok = unsafe { GetCursorPos(&mut p) } != 0;
    ok.then(|| DesktopPoint::new(p.x, p.y))
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
    if moves.is_empty() {
        return Ok(());
    }
    // Every handle first: a window without one fails before a batch is open.
    let targets = moves
        .iter()
        .map(|(w, at)| hwnd(w).map(|h| (h, *at)))
        .collect::<Option<Vec<_>>>()
        .ok_or("a window has no native handle")?;
    let count = i32::try_from(targets.len()).map_err(|_| "too many windows to move")?;
    // SAFETY: the call takes a count and returns a batch handle or null.
    let mut batch = unsafe { BeginDeferWindowPos(count) };
    if batch.is_null() {
        return Err("BeginDeferWindowPos failed".into());
    }
    for (h, at) in targets {
        // SAFETY: `batch` is the live batch from the last call, and `h` a window handle from
        // winit. The call returns the batch to use next, or null after freeing it.
        batch = unsafe {
            DeferWindowPos(
                batch,
                h,
                std::ptr::null_mut(),
                at.x,
                at.y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )
        };
        if batch.is_null() {
            // The OS has freed the batch already: ending it would be a second free.
            return Err("DeferWindowPos failed".into());
        }
    }
    // SAFETY: `batch` is the live batch from the last `DeferWindowPos`, ended once.
    if unsafe { EndDeferWindowPos(batch) } == 0 {
        return Err("EndDeferWindowPos failed".into());
    }
    Ok(())
}

pub(crate) fn units_per_logical(window: &Window) -> f32 {
    window.scale_factor() as f32
}

pub(crate) fn window_at(window: &Window, at: DesktopPoint) -> Option<bool> {
    let own = hwnd(window)?;
    // SAFETY: both calls take plain values; a null result (nothing at the point) is compared as is.
    let top = unsafe { GetAncestor(WindowFromPoint(POINT { x: at.x, y: at.y }), GA_ROOT) };
    Some(top == own)
}
