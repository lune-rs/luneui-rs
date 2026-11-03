//! Duration tokens, the 15 animation kinds, the tiers and the Reduced duration map
//! (`docs/motion.md`).

use std::time::Duration;

use super::Easing;

/// A duration token; components name one, never milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Token {
    Instant,
    Fast,
    Base,
    Moderate,
    Slow,
}

impl Token {
    /// The five tokens, shortest first.
    pub const ALL: [Token; 5] = [
        Token::Instant,
        Token::Fast,
        Token::Base,
        Token::Moderate,
        Token::Slow,
    ];
}

/// The kinds of `docs/motion.md`, in its order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Hover,
    Press,
    Focus,
    Appear,
    Dismiss,
    PageSwap,
    Drawer,
    Pop,
    Selection,
    Move,
    Expand,
    Value,
    Spin,
    Scroll,
    Settle,
}

impl Kind {
    /// Every kind, in `docs/motion.md` order.
    pub const ALL: [Kind; 15] = [
        Kind::Hover,
        Kind::Press,
        Kind::Focus,
        Kind::Appear,
        Kind::Dismiss,
        Kind::PageSwap,
        Kind::Drawer,
        Kind::Pop,
        Kind::Selection,
        Kind::Move,
        Kind::Expand,
        Kind::Value,
        Kind::Spin,
        Kind::Scroll,
        Kind::Settle,
    ];
}

/// What a transition does under [`MotionTier::Reduced`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reduce {
    /// A colour or opacity change: keeps its tween at the mapped duration.
    Keep,
    /// A transform: replaced by an opacity fade at the mapped duration.
    Fade,
    /// No opacity or colour part: snaps.
    Snap,
}

/// The motion tier the app passes each frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MotionTier {
    #[default]
    Full,
    Reduced,
    Off,
}

/// One row of a component's Transitions table: kind, token, easing and Reduced rule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    pub kind: Kind,
    pub token: Token,
    pub easing: Easing,
    pub reduced: Reduce,
    /// The row's explicit Reduced duration, overriding the map (D12).
    pub reduced_token: Option<Token>,
}

impl Motion {
    /// The kinds table's default token and easing for `kind`, with its default Reduced rule.
    /// `Spin` is an indicator, not a transition (see [`spin_angle`](super::spin_angle)); its row
    /// exists so every kind maps, with the linear easing and `Slow`, the scale's closest token to
    /// its 1200 ms period.
    pub const fn of(kind: Kind) -> Motion {
        use Easing::*;
        use Reduce::*;
        use Token::*;
        let (token, easing, reduced) = match kind {
            Kind::Hover => (Instant, Linear, Keep),
            Kind::Press => (Fast, QuadOut, Snap),
            Kind::Focus => (Fast, Linear, Keep),
            Kind::Appear => (Base, QuadOut, Fade),
            Kind::Dismiss => (Fast, QuadIn, Fade),
            Kind::PageSwap => (Moderate, QuadOut, Fade),
            Kind::Drawer => (Moderate, SineInOut, Fade),
            Kind::Pop => (Base, QuadOut, Fade),
            Kind::Selection => (Fast, Linear, Keep),
            Kind::Move => (Fast, SineInOut, Snap),
            Kind::Expand => (Base, SineInOut, Snap),
            Kind::Value => (Fast, QuadOut, Snap),
            Kind::Spin => (Slow, Linear, Keep),
            Kind::Scroll => (Fast, QuadOut, Snap),
            Kind::Settle => (Fast, SineInOut, Snap),
        };
        Motion {
            kind,
            token,
            easing,
            reduced,
            reduced_token: None,
        }
    }
}

/// The Reduced tier's token map (D12): each Full token's Reduced token, with `fast` split by
/// kind (selection keeps `fast_selection`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReducedMap {
    pub instant: Token,
    pub fast: Token,
    pub fast_selection: Token,
    pub base: Token,
    pub moderate: Token,
    pub slow: Token,
}

impl ReducedMap {
    /// The Reduced token for a Full `token` of `kind`.
    pub fn map(&self, token: Token, kind: Kind) -> Token {
        match token {
            Token::Instant => self.instant,
            Token::Fast if kind == Kind::Selection => self.fast_selection,
            Token::Fast => self.fast,
            Token::Base => self.base,
            Token::Moderate => self.moderate,
            Token::Slow => self.slow,
        }
    }
}

/// The theme's motion tokens: the five durations and the Reduced map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MotionTokens {
    pub instant: Duration,
    pub fast: Duration,
    pub base: Duration,
    pub moderate: Duration,
    pub slow: Duration,
    pub reduced: ReducedMap,
}

/// A [`Motion`] resolved under a tier: what to run, and whether transforms (slide, scale) apply.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resolved {
    pub duration: Duration,
    pub easing: Easing,
    pub transforms: bool,
}

/// `docs/motion.md`'s tokens: 120 / 180 / 250 / 350 / 500 ms and D12's Reduced map
/// (instant→instant, fast→instant with selection staying fast, base→fast, moderate→fast,
/// slow→base).
pub const MOTION: MotionTokens = MotionTokens {
    instant: Duration::from_millis(120),
    fast: Duration::from_millis(180),
    base: Duration::from_millis(250),
    moderate: Duration::from_millis(350),
    slow: Duration::from_millis(500),
    reduced: ReducedMap {
        instant: Token::Instant,
        fast: Token::Instant,
        fast_selection: Token::Fast,
        base: Token::Fast,
        moderate: Token::Fast,
        slow: Token::Base,
    },
};

impl MotionTokens {
    /// The duration of `token`.
    pub fn duration(&self, token: Token) -> Duration {
        match token {
            Token::Instant => self.instant,
            Token::Fast => self.fast,
            Token::Base => self.base,
            Token::Moderate => self.moderate,
            Token::Slow => self.slow,
        }
    }

    /// What `motion` runs under `tier`; `None` means the change snaps. Off snaps everything;
    /// Reduced snaps `Snap` rows, keeps `Keep` rows with transforms and fades `Fade` rows without
    /// them, both at the row's `reduced_token` or else the mapped token; Full runs the row's own
    /// token.
    pub fn resolve(&self, motion: &Motion, tier: MotionTier) -> Option<Resolved> {
        let (token, transforms) = match tier {
            MotionTier::Off => return None,
            MotionTier::Full => (motion.token, true),
            MotionTier::Reduced => {
                let mapped = motion
                    .reduced_token
                    .unwrap_or_else(|| self.reduced.map(motion.token, motion.kind));
                match motion.reduced {
                    Reduce::Snap => return None,
                    Reduce::Keep => (mapped, true),
                    Reduce::Fade => (mapped, false),
                }
            }
        };
        Some(Resolved {
            duration: self.duration(token),
            easing: motion.easing,
            transforms,
        })
    }
}
