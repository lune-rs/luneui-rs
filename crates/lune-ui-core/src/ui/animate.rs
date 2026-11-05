//! Animated values keyed by widget id: a retargetable [`Transition`] per id, resolved through the
//! theme's motion tokens and the frame's tier.

use crate::color::Rgba;
use crate::motion::{Motion, Transition};

use super::store::Anim;
use super::{Ui, WidgetId};

fn to_f32(c: Rgba) -> [f32; 4] {
    c.map(f32::from)
}

fn to_rgba(c: [f32; 4]) -> Rgba {
    c.map(|v| v.round().clamp(0.0, 255.0) as u8)
}

impl Ui<'_> {
    /// `target` animated per `motion`: the first call for `id` returns `target` and is not in
    /// flight; a changed target retargets (keeping position and velocity, D11), or snaps when the
    /// motion resolves to nothing in this tier. While the value moves the frame asks for
    /// `Repaint::Now`.
    pub fn animate_f32(&mut self, id: WidgetId, target: f32, motion: &Motion) -> f32 {
        let (now, frame) = (self.time(), self.store.frame);
        let resolved = self.config.theme.motion.resolve(motion, self.config.tier);
        let a = self.store.anim_f32.entry(id).or_insert(Anim {
            used: frame,
            transition: Transition::settled(target),
        });
        a.used = frame;
        if a.transition.target() != target {
            match resolved {
                Some(r) => a.transition.retarget(now, target, r.duration, r.easing),
                None => a.transition.snap(target),
            }
        }
        let (v, moving) = a.transition.sample(now);
        self.in_flight |= moving;
        v
    }

    /// Settles the transition of `id` at `v` and returns `v`: a later [`Self::animate_f32`] with
    /// target `v` is not in flight, one with a new target starts from `v`.
    pub fn snap_f32(&mut self, id: WidgetId, v: f32) -> f32 {
        let frame = self.store.frame;
        let a = self.store.anim_f32.entry(id).or_insert(Anim {
            used: frame,
            transition: Transition::settled(v),
        });
        a.used = frame;
        a.transition.snap(v);
        v
    }

    /// [`Self::animate_f32`] for a colour, channel by channel (straight alpha); exactly `target`
    /// once settled.
    pub fn animate_color(&mut self, id: WidgetId, target: Rgba, motion: &Motion) -> Rgba {
        let (now, frame) = (self.time(), self.store.frame);
        let resolved = self.config.theme.motion.resolve(motion, self.config.tier);
        let goal = to_f32(target);
        let a = self.store.anim_color.entry(id).or_insert(Anim {
            used: frame,
            transition: Transition::settled(goal),
        });
        a.used = frame;
        if a.transition.target() != goal {
            match resolved {
                Some(r) => a.transition.retarget(now, goal, r.duration, r.easing),
                None => a.transition.snap(goal),
            }
        }
        let (v, moving) = a.transition.sample(now);
        self.in_flight |= moving;
        if moving { to_rgba(v) } else { target }
    }
}
