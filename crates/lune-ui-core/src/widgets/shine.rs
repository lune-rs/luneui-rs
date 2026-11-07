//! A band of light over a line of text (`widget.tab-strip`, R3).
//!
//! The title is drawn outside the band in its own colour and inside it in vertical strips, each a
//! [`Spec`] of one colour under a clip of its own: the colours follow a smooth bell from the
//! title's colour at both band edges to the light at the centre, so the band reads as a soft beam
//! passing through the glyphs. The strips and the two outer pieces partition the line, so no glyph
//! pixel draws twice and none is cut. The renderer and the text shaper are untouched: the glyphs
//! are the same ones in every piece, so there is no jump at a strip edge.
//!
//! Under Full the band travels left to right off both ends ([`PERIOD`]); under Reduced nothing
//! moves and the whole line breathes between its colour and the light; under Off the line is a
//! fixed mix and nothing repaints.

use std::f32::consts::TAU;
use std::time::Duration;

use super::progress::{BREATH_LOW, breath_opacity};
use crate::color::Rgba;
use crate::motion::{MotionTier, Time};
use crate::paint::{Painter, Rect};
use crate::text::Spec;

/// One pass of the band, from fully left of the line to fully right of it. Provisional (D11).
pub const PERIOD: Duration = Duration::from_millis(1800);
/// The band's width, in logical px. Provisional (D11).
pub const BAND_WIDTH: f32 = 48.0;
/// How many strips the band is cut into: one logical px each, so the colour step between
/// neighbours stays small for the strongest light.
pub const STRIPS: usize = 48;
/// The mix of the light into the line under Off.
pub const STATIC_MIX: f32 = 0.5;

/// Where the light is: `left` and `width` in the space draws are made in, `light` its colour at
/// the centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    pub left: f32,
    pub width: f32,
    pub light: Rgba,
}

/// How far through its pass the band is at `t`, `0` to just under `1`, looping every [`PERIOD`].
pub fn phase(t: Time) -> f32 {
    let into = t.as_nanos() % PERIOD.as_nanos();
    (into as f64 / PERIOD.as_nanos() as f64) as f32
}

/// The band's centre at `t` over a line `width` wide that starts at `x`: at phase 0 the band's
/// right edge is at `x`, at phase 1 its left edge is at `x + width`.
pub fn band_center(t: Time, x: f32, width: f32) -> f32 {
    x - BAND_WIDTH / 2.0 + phase(t) * (width + BAND_WIDTH)
}

/// The light's weight at `f` of the way across the band (`0` to `1`): `0` at both edges, `1` at
/// the centre, with zero slope at the edges.
pub fn bell(f: f32) -> f32 {
    0.5 - 0.5 * (TAU * f.clamp(0.0, 1.0)).cos()
}

/// How much light the whole line takes under Reduced at `t`: the breath's curve, from none to all.
pub fn breath_mix(t: Time) -> f32 {
    ((breath_opacity(t) - BREATH_LOW) / (1.0 - BREATH_LOW)).clamp(0.0, 1.0)
}

/// `base` with `k` of `light` mixed in, each channel: [`crate::color::mix`].
pub use crate::color::mix;

/// `spec` with every run's colour mixed `k` of the way to `light`.
pub fn tinted(spec: &Spec, light: Rgba, k: f32) -> Spec {
    let mut out = spec.clone();
    for (_, style) in &mut out.runs {
        style.color = mix(style.color, light, k);
    }
    out
}

/// The band's strips as `(left, right, weight)`: the edges lie on whole device pixels at `scale`
/// and each strip's right edge is the next one's left, so they partition the band. A strip with
/// no width at this scale is left out.
pub fn strips(band: &Band, scale: f32) -> Vec<(f32, f32, f32)> {
    let snap = |v: f32| (v * scale).round() / scale;
    let edge = |i: usize| snap(band.left + band.width * i as f32 / STRIPS as f32);
    (0..STRIPS)
        .filter_map(|i| {
            let (a, b) = (edge(i), edge(i + 1));
            (b > a).then(|| (a, b, bell((i as f32 + 0.5) / STRIPS as f32)))
        })
        .collect()
}

/// Draws `spec` with its top-left corner at `x`, `y` and `band` of light over it.
pub fn paint<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    spec: &Spec,
    x: f32,
    y: f32,
    band: &Band,
) {
    let (w, h) = p.measure(spec);
    // The band is off the line: it is drawn as it is.
    if band.left + band.width <= x || band.left >= x + w || band.width <= 0.0 {
        p.text(spec, x, y);
        return;
    }
    let tall = |x0: f32, x1: f32| Rect::new(x0, y - h, x1 - x0, h * 3.0);
    let pieces = strips(band, p.s);
    let (Some(first), Some(last)) = (pieces.first(), pieces.last()) else {
        p.text(spec, x, y);
        return;
    };
    let (lo, hi) = (first.0, last.1);
    // Outside the band, in the line's own colour; the outer clips stop at the band's edges and
    // reach past the line's ends by a line height, for a glyph that overhangs.
    if lo > x - h {
        p.push_clip(tall(x - h, lo));
        p.text(spec, x, y);
        p.pop_clip();
    }
    if hi < x + w + h {
        p.push_clip(tall(hi, x + w + h));
        p.text(spec, x, y);
        p.pop_clip();
    }
    for (a, b, k) in pieces {
        // A strip with no glyph under it draws nothing.
        if b <= x - 1.0 || a >= x + w + 1.0 {
            continue;
        }
        p.push_clip(tall(a, b));
        p.text(&tinted(spec, band.light, k), x, y);
        p.pop_clip();
    }
}

/// Draws `spec` at `x`, `y` shining with `light` at `t` under `tier`: the travelling band under
/// Full, the whole line breathing under Reduced, a fixed mix under Off. It is vertically placed
/// like [`Painter::text_in`] in the row `row`, given as its top and its height.
pub fn paint_at<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    spec: &Spec,
    x: f32,
    row: (f32, f32),
    light: Rgba,
    tier: MotionTier,
    t: Time,
) {
    let (w, h) = p.measure(spec);
    let y = row.0 + (row.1 - h) / 2.0;
    match tier {
        MotionTier::Full => {
            let band = Band {
                left: band_center(t, x, w) - BAND_WIDTH / 2.0,
                width: BAND_WIDTH,
                light,
            };
            paint(p, spec, x, y, &band);
        }
        MotionTier::Reduced => {
            p.text(&tinted(spec, light, breath_mix(t)), x, y);
        }
        MotionTier::Off => {
            p.text(&tinted(spec, light, STATIC_MIX), x, y);
        }
    }
}
