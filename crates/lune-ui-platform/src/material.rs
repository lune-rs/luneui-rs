//! OS backdrop material behind a transparent window.
//!
//! The effect is applied by the platform: `NSVisualEffectView` / `NSGlassEffectView` on macOS,
//! DWM system backdrops on Windows, nothing elsewhere. The platform code lives in `#[cfg]`
//! submodules; this file holds the shared entry point and the [`Backdrop`] implementation for a winit
//! window. The request types are in [`crate::backdrop`].

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod other;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
use macos::BEST_SUPPORTED;
#[cfg(target_os = "macos")]
use macos::apply as apply_os;
#[cfg(target_os = "macos")]
use macos::apply_rounded as apply_rounded_os;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use other::BEST_SUPPORTED;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use other::apply as apply_os;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use other::apply_rounded as apply_rounded_os;
#[cfg(target_os = "windows")]
use windows::BEST_SUPPORTED;
#[cfg(target_os = "windows")]
use windows::apply as apply_os;
#[cfg(target_os = "windows")]
use windows::apply_rounded as apply_rounded_os;

use crate::backdrop::{Backdrop, BackdropKind};
pub use crate::backdrop::{
    LiquidStyle, MacosMaterial, Material, MaterialAppearance, MaterialState, OsMaterial,
    VibrancyEffect, WindowsMaterial,
};

/// Applies `material` to `window`, first clearing any material applied earlier.
///
/// `Opaque` is `Ok` everywhere. A glass material on a platform without OS glass is an `Err`.
pub fn apply(window: &winit::window::Window, material: Material) -> Result<(), String> {
    apply_os(window, material)
}

/// [`apply`], with the OS glass view's corners rounded to `radius` logical pixels, for a window whose
/// card is rounded. On macOS the glass view follows the radius; elsewhere the radius is ignored: the
/// window's own corners are the OS's, and this is [`apply`].
pub fn apply_rounded(
    window: &winit::window::Window,
    material: Material,
    radius: f64,
) -> Result<(), String> {
    apply_rounded_os(window, material, radius)
}

/// A winit window as the OS backdrop: [`Backdrop`] over [`apply`].
#[derive(Debug)]
pub struct WindowBackdrop<'w>(pub &'w winit::window::Window);

impl Backdrop for WindowBackdrop<'_> {
    fn best_supported(&self) -> BackdropKind {
        BEST_SUPPORTED
    }

    fn apply(&mut self, material: Material) -> Result<(), String> {
        apply(self.0, material)
    }
}
