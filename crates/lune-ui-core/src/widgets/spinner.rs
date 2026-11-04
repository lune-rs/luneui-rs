//! A turning busy indicator (`widget.spinner`): the sync glyph rotating about its centre, one
//! revolution per 1200 ms while busy (D117). Under Reduced and Off it is a static glyph at angle
//! 0 and reports `Idle`: a transform is not allowed there, and a spinner has no colour change to
//! keep.
//!
//! The angle is held per instance as the angle it stopped at plus the time elapsed since it
//! started being busy, so elapsed time alone sets it (a late frame does not slow it), a stop
//! holds it, and a restart continues from it rather than jumping to 0. The state is transient:
//! a spinner not drawn for a frame starts over.

use crate::access::{Node, Role, State};
use crate::color::Rgba;
use crate::icon;
use crate::motion::{MotionTier, Time, spin_angle};
use crate::paint::{Painter, Rect};
use crate::ui::Ui;
use std::hash::Hash;

/// The glyph size when the caller names none.
pub const SIZE: f32 = 14.0;

/// What a spinner shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpinnerOptions<'a> {
    /// Turning while true; a false flag holds the glyph at its angle.
    pub busy: bool,
    /// The glyph's size.
    pub size: f32,
    /// The glyph colour; `None` is `fg_secondary`.
    pub color: Option<Rgba>,
    /// The accessible name.
    pub name: &'a str,
}

impl Default for SpinnerOptions<'_> {
    fn default() -> Self {
        Self {
            busy: true,
            size: SIZE,
            color: None,
            name: "",
        }
    }
}

/// The retained phase of one spinner.
#[derive(Debug, Clone, Copy, Default)]
struct Phase {
    /// The angle in degrees it stopped at (0 to 360).
    held: f32,
    /// When the current run began; `None` while stopped.
    since: Option<Time>,
}

impl Phase {
    /// The angle at `now`.
    fn angle(&self, now: Time) -> f32 {
        match self.since {
            None => self.held,
            Some(t0) => {
                let a = self.held + spin_angle(now.saturating_sub(t0));
                if a >= 360.0 { a - 360.0 } else { a }
            }
        }
    }
}

/// Draws the glyph centred on `center`, turned by `degrees` clockwise.
pub fn paint<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    center: (f32, f32),
    size: f32,
    color: Rgba,
    degrees: f32,
) {
    p.text_rotated(
        &icon::glyph(icon::SYNC, size, color),
        center,
        degrees.to_radians(),
    );
}

impl Ui<'_> {
    /// A spinner under `key`, the glyph centred on `center`. Turns while `opts.busy` under Full
    /// (asking for the next frame); a static glyph at angle 0 under Reduced and Off. The node is
    /// `Progress` named `opts.name`, `Busy` while busy, with no value.
    pub fn spinner(&mut self, key: impl Hash, center: (f32, f32), opts: &SpinnerOptions<'_>) {
        let id = self.id(key);
        let (tier, now) = (self.tier(), self.time());
        let turning = opts.busy && tier == MotionTier::Full;
        let phase: &mut Phase = self.transient(id);
        match (turning, phase.since) {
            (true, None) => phase.since = Some(now),
            (false, Some(_)) => {
                phase.held = phase.angle(now);
                phase.since = None;
            }
            _ => {}
        }
        let degrees = if tier == MotionTier::Full {
            phase.angle(now)
        } else {
            0.0
        };
        let color = opts.color.unwrap_or(self.theme().fg_secondary);
        paint(self.painter(), center, opts.size, color, degrees);
        if turning {
            self.mark_moving();
        }
        let half = opts.size / 2.0;
        let mut node = Node::new(
            id,
            Role::Progress,
            opts.name,
            Rect::new(center.0 - half, center.1 - half, opts.size, opts.size),
        );
        if opts.busy {
            node.states.insert(State::Busy);
        }
        self.node(node);
    }
}
