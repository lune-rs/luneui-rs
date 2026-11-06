//! Per-platform window attributes (the glass look and the popover window of each platform) and keyboard conventions. The bodies are in
//! `platform/`, one file per platform, selected by `cfg`.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod other;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub(crate) use macos::{glass, keymap, popover};
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) use other::{glass, keymap, popover};
#[cfg(target_os = "windows")]
pub(crate) use windows::{glass, keymap, popover};
