//! The OS code of window groups, one file per platform; the safe API is `crate::group`.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod other;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub(crate) use macos::{check_member, hide_detached, owned_by, raise, set_docked, show_detached};
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) use other::{check_member, hide_detached, owned_by, raise, set_docked, show_detached};
#[cfg(target_os = "windows")]
pub(crate) use windows::{check_member, hide_detached, owned_by, raise, set_docked, show_detached};
