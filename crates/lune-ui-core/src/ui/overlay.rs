//! The overlay stack: overlays push in open order, draw on their layer with hits tagged to them,
//! move through Opening, Open and Closing (D7), and keep drawing through an app-built exit painter
//! after the app stops calling them (D8). The Ui never closes an overlay; the app stops calling it.

use crate::motion::{Motion, MotionTier, MotionTokens, Resolved, Time};
use crate::paint::{LayerId, Painter};

use super::{Ui, WidgetId};

/// How an overlay behaves: where it draws, whether it captures input, what dismisses it, where
/// focus goes and how it moves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayOptions {
    pub layer: LayerId,
    /// Hits outside the overlay (on its layer or below) get no hover, press, scroll, keys or
    /// accessibility actions while it is open or opening; Tab never leaves it.
    pub modal: bool,
    pub dismiss_on_escape: bool,
    pub dismiss_on_outside_press: bool,
    pub dismiss_on_focus_loss: bool,
    /// The widget focus returns to on close when the one focused at open is gone.
    pub anchor: Option<WidgetId>,
    /// The widget focused on open, if it is a tab stop of the overlay; else its first tab stop.
    pub initial_focus: Option<WidgetId>,
    pub open: Motion,
    pub close: Motion,
}

/// Where an overlay is in its opening motion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OverlayPhase {
    /// Linear progress `p` in 0..1; the overlay maps it to its look through its own curve.
    Opening(f32),
    Open,
}

/// Why the Ui asks the app to close an overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dismissal {
    Escape,
    OutsidePress,
    FocusLoss,
    /// An assistive technology's dismiss action.
    Access,
}

/// One frame of an overlay, as [`Ui::overlay`] reports it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayFrame {
    pub id: WidgetId,
    pub phase: OverlayPhase,
    /// Set on the frame the Ui saw a dismiss request; the app closes by not calling it again.
    pub dismissed: Option<Dismissal>,
    /// False when the resolved motion drops transforms (the Reduced fade): opacity only.
    pub transforms: bool,
}

/// The hit tag of exit painters: their hits are dropped.
pub(crate) const SUPPRESSED: u32 = u32::MAX;

/// An overlay's retained phase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum RecPhase {
    Opening { start: Time, p0: f32 },
    Open,
    Closing { start: Time, p0: f32 },
}

pub(crate) type ExitPainter = Box<dyn Fn(&mut Painter<'_, WidgetId>, f32, bool)>;

/// One overlay of the store's stack.
pub(crate) struct OverlayRec {
    pub(crate) id: WidgetId,
    /// Unique per open, increasing in open order; the hits of its content carry it.
    pub(crate) tag: u32,
    pub(crate) opts: OverlayOptions,
    pub(crate) phase: RecPhase,
    pub(crate) exit: Option<ExitPainter>,
    pub(crate) saved_focus: Option<WidgetId>,
    pub(crate) called: bool,
}

impl std::fmt::Debug for OverlayRec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OverlayRec")
            .field("id", &self.id)
            .field("tag", &self.tag)
            .field("phase", &self.phase)
            .field("saved_focus", &self.saved_focus)
            .finish_non_exhaustive()
    }
}

/// `r` unless the motion is absent or takes no time.
fn usable(r: Option<Resolved>) -> Option<Resolved> {
    r.filter(|r| !r.duration.is_zero())
}

/// `p0 + elapsed / duration` (D7), linear.
fn progress(p0: f32, start: Time, now: Time, r: &Resolved) -> f32 {
    p0 + now.saturating_sub(start).as_secs_f32() / r.duration.as_secs_f32()
}

impl OverlayRec {
    /// Open or opening (not closing).
    pub(crate) fn active(&self) -> bool {
        !matches!(self.phase, RecPhase::Closing { .. })
    }

    /// The opening phase at `now`, settling the record to `Open` once done; and `transforms`.
    pub(crate) fn opening(
        &mut self,
        now: Time,
        tokens: &MotionTokens,
        tier: MotionTier,
    ) -> (OverlayPhase, bool) {
        let resolved = tokens.resolve(&self.opts.open, tier);
        let transforms = resolved.is_none_or(|r| r.transforms);
        let phase = match (self.phase, usable(resolved)) {
            (RecPhase::Opening { start, p0 }, Some(r)) => {
                let p = progress(p0, start, now, &r);
                if p >= 1.0 {
                    OverlayPhase::Open
                } else {
                    OverlayPhase::Opening(p)
                }
            }
            _ => OverlayPhase::Open,
        };
        if phase == OverlayPhase::Open {
            self.phase = RecPhase::Open;
        }
        (phase, transforms)
    }

    /// How visible the overlay is at `now`: opening `E(p)`, open 1, closing `1 − E(p)`.
    fn visibility(&self, now: Time, tokens: &MotionTokens, tier: MotionTier) -> f32 {
        match self.phase {
            RecPhase::Open => 1.0,
            RecPhase::Opening { start, p0 } => {
                match usable(tokens.resolve(&self.opts.open, tier)) {
                    Some(r) => r.easing.eval(progress(p0, start, now, &r)),
                    None => 1.0,
                }
            }
            RecPhase::Closing { start, p0 } => {
                match usable(tokens.resolve(&self.opts.close, tier)) {
                    Some(r) => 1.0 - r.easing.eval(progress(p0, start, now, &r)),
                    None => 0.0,
                }
            }
        }
    }

    /// Starts closing at `now` from the current visibility. Returns false when the close motion
    /// is absent (the record is to be removed at once).
    pub(crate) fn start_closing(
        &mut self,
        now: Time,
        tokens: &MotionTokens,
        tier: MotionTier,
    ) -> bool {
        let v = self.visibility(now, tokens, tier);
        let Some(r) = usable(tokens.resolve(&self.opts.close, tier)) else {
            return false;
        };
        let p0 = if v >= 1.0 {
            0.0
        } else {
            r.easing.inverse(1.0 - v)
        };
        self.phase = RecPhase::Closing { start: now, p0 };
        true
    }

    /// Reopens a closing record at `now` from the matching visibility.
    pub(crate) fn reopen(&mut self, now: Time, tokens: &MotionTokens, tier: MotionTier) {
        let v = self.visibility(now, tokens, tier);
        self.phase = match usable(tokens.resolve(&self.opts.open, tier)) {
            Some(r) => RecPhase::Opening {
                start: now,
                p0: if v <= 0.0 { 0.0 } else { r.easing.inverse(v) },
            },
            None => RecPhase::Open,
        };
    }

    /// The closing progress at `now` and `transforms`, or `None` once the close is over.
    pub(crate) fn closing(
        &self,
        now: Time,
        tokens: &MotionTokens,
        tier: MotionTier,
    ) -> Option<(f32, bool)> {
        let RecPhase::Closing { start, p0 } = self.phase else {
            return None;
        };
        let r = usable(tokens.resolve(&self.opts.close, tier))?;
        let p = progress(p0, start, now, &r);
        (p < 1.0).then_some((p, r.transforms))
    }

    /// Whether the record is moving (opening or closing).
    pub(crate) fn moving(&self) -> bool {
        !matches!(self.phase, RecPhase::Open)
    }
}

impl<'a> Ui<'a> {
    /// Draws the overlay `key` this frame: pushes it on the stack when it is not open (or reopens
    /// it from where its close got to), runs `content` on `options.layer` with its hits tagged to
    /// the overlay and its own focus scope, and keeps `exit` to draw its closing frames once the
    /// app stops calling it. `exit` gets the linear close progress and `transforms`.
    pub fn overlay<R>(
        &mut self,
        key: impl std::hash::Hash,
        options: &OverlayOptions,
        exit: impl Fn(&mut Painter<'_, WidgetId>, f32, bool) + 'static,
        content: impl FnOnce(&mut Ui<'a>, &OverlayFrame) -> R,
    ) -> (OverlayFrame, R) {
        let id = self.id(key);
        let now = self.time();
        let (tokens, tier) = (self.config.theme.motion, self.config.tier);
        let focused = self.store.focus.focused;
        let index = match self.store.overlays.iter().position(|o| o.id == id) {
            Some(i) => i,
            None => {
                // Base content is tag 0; overlays count up from 1 in open order.
                let tag = self.store.next_tag.saturating_add(1).min(SUPPRESSED - 1);
                self.store.next_tag = tag;
                self.store.overlays.push(OverlayRec {
                    id,
                    tag,
                    opts: *options,
                    phase: RecPhase::Opening {
                        start: now,
                        p0: 0.0,
                    },
                    exit: None,
                    saved_focus: focused,
                    called: false,
                });
                self.opened.push(id);
                self.store.overlays.len() - 1
            }
        };
        let rec = &mut self.store.overlays[index];
        if !rec.active() {
            rec.reopen(now, &tokens, tier);
            rec.saved_focus = focused;
            self.opened.push(id);
        }
        rec.opts = *options;
        rec.exit = Some(Box::new(exit));
        rec.called = true;
        let tag = rec.tag;
        let (phase, transforms) = rec.opening(now, &tokens, tier);
        let frame = OverlayFrame {
            id,
            phase,
            dismissed: self.dismissed.remove(&id),
            transforms,
        };
        let saved = self.painter.save_state();
        self.painter.enter(options.layer, tag);
        let first_hit = self.painter.hits.len();
        // Scopes opened by the caller are outside the overlay for `Scope::Within`.
        let floor = std::mem::replace(&mut self.scope_floor, self.scopes.len());
        let r = content(self, &frame);
        self.scope_floor = floor;
        self.painter.restore_state(saved);
        if self.opened.contains(&id) {
            self.initial_focus(tag, first_hit, options.initial_focus, options.modal);
        }
        (frame, r)
    }

    /// Runs `f` and returns what it drew as owned painter calls, for an exit painter to replay.
    /// Nests: inside another `record` the outer recording also gets what `f` drew.
    pub fn record<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> (R, crate::paint::Ops) {
        let outer = self.painter.nest_recording();
        let r = f(self);
        (r, self.painter.unnest_recording(outer))
    }

    /// At finish: starts closing every active overlay the app did not call (restoring focus),
    /// draws every closing overlay's exit frame and drops finished ones. Returns whether any
    /// overlay is still moving.
    ///
    /// Focus follows the scope stack: a closing overlay hands focus back only when focus is in it
    /// (or gone), and a later overlay whose saved focus was inside the closing one inherits the
    /// closing one's saved focus, so its own close restores to a widget that still exists.
    pub(crate) fn settle_overlays(
        store: &mut super::UiStore,
        painter: &mut Painter<'_, WidgetId>,
        registered: &std::collections::BTreeMap<WidgetId, u32>,
        now: Time,
        tokens: &MotionTokens,
        tier: MotionTier,
    ) -> bool {
        let live: Vec<(u32, bool)> = store
            .overlays
            .iter()
            .filter(|o| o.active() && o.called)
            .map(|o| (o.tag, o.opts.modal))
            .collect();
        let mut i = 0;
        while i < store.overlays.len() {
            let rec = &mut store.overlays[i];
            if rec.active() && !rec.called {
                let (tag, saved, anchor) = (rec.tag, rec.saved_focus, rec.opts.anchor);
                let keep = rec.start_closing(now, tokens, tier);
                super::focus::restore_on_close(store, registered, &live, tag, saved, anchor);
                let in_closing = |id: WidgetId| {
                    !registered.contains_key(&id)
                        && store
                            .prev_hits
                            .iter()
                            .filter(|h| h.id == id)
                            .map(|h| h.tag)
                            .max()
                            == Some(tag)
                };
                let heirs: Vec<usize> = (i + 1..store.overlays.len())
                    .filter(|&j| store.overlays[j].saved_focus.is_some_and(in_closing))
                    .collect();
                for j in heirs {
                    store.overlays[j].saved_focus = saved;
                }
                if !keep {
                    store.overlays.remove(i);
                    continue;
                }
            }
            i += 1;
        }
        store
            .overlays
            .retain(|rec| rec.active() || rec.closing(now, tokens, tier).is_some());
        for rec in &store.overlays {
            if let (Some((p, transforms)), Some(exit)) =
                (rec.closing(now, tokens, tier), rec.exit.as_ref())
            {
                painter.enter(rec.opts.layer, SUPPRESSED);
                exit(painter, p, transforms);
            }
        }
        let mut moving = false;
        for rec in &mut store.overlays {
            rec.called = false;
            moving |= rec.moving();
        }
        moving
    }
}
