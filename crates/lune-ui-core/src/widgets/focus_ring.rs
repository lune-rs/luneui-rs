//! Focus rings (`core.focus` ring looks): a 1 px outline that fades with the focus-visible
//! transition. Painted by the widgets' `paint` functions with the continuous ring opacity of their
//! `Look`; at opacity 0 nothing is emitted.

use crate::color::Rgba;
use crate::paint::{Painter, Rect};

/// Which ring look a control uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RingKind {
    /// Button: 1 px `fg_secondary` outline on the rect's own edge (offset -1: inside the border box).
    Button,
    /// Primary button: as [`RingKind::Button`] in `on_accent_text`.
    PrimaryButton,
    /// Toggle, checkbox, radio: 1 px `fg_secondary` outline 2 px outside the control, which has
    /// corner radius `radius`.
    Offset { radius: f32 },
    /// Field, stepper, open combo: the `accent` border over the box's own border.
    Border,
    /// A button's ring on the rect's own edge at its own corner `radius` (the small button, the
    /// icon button, the star): `fg_secondary`, or `on_accent_text` with `on_accent`.
    Edge { radius: f32, on_accent: bool },
}

/// `color` with its alpha scaled by `opacity` (0 to 1).
fn faded(color: Rgba, opacity: f32) -> Rgba {
    let a = (f32::from(color[3]) * opacity).round().clamp(0.0, 255.0) as u8;
    [color[0], color[1], color[2], a]
}

/// Draws the ring of `kind` around `rect` at `opacity`; nothing when `opacity` is not above 0.
pub fn paint<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    rect: Rect,
    kind: RingKind,
    opacity: f32,
) {
    if opacity.is_nan() || opacity <= 0.0 {
        return;
    }
    let t = p.theme;
    match kind {
        RingKind::Button => p.ring(
            rect,
            t.item_radius,
            1.0,
            faded(t.fg_secondary, opacity),
            false,
        ),
        RingKind::PrimaryButton => {
            p.ring(
                rect,
                t.item_radius,
                1.0,
                faded(t.on_accent_text, opacity),
                false,
            );
        }
        RingKind::Offset { radius } => {
            // Outline offset 2: a 2 px gap, then the 1 px line.
            p.ring(
                rect.inset(-3.0, -3.0),
                radius + 3.0,
                1.0,
                faded(t.fg_secondary, opacity),
                false,
            );
        }
        RingKind::Border => p.ring(rect, t.item_radius, 1.0, faded(t.accent, opacity), false),
        RingKind::Edge { radius, on_accent } => {
            let color = if on_accent {
                t.on_accent_text
            } else {
                t.fg_secondary
            };
            p.ring(rect, radius, 1.0, faded(color, opacity), false);
        }
    }
}
