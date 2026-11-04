//! The overlay scrollbar of a scroll area (`widget.scrollbar`): a 9 px transparent track with a
//! 6 px rounded thumb in the `accent` colour, shown while the pointer is over the area or the thumb
//! is held, taking no layout space. [`thumb_geometry`] is the pure part; [`Ui::scrollbar`] (used by
//! [`Ui::scroll_area`](crate::ui::Ui::scroll_area), not called by apps) registers and draws it.
//!
//! The bar has no node of its own and is no tab stop: the owning area's node carries the offset
//! and the actions.

use crate::color::premul;
use crate::layout::Axis;
use crate::motion::{Kind, Motion, Token};
use crate::paint::Rect;
use crate::ui::{Sense, Ui, WidgetId};

/// The bar's thickness, across its axis.
pub const BAR: f32 = 9.0;
/// The thumb's thickness.
pub const THUMB: f32 = 6.0;
/// The gap between the thumb and the edge of the viewport.
pub const THUMB_INSET: f32 = 1.0;
/// The thumb's corner radius.
pub const RADIUS: f32 = 3.0;
/// The thumb's minimum length.
pub const MIN_LENGTH: f32 = 28.0;

/// The thumb reveal and hide: `hover` kind at `fast`, linear; Reduced keeps it (at 120 ms).
pub const REVEAL: Motion = Motion {
    token: Token::Fast,
    ..Motion::of(Kind::Hover)
};
/// The thumb's hover colour: `hover` kind at `instant`.
pub const HOVER: Motion = Motion::of(Kind::Hover);

/// Where a bar and its thumb lie, in the viewport's own space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThumbGeometry {
    /// The 9 px bar, along the viewport's right (vertical) or bottom (horizontal) edge.
    pub track: Rect,
    /// The 6 px thumb inside it, 1 px from the viewport's edge.
    pub thumb: Rect,
    /// How far the thumb can travel along the axis: the track's length minus the thumb's.
    pub travel: f64,
    /// The offset range the travel maps to: `extent - viewport`.
    pub range: f64,
}

impl ThumbGeometry {
    /// The offset change that moving the thumb by `delta` px along its axis makes: a full travel
    /// is the whole range.
    pub fn offset_delta(&self, delta: f64) -> f64 {
        if self.travel > 0.0 {
            delta * self.range / self.travel
        } else {
            0.0
        }
    }
}

/// The bar of `axis` for a `viewport` showing `extent` px of content at `offset`, or `None` when
/// the content fits (or the numbers are not usable). The thumb's length is
/// `viewport / extent * track`, at least [`MIN_LENGTH`]; its place is `offset / (extent -
/// viewport)` of the travel. `corner` says the other bar is shown too: this track then stops
/// short of it by [`BAR`].
pub fn thumb_geometry(
    axis: Axis,
    viewport: Rect,
    offset: f64,
    extent: f64,
    corner: bool,
) -> Option<ThumbGeometry> {
    let vp = f64::from(match axis {
        Axis::Vertical => viewport.h,
        Axis::Horizontal => viewport.w,
    });
    if !(extent.is_finite() && vp.is_finite() && vp > 0.0 && extent > vp) {
        return None;
    }
    let range = extent - vp;
    let track_len = vp - if corner { f64::from(BAR) } else { 0.0 };
    if track_len <= 0.0 {
        return None;
    }
    let len = (vp / extent * track_len)
        .max(f64::from(MIN_LENGTH))
        .min(track_len);
    let travel = track_len - len;
    let at = if offset.is_finite() {
        (offset / range).clamp(0.0, 1.0)
    } else {
        0.0
    } * travel;
    let (track, thumb) = match axis {
        Axis::Vertical => (
            Rect::new(viewport.right() - BAR, viewport.y, BAR, track_len as f32),
            Rect::new(
                viewport.right() - THUMB_INSET - THUMB,
                viewport.y + at as f32,
                THUMB,
                len as f32,
            ),
        ),
        Axis::Horizontal => (
            Rect::new(viewport.x, viewport.bottom() - BAR, track_len as f32, BAR),
            Rect::new(
                viewport.x + at as f32,
                viewport.bottom() - THUMB_INSET - THUMB,
                len as f32,
                THUMB,
            ),
        ),
    };
    Some(ThumbGeometry {
        track,
        thumb,
        travel,
        range,
    })
}

impl Ui<'_> {
    /// Registers and draws one bar: the track (a region that blocks the content under it, so a
    /// press on the empty track does nothing) and the thumb (above it, above the content). `reveal`
    /// is whether the pointer is over the area; a held thumb stays shown wherever the pointer is.
    /// Returns the pointer travel along the axis since the last frame while the thumb is dragged.
    pub(crate) fn scrollbar(
        &mut self,
        id: WidgetId,
        axis: Axis,
        g: &ThumbGeometry,
        reveal: bool,
    ) -> f64 {
        let t = *self.theme();
        let _track = self.interact(id.with("track"), g.track, Sense::HOVER);
        let thumb = self.interact(id.with("thumb"), g.thumb, Sense::DRAG);
        let shown = if reveal || thumb.pressed { 1.0 } else { 0.0 };
        let opacity = self.animate_f32(id.with("opacity"), shown, &REVEAL);
        let hot = thumb.hovered || thumb.pressed;
        let fill = self.animate_color(
            id.with("fill"),
            if hot {
                t.scroll_thumb_hover
            } else {
                t.scroll_thumb
            },
            &HOVER,
        );
        if opacity > 0.0 {
            let c = premul(fill).map(|v| v * opacity);
            self.painter().fill_over(g.thumb, RADIUS, c);
        }
        f64::from(match axis {
            Axis::Vertical => thumb.drag_delta.1,
            Axis::Horizontal => thumb.drag_delta.0,
        })
    }
}
