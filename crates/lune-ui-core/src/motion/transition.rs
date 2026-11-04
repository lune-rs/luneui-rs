//! A retargetable transition: a tween from the value at the retarget instant that keeps the
//! velocity it had when it was interrupted.

use std::time::Duration;

use super::tween::progress;
use super::{Easing, Lerp, Time};

/// A value moving towards its latest target (D11). From a settled state a retarget is a plain
/// tween; a retarget while in flight starts from the current value and adds a Hermite term
/// `K·p(1 − p)²` with `K = v·d − (target − current)·E′(0)` so the velocity carries over. A sample
/// at or after the end returns the target exactly and reports settled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition<T: Lerp> {
    from: T,
    to: T,
    start: Time,
    duration: Duration,
    easing: Easing,
    /// Velocity correction, in value units (velocity × duration); zero unless `kicked`.
    kick: T,
    kicked: bool,
}

/// `h(p) = p(1 − p)²`: zero at both ends, slope 1 at 0 and 0 at 1.
fn hermite(p: f32) -> f32 {
    p * (1.0 - p) * (1.0 - p)
}

/// `h′(p) = (1 − p)² − 2p(1 − p)`.
fn hermite_slope(p: f32) -> f32 {
    (1.0 - p) * (1.0 - p) - 2.0 * p * (1.0 - p)
}

impl<T: Lerp> Transition<T> {
    /// At rest on `value`; never in flight until retargeted.
    pub fn settled(value: T) -> Self {
        Transition {
            from: value,
            to: value,
            start: Time::ZERO,
            duration: Duration::ZERO,
            easing: Easing::Linear,
            kick: value.sub(value),
            kicked: false,
        }
    }

    /// The latest target.
    pub fn target(&self) -> T {
        self.to
    }

    fn end(&self) -> Time {
        self.start.saturating_add(self.duration)
    }

    /// Whether the transition is still moving at `now`.
    pub fn in_flight(&self, now: Time) -> bool {
        now < self.end()
    }

    /// Moves towards `target` over `duration` with `easing`, starting at `now`: a plain tween
    /// from the current value when settled, velocity-preserving when in flight. A zero duration
    /// snaps.
    pub fn retarget(&mut self, now: Time, target: T, duration: Duration, easing: Easing) {
        if duration.is_zero() {
            self.snap(target);
            return;
        }
        let (current, moving) = self.sample(now);
        let zero = current.sub(current);
        if moving {
            let v = self.velocity(now);
            let d = duration.as_secs_f32();
            self.kick = zero
                .add_scaled(v, d)
                .add_scaled(target.sub(current), -easing.derivative(0.0));
            self.kicked = true;
        } else {
            self.kick = zero;
            self.kicked = false;
        }
        self.from = current;
        self.to = target;
        self.start = now;
        self.duration = duration;
        self.easing = easing;
    }

    /// Jumps to `target` and settles there.
    pub fn snap(&mut self, target: T) {
        *self = Transition::settled(target);
    }

    /// The value at `now` and whether it is still in flight; exactly the target and `false` once
    /// `now` reaches the end. Time before the start samples the start value.
    pub fn sample(&self, now: Time) -> (T, bool) {
        if now >= self.end() {
            return (self.to, false);
        }
        let p = progress(now, self.start, self.duration);
        let mut x = self.from.lerp(self.to, self.easing.eval(p));
        if self.kicked {
            x = x.add_scaled(self.kick, hermite(p));
        }
        (x, true)
    }

    /// The rate of change at `now`, in value units per second; zero once settled.
    pub fn velocity(&self, now: Time) -> T {
        let zero = self.to.sub(self.to);
        if now >= self.end() {
            return zero;
        }
        let p = progress(now, self.start, self.duration);
        let d = self.duration.as_secs_f32();
        let mut v = zero.add_scaled(self.to.sub(self.from), self.easing.derivative(p) / d);
        if self.kicked {
            v = v.add_scaled(self.kick, hermite_slope(p) / d);
        }
        v
    }
}
