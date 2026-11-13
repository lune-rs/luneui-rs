//! Other platforms: no owner or child windows; a group only records what the app asks.

use winit::window::{Window, WindowAttributes};

pub(crate) fn owned_by(attrs: WindowAttributes, _owner: &Window) -> WindowAttributes {
    attrs
}

pub(crate) fn check_member(_leader: &Window, _member: &Window) -> Result<(), String> {
    Ok(())
}

pub(crate) fn set_docked(_leader: &Window, _member: &Window, _docked: bool) -> Result<(), String> {
    Ok(())
}

pub(crate) fn raise(_windows: &[&Window], _docked: &[bool], _front: usize) {}

pub(crate) fn hide_detached(_windows: &[&Window], _docked: &[bool]) {}

pub(crate) fn show_detached(_windows: &[&Window], _docked: &[bool]) {}
