//! Windows 11 DWM system backdrops. Avalonia builds its acrylic from the WinUI composition recipe
//! (host backdrop → blur → saturation → luminosity/tint); DWM's acrylic is the system version of
//! the same recipe.

use window_vibrancy::{
    apply_acrylic, apply_mica, apply_tabbed, clear_acrylic, clear_mica, clear_tabbed,
};

use super::{Material, WindowsMaterial};

/// The best backdrop kind this platform shows.
pub(super) const BEST_SUPPORTED: crate::backdrop::BackdropKind =
    crate::backdrop::BackdropKind::Blur;

pub(super) fn apply(window: &winit::window::Window, material: Material) -> Result<(), String> {
    // Clearing a backdrop that was never applied is harmless.
    let _ = clear_acrylic(window);
    let _ = clear_mica(window);
    let _ = clear_tabbed(window);
    // `Opaque` stands for no descriptor: nothing to apply once the old backdrop is gone.
    let Some(os) = material.os_material() else {
        return Ok(());
    };
    let win = os
        .windows
        .ok_or("this material has no Windows form (its `windows` is None)")?;
    let result = match win {
        WindowsMaterial::Acrylic { tint, dark } => {
            if let Some(dark) = dark {
                // `apply_acrylic` takes no dark flag, but `apply_mica` sets the window's
                // immersive-dark attribute before it sets the backdrop, and `clear_mica` does not
                // unset it. So: set the flag through mica, drop mica's backdrop, then apply
                // acrylic. Both results are ignored: on a build without Mica the flag is already
                // set when `apply_mica` errs, and the acrylic call below reports the real outcome.
                // Unverified on a real Windows machine: whether the flag darkens acrylic (D6).
                let _ = apply_mica(window, Some(dark));
                let _ = clear_mica(window);
            }
            apply_acrylic(window, tint.map(|[r, g, b, a]| (r, g, b, a)))
        }
        WindowsMaterial::Mica { dark } => apply_mica(window, dark),
        WindowsMaterial::Tabbed { dark } => apply_tabbed(window, dark),
    };
    result.map_err(|e| e.to_string())
}

/// Windows rounds the window itself (`CornerPreference`), so the radius has nothing to do.
pub(super) fn apply_rounded(
    window: &winit::window::Window,
    material: Material,
    _radius: f64,
) -> Result<(), String> {
    apply(window, material)
}
