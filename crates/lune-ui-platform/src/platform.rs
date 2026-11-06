//! The OS code of the tray item and the popover window, one file per platform. macOS and Windows
//! share the `tray-icon` wrapper in `platform/item.rs`.

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod item;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod other;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) use item::{TrayItem, set_event_handler};
#[cfg(target_os = "macos")]
pub(crate) use macos::{Panel, bar_is_dark, frontmost_app};
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) use other::{Panel, TrayItem, frontmost_app, set_event_handler};
#[cfg(target_os = "windows")]
pub(crate) use windows::{Panel, bar_is_dark, frontmost_app};
