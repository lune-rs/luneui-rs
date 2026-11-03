//! Per-platform choices of the GPU instance: the backend and the swapchain presentation. The
//! bodies are in `platform/`, one file per platform, selected by `cfg`.

#[cfg(not(target_os = "windows"))]
mod other;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(not(target_os = "windows"))]
pub(crate) use other::configure;
#[cfg(target_os = "windows")]
pub(crate) use windows::configure;
