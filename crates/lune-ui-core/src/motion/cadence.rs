//! How often a continuous loop asks for its next frame (D298): a grid of ticks on the app's own
//! time, shared by every loop and window.

use std::num::NonZeroU16;
use std::time::Duration;

use super::Time;

const NS: u128 = 1_000_000_000;

/// A rate at which something that moves without end (a shine, a spinner, an indeterminate bar)
/// asks for its next frame. Transitions, input and catch-up frames are never held to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cadence {
    /// Every frame: the loop asks for the next one at once, so the display's rate sets the pace
    /// (0.4.0's behaviour).
    EveryFrame,
    /// This many ticks a second, at whole multiples of 1 s / n of app time.
    Hz(NonZeroU16),
}

impl Cadence {
    /// The default for busy loops: 30 a second.
    pub const BUSY: Cadence = Cadence::hz(30);

    /// `n` ticks a second; 0 is [`Cadence::EveryFrame`].
    pub const fn hz(n: u16) -> Cadence {
        match NonZeroU16::new(n) {
            Some(n) => Cadence::Hz(n),
            None => Cadence::EveryFrame,
        }
    }

    /// The first tick strictly after `now`: the smallest `ceil(k * 10^9 / n)` ns for an integer
    /// `k >= 1` that is greater than `now`. The tick is computed from its index, never by adding
    /// a period, so a rate that is not a whole number of nanoseconds does not drift. `None` under
    /// [`Cadence::EveryFrame`] and when the tick lies past [`Duration::MAX`]; the caller then
    /// asks for the frame at once.
    pub fn next_tick(self, now: Time) -> Option<Time> {
        let Cadence::Hz(n) = self else { return None };
        let n = u128::from(n.get());
        let k = now.as_nanos() * n / NS + 1;
        let at = (k * NS).div_ceil(n);
        let secs = u64::try_from(at / NS).ok()?;
        Some(Duration::new(secs, (at % NS) as u32))
    }
}

impl Default for Cadence {
    /// [`Cadence::BUSY`].
    fn default() -> Self {
        Cadence::BUSY
    }
}
