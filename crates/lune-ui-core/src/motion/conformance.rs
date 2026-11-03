//! The 60 Hz conformance harness (D41): every transition is sampled on a 60 Hz grid from its
//! start and checked for jumps, an exact end, a timely settle and finite values.

use std::fmt;
use std::time::Duration;

use super::{Easing, Lerp, Motion, MotionTier, MotionTokens, Time, Transition};

/// One 60 Hz frame, rounded down to the nanosecond.
pub const FRAME: Duration = Duration::from_nanos(1_000_000_000 / 60);

/// Largest step per frame, as a fraction of the run's distance (0.45 for `BackOut`).
pub const STEP_BOUND: f32 = 0.35;
pub const STEP_BOUND_BACK_OUT: f32 = 0.45;

/// Settled samples checked after the first settled one.
const TAIL_FRAMES: u64 = 2;

/// Time of frame `k`: `k / 60` s, computed exactly in nanoseconds (no accumulated drift).
fn frame_time(k: u64) -> Time {
    Duration::from_nanos(k.saturating_mul(1_000_000_000) / 60)
}

/// A passing run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Conformance {
    /// Samples taken.
    pub frames: u64,
    /// Time of the first settled sample.
    pub settled_at: Time,
    /// Largest distance between consecutive samples.
    pub max_step: f32,
}

/// Why a run failed; each names the frame (index on the 60 Hz grid) and its time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConformanceError {
    /// A step moved further than the bound.
    Jump {
        frame: u64,
        at: Time,
        step: f32,
        bound: f32,
    },
    /// The first sample at or after the end is not the target exactly or still reports in
    /// flight, or a settled sample is not the target.
    End { frame: u64, at: Time },
    /// It was in flight again after settling, or still in flight past duration plus one frame.
    Late { frame: u64, at: Time, limit: Time },
    /// A sample is NaN or infinite.
    NotFinite { frame: u64, at: Time },
}

impl fmt::Display for ConformanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ConformanceError::Jump {
                frame,
                at,
                step,
                bound,
            } => write!(
                f,
                "Jump: frame {frame} ({at:?}) moved {step}, bound {bound}"
            ),
            ConformanceError::End { frame, at } => {
                write!(
                    f,
                    "End: frame {frame} ({at:?}) is not settled on the target"
                )
            }
            ConformanceError::Late { frame, at, limit } => write!(
                f,
                "Late: frame {frame} ({at:?}) in flight after settling or past {limit:?}"
            ),
            ConformanceError::NotFinite { frame, at } => {
                write!(f, "NotFinite: frame {frame} ({at:?})")
            }
        }
    }
}

impl std::error::Error for ConformanceError {}

/// Runs `motion` from `from` to `to` under `tier` with `tokens`: a [`Transition`] settled on
/// `from` and retarget at time 0 (snapped when the tier resolves to no motion), checked by
/// [`conformance_of`] with a step bound of [`STEP_BOUND`] (or [`STEP_BOUND_BACK_OUT`]) times
/// `from.distance(to)`.
pub fn conformance<T: Lerp>(
    motion: &Motion,
    tokens: &MotionTokens,
    tier: MotionTier,
    from: T,
    to: T,
) -> Result<Conformance, ConformanceError> {
    let resolved = tokens.resolve(motion, tier);
    let mut tr = Transition::settled(from);
    let end = match resolved {
        Some(r) => {
            tr.retarget(Time::ZERO, to, r.duration, r.easing);
            r.duration
        }
        None => {
            tr.snap(to);
            Time::ZERO
        }
    };
    let fraction = if motion.easing == Easing::BackOut {
        STEP_BOUND_BACK_OUT
    } else {
        STEP_BOUND
    };
    conformance_of(|t| tr.sample(t), end, to, fraction * from.distance(to))
}

/// Checks any sampler `t -> (value, in_flight)` on the 60 Hz grid from time 0: no step between
/// consecutive samples longer than `bound` (`Jump`), every sample finite (`NotFinite`), the first
/// sample at or after `end` exactly `target` and settled (`End`), and no sample in flight after
/// the first settled one or past `end` plus one frame (`Late`). Sampling stops two frames after
/// the first settled sample.
pub fn conformance_of<T: Lerp>(
    mut sampler: impl FnMut(Time) -> (T, bool),
    end: Time,
    target: T,
    bound: f32,
) -> Result<Conformance, ConformanceError> {
    let limit = end.saturating_add(FRAME);
    let mut prev: Option<T> = None;
    let mut settled: Option<(u64, Time)> = None;
    let mut max_step = 0.0f32;
    let mut k = 0u64;
    loop {
        let at = frame_time(k);
        let (x, moving) = sampler(at);
        if !x.is_finite() {
            return Err(ConformanceError::NotFinite { frame: k, at });
        }
        if let Some(p) = prev {
            let step = x.distance(p);
            if step.is_nan() || step > bound {
                return Err(ConformanceError::Jump {
                    frame: k,
                    at,
                    step,
                    bound,
                });
            }
            max_step = max_step.max(step);
        }
        prev = Some(x);
        if moving {
            if settled.is_some() || at > limit {
                return Err(ConformanceError::Late {
                    frame: k,
                    at,
                    limit,
                });
            }
            if at >= end {
                return Err(ConformanceError::End { frame: k, at });
            }
        } else {
            if x != target {
                return Err(ConformanceError::End { frame: k, at });
            }
            if settled.is_none() {
                settled = Some((k, at));
            }
        }
        if let Some((first, settled_at)) = settled {
            if k >= first + TAIL_FRAMES {
                return Ok(Conformance {
                    frames: k + 1,
                    settled_at,
                    max_step,
                });
            }
        }
        k += 1;
    }
}
