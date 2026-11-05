//! Motion: the closed-form animation model (`docs/motion.md`). Every value is a pure function of
//! the app's [`Time`] and stored parameters; nothing integrates per frame and nothing reads a
//! clock, so a frame is reproducible.
//!
//! - [`Easing`]: the seven easings (`CUBIC_OUT` and `EASE_IN` are the moved menu curves).
//! - [`Lerp`], [`Tween`], [`Keyframes`], [`Spring`]: interpolable values and fixed runs.
//! - [`Transition`]: a retargetable run that keeps its velocity when interrupted (D11).
//! - [`Token`], [`Kind`], [`Motion`], [`MotionTier`], [`MotionTokens`], [`MOTION`]: the
//!   vocabulary and the tier rules (D12).
//! - [`conformance`], [`conformance_of`]: the 60 Hz harness (D41).

mod conformance;
mod easing;
mod tokens;
mod transition;
mod tween;

use std::time::Duration;

pub use conformance::{
    Conformance, ConformanceError, FRAME, STEP_BOUND, STEP_BOUND_BACK_OUT, conformance,
    conformance_of,
};
pub use easing::{Easing, cubic_bezier};
pub use tokens::{
    Kind, MOTION, Motion, MotionTier, MotionTokens, Reduce, ReducedMap, Resolved, Token,
};
pub use transition::Transition;
pub use tween::{Keyframes, Lerp, Spring, Tween};

/// Time since the app's origin, as the app supplies it in `RawInput`. The toolkit never reads a
/// clock itself.
pub type Time = std::time::Duration;

/// One spinner revolution.
pub const SPIN_PERIOD: Duration = Duration::from_millis(1200);

/// Spinner angle in degrees at `t`: `360 · frac(t / 1200 ms)`, from the app's time so it does not
/// accumulate per frame. This is the raw phase, whatever the tier: the spinner widget decides by
/// tier (a turning glyph under Full, a static one at angle 0 under Reduced and Off, D117).
pub fn spin_angle(t: Time) -> f32 {
    let period = SPIN_PERIOD.as_nanos();
    let into = t.as_nanos() % period;
    (into as f64 / period as f64 * 360.0) as f32
}
