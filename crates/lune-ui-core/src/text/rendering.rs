//! How glyphs are rasterized: the [`TextRendering`] setting an app passes per frame, the
//! [`StemDarkening`] it holds, and the [`GlyphParams`] a single glyph is rasterized with. Pure
//! values; the rasterizer that reads them lives in an adapter.

/// Stem darkening: how far an outline glyph is grown on each side before it is filled, so thin
/// stems do not look washed out at small sizes. The growth is `factors` in px per ppem (pixels per
/// em), capped at `max_px`, and none above `max_ppem`. Each pair is x then y.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StemDarkening {
    /// Growth a side in px per ppem, x then y.
    pub factors: [f32; 2],
    /// The most a side grows, in px, x then y. [`Self::effective`] holds it to [`Self::MAX_PX`].
    pub max_px: [f32; 2],
    /// No growth above this size, in ppem.
    pub max_ppem: f32,
}

impl StemDarkening {
    /// The numbers Pathfinder uses (servo/pathfinder, `content/src/effects.rs`, "should match
    /// macOS 10.13"): 0.0121 px per ppem in x, 1.25 times that in y, at most 0.3 px a side, none
    /// above 72 ppem.
    pub const MACOS: StemDarkening = StemDarkening {
        factors: [0.0121, 0.0121 * 1.25],
        max_px: [0.3, 0.3],
        max_ppem: 72.0,
    };
    /// The most a side may grow, in px. A mask is at most two of these wider than its glyph, so no
    /// value can make a huge one.
    pub const MAX_PX: f32 = 1.0;

    /// The value as the rasterizer uses it. Each of the five numbers that is not finite or is
    /// negative is the matching number of [`Self::MACOS`]; a `max_px` above [`Self::MAX_PX`] is
    /// `MAX_PX`; `-0.0` is `0.0`. Nothing else changes.
    pub fn effective(self) -> StemDarkening {
        let m = Self::MACOS;
        // Adding 0.0 turns -0.0 into +0.0.
        let fix = |v: f32, default: f32| {
            if v.is_finite() && v >= 0.0 {
                v + 0.0
            } else {
                default
            }
        };
        StemDarkening {
            factors: [
                fix(self.factors[0], m.factors[0]),
                fix(self.factors[1], m.factors[1]),
            ],
            max_px: [
                fix(self.max_px[0], m.max_px[0]).min(Self::MAX_PX),
                fix(self.max_px[1], m.max_px[1]).min(Self::MAX_PX),
            ],
            max_ppem: fix(self.max_ppem, m.max_ppem),
        }
    }

    /// How far a glyph of `ppem` grows on each side, in px, x then y, from the effective value.
    /// Zero when `ppem` is not finite, not positive or above `max_ppem`.
    pub fn at(self, ppem: f32) -> [f32; 2] {
        let e = self.effective();
        if !ppem.is_finite() || ppem <= 0.0 || ppem > e.max_ppem {
            return [0.0, 0.0];
        }
        [
            (e.factors[0] * ppem).min(e.max_px[0]),
            (e.factors[1] * ppem).min(e.max_px[1]),
        ]
    }
}

impl Default for StemDarkening {
    /// [`StemDarkening::MACOS`].
    fn default() -> Self {
        Self::MACOS
    }
}

/// How the toolkit rasterizes text glyphs. An app passes it in every frame's parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextRendering {
    /// Whether the smoothing look is on. When false, glyphs are rasterized as the toolkit always
    /// did (hinted, not darkened) and `hinting` and `stem_darkening` are ignored.
    pub smoothing: bool,
    /// The toolkit's rasterization hinting: glyph outlines snapped to the pixel grid when drawn.
    /// It is not the text shaper's metrics hinting (cosmic-text's `Buffer::set_hinting`, which the
    /// toolkit never sets).
    pub hinting: bool,
    /// How far outline glyphs are grown before they are filled.
    pub stem_darkening: StemDarkening,
}

impl TextRendering {
    /// The default: smoothing on, no hinting, the macOS stem darkening.
    pub const SMOOTH: TextRendering = TextRendering {
        smoothing: true,
        hinting: false,
        stem_darkening: StemDarkening::MACOS,
    };
    /// The look before smoothing existed: hinted, no darkening.
    pub const OFF: TextRendering = TextRendering {
        smoothing: false,
        hinting: true,
        stem_darkening: StemDarkening::MACOS,
    };

    /// The setting as the rasterizer uses it. Smoothing off is exactly [`Self::OFF`]; otherwise
    /// the stem darkening is its [`StemDarkening::effective`] value.
    pub fn effective(self) -> TextRendering {
        if !self.smoothing {
            return Self::OFF;
        }
        TextRendering {
            smoothing: true,
            hinting: self.hinting,
            stem_darkening: self.stem_darkening.effective(),
        }
    }

    /// What a glyph of `ppem` is rasterized with under this setting: [`GlyphParams::OFF`] when
    /// smoothing is off, else the hinting flag and the darkening at `ppem`.
    pub fn glyph_params(self, ppem: f32) -> GlyphParams {
        let e = self.effective();
        if !e.smoothing {
            return GlyphParams::OFF;
        }
        GlyphParams::new(e.hinting, e.stem_darkening.at(ppem))
    }
}

impl Default for TextRendering {
    /// [`TextRendering::SMOOTH`].
    fn default() -> Self {
        Self::SMOOTH
    }
}

/// What one glyph is rasterized with: its hinting and how far it grows on each side. Built from a
/// [`TextRendering`] and the glyph's size, it keys the glyph atlas, so two settings that give a
/// glyph the same values share one raster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphParams {
    hinting: bool,
    /// The growth a side in px, x then y, as `f32` bits (so the value hashes and compares exactly).
    dilation: [u32; 2],
}

impl GlyphParams {
    /// Hinted, no growth: what the toolkit rasterized before smoothing existed.
    pub const OFF: GlyphParams = GlyphParams {
        hinting: true,
        dilation: [0, 0],
    };

    /// Params with `hinting` and a growth of `dilation` px a side, x then y. A component that is
    /// not finite or not positive is 0; one above [`StemDarkening::MAX_PX`] is `MAX_PX`.
    pub fn new(hinting: bool, dilation: [f32; 2]) -> GlyphParams {
        let bits = |v: f32| {
            if v.is_finite() && v > 0.0 {
                v.min(StemDarkening::MAX_PX).to_bits()
            } else {
                0
            }
        };
        GlyphParams {
            hinting,
            dilation: [bits(dilation[0]), bits(dilation[1])],
        }
    }

    /// Whether the outline is hinted.
    pub fn hinting(self) -> bool {
        self.hinting
    }

    /// The growth a side in px, x then y.
    pub fn dilation(self) -> [f32; 2] {
        [
            f32::from_bits(self.dilation[0]),
            f32::from_bits(self.dilation[1]),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NON_VALUES: [f32; 3] = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY];

    fn dil(r: TextRendering, ppem: f32) -> [f32; 2] {
        r.glyph_params(ppem).dilation()
    }

    #[test]
    fn macos_darkening_at_the_listed_sizes() {
        assert_eq!(
            TextRendering::SMOOTH.glyph_params(26.0),
            GlyphParams::new(false, [0.3, 0.3]),
            "26 ppem hits the cap"
        );
        let d = dil(TextRendering::SMOOTH, 13.0);
        assert!((d[0] - 0.1573).abs() < 1e-6, "13 ppem x: {}", d[0]);
        assert!((d[1] - 0.196_625).abs() < 1e-6, "13 ppem y: {}", d[1]);
        assert_eq!(dil(TextRendering::SMOOTH, 72.0), [0.3, 0.3], "72 ppem");
    }

    #[test]
    fn no_darkening_above_the_limit_or_for_a_bad_size() {
        for ppem in [72.000_01, f32::NAN, f32::INFINITY, 0.0, -1.0] {
            let p = TextRendering::SMOOTH.glyph_params(ppem);
            assert_eq!(p.dilation(), [0.0, 0.0], "ppem {ppem}");
            assert!(!p.hinting(), "hinting stays off at ppem {ppem}");
        }
    }

    #[test]
    fn smoothing_off_is_always_the_old_look() {
        assert_eq!(
            TextRendering::OFF.glyph_params(13.0),
            GlyphParams::OFF,
            "OFF at 13"
        );
        let off_unhinted = TextRendering {
            smoothing: false,
            hinting: false,
            ..TextRendering::SMOOTH
        };
        assert_eq!(
            off_unhinted.glyph_params(13.0),
            GlyphParams::OFF,
            "smoothing false ignores hinting"
        );
        assert_eq!(off_unhinted.effective(), TextRendering::OFF);
    }

    #[test]
    fn a_bad_factor_is_the_macos_one_for_that_component() {
        let with = |factors| TextRendering {
            stem_darkening: StemDarkening {
                factors,
                ..StemDarkening::MACOS
            },
            ..TextRendering::SMOOTH
        };
        assert_eq!(
            with([f32::NAN, 0.02]).effective().stem_darkening.factors,
            [0.0121, 0.02],
            "factors [NaN, 0.02]"
        );
        assert_eq!(
            with([-0.01, f32::INFINITY])
                .effective()
                .stem_darkening
                .factors,
            StemDarkening::MACOS.factors,
            "factors [-0.01, INFINITY]"
        );
        for bad in NON_VALUES {
            assert_eq!(
                with([bad, bad]).effective().stem_darkening.factors,
                StemDarkening::MACOS.factors,
                "factors {bad}"
            );
        }
    }

    #[test]
    fn max_px_is_capped_and_bad_ones_fail_to_the_default() {
        let with = |max_px| StemDarkening {
            max_px,
            ..StemDarkening::MACOS
        };
        assert_eq!(with([5.0, -1.0]).effective().max_px, [1.0, 0.3], "[5, -1]");
        assert_eq!(with([1.0, 1.0]).effective().max_px, [1.0, 1.0]);
        for bad in NON_VALUES {
            assert_eq!(with([bad, bad]).effective().max_px, [0.3, 0.3], "{bad}");
        }
    }

    #[test]
    fn a_bad_max_ppem_is_72() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0] {
            let d = StemDarkening {
                max_ppem: bad,
                ..StemDarkening::MACOS
            };
            assert_eq!(d.effective().max_ppem, 72.0, "max_ppem {bad}");
        }
        let d = StemDarkening {
            max_ppem: 100.0,
            ..StemDarkening::MACOS
        };
        assert_eq!(d.effective().max_ppem, 100.0);
    }

    #[test]
    fn negative_zero_becomes_zero() {
        let d = StemDarkening {
            factors: [-0.0, -0.0],
            max_px: [-0.0, -0.0],
            max_ppem: -0.0,
        }
        .effective();
        for (name, v) in [
            ("factors x", d.factors[0]),
            ("factors y", d.factors[1]),
            ("max_px x", d.max_px[0]),
            ("max_px y", d.max_px[1]),
            ("max_ppem", d.max_ppem),
        ] {
            assert_eq!(v.to_bits(), 0, "{name} is +0.0");
        }
        assert_eq!(GlyphParams::new(true, [-0.0, 0.0]), GlyphParams::OFF);
        assert_eq!(
            GlyphParams::new(true, [-0.0, 0.0]).dilation()[0].to_bits(),
            0
        );
    }

    #[test]
    fn glyph_params_clamp_their_dilation() {
        assert_eq!(
            GlyphParams::new(false, [f32::NAN, 2.0]).dilation(),
            [0.0, 1.0]
        );
        for bad in [f32::INFINITY, f32::NEG_INFINITY, -1.0] {
            assert_eq!(GlyphParams::new(false, [bad, bad]).dilation(), [0.0, 0.0]);
        }
        assert!(GlyphParams::OFF.hinting());
        assert_eq!(GlyphParams::OFF.dilation(), [0.0, 0.0]);
    }

    #[test]
    fn keys_follow_the_effective_per_glyph_values() {
        let capped = TextRendering {
            stem_darkening: StemDarkening {
                max_px: [0.5, 0.5],
                ..StemDarkening::MACOS
            },
            ..TextRendering::SMOOTH
        };
        // At 13 ppem the cap is not reached, so both settings give a glyph the same params.
        assert_eq!(
            capped.glyph_params(13.0),
            TextRendering::SMOOTH.glyph_params(13.0)
        );
    }

    #[test]
    fn defaults_and_effective_values() {
        assert_eq!(TextRendering::SMOOTH.effective(), TextRendering::SMOOTH);
        assert_eq!(TextRendering::OFF.effective(), TextRendering::OFF);
        assert_eq!(TextRendering::default(), TextRendering::SMOOTH);
        assert_eq!(StemDarkening::default(), StemDarkening::MACOS);
        assert_eq!(StemDarkening::MACOS.effective(), StemDarkening::MACOS);
    }
}
