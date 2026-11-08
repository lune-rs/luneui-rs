//! The near-black look as one value. This file is the one place to tune it: every number that
//! shapes the look is a constant below, and each constant's doc comment says which way raising
//! it moves the look.
//!
//! - `WINDOW_BLACK_MAX`, `PANEL_BLACK_MAX`, `CARD_BLACK_MAX`: raise for blacker glass with less
//!   backdrop (less blur) showing through; lower for lighter, blurrier glass.
//! - `MATERIAL_OPACITY`: the grey layer under the black tint. Raise for lighter glass with less
//!   blur; lower for darker glass with more blur.
//! - `MATERIAL`: the OS material the window asks for. It is for a dark system appearance.
//! - `HEADER_TINT`: the tab header's band over the card. Raise for a lighter header.
//!
//! The tests pin the approved values. A deliberate retune edits them too.

use super::{DARK, Theme, ThemeGlass};
use crate::GlassLayer;
use crate::backdrop::{
    LuminosityCurve, MacosMaterial, Material, MaterialState, OsMaterial, VibrancyEffect,
    WindowsMaterial,
};
use crate::color::Rgba;

/// The tint under every glass layer. Black, so the glass is neutral; a coloured tint would
/// change the hue, not the darkness.
pub const TINT: Rgba = [0, 0, 0, 255];

/// The grey luminosity layer under the black tint, in material opacity. Raise it for lighter
/// glass with less blur showing; lower it for darker glass with more blur showing.
pub const MATERIAL_OPACITY: f32 = 0.1;

/// The window layer's tint opacity at pure black. Raise it for blacker glass with less blur;
/// lower it for lighter glass with more blur.
pub const WINDOW_BLACK_MAX: f64 = 0.60;

/// The panel layer's tint opacity at pure black. Raise it for blacker panels with less blur;
/// lower it for lighter panels with more blur.
pub const PANEL_BLACK_MAX: f64 = 0.55;

/// The card layer's tint opacity at pure black. Raise it for a blacker card with less blur;
/// lower it for a lighter card with more blur.
pub const CARD_BLACK_MAX: f64 = 0.65;

/// The window's glass layer, the window background. Its tint sits between the panel's and the
/// card's.
pub const WINDOW: GlassLayer = GlassLayer {
    tint: TINT,
    tint_opacity: 1.0,
    material_opacity: MATERIAL_OPACITY,
    curve: LuminosityCurve {
        black_max_opacity: WINDOW_BLACK_MAX,
        ..LuminosityCurve::AVALONIA
    },
};

/// The panel's glass layer.
pub const PANEL: GlassLayer = GlassLayer {
    tint: TINT,
    tint_opacity: 1.0,
    material_opacity: MATERIAL_OPACITY,
    curve: LuminosityCurve {
        black_max_opacity: PANEL_BLACK_MAX,
        ..LuminosityCurve::AVALONIA
    },
};

/// The card's glass layer. Its tint is the most opaque of the three, so the card shows the least
/// backdrop.
pub const CARD: GlassLayer = GlassLayer {
    tint: TINT,
    tint_opacity: 1.0,
    material_opacity: MATERIAL_OPACITY,
    curve: LuminosityCurve {
        black_max_opacity: CARD_BLACK_MAX,
        ..LuminosityCurve::AVALONIA
    },
};

/// The three glass layers of the near-black look.
pub const GLASS: ThemeGlass = ThemeGlass {
    window: WINDOW,
    panel: PANEL,
    card: CARD,
};

/// The dark theme with the near-black glass. Every other value is [`DARK`]'s.
pub const THEME: Theme = Theme {
    glass: GLASS,
    ..DARK
};

/// The OS material the window asks for: macOS `UltraDark` vibrancy, active; Windows acrylic with a
/// black tint of alpha 160, dark. The Windows tint is honoured only on builds 17763 to 22522; on
/// later builds the layers above do the darkening, and `dark` asks for the dark variant.
pub const MATERIAL: Material = Material::Custom(OsMaterial {
    macos: Some(MacosMaterial::Vibrancy {
        effect: VibrancyEffect::UltraDark,
        state: MaterialState::Active,
        appearance: None,
    }),
    windows: Some(WindowsMaterial::Acrylic {
        tint: Some([0, 0, 0, 160]),
        dark: Some(true),
    }),
});

/// The tab header's band, drawn over the card: a 4% white wash. Raise it for a lighter header;
/// 0 alpha removes it, so the header matches the card.
pub const HEADER_TINT: Rgba = [0xFF, 0xFF, 0xFF, 0x0A];

#[cfg(test)]
mod tests {
    use super::{CARD, GLASS, HEADER_TINT, PANEL, THEME, WINDOW};
    use crate::GlassLayer;
    use crate::backdrop::LuminosityCurve;
    use crate::theme::{AcrylicLevel, DARK, Theme};

    /// A layer's premultiplied output composited over an opaque grey backdrop (0..=1).
    fn over(p: [f32; 4], backdrop: f32) -> [f32; 3] {
        [p[0], p[1], p[2]].map(|c| c + (1.0 - p[3]) * backdrop)
    }

    /// A 0..=1 channel as the byte it draws as.
    fn hex(v: f32) -> u8 {
        (v * 255.0).round() as u8
    }

    #[test]
    fn near_black_approved_values_are_pinned_retune_them_here() {
        let curve = |black_max_opacity| LuminosityCurve {
            black_max_opacity,
            ..LuminosityCurve::AVALONIA
        };
        let layer = |black_max| GlassLayer {
            tint: [0, 0, 0, 255],
            tint_opacity: 1.0,
            material_opacity: 0.1,
            curve: curve(black_max),
        };
        assert_eq!(WINDOW, layer(0.60));
        assert_eq!(PANEL, layer(0.55));
        assert_eq!(CARD, layer(0.65));
        assert_eq!(GLASS.window, WINDOW);
        assert_eq!(GLASS.panel, PANEL);
        assert_eq!(GLASS.card, CARD);
        assert_eq!(HEADER_TINT, [0xFF, 0xFF, 0xFF, 0x0A]);
    }

    #[test]
    fn near_black_over_a_ten_percent_backdrop_reads_11_13_0f() {
        // The backdrop share in percent: 36.1, 40.7, 31.8 (0.2 points is the 0.002 tolerance).
        let want = [
            (WINDOW, 0x11, 36.1),
            (PANEL, 0x13, 40.7),
            (CARD, 0x0F, 31.8),
        ];
        for (layer, byte, share_percent) in want {
            let p = layer.premultiplied();
            let out = over(p, 0.1);
            assert_eq!(hex(out[0]), byte);
            assert!(
                out.iter().all(|&v| hex(v) == hex(out[0])),
                "not neutral: {out:?}"
            );
            let backdrop_share = f64::from(1.0 - p[3]) * 100.0;
            assert!(
                (backdrop_share - share_percent).abs() <= 0.2,
                "share {backdrop_share} not within 0.2 of {share_percent}"
            );
        }
    }

    #[test]
    fn near_black_card_over_a_full_backdrop_is_at_least_0x50() {
        let out = over(CARD.premultiplied(), 1.0);
        assert!(hex(out[0]) >= 0x50, "card over 100%: {:#x}", hex(out[0]));
    }

    #[test]
    fn near_black_opaque_is_opaque_and_under_0x10() {
        let opaque = THEME.with_acrylic(AcrylicLevel::Opaque);
        for layer in [opaque.glass.window, opaque.glass.panel, opaque.glass.card] {
            let p = layer.premultiplied();
            assert!((p[3] - 1.0).abs() < 1e-5, "alpha {}", p[3]);
            for c in p.iter().take(3) {
                assert!(hex(*c) < 0x10, "channel {:#x} not below 0x10", hex(*c));
            }
        }
    }

    #[test]
    fn near_black_theme_is_dark_except_its_glass() {
        assert_eq!(
            Theme {
                glass: DARK.glass,
                ..THEME
            },
            DARK
        );
        assert_ne!(THEME.glass, DARK.glass);
    }

    #[test]
    fn near_black_header_tint_lifts_the_card_by_six_levels() {
        let card: f32 = over(CARD.premultiplied(), 0.1)[0];
        let [r, _, _, a] = HEADER_TINT;
        let alpha = f32::from(a) / 255.0;
        let header = f32::from(r) / 255.0 * alpha + card * (1.0 - alpha);
        assert!(
            i32::from(hex(header)) >= i32::from(hex(card)) + 6,
            "header {:#x} card {:#x}",
            hex(header),
            hex(card)
        );
    }
}
