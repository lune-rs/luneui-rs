//! The material seam: the OS backdrop behind a transparent window. Plain data and one capability
//! trait; the platform implements [`Backdrop`] for its window type, tests use a null material.

use crate::color::Rgba;

/// One app-drawn acrylic layer, in Avalonia `ExperimentalAcrylicMaterial` terms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassLayer {
    pub tint: Rgba,
    pub tint_opacity: f32,
    pub material_opacity: f32,
    /// How the tint's own luminosity suppresses its opacity.
    pub curve: LuminosityCurve,
}

/// Avalonia's luminosity-to-opacity curve for an acrylic tint (`GetTintOpacityModifier`): a tint
/// that is nearly white or black is made less opaque, so a light tint does not wash the glass out.
/// `mid_point` is the brightness (HSV value, 0..=1) at which the tint keeps `mid_point_max_opacity`
/// of its opacity; towards white the modifier falls to `white_max_opacity`, towards black to
/// `black_max_opacity`, and a saturated tint is suppressed less.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LuminosityCurve {
    /// The brightness at which the modifier is `mid_point_max_opacity`.
    pub mid_point: f64,
    /// The modifier at pure white.
    pub white_max_opacity: f64,
    /// The modifier at `mid_point`.
    pub mid_point_max_opacity: f64,
    /// The modifier at pure black.
    pub black_max_opacity: f64,
}

impl LuminosityCurve {
    /// Avalonia's values (`ExperimentalAcrylicMaterial`).
    pub const AVALONIA: LuminosityCurve = LuminosityCurve {
        mid_point: 0.5,
        white_max_opacity: 0.2,
        mid_point_max_opacity: 0.45,
        black_max_opacity: 0.45,
    };
}

impl Default for LuminosityCurve {
    /// [`Self::AVALONIA`].
    fn default() -> Self {
        Self::AVALONIA
    }
}

/// OS backdrop material requested for the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackdropKind {
    /// Strongest blur the platform offers (macOS vibrancy, Windows acrylic, KDE blur).
    Blur,
    /// Transparent window without blur (Linux compositors without a blur protocol).
    Transparent,
    /// Opaque fallback (no compositor, or the user turned glass off).
    Opaque,
}

/// The material requested for the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Material {
    /// Light vibrancy (macOS) or acrylic (Windows), kept active when the window is unfocused.
    Vibrancy,
    /// Like [`Material::Vibrancy`], but the material follows the OS: it follows the window's active
    /// state (macOS; acrylic on Windows). An explicit choice of the app; the toolkit's own
    /// drawing does not depend on the window's state (R13).
    VibrancyFollowsWindow,
    /// Clear liquid glass (macOS) or Mica (Windows).
    LiquidClear,
    /// Regular liquid glass (macOS) or tabbed Mica (Windows).
    LiquidRegular,
    /// No OS material.
    Opaque,
}

impl Material {
    /// True for every material except [`Material::Opaque`].
    pub fn is_glass(self) -> bool {
        self != Self::Opaque
    }
}

/// Applies an OS backdrop to a native window. One implementation per platform.
pub trait Backdrop {
    /// Best kind this platform/session supports; callers fall back down
    /// `Blur → Transparent → Opaque` like Avalonia's `TransparencyLevelHint`.
    fn best_supported(&self) -> BackdropKind;

    /// Applies `material`, first clearing whatever was applied earlier. `Opaque` always succeeds;
    /// a glass material the platform can't show is an `Err`.
    fn apply(&mut self, material: Material) -> Result<(), String>;
}
