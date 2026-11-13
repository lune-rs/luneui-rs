//! macOS (not run on the development host: compiled only). A member docked to the leader is a
//! child window of the leader, so AppKit moves, orders and minimises it with the leader; a
//! detached member is a plain window the group orders by hand. Every function needs the main
//! thread and does nothing (or answers `Err`) elsewhere.

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSWindow, NSWindowOrderingMode};
use winit::window::{Window, WindowAttributes};

use crate::group::{hidden_with_leader, raise_order};
use crate::platform::macos::ns_window;

/// How far up a parent chain the cycle check looks; AppKit nests child windows a level or two.
const MAX_DEPTH: usize = 64;

/// The native windows, in the group's order, or `None` off the main thread or for a window with
/// no native handle.
fn natives(windows: &[&Window]) -> Option<Vec<Retained<NSWindow>>> {
    MainThreadMarker::new()?;
    windows.iter().map(|w| ns_window(w)).collect()
}

pub(crate) fn owned_by(attrs: WindowAttributes, _owner: &Window) -> WindowAttributes {
    attrs
}

pub(crate) fn check_member(_leader: &Window, _member: &Window) -> Result<(), String> {
    Ok(())
}

pub(crate) fn set_docked(leader: &Window, member: &Window, docked: bool) -> Result<(), String> {
    let natives =
        natives(&[leader, member]).ok_or("not on the main thread, or no native window")?;
    let (leader_ns, member_ns) = (&natives[0], &natives[1]);
    if std::ptr::eq(&**leader_ns, &**member_ns) {
        return Err("the leader cannot be its own member".into());
    }
    let parent = member_ns.parentWindow();
    let attached = parent
        .as_deref()
        .is_some_and(|p| std::ptr::eq(p, &**leader_ns));
    if !docked {
        if attached {
            leader_ns.removeChildWindow(member_ns);
        }
        return Ok(());
    }
    if attached {
        return Ok(());
    }
    if parent.is_some() {
        return Err("the member is already a child of another window".into());
    }
    // The leader must not hang below the member, or attaching would close a loop.
    let mut up = leader_ns.parentWindow();
    for _ in 0..MAX_DEPTH {
        let Some(window) = up else { break };
        if std::ptr::eq(&*window, &**member_ns) {
            return Err("the leader is a child of the member".into());
        }
        up = window.parentWindow();
    }
    if up.is_some() {
        return Err("the leader's parent windows nest too deep to check".into());
    }
    // SAFETY: AppKit's rule for this call is that no cycle of child windows may form. The member
    // has no parent (checked just above), it is not the leader, and the leader is not below it
    // in any parent chain (walked to its end just above), so making the member a child of the
    // leader cannot close a loop. A group attaches members only to its leader, and only ones not
    // already its children (`attached` returned early). Both windows are live: winit keeps them
    // for as long as the `Window`s borrowed here, and this runs on the main thread.
    unsafe { leader_ns.addChildWindow_ordered(member_ns, NSWindowOrderingMode::Above) };
    Ok(())
}

pub(crate) fn raise(windows: &[&Window], docked: &[bool], front: usize) {
    let Some(natives) = natives(windows) else {
        return;
    };
    let Some(front_ns) = natives.get(front) else {
        return;
    };
    let number = front_ns.windowNumber();
    for i in raise_order(front, docked) {
        if let Some(ns) = natives.get(i) {
            ns.orderWindow_relativeTo(NSWindowOrderingMode::Below, number);
        }
    }
}

pub(crate) fn hide_detached(windows: &[&Window], docked: &[bool]) {
    let Some(natives) = natives(windows) else {
        return;
    };
    for i in hidden_with_leader(docked) {
        if let Some(ns) = natives.get(i) {
            ns.orderOut(None);
        }
    }
}

pub(crate) fn show_detached(windows: &[&Window], docked: &[bool]) {
    let Some(natives) = natives(windows) else {
        return;
    };
    for i in hidden_with_leader(docked) {
        if let Some(ns) = natives.get(i) {
            // Ordered front without becoming key, so the app's key window stays as it was.
            ns.orderFront(None);
        }
    }
}
