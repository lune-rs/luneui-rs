//! The OS code of the desktop functions, one file per platform; the safe API is `crate::desktop`.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod other;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub(crate) use macos::{
    monitors, move_windows, pointer, set_window_origin, units_per_logical, window_at, window_frame,
};
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) use other::{
    monitors, move_windows, pointer, set_window_origin, units_per_logical, window_at, window_frame,
};
#[cfg(target_os = "windows")]
pub(crate) use windows::{
    monitors, move_windows, pointer, set_window_origin, units_per_logical, window_at, window_frame,
};
