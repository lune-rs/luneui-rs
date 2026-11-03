//! Interpolable values, tweens, keyframes and the critically damped spring, all closed-form in
//! the app's [`Time`].

use std::time::Duration;

use super::{Easing, Time};

/// A value a transition can move: interpolation plus the vector operations the velocity-preserving
/// retarget and the conformance harness need.
pub trait Lerp: Copy + PartialEq {
    /// `self·(1 − t) + to·t`, exact at `t = 0` and `t = 1`.
    fn lerp(self, to: Self, t: f32) -> Self;
    /// `self + d·k`.
    fn add_scaled(self, d: Self, k: f32) -> Self;
    /// `self − other`.
    fn sub(self, other: Self) -> Self;
    /// Euclidean distance.
    fn distance(self, other: Self) -> f32;
    /// Every component is finite.
    fn is_finite(self) -> bool;
}

impl Lerp for f32 {
    fn lerp(self, to: f32, t: f32) -> f32 {
        self * (1.0 - t) + to * t
    }
    fn add_scaled(self, d: f32, k: f32) -> f32 {
        self + d * k
    }
    fn sub(self, other: f32) -> f32 {
        self - other
    }
    fn distance(self, other: f32) -> f32 {
        (self - other).abs()
    }
    fn is_finite(self) -> bool {
        f32::is_finite(self)
    }
}

impl<const N: usize> Lerp for [f32; N] {
    fn lerp(self, to: Self, t: f32) -> Self {
        std::array::from_fn(|i| self[i].lerp(to[i], t))
    }
    fn add_scaled(self, d: Self, k: f32) -> Self {
        std::array::from_fn(|i| self[i] + d[i] * k)
    }
    fn sub(self, other: Self) -> Self {
        std::array::from_fn(|i| self[i] - other[i])
    }
    fn distance(self, other: Self) -> f32 {
        self.iter()
            .zip(other.iter())
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f32>()
            .sqrt()
    }
    fn is_finite(self) -> bool {
        self.iter().all(|c| c.is_finite())
    }
}

/// Linear progress of `now` through `start .. start + duration`, clamped to 0..1. Time before
/// `start` counts as `start` (saturating); a zero duration is complete at `start`.
pub(crate) fn progress(now: Time, start: Time, duration: Duration) -> f32 {
    if duration.is_zero() {
        return if now >= start { 1.0 } else { 0.0 };
    }
    let into = now.saturating_sub(start);
    (into.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

/// A fixed run from `from` to `to`: `from` before `start`, exactly `to` at and after
/// `start + duration`, `from.lerp(to, easing(progress))` between.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tween<T: Lerp> {
    pub from: T,
    pub to: T,
    pub start: Time,
    pub duration: Duration,
    pub easing: Easing,
}

impl<T: Lerp> Tween<T> {
    /// The value at `now`.
    pub fn sample(&self, now: Time) -> T {
        if now >= self.start.saturating_add(self.duration) {
            return self.to;
        }
        if now < self.start {
            return self.from;
        }
        let p = progress(now, self.start, self.duration);
        self.from.lerp(self.to, self.easing.eval(p))
    }
}

/// Stops `(progress, value, easing)` sampled by progress 0..1. A segment uses its destination
/// stop's easing.
#[derive(Debug, Clone, PartialEq)]
pub struct Keyframes<T: Lerp> {
    stops: Vec<(f32, T, Easing)>,
}

impl<T: Lerp> Keyframes<T> {
    /// `None` when `stops` is empty, a progress is outside 0..1 (or NaN), progresses decrease, or
    /// the last stop is not at 1.0. Equal neighbouring progresses make a step.
    pub fn new(stops: Vec<(f32, T, Easing)>) -> Option<Self> {
        let last = stops.last()?;
        if last.0 != 1.0 {
            return None;
        }
        if !stops.iter().all(|s| (0.0..=1.0).contains(&s.0)) {
            return None;
        }
        if stops.windows(2).any(|w| w[1].0 < w[0].0) {
            return None;
        }
        Some(Keyframes { stops })
    }

    /// The stops, in order.
    pub fn stops(&self) -> &[(f32, T, Easing)] {
        &self.stops
    }

    /// The value at progress `p` (clamped to 0..1); the last value exactly at 1, the first value
    /// before the first stop.
    pub fn sample(&self, p: f32) -> T {
        let p = if p.is_nan() { 0.0 } else { p.clamp(0.0, 1.0) };
        let (first, last) = (self.stops[0], self.stops[self.stops.len() - 1]);
        if p >= 1.0 {
            return last.1;
        }
        if p < first.0 {
            return first.1;
        }
        for w in self.stops.windows(2) {
            let ((p0, v0, _), (p1, v1, easing)) = (w[0], w[1]);
            if p < p1 {
                let local = (p - p0) / (p1 - p0);
                return v0.lerp(v1, easing.eval(local));
            }
        }
        last.1
    }
}

/// A critically damped spring towards `target` from `x0` with velocity `v0` (units per second)
/// at `start`, angular rate `rate` (1/s): `x(t) = target + (d0 + (v0 + w·d0)·t)·e^(−w·t)` with
/// `d0 = x0 − target`. It never overshoots when `d0` and `v0` share a sign. There is no default
/// rate. A rate that is not a positive finite number holds `x0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring<T: Lerp> {
    pub target: T,
    pub rate: f32,
    pub x0: T,
    pub v0: T,
    pub start: Time,
}

impl<T: Lerp> Spring<T> {
    /// The value at `now`; `x0` before `start`.
    pub fn sample(&self, now: Time) -> T {
        let w = self.rate;
        if !(w.is_finite() && w > 0.0) {
            return self.x0;
        }
        let t = now.saturating_sub(self.start).as_secs_f32();
        let decay = (-w * t).exp();
        let d0 = self.x0.sub(self.target);
        self.target
            .add_scaled(d0, (1.0 + w * t) * decay)
            .add_scaled(self.v0, t * decay)
    }
}
