//! The seven easings of `docs/motion.md` and the CSS cubic-bezier solver the moved menu curves use.

use std::f32::consts::PI;

/// Back-out overshoot constants (`c1`, `c3 = c1 + 1`).
const BACK_C1: f32 = 1.70158;
const BACK_C3: f32 = BACK_C1 + 1.0;

/// Bisection steps of [`Easing::inverse`].
const INVERSE_STEPS: u32 = 32;

/// CSS `cubic-bezier(x1, y1, x2, y2)` at time `t`.
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let coord = |a: f32, b: f32, u: f32| {
        3.0 * a * u * (1.0 - u).powi(2) + 3.0 * b * u * u * (1.0 - u) + u.powi(3)
    };
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if coord(x1, x2, mid) < t {
            lo = mid
        } else {
            hi = mid
        }
    }
    coord(y1, y2, (lo + hi) / 2.0)
}

/// The curve parameter `u` [`cubic_bezier`] settles on for `t`: the same 24 bisection steps.
fn bezier_param(x1: f32, x2: f32, t: f32) -> f32 {
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if bezier_coord(x1, x2, mid) < t {
            lo = mid
        } else {
            hi = mid
        }
    }
    (lo + hi) / 2.0
}

fn bezier_coord(a: f32, b: f32, u: f32) -> f32 {
    3.0 * a * u * (1.0 - u).powi(2) + 3.0 * b * u * u * (1.0 - u) + u.powi(3)
}

/// d/du of one bezier coordinate with control values `a`, `b` (end points 0 and 1).
fn bezier_slope(a: f32, b: f32, u: f32) -> f32 {
    3.0 * a * (1.0 - u).powi(2) + 6.0 * (b - a) * u * (1.0 - u) + 3.0 * (1.0 - b) * u * u
}

/// An easing: maps progress 0..1 to eased progress, clamping outside 0..1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    /// `t`: hover and colour, focus.
    Linear,
    /// `1 − (1 − t)²`: enter, press.
    QuadOut,
    /// `t²`: exit.
    QuadIn,
    /// `−(cos(πt) − 1) / 2`: move, resize.
    SineInOut,
    /// `1 + c3 (t − 1)³ + c1 (t − 1)²`, `c1 = 1.70158`, `c3 = c1 + 1`: overshoots 1, emphasis only.
    BackOut,
    /// CSS `cubic-bezier(x1, y1, x2, y2)`, evaluated by [`cubic_bezier`].
    CubicBezier(f32, f32, f32, f32),
}

impl Easing {
    /// `cubic-out`: the moved menu's open curve.
    pub const CUBIC_OUT: Easing = Easing::CubicBezier(0.215, 0.61, 0.355, 1.0);
    /// `ease-in`: the moved menu's close curve.
    pub const EASE_IN: Easing = Easing::CubicBezier(0.42, 0.0, 1.0, 1.0);

    /// Eased progress at `t`, `t` clamped to 0..1.
    pub fn eval(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::QuadOut => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::QuadIn => t * t,
            Easing::SineInOut => -((PI * t).cos() - 1.0) / 2.0,
            Easing::BackOut => {
                let s = t - 1.0;
                1.0 + BACK_C3 * s * s * s + BACK_C1 * s * s
            }
            Easing::CubicBezier(x1, y1, x2, y2) => cubic_bezier(x1, y1, x2, y2, t),
        }
    }

    /// d(eval)/dt at `t`, `t` clamped to 0..1. A cubic-bezier is solved for its curve parameter
    /// with the same bisection as [`eval`](Self::eval); where the curve's x slope vanishes the
    /// slope is a central difference of `eval` instead.
    pub fn derivative(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => 1.0,
            Easing::QuadOut => 2.0 * (1.0 - t),
            Easing::QuadIn => 2.0 * t,
            Easing::SineInOut => PI * (PI * t).sin() / 2.0,
            Easing::BackOut => {
                let s = t - 1.0;
                3.0 * BACK_C3 * s * s + 2.0 * BACK_C1 * s
            }
            Easing::CubicBezier(x1, y1, x2, y2) => {
                let u = bezier_param(x1, x2, t);
                let dx = bezier_slope(x1, x2, u);
                if dx.abs() >= 1e-6 {
                    bezier_slope(y1, y2, u) / dx
                } else {
                    const H: f32 = 1e-3;
                    let (a, b) = ((t - H).max(0.0), (t + H).min(1.0));
                    (self.eval(b) - self.eval(a)) / (b - a)
                }
            }
        }
    }

    /// The progress `t` in 0..1 at which `eval(t)` reaches `y`, by bisection. Meaningful only
    /// for monotone easings: `BackOut` overshoots, so its result is one of the crossings and is
    /// not supported. `y` outside the curve's range gives the nearest end.
    pub fn inverse(self, y: f32) -> f32 {
        let (mut lo, mut hi) = (0.0f32, 1.0f32);
        for _ in 0..INVERSE_STEPS {
            let mid = (lo + hi) / 2.0;
            if self.eval(mid) < y {
                lo = mid
            } else {
                hi = mid
            }
        }
        (lo + hi) / 2.0
    }
}
