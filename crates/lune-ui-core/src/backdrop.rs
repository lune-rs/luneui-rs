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
    /// A material described as data, per OS: a new look needs no toolkit change.
    Custom(OsMaterial),
    /// No OS material.
    Opaque,
}

impl Material {
    /// True for every material except [`Material::Opaque`].
    pub fn is_glass(self) -> bool {
        self != Self::Opaque
    }

    /// The per-OS descriptor this material stands for: `None` for [`Material::Opaque`], the
    /// carried value for [`Material::Custom`], and for the four named forms the descriptor they
    /// have always meant, so an adapter maps one type only.
    pub fn os_material(self) -> Option<OsMaterial> {
        let named = |macos, windows| {
            Some(OsMaterial {
                macos: Some(macos),
                windows: Some(windows),
            })
        };
        let vibrancy = |state| MacosMaterial::Vibrancy {
            effect: VibrancyEffect::Light,
            state,
            appearance: None,
        };
        let liquid = |style| MacosMaterial::Liquid {
            style,
            tint: None,
            opaque: None,
        };
        let acrylic = WindowsMaterial::Acrylic {
            tint: None,
            dark: None,
        };
        match self {
            Self::Opaque => None,
            Self::Custom(m) => Some(m),
            Self::Vibrancy => named(vibrancy(MaterialState::Active), acrylic),
            Self::VibrancyFollowsWindow => named(vibrancy(MaterialState::FollowsWindow), acrylic),
            Self::LiquidClear => named(
                liquid(LiquidStyle::Clear),
                WindowsMaterial::Mica { dark: Some(true) },
            ),
            Self::LiquidRegular => named(
                liquid(LiquidStyle::Regular),
                WindowsMaterial::Tabbed { dark: Some(true) },
            ),
        }
    }
}

/// An OS material as plain data, one optional description per OS. `None` means "no glass on this
/// OS": applying it there is an `Err`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OsMaterial {
    /// What to show on macOS.
    pub macos: Option<MacosMaterial>,
    /// What to show on Windows.
    pub windows: Option<WindowsMaterial>,
}

/// A macOS window material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacosMaterial {
    /// An `NSVisualEffectView` (vibrancy).
    Vibrancy {
        /// The effect view's material.
        effect: VibrancyEffect,
        /// Whether the effect is drawn active, inactive, or follows the window.
        state: MaterialState,
        /// Forces the effect view's appearance; `None` leaves the system's.
        appearance: Option<MaterialAppearance>,
    },
    /// An `NSGlassEffectView` (liquid glass).
    Liquid {
        /// The glass style.
        style: LiquidStyle,
        /// A tint over the glass; `None` leaves it untinted.
        tint: Option<Rgba>,
        /// Whether the glass is opaque; `None` leaves the system's default.
        opaque: Option<bool>,
    },
}

/// The 19 `NSVisualEffectView` materials, under the names the platform crate uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VibrancyEffect {
    /// Follows the system appearance (deprecated by macOS).
    AppearanceBased,
    /// A light material.
    Light,
    /// A dark material.
    Dark,
    /// The window title bar's material.
    Titlebar,
    /// The material of a selection.
    Selection,
    /// The material of a menu.
    Menu,
    /// The material of a popover.
    Popover,
    /// The material of a sidebar.
    Sidebar,
    /// A medium-light material (deprecated by macOS).
    MediumLight,
    /// A very dark material (deprecated by macOS).
    UltraDark,
    /// The material of a header view.
    HeaderView,
    /// The material of a sheet.
    Sheet,
    /// The material of a window background.
    WindowBackground,
    /// The material of a heads-up display window.
    HudWindow,
    /// The material of a full-screen UI.
    FullScreenUI,
    /// The material of a tooltip.
    Tooltip,
    /// The material of a content background.
    ContentBackground,
    /// The material under a window's background.
    UnderWindowBackground,
    /// The material under a page.
    UnderPageBackground,
}

/// When a macOS effect view draws its material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialState {
    /// Always drawn active.
    Active,
    /// Always drawn inactive.
    Inactive,
    /// Follows the window's active state.
    FollowsWindow,
}

/// An appearance forced on a macOS effect view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialAppearance {
    /// The dark (aqua) appearance.
    Dark,
    /// The light (aqua) appearance.
    Light,
}

/// The style of a macOS liquid glass view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiquidStyle {
    /// Clear glass.
    Clear,
    /// Regular glass.
    Regular,
}

/// A Windows window material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsMaterial {
    /// Acrylic.
    Acrylic {
        /// A tint over the acrylic. Honoured only on Windows builds 17763 to 22522; newer builds
        /// ignore it.
        tint: Option<Rgba>,
        /// Asks DWM for the dark (`Some(true)`) or light variant; `None` leaves the system's.
        dark: Option<bool>,
    },
    /// Mica.
    Mica {
        /// Asks DWM for the dark (`Some(true)`) or light variant; `None` leaves the system's.
        dark: Option<bool>,
    },
    /// Tabbed Mica.
    Tabbed {
        /// Asks DWM for the dark (`Some(true)`) or light variant; `None` leaves the system's.
        dark: Option<bool>,
    },
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
