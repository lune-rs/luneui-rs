//! Windows: members are owned windows of the leader (created with it as their owner). The OS
//! then shows one taskbar button for the group, and raises, keeps on top and minimises it as one,
//! so nothing more is done here. An owned window does not follow its owner's moves: the app moves
//! a docked set itself.

use windows_sys::Win32::UI::WindowsAndMessaging::{GW_OWNER, GetWindow};
use winit::platform::windows::WindowAttributesExtWindows;
use winit::window::{Window, WindowAttributes};

use crate::platform::desktop::hwnd;

pub(crate) fn owned_by(attrs: WindowAttributes, owner: &Window) -> WindowAttributes {
    match hwnd(owner) {
        Some(h) => attrs.with_owner_window(h as isize),
        None => attrs,
    }
}

pub(crate) fn check_member(leader: &Window, member: &Window) -> Result<(), String> {
    let (Some(leader), Some(member)) = (hwnd(leader), hwnd(member)) else {
        return Err("a window has no native handle".into());
    };
    // SAFETY: the call takes a window handle from winit and a constant, and returns a handle or
    // null; nothing is written.
    let owner = unsafe { GetWindow(member, GW_OWNER) };
    if owner == leader {
        Ok(())
    } else {
        Err(
            "a member must be created with WindowGroup::member_attributes (owned by the leader)"
                .into(),
        )
    }
}

pub(crate) fn set_docked(_leader: &Window, _member: &Window, _docked: bool) -> Result<(), String> {
    Ok(())
}

pub(crate) fn raise(_windows: &[&Window], _docked: &[bool], _front: usize) {}

pub(crate) fn hide_detached(_windows: &[&Window], _docked: &[bool]) {}

pub(crate) fn show_detached(_windows: &[&Window], _docked: &[bool]) {}
