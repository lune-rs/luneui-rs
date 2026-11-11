//! Tabs with close, overflow, drag reorder, the outline and a busy cue (`widget.tab-strip`).
//!
//! The strip owns no list and no menu: it draws `tabs` and reports [`TabEvent`]s; closing and the
//! order are the app's, and a secondary click comes back as [`TabStripResponse::context`], where
//! the app opens its own menu (D292). One frame computes the layout once, and everything reads it:
//!
//! - The widths come from one place: measured, or while a gesture lives the press frame's (D123),
//!   so an activation, a hover or a title change cannot move a slot under the pointer. The
//!   gesture ends the frame the strip's tab list stops matching the one it froze.
//! - The active tab's drawn x comes from one place too: the tab's paint records it and the outline
//!   reads it, so the notch follows a dragged tab.
//! - A change of the active key slides the notch from where it was drawn to the new tab
//!   ([`SLIDE`]); nothing else does (a drag, a reorder, a tab opened or closed, a new title), so
//!   the notch is on its tab every other frame.
//! - Every id comes from a [`TabKey`], never an index, so applying a `Reorder` or removing a tab
//!   moves no state to another tab.

use std::hash::Hash;
use std::mem;

use super::button::{self, ButtonKind, ButtonLook, ButtonState, IconButton};
use super::focus_ring::{self, RingKind};
use super::progress::{breath_opacity, travel_phase};
use super::scroll_area::{Bars, Extent, Reveal, ScrollOptions, ScrollView, Target};
use super::shine;
use super::tone::Tone;
use crate::access::{Action, Node, Role, State};
use crate::chrome::{self, TabSpan};
use crate::color::{Rgba, with_alpha};
use crate::drag::{DropResult, Metrics, ReorderDrag, Strip};
use crate::icon;
use crate::input::NamedKey;
use crate::motion::{Easing, Kind, Motion, MotionTier, Reduce, Time, Token};
use crate::paint::{Painter, Rect, StrokeStyle, Xform};
use crate::text::{Spec, Style};
use crate::ui::{Response, RoveAxis, Sense, Ui, WidgetId};

/// The strip's height and the outline's seam.
pub const HEIGHT: f32 = 32.0;
/// The narrowest and widest tab.
pub const MIN_WIDTH: f32 = 108.0;
pub const MAX_WIDTH: f32 = 270.0;
/// Between tabs only: the first tab sits flush in the card's corner.
pub const GAP: f32 = 5.0;
/// A tab's top corners; its bottom corners are square.
pub const RADIUS: f32 = 8.0;
/// The close button and its glyph.
pub const CLOSE_SIZE: f32 = 20.0;
pub const CLOSE_GLYPH: f32 = 10.0;
/// The new-tab button and its glyph.
pub const NEW_SIZE: f32 = 20.0;
pub const NEW_GLYPH: f32 = 14.0;
/// The dash of [`BusyCue::Comet`], as a fraction of the contour.
pub const COMET_FRACTION: f32 = 0.14;
/// The sweep's two passes, widest first: width and opacity.
pub const SWEEP_PASSES: [(f32, f32); 2] = [(3.0, 0.25), (chrome::STROKE, 1.0)];
/// Under this visible width the active tab has no notch (twice the flare radius).
pub const MIN_NOTCH: f32 = 12.0;

const PAD_LEFT: f32 = 10.0;
const PAD_RIGHT: f32 = 4.0;
const TITLE_MARGIN: f32 = 4.0;
const CLOSE_MARGIN: f32 = 8.0;
const BUTTON_RADIUS: f32 = 4.0;
const NEW_MARGIN: f32 = 6.0;

/// The tab fill and the close button's opacity.
pub const HOVER: Motion = Motion::of(Kind::Hover);
/// The tab's focus ring.
pub const FOCUS: Motion = Motion::of(Kind::Focus);
/// The active tab's notch moving to another tab: the progress of a switch, which restarts only
/// when the active key changes.
pub const SLIDE: Motion = Motion::of(Kind::Move);
/// The outline brush between focused and unfocused.
pub const BRUSH: Motion = Motion {
    token: Token::Instant,
    ..Motion::of(Kind::Focus)
};
/// The card body's crossfade on an activation, which the app draws: base, sine in and out, fast
/// under Reduced and nothing under Off.
pub const CONTENT_SWAP: Motion = Motion {
    kind: Kind::PageSwap,
    token: Token::Base,
    easing: Easing::SineInOut,
    reduced: Reduce::Keep,
    reduced_token: Some(Token::Fast),
};

/// The app's identity of a tab: stable across a reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabKey(pub u64);

/// One tab.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tab<'a> {
    pub key: TabKey,
    pub title: &'a str,
    /// Shows the close button.
    pub closable: bool,
    /// The title shines while the work runs: a band of light passes through it, in the tone's
    /// light (`Neutral` is `fg_bright`). The tab is no wider for it.
    pub progress: Option<Tone>,
}

/// The outline's treatment while the active tab is busy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BusyCue {
    /// A bright arc travels the loop.
    #[default]
    Sweep,
    /// A dash travels the contour.
    Comet,
    /// The outline's opacity breathes.
    Breath,
}

/// What a strip shows besides its tabs.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TabStripOptions<'a> {
    pub active: Option<TabKey>,
    /// The panel has focus within it: the outline takes `border`, else `border_subtle`.
    pub panel_focused: bool,
    /// The cue the outline runs; `None` when nothing is running.
    pub busy: Option<BusyCue>,
    /// The cue's colour (`Neutral` is `accent`).
    pub busy_tone: Tone,
    /// The card the strip heads, in the strip's own space: the outline wraps it.
    pub card: Rect,
    /// The tab list's accessible name.
    pub name: &'a str,
    /// The header's tint, punched out by the outline so the active tab reads as cut out of the
    /// header. A material of the app's preset, so the app passes it; `None` draws none.
    pub header_tint: Option<Rgba>,
}

/// What the user asked for this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabEvent {
    /// A press, a key or an assistive technology activated the tab, the active one too.
    Activate(TabKey),
    Close(TabKey),
    New,
    /// The tab at `from` moves to index `to` (remove, then insert).
    Reorder {
        from: usize,
        to: usize,
    },
}

/// What [`Ui::tab_strip`] returns.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TabStripResponse {
    pub events: Vec<TabEvent>,
    /// The tab a secondary click or a show-menu action landed on, and its rect: the app opens its
    /// menu there.
    pub context: Option<(TabKey, Rect)>,
}

/// The treatment that runs for `cue` under `tier`: Reduced forces the breath, Off is static.
pub fn busy_treatment(cue: BusyCue, tier: MotionTier) -> Option<BusyCue> {
    match tier {
        MotionTier::Full => Some(cue),
        MotionTier::Reduced => Some(BusyCue::Breath),
        MotionTier::Off => None,
    }
}

/// A tab's width with a title `title` wide: the padding and the close column it reserves (a
/// closable tab reserves it hovered or not, so a hover or an activation moves no slot), within
/// [`MIN_WIDTH`] and [`MAX_WIDTH`]. Rounded up, so a title that fits is not elided.
fn tab_width(title: f32, close: bool) -> f32 {
    (chrome_width(close) + title.ceil()).clamp(MIN_WIDTH, MAX_WIDTH)
}

/// What a tab holds besides its title.
fn chrome_width(close: bool) -> f32 {
    let close = if close {
        CLOSE_SIZE + CLOSE_MARGIN
    } else {
        0.0
    };
    PAD_LEFT + TITLE_MARGIN + PAD_RIGHT + close
}

/// `tone`'s colour for the busy cue: `Neutral` is `accent`, `Warn` and `Err` are the light
/// status brushes the source names for the cue (D132).
fn cue_color(p: &Painter<'_, WidgetId>, tone: Tone) -> Rgba {
    match tone {
        Tone::Neutral => p.theme.accent,
        Tone::Warn => p.theme.status_warning_light,
        Tone::Err => p.theme.status_error_light,
        _ => tone.color(&p.theme),
    }
}

/// The light a shining title takes: `Neutral` is `fg_bright`, `Warn` and `Err` the light status
/// brushes (D132).
fn shine_light(p: &Painter<'_, WidgetId>, tone: Tone) -> Rgba {
    match tone {
        Tone::Neutral => p.theme.fg_bright,
        Tone::Warn => p.theme.status_warning_light,
        Tone::Err => p.theme.status_error_light,
        _ => tone.color(&p.theme),
    }
}

/// Draws `treatment` over the loop `pts` around `center` at time `t`.
fn paint_busy(
    p: &mut Painter<'_, WidgetId>,
    pts: &[[f32; 2]],
    center: [f32; 2],
    treatment: BusyCue,
    color: Rgba,
    t: Time,
) {
    match treatment {
        BusyCue::Sweep => {
            for (width, opacity) in SWEEP_PASSES {
                p.stroke_sweep(
                    pts,
                    with_alpha(color, opacity),
                    width,
                    travel_phase(t),
                    center,
                );
            }
        }
        BusyCue::Comet => {
            p.stroke_comet(pts, color, chrome::STROKE, COMET_FRACTION, travel_phase(t));
        }
        BusyCue::Breath => {
            let style = StrokeStyle::solid(with_alpha(color, breath_opacity(t)), chrome::STROKE);
            p.stroke(pts, true, style);
        }
    }
}

/// `n` points of the quarter arc of `radius` about `c` from angle `a0` to `a1` (radians, y down).
fn quarter(c: [f32; 2], radius: f32, a0: f32, a1: f32) -> impl Iterator<Item = [f32; 2]> {
    const STEPS: usize = 6;
    (0..=STEPS).map(move |i| {
        let a = a0 + (a1 - a0) * i as f32 / STEPS as f32;
        [c[0] + radius * a.cos(), c[1] + radius * a.sin()]
    })
}

/// The edge the header tint stops at, for the loop `pts` of a `w` wide card at `scale` physical px
/// per logical px: the stroke's outer edge, plus a third of a physical pixel, where a fill's
/// one-pixel fringe (full at its edge, none two thirds of a pixel out) crosses half coverage at the
/// stroke's own half-coverage line. So the tint and the stroke share no coverage: a tint under the
/// stroke lightened (or darkened) it, only on the arcs, a seam at each tab corner. Held inside the
/// card. A `scale` that is not finite and positive counts as 1.
fn tint_edge(pts: &[[f32; 2]], w: f32, scale: f32) -> Vec<[f32; 2]> {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    chrome::outset(pts, chrome::STROKE / 2.0 + 1.0 / (3.0 * scale))
        .into_iter()
        .map(|q| [q[0].clamp(0.0, w), q[1].max(0.0)])
        .collect()
}

/// The header of a `w` wide card minus the loop `pts` (card-local, the shape [`chrome::outline`]
/// makes for a seam at [`HEIGHT`], moved out to [`tint_edge`]) as simple polygons: the band above the loop, in two where the
/// active tab reaches the card's top. A flush tab leaves nothing on its side.
fn header_tint(pts: &[[f32; 2]], w: f32) -> Vec<Vec<[f32; 2]>> {
    use std::f32::consts::{FRAC_PI_2, PI};
    const R: f32 = chrome::CARD_RADIUS;
    let top_y = chrome::STROKE / 2.0 + 0.01;
    let seam_end = HEIGHT + chrome::SEAM_RADIUS + 0.01;
    let path: Vec<[f32; 2]> = pts.iter().copied().filter(|q| q[1] <= seam_end).collect();
    let (Some(first), Some(last)) = (path.first(), path.last()) else {
        return Vec::new();
    };
    let left_corner = || quarter([R, R], R, -FRAC_PI_2, -PI);
    let right_corner = || quarter([w - R, R], R, 0.0, -FRAC_PI_2);
    let left_edge = [0.0, first[1]];
    let right_edge = [w, last[1]];
    let touch = path.iter().position(|q| q[1] <= top_y);
    let Some(from) = touch else {
        let mut poly = path.clone();
        poly.push(right_edge);
        poly.extend(right_corner());
        poly.extend(left_corner());
        poly.push(left_edge);
        return vec![poly];
    };
    let to = path.iter().rposition(|q| q[1] <= top_y).unwrap_or(from);
    let mut out = Vec::new();
    if path[from][0] > R + 1.0 {
        let mut poly = path[..=from].to_vec();
        poly.push([path[from][0], 0.0]);
        poly.extend(left_corner());
        poly.push(left_edge);
        out.push(poly);
    }
    if path[to][0] < w - R - 1.0 {
        let mut poly = path[to..].to_vec();
        poly.push(right_edge);
        poly.extend(right_corner());
        poly.push([path[to][0], 0.0]);
        out.push(poly);
    }
    out
}

/// The frozen layout of a gesture: the tabs it started on, and their press-frame widths.
#[derive(Debug, Clone, PartialEq)]
struct Frozen {
    keys: Vec<TabKey>,
    widths: Vec<f64>,
}

/// The retained state of one strip.
#[derive(Debug, Default)]
struct StripState {
    drag: ReorderDrag,
    /// Set for as long as `drag` is active.
    frozen: Option<Frozen>,
    /// The tab a press activated, until its release, so that release is not a second activation.
    pressed: Option<TabKey>,
    /// The tab dragged or settling, which paints last.
    lifted: Option<TabKey>,
    /// The `active` already revealed.
    revealed: Option<TabKey>,
    /// The active tab last drawn and its drawn span (left and width in content space, the drag
    /// shift included): where a switch slides from.
    shown: Option<(TabKey, (f32, f32))>,
    /// Where the slide in flight started.
    slide_from: Option<(f32, f32)>,
}

/// One tab's ids and what the frame made of its layout.
#[derive(Debug, Clone, Copy)]
struct Item {
    id: WidgetId,
    close_id: WidgetId,
    hovered: bool,
    /// Shows the close button this frame.
    close: bool,
}

/// What the content of the scroll area hands back.
#[derive(Debug)]
struct Laid {
    responses: Vec<Response>,
    /// The active tab's live left edge and width, in content space (the scroll offset not
    /// applied), where its slot and any drag shift put it.
    active: Option<(f32, f32)>,
    /// The scroll offset the content was drawn at.
    offset: f64,
    scroll_by: f64,
}

impl Ui<'_> {
    /// A tab strip under `key` over `rect` (normally [`HEIGHT`] high, at the top of `opts.card`),
    /// with the new-tab button after the last tab, or at the right end when the tabs overflow.
    /// The strip is only as wide as its tabs, so a press in the space after them reaches whatever
    /// is under the strip. A closable tab reserves its close column, so a hover or an activation
    /// moves no tab; a change of `opts.active` slides the outline's notch to the new tab ([`SLIDE`]).
    /// Call it every frame. Read `events` and apply them to the app's list; the strip never
    /// changes it. A secondary click (or the show-menu action) on a tab comes back as `context`,
    /// the tab and its rect; the strip opens no menu, the app opens its own with [`Ui::menu`]
    /// there. See the module docs.
    pub fn tab_strip(
        &mut self,
        key: impl Hash,
        rect: Rect,
        tabs: &[Tab<'_>],
        opts: &TabStripOptions<'_>,
    ) -> TabStripResponse {
        let id = self.id(&key);
        let mut st = mem::take(self.transient::<StripState>(id));
        let mut events = Vec::new();
        let mut context = None;
        let mut node = Node::new(id, Role::TabList, opts.name, rect);
        if !self.is_enabled() {
            node.states.insert(State::Disabled);
        }
        self.scope(&key, |ui| {
            ui.node_container(node, |ui| {
                ui.strip_frame(rect, tabs, opts, &mut st, &mut events, &mut context);
            });
        });
        *self.transient::<StripState>(id) = st;
        TabStripResponse { events, context }
    }

    fn strip_frame(
        &mut self,
        rect: Rect,
        tabs: &[Tab<'_>],
        opts: &TabStripOptions<'_>,
        st: &mut StripState,
        events: &mut Vec<TabEvent>,
        context: &mut Option<(TabKey, Rect)>,
    ) {
        let keys: Vec<TabKey> = tabs.iter().map(|t| t.key).collect();
        // A tab added, closed or moved under a gesture ends it. The machine sees only a count, and
        // only through the owner's own call, which a closed owner never makes.
        if st.frozen.as_ref().is_some_and(|f| f.keys != keys) {
            st.drag.cancel();
            st.frozen = None;
        }
        let known = |k: &TabKey| keys.contains(k);
        st.pressed = st.pressed.filter(known);
        st.lifted = st.lifted.filter(known);

        let active = opts.active.and_then(|a| keys.iter().position(|k| *k == a));
        let hover = self.painter().hover;
        let items: Vec<Item> = tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let id = self.id(("tab", tab.key.0));
                let close_id = self.id(("close", tab.key.0));
                let hovered = hover.is_some_and(|h| h == id || h == close_id);
                Item {
                    id,
                    close_id,
                    hovered,
                    close: tab.closable && (hovered || Some(i) == active),
                }
            })
            .collect();
        let widths = st
            .frozen
            .as_ref()
            .map_or_else(|| self.measured_widths(tabs), |f| f.widths.clone());
        let (xs, extent) = slots(&widths);

        // The new-tab button follows the last tab and stops at the right end when the tabs
        // overflow. The viewport is only as wide as the tabs, so the space after them is no
        // region of the strip and a press there reaches whatever the app put under it.
        let pinned = rect.right() - NEW_MARGIN - NEW_SIZE;
        let room = f64::from((pinned - rect.x).max(0.0));
        let new_x = (rect.x + extent as f32 + NEW_MARGIN).min(pinned);
        let new_rect = Rect::new(
            new_x,
            rect.y + (rect.h - NEW_SIZE) / 2.0,
            NEW_SIZE,
            NEW_SIZE,
        );
        let viewport = Rect::new(rect.x, rect.y, extent.min(room) as f32, rect.h);

        // A reveal scrolls the slots under a held pointer, so it waits for the gesture to end.
        if !st.drag.is_active() && st.revealed != opts.active {
            if let Some(a) = active {
                let target = Target::span_x(xs[a], widths[a]);
                self.scroll_to("tabs", target, Reveal::Nearest);
            }
            st.revealed = opts.active;
        }

        let enabled = self.is_enabled();
        let roving: Vec<(WidgetId, bool)> = items.iter().map(|i| (i.id, enabled)).collect();
        let stop = self.rove_stop(&roving, active);
        self.rove_keys(&roving, RoveAxis::Horizontal, true);

        // Window space, taken outside the area: inside it the transform holds the scroll too.
        let origin = f64::from(self.painter().transformed(viewport).x);
        let area = ScrollOptions {
            bars: (Bars::Hidden.unless_fits(extent, room), Bars::Disabled),
            wheel_x: true,
            extent: Some((extent, f64::from(viewport.h))),
            ..ScrollOptions::default()
        };
        let frame = StripFrame {
            tabs,
            opts,
            keys: &keys,
            items: &items,
            widths: &widths,
            xs: &xs,
            extent,
            origin,
            active,
            stop,
        };
        let response = self.scroll_area("tabs", viewport, &area, |ui, view| {
            let laid = ui.strip_content(&frame, view, st, events, context);
            (laid, Extent::default())
        });
        let laid = response.value;
        if laid.scroll_by != 0.0 {
            self.scroll_shift_x("tabs", laid.scroll_by);
        }

        let span = self.slid_span(tabs, active, laid.active, st);
        let drawn = span.map(|(x, w)| ((f64::from(x) - laid.offset) as f32, w));
        // Up to the button, not the fitted viewport: a slide may start where a closed tab was.
        let clip = Rect::new(viewport.x, viewport.y, room as f32, viewport.h);
        self.paint_outline(clip, opts, drawn);

        let new_button = IconButton {
            glyph: icon::ADD,
            glyph_size: NEW_GLYPH,
            color: Some(self.theme().fg_muted),
            radius: BUTTON_RADIUS,
            name: self.tr("luneui.tab-strip.new"),
        };
        if self.icon_button("new", new_rect, &new_button).clicked {
            events.push(TabEvent::New);
        }

        // No tooltip under a gesture: the pointer crosses the other tabs and rests on them. The
        // pressed tab keeps its call, which is what hides its tooltip until the pointer leaves.
        let gesture = st.drag.is_active();
        for (tab, resp) in tabs.iter().zip(&laid.responses) {
            if !gesture || resp.pressed {
                self.tooltip(resp, tab.title);
            }
        }
    }

    /// The active tab's span to draw (content space): the live one, or while a switch slides the
    /// mix of the span drawn before it and the live one. Only a change of the active key starts a
    /// slide, so a drag, a drop, a reorder, a tab opened or closed beside it and a title change
    /// leave the notch exactly on its tab, and so do the first frame and a gap in the active tab.
    fn slid_span(
        &mut self,
        tabs: &[Tab<'_>],
        active: Option<usize>,
        live: Option<(f32, f32)>,
        st: &mut StripState,
    ) -> Option<(f32, f32)> {
        let id = self.id("slide");
        let (Some(i), Some(live)) = (active, live) else {
            st.shown = None;
            st.slide_from = None;
            self.snap_f32(id, 1.0);
            return None;
        };
        let key = tabs[i].key;
        if let Some((prev, span)) = st.shown
            && prev != key
        {
            st.slide_from = Some(span);
            self.snap_f32(id, 0.0);
        }
        let p = self.animate_f32(id, 1.0, &SLIDE);
        let drawn = match st.slide_from {
            Some((x, w)) if p < 1.0 => (x + (live.0 - x) * p, w + (live.1 - w) * p),
            _ => {
                st.slide_from = None;
                live
            }
        };
        st.shown = Some((key, drawn));
        Some(drawn)
    }

    /// Every tab's width from its measured title.
    fn measured_widths(&mut self, tabs: &[Tab<'_>]) -> Vec<f64> {
        tabs.iter()
            .map(|tab| {
                let spec = title_spec(self, tab.title, false);
                let title = self.painter().measure(&spec).0;
                f64::from(tab_width(title, tab.closable))
            })
            .collect()
    }

    /// The tabs: regions and nodes, input, the gesture, then the paint with the dragged tab last.
    fn strip_content(
        &mut self,
        f: &StripFrame<'_>,
        view: &ScrollView,
        st: &mut StripState,
        events: &mut Vec<TabEvent>,
        context: &mut Option<(TabKey, Rect)>,
    ) -> Laid {
        let viewport = view.viewport;
        let slot = |x: f64, i: usize| {
            Rect::new(
                viewport.x + x as f32,
                viewport.y,
                f.widths[i] as f32,
                viewport.h,
            )
        };

        let mut responses = Vec::with_capacity(f.tabs.len());
        let mut close_responses = Vec::with_capacity(f.tabs.len());
        for (i, (tab, item)) in f.tabs.iter().zip(f.items).enumerate() {
            let rect = slot(f.xs[i], i);
            let sense = Sense {
                focusable: true,
                skip_tab: f.stop != Some(i),
                ..Sense::DRAG
            };
            let resp = self.interact(item.id, rect, sense);
            let mut node = Node::new(item.id, Role::Tab, tab.title, rect);
            node.description = Some(tab.title.to_owned());
            if resp.enabled {
                node.states.insert(State::Focusable);
                for action in [
                    Action::Default,
                    Action::Focus,
                    Action::ShowMenu,
                    Action::ScrollIntoView,
                ] {
                    node.actions.insert(action);
                }
            } else {
                node.states.insert(State::Disabled);
            }
            if f.active == Some(i) {
                node.states.insert(State::Selected);
            }
            if tab.progress.is_some() || (f.active == Some(i) && f.opts.busy.is_some()) {
                node.states.insert(State::Busy);
            }
            self.node(node);
            // Registered after its tab, so it is on top. It is no tab stop: the list is one stop.
            let close = item.close.then(|| {
                let rect = close_rect(rect);
                let resp = self.interact(item.close_id, rect, Sense::CLICK);
                let name = self.tr("luneui.tab-strip.close");
                let mut node = Node::new(item.close_id, Role::Button, name, rect);
                if resp.enabled {
                    node.actions.insert(Action::Default);
                } else {
                    node.states.insert(State::Disabled);
                }
                self.node(node);
                resp
            });

            let pressed_here = st.pressed == Some(tab.key);
            // Both keys are taken whether or not the strip is enabled, so a focused tab of a
            // disabled strip does not let Enter or Space fall through to whatever handles them
            // next; a Control, Alt or Command chord on them stays a shortcut (D157).
            let keyed =
                self.activation_keys(item.id, resp.enabled, &[NamedKey::Enter, NamedKey::Space]);
            let activated =
                resp.enabled && (resp.press_started || (resp.clicked && !pressed_here) || keyed);
            if activated {
                events.push(TabEvent::Activate(tab.key));
            }
            if resp.press_started {
                st.pressed = Some(tab.key);
            }
            // A disabled response reports no press whatever the pointer does, so it must not end
            // the capture: the release after it would read as a fresh click.
            if resp.enabled && st.pressed == Some(tab.key) && !resp.pressed {
                st.pressed = None;
            }
            let close_resp = close.as_ref();
            if close_resp.is_some_and(|c| c.clicked) {
                events.push(TabEvent::Close(tab.key));
            }
            if resp.secondary_clicked || close_resp.is_some_and(|c| c.secondary_clicked) {
                *context = Some((tab.key, resp.rect));
            }
            responses.push(resp);
            close_responses.push(close);
        }

        // Every gesture before any offset, so the offsets are this frame's.
        let metrics = Metrics {
            widths: f.widths,
            spacing: f64::from(GAP),
            viewport: f64::from(viewport.w),
            offset: view.offset.0,
            extent: f.extent,
        };
        let mut scroll_by = 0.0;
        let mut commit = None;
        for (i, (item, resp)) in f.items.iter().zip(&responses).enumerate() {
            let strip = Strip {
                index: i,
                origin: f.origin,
                metrics,
            };
            let frame = self.drag_gesture(item.id, resp, &mut st.drag, &strip);
            if let DropResult::Commit { from, to } = frame.result {
                commit = Some((from, to));
            }
            scroll_by += frame.scroll_by;
        }
        match (st.drag.is_active(), &st.frozen) {
            (true, None) => {
                st.frozen = Some(Frozen {
                    keys: f.keys.to_vec(),
                    widths: f.widths.to_vec(),
                });
            }
            (false, Some(_)) => st.frozen = None,
            _ => {}
        }
        if let Some((from, to)) = commit {
            events.push(TabEvent::Reorder { from, to });
        }

        // The layout a drop lands in: the app applies it after this frame, and the settle must
        // aim where the tab will be, not where it came from.
        let xs = commit.map_or_else(
            || f.xs.to_vec(),
            |(from, to)| committed_slots(f.widths, from, to),
        );
        let shifts: Vec<f32> = f
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| self.drag_offset(item.id, &st.drag, i, xs[i] as f32))
            .collect();
        if let Some(d) = st.drag.dragged_index() {
            st.lifted = Some(f.keys[d]);
        } else if let Some(i) = st.lifted.and_then(|k| f.keys.iter().position(|q| *q == k))
            && shifts[i] == 0.0
        {
            st.lifted = None;
        }
        for (i, item) in f.items.iter().enumerate() {
            if st.drag.dragged_index() == Some(i) {
                self.patch_node(item.id, |n| {
                    n.states.insert(State::Dragging);
                });
            }
        }

        let lifted = st.lifted.and_then(|k| f.keys.iter().position(|q| *q == k));
        let order = (0..f.tabs.len())
            .filter(|i| Some(*i) != lifted)
            .chain(lifted);
        let mut active = None;
        for i in order {
            let rect = slot(xs[i] + f64::from(shifts[i]), i);
            if f.active == Some(i) {
                active = Some((rect.x, rect.w));
            }
            let looks = TabLooks {
                resp: &responses[i],
                close: close_responses[i].as_ref(),
                lifted: lifted == Some(i),
                active: f.active == Some(i),
            };
            self.paint_tab(&f.tabs[i], &f.items[i], rect, &looks);
        }
        Laid {
            responses,
            active,
            offset: view.offset.0,
            scroll_by,
        }
    }

    /// One tab at `rect`, where it is drawn this frame.
    fn paint_tab(&mut self, tab: &Tab<'_>, item: &Item, rect: Rect, look: &TabLooks<'_>) {
        let t = *self.theme();
        let filled = item.hovered || look.lifted;
        let target = with_alpha(t.surface_hover, if filled { 1.0 } else { 0.0 });
        let fill = self.animate_color(item.id.with("hover"), target, &HOVER);
        let shown = if item.close { 1.0 } else { 0.0 };
        let close_alpha = self.animate_f32(item.id.with("close"), shown, &HOVER);
        let ring = if look.resp.focus_visible { 1.0 } else { 0.0 };
        let ring = self.animate_f32(item.id.with("ring"), ring, &FOCUS);
        let (tier, now) = (self.tier(), self.time());
        // The shine moves under Full and breathes under Reduced; under Off it is a fixed mix.
        if tab.progress.is_some() && tier != MotionTier::Off {
            self.request_busy_frame(rect);
        }
        let close_state = look.close.map_or(ButtonState::default(), |c| ButtonState {
            hovered: c.hovered,
            pressed: c.pressed,
            focus_visible: false,
        });
        let close_target = ButtonLook::of(&t, ButtonKind::Ghost, close_state);
        let close_fill = self.animate_color(item.close_id.with("fill"), close_target.fill, &HOVER);
        let title_spec = title_spec(self, tab.title, look.active);

        let p = self.painter();
        if fill[3] > 0 {
            p.fill_radii(rect, [RADIUS, RADIUS, 0.0, 0.0], fill);
        }
        let x = rect.x + PAD_LEFT;
        // The column is reserved whether or not the button shows, so the title never re-elides.
        let room = rect.right()
            - x
            - TITLE_MARGIN
            - PAD_RIGHT
            - if tab.closable {
                CLOSE_SIZE + CLOSE_MARGIN
            } else {
                0.0
            };
        let elided = p.elide(&title_spec, room.max(0.0));
        match tab.progress {
            Some(tone) => {
                let light = shine_light(p, tone);
                shine::paint_at(p, &elided, x, (rect.y, rect.h), light, tier, now);
            }
            None => {
                p.text_in(&elided, Rect::new(x, rect.y, 0.0, rect.h), 0.0);
            }
        }
        if close_alpha > 0.0 {
            // The name is looked up only while the button shows.
            let name = self.tr("luneui.tab-strip.close");
            let p = self.painter();
            p.push_xform(Xform {
                alpha: close_alpha,
                ..Xform::translate(0.0, 0.0)
            });
            let button = IconButton {
                glyph: icon::CLOSE,
                glyph_size: CLOSE_GLYPH,
                color: Some(t.fg_tertiary),
                radius: BUTTON_RADIUS,
                name,
            };
            let close_look = ButtonLook {
                fill: close_fill,
                ring: 0.0,
                ..close_target
            };
            button::paint_icon(p, close_rect(rect), &button, &close_look);
            p.pop_xform();
        }
        let p = self.painter();
        focus_ring::paint(p, rect, RingKind::Edge { radius: RADIUS }, ring);
    }

    /// The header tint, the loop and the busy cue. `active` is the tab's drawn left and width;
    /// `viewport` is the area the notch is clipped to.
    fn paint_outline(
        &mut self,
        viewport: Rect,
        opts: &TabStripOptions<'_>,
        active: Option<(f32, f32)>,
    ) {
        let card = opts.card;
        if card.w <= 0.0 || card.h <= 0.0 {
            return;
        }
        let span = active.and_then(|(x, w)| {
            let left = x.max(viewport.x);
            let right = (x + w).min(viewport.right());
            (right - left >= MIN_NOTCH).then_some(TabSpan {
                left: left - card.x,
                right: right - card.x,
            })
        });
        let local = chrome::outline(card.w, card.h, HEIGHT, span);
        let t = *self.theme();
        let target = if opts.panel_focused {
            t.border
        } else {
            t.border_subtle
        };
        let brush = self.animate_color(self.id("brush"), target, &BRUSH);
        let (tier, now) = (self.tier(), self.time());
        let treatment = opts.busy.and_then(|cue| busy_treatment(cue, tier));
        if treatment.is_some() {
            self.request_busy_frame(opts.card);
        }
        let at = |q: &[f32; 2]| [q[0] + card.x, q[1] + card.y];
        let p = self.painter();
        if let Some(tint) = opts.header_tint {
            for poly in header_tint(&tint_edge(&local, card.w, p.s), card.w) {
                let pts: Vec<[f32; 2]> = poly.iter().map(at).collect();
                p.fill_path(&pts, tint);
            }
        }
        let pts: Vec<[f32; 2]> = local.iter().map(at).collect();
        p.stroke(&pts, true, StrokeStyle::solid(brush, chrome::STROKE));
        if let Some(treatment) = treatment {
            let center = [card.x + card.w / 2.0, card.y + card.h / 2.0];
            let color = cue_color(p, opts.busy_tone);
            paint_busy(p, &pts, center, treatment, color, now);
        }
    }
}

/// The frame's layout, as the content of the scroll area reads it.
#[derive(Debug)]
struct StripFrame<'f> {
    tabs: &'f [Tab<'f>],
    opts: &'f TabStripOptions<'f>,
    keys: &'f [TabKey],
    items: &'f [Item],
    widths: &'f [f64],
    xs: &'f [f64],
    extent: f64,
    /// The viewport's left edge in window space.
    origin: f64,
    active: Option<usize>,
    /// The roving stop.
    stop: Option<usize>,
}

/// What a tab's paint reads from the frame's input.
#[derive(Debug)]
struct TabLooks<'r> {
    resp: &'r Response,
    close: Option<&'r Response>,
    lifted: bool,
    active: bool,
}

fn title_spec(ui: &Ui<'_>, title: &str, active: bool) -> Spec {
    let t = ui.theme();
    let color = if active { t.fg_primary } else { t.fg_muted };
    Spec::plain(title, Style::ui(t.font_s, color))
}

/// The close button of a tab drawn at `tab`.
fn close_rect(tab: Rect) -> Rect {
    Rect::new(
        tab.right() - CLOSE_MARGIN - CLOSE_SIZE,
        tab.y + (tab.h - CLOSE_SIZE) / 2.0,
        CLOSE_SIZE,
        CLOSE_SIZE,
    )
}

/// Each tab's content x, and the strip's extent.
fn slots(widths: &[f64]) -> (Vec<f64>, f64) {
    let mut at = 0.0;
    let xs: Vec<f64> = widths
        .iter()
        .map(|w| {
            let x = at;
            at += w + f64::from(GAP);
            x
        })
        .collect();
    (xs, (at - f64::from(GAP)).max(0.0))
}

/// Each tab's content x, by its index before the move, once the tab at `from` sits at `to`.
fn committed_slots(widths: &[f64], from: usize, to: usize) -> Vec<f64> {
    let mut order: Vec<usize> = (0..widths.len()).collect();
    let moved = order.remove(from);
    order.insert(to, moved);
    let mut xs = vec![0.0; widths.len()];
    let mut at = 0.0;
    for i in order {
        xs[i] = at;
        at += widths[i] + f64::from(GAP);
    }
    xs
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The distance from `q` to the closed polyline `pts`.
    fn to_loop(q: [f32; 2], pts: &[[f32; 2]]) -> f32 {
        let n = pts.len();
        (0..n)
            .map(|k| {
                let (a, b) = (pts[k], pts[(k + 1) % n]);
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                let len2 = dx * dx + dy * dy;
                let t = if len2 > 0.0 {
                    (((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                ((q[0] - a[0] - t * dx).powi(2) + (q[1] - a[1] - t * dy).powi(2)).sqrt()
            })
            .fold(f32::INFINITY, f32::min)
    }

    #[test]
    fn the_header_tint_stops_at_the_stroke_s_outer_edge() {
        let (w, h) = (720.0, 300.0);
        let spans = [
            Some(TabSpan {
                left: 113.0,
                right: 221.0,
            }),
            Some(TabSpan {
                left: 0.0,
                right: 108.0,
            }),
            Some(TabSpan {
                left: 612.0,
                right: 720.0,
            }),
            None,
        ];
        for span in spans {
            let pts = chrome::outline(w, h, HEIGHT, span);
            for scale in [1.0, 1.5, 2.0] {
                let want = chrome::STROKE / 2.0 + 1.0 / (3.0 * scale) - 1e-3;
                let polys = header_tint(&tint_edge(&pts, w, scale), w);
                assert!(!polys.is_empty(), "{span:?} at {scale}");
                for q in polys.iter().flatten() {
                    // A point held on the card's edge has no stroke outside it to clear.
                    if q[0] <= 0.0 || q[0] >= w || q[1] <= 0.0 {
                        continue;
                    }
                    let d = to_loop(*q, &pts);
                    assert!(d >= want, "{span:?} at {scale}: {q:?} is {d} from the loop");
                }
            }
        }
    }
}
