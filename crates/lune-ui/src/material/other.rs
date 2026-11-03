//! Platforms without an OS backdrop material.

use super::Material;

/// The best backdrop kind this platform shows.
pub(super) const BEST_SUPPORTED: crate::backdrop::BackdropKind =
    crate::backdrop::BackdropKind::Opaque;

pub(super) fn apply(_window: &winit::window::Window, material: Material) -> Result<(), String> {
    if material.is_glass() {
        Err("no OS glass on this platform; use the opaque material".into())
    } else {
        Ok(())
    }
}
