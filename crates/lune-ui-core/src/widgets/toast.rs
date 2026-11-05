//! Toasts and the notification history (`overlay.toast-notifications`, D146).
//!
//! The app owns a [`Toasts`] model and pushes into it from anywhere; [`Ui::toasts`] draws it
//! straight on the toast layer and [`Ui::toast_history`] draws the archive. Time is the app's:
//! every rule reads the `now` the app passes or `ui.time()`.
//!
//! Rules (the shipped app's): at most 3 toasts show; a new one evicts the oldest that is not
//! loading and has no actions, without archiving it, and when none qualifies the list grows. An
//! info toast times out after 4 s, a success after 3 s, a warning after 6 s; an error, a loading
//! toast and a toast with actions never do. A deadline that passes while the toast is expanded
//! waits for the collapse. A toast that leaves (a timer, Dismiss, an action, [`Toasts::dismiss`])
//! is archived once at the front of the history, which keeps 50, and counts as unread. An evicted
//! toast leaves without being archived. The shipped app drops the newest history entry when the
//! archive is full, which reads as a slip; this one drops the oldest.
//!
//! Drawing: the stack sits bottom right (margin 16, gap 8, newest at the bottom), 320 wide or 720
//! expanded, without an overlay record and without a focus scope: a toast opened by an overlay
//! must not take Tab or Escape from the dialog under it, so only its buttons are tab stops, of the
//! scope the app called [`Ui::toasts`] in. A press on the body toggles the log; Dismiss and Cancel
//! do not. Motion (none in the shipped app, so D40's kinds at their defaults): appear is opacity
//! and a rise of 8 px, leave is opacity, a neighbour's move after an arrival or a removal is one
//! retargeted offset, and the expand animates the height with the text wrapped once at the target
//! width. Reduced fades instead of moving, Off snaps.

use std::collections::VecDeque;
use std::hash::Hash;
use std::time::Duration;

use super::button::{self, ButtonKind, ButtonLook, FOCUS, HOVER, IconButton, PRESS};
use super::progress::{self, ProgressOptions};
use super::scroll_area::{Extent, ScrollOptions};
use super::spinner::{self, SpinnerOptions};
use super::tone::Tone;
use crate::access::{Action, Live, Node, Role, State};
use crate::color::Rgba;
use crate::icon;
use crate::input::{CursorIcon, Event, NamedKey};
use crate::motion::{Kind, Motion, Time};
use crate::paint::{Painter, Rect, Xform};
use crate::scene::LayerId;
use crate::text::{Spec, Style};
use crate::theme::Theme;
use crate::ui::{Sense, Ui};

/// A toast appearing: opacity 0 to 1 and a rise from [`APPEAR_RISE`] px below.
pub const APPEAR: Motion = Motion::of(Kind::Appear);
/// A toast leaving: opacity 1 to 0.
pub const LEAVE: Motion = Motion::of(Kind::Dismiss);
/// A toast moving after an arrival or a removal above or below it.
pub const SHIFT: Motion = Motion::of(Kind::Move);
/// A toast growing or shrinking to show or hide its log.
pub const EXPAND: Motion = Motion::of(Kind::Expand);
/// How far below its place a toast starts.
pub const APPEAR_RISE: f32 = 8.0;

/// How many toasts show at once.
pub const MAX_VISIBLE: usize = 3;
/// How many archived toasts the history keeps.
pub const HISTORY_CAP: usize = 50;
/// When an info, a success and a warning toast time out.
pub const INFO_TIMEOUT: Duration = Duration::from_secs(4);
pub const SUCCESS_TIMEOUT: Duration = Duration::from_secs(3);
pub const WARNING_TIMEOUT: Duration = Duration::from_secs(6);

/// The collapsed and the expanded width.
pub const WIDTH: f32 = 320.0;
pub const WIDTH_EXPANDED: f32 = 720.0;
/// The distance from the window's right and bottom edge, and between toasts.
pub const MARGIN: f32 = 16.0;
pub const GAP: f32 = 8.0;
/// The corner radius, the border and the padding inside it.
pub const RADIUS: f32 = 8.0;
pub const BORDER: f32 = 1.0;
pub const PAD: f32 = 12.0;
/// The kind glyph and the gap after it; the detail's, the actions' and the log's left inset is
/// the glyph plus its gap.
pub const ICON: f32 = 16.0;
pub const ICON_GAP: f32 = 8.0;
pub const TEXT_INSET: f32 = ICON + ICON_GAP;
/// The Dismiss and Cancel glyph, the button around it (glyph plus 4 px each side), its corner
/// radius, and the space before Cancel and before Dismiss.
pub const BUTTON_GLYPH: f32 = 12.0;
pub const BUTTON: f32 = 20.0;
pub const BUTTON_RADIUS: f32 = 4.0;
pub const CANCEL_MARGIN: f32 = 8.0;
pub const DISMISS_MARGIN: f32 = 4.0;
/// The space above the detail, above the progress bar, the log block and the actions.
pub const DETAIL_TOP: f32 = 4.0;
pub const SECTION_TOP: f32 = 8.0;
/// The log block: its height, its padding, its corner radius and a line.
pub const LOG_HEIGHT: f32 = 200.0;
pub const LOG_PAD: f32 = 8.0;
pub const LOG_RADIUS: f32 = 4.0;
pub const LOG_LINE: f32 = 14.0;
/// An action button's padding, horizontal then vertical, and the space between buttons.
pub const ACTION_PAD: (f32, f32) = (16.0, 8.0);
pub const ACTION_GAP: f32 = 8.0;

/// The history panel: the space above the header, its row, the space below it and above the list
/// (the shipped panel's margins), the clear-all button, its glyph, and a row's padding, margin and
/// glyph.
pub const HISTORY_HEADER_TOP: f32 = 5.0;
pub const HISTORY_HEADER: f32 = 24.0;
pub const HISTORY_HEADER_BOTTOM: f32 = 4.0;
pub const HISTORY_LIST_TOP: f32 = 4.0;
pub const CLEAR_GLYPH: f32 = 14.0;
pub const HISTORY_PAD: (f32, f32) = (8.0, 6.0);
pub const HISTORY_MARGIN: f32 = 1.0;
pub const HISTORY_ICON: f32 = 14.0;
pub const HISTORY_GAP: f32 = 8.0;
pub const HISTORY_DETAIL_INSET: f32 = 22.0;

/// A toast's identity, stable from [`Toasts::push`] until it is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToastId(pub u64);

/// What a toast says it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
    /// A running operation: a turning glyph and a bar, determinate with a fraction or not.
    Loading {
        /// The fraction 0 to 1; `None` is an indeterminate bar.
        progress: Option<f32>,
        /// Shows the Cancel button.
        cancellable: bool,
    },
}

/// An action button of a toast (a prompt): the app's label and its own number for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastAction {
    pub label: String,
    /// The look: the filled `surface_input` button; the others are ghosts. Position is the app's order.
    pub primary: bool,
    pub id: u64,
}

/// What a toast shows. Texts are the app's.
#[derive(Debug, Clone, PartialEq)]
pub struct ToastSpec {
    pub kind: ToastKind,
    pub message: String,
    pub detail: Option<String>,
    /// The lines of the log block an expanded toast shows.
    pub log: Vec<String>,
    pub actions: Vec<ToastAction>,
}

impl ToastSpec {
    /// A toast of `kind` that says `message`, with no detail, log or actions.
    pub fn new(kind: ToastKind, message: impl Into<String>) -> Self {
        ToastSpec {
            kind,
            message: message.into(),
            detail: None,
            log: Vec::new(),
            actions: Vec::new(),
        }
    }

    /// How long this toast lives, or `None` when it stays until it is dismissed.
    fn timeout(&self) -> Option<Duration> {
        if !self.actions.is_empty() {
            return None;
        }
        match self.kind {
            ToastKind::Info => Some(INFO_TIMEOUT),
            ToastKind::Success => Some(SUCCESS_TIMEOUT),
            ToastKind::Warning => Some(WARNING_TIMEOUT),
            ToastKind::Error | ToastKind::Loading { .. } => None,
        }
    }

    /// Whether an eviction may take this toast: not loading, no actions.
    fn evictable(&self) -> bool {
        !matches!(self.kind, ToastKind::Loading { .. }) && self.actions.is_empty()
    }
}

/// When a toast times out.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Timer {
    /// Never.
    Off,
    /// It should run, from the next frame (an update turned a toast into a timed one).
    Pending,
    At(Time),
}

#[derive(Debug, Clone, PartialEq)]
struct Entry {
    id: ToastId,
    spec: ToastSpec,
    timer: Timer,
    expanded: bool,
    /// Fading out; already archived (or evicted), gone once the fade ends.
    leaving: bool,
}

/// The toasts an app has up and the archive of those that left. The app owns it, pushes from
/// anywhere and hands it to [`Ui::toasts`] and [`Ui::toast_history`] every frame.
#[derive(Debug, Clone, Default)]
pub struct Toasts {
    entries: Vec<Entry>,
    history: VecDeque<ToastSpec>,
    next: u64,
    unread: usize,
}

impl Toasts {
    /// Shows `spec` at the bottom of the stack. While 3 toasts show, the oldest that is not
    /// loading and has no actions leaves first (not archived); when none qualifies, the stack
    /// grows. The timer of a timed toast starts at `now`.
    pub fn push(&mut self, spec: ToastSpec, now: Time) -> ToastId {
        while self.entries.iter().filter(|e| !e.leaving).count() >= MAX_VISIBLE {
            let Some(oldest) = self
                .entries
                .iter_mut()
                .find(|e| !e.leaving && e.spec.evictable())
            else {
                break;
            };
            oldest.leaving = true;
            oldest.timer = Timer::Off;
        }
        self.next += 1;
        let id = ToastId(self.next);
        let timer = spec.timeout().map_or(Timer::Off, |d| Timer::At(now + d));
        self.entries.push(Entry {
            id,
            spec,
            timer,
            expanded: false,
            leaving: false,
        });
        id
    }

    /// Replaces what toast `id` shows (a loading toast that finished, a new message). A toast
    /// that was not timed and now is starts its timer at the next frame; one already timed keeps
    /// its deadline; one that is not timed any more loses it. Does nothing once the toast is
    /// leaving or gone.
    pub fn update(&mut self, id: ToastId, spec: ToastSpec) {
        let Some(e) = self.entries.iter_mut().find(|e| e.id == id && !e.leaving) else {
            return;
        };
        e.timer = match (spec.timeout(), e.timer) {
            (None, _) => Timer::Off,
            (Some(_), Timer::At(t)) => Timer::At(t),
            (Some(_), _) => Timer::Pending,
        };
        e.spec = spec;
    }

    /// Dismisses toast `id`: it fades out and is archived. Does nothing for a toast that is
    /// leaving or gone.
    pub fn dismiss(&mut self, id: ToastId, _now: Time) {
        if let Some(i) = self.entries.iter().position(|e| e.id == id && !e.leaving) {
            self.leave(i, true);
        }
    }

    /// Whether toast `id` shows its log.
    pub fn expanded(&self, id: ToastId) -> bool {
        self.entries.iter().any(|e| e.id == id && e.expanded)
    }

    /// Expands or collapses toast `id` (a press on its body toggles it).
    pub fn set_expanded(&mut self, id: ToastId, expanded: bool) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id && !e.leaving) {
            e.expanded = expanded;
        }
    }

    /// The archived toasts, newest first.
    pub fn history(&self) -> impl Iterator<Item = &ToastSpec> {
        self.history.iter()
    }

    /// How many toasts were archived since [`Self::mark_read`] or [`Self::clear_history`].
    pub fn unread(&self) -> usize {
        self.unread
    }

    pub fn mark_read(&mut self) {
        self.unread = 0;
    }

    /// Empties the history and the unread count.
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.unread = 0;
    }

    /// Whether no toast shows and none is still fading out.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Starts entry `i` leaving and archives its spec once when `archive`.
    fn leave(&mut self, i: usize, archive: bool) {
        let e = &mut self.entries[i];
        if e.leaving {
            return;
        }
        e.leaving = true;
        e.timer = Timer::Off;
        if archive {
            let spec = e.spec.clone();
            self.history.push_front(spec);
            self.history.truncate(HISTORY_CAP);
            self.unread += 1;
        }
    }

    /// The frame's timers at `now`: starts the pending ones and dismisses the expired ones that
    /// are not expanded.
    fn tick(&mut self, now: Time) {
        for i in 0..self.entries.len() {
            let e = &mut self.entries[i];
            if e.leaving {
                continue;
            }
            if e.timer == Timer::Pending
                && let Some(d) = e.spec.timeout()
            {
                e.timer = Timer::At(now + d);
            }
            if let Timer::At(t) = e.timer
                && t <= now
                && !e.expanded
            {
                self.leave(i, true);
            }
        }
    }

    /// The earliest deadline a frame is due at: a timed toast that is not expanded.
    fn next_deadline(&self) -> Option<Time> {
        self.entries
            .iter()
            .filter(|e| !e.leaving && !e.expanded)
            .filter_map(|e| match e.timer {
                Timer::At(t) => Some(t),
                _ => None,
            })
            .min()
    }
}

/// What [`Ui::toasts`] reports for a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToastResponse {
    /// The action button pressed: the toast and the action's own number. The toast leaves too.
    pub action: Option<(ToastId, u64)>,
    /// The toast whose Cancel button was pressed. The toast stays; the app updates it.
    pub cancelled: Option<ToastId>,
}

/// The measured parts of a toast at one width.
#[derive(Debug, Clone, PartialEq)]
struct Parts {
    width: f32,
    /// The width inside the border and the padding.
    inner: f32,
    msg_w: f32,
    msg_h: f32,
    row: f32,
    detail_h: f32,
    bar_h: f32,
    actions_h: f32,
    action_widths: Vec<f32>,
    total: f32,
}

/// The bar is 3 px high in both states (`MainWindow.axaml:369`, `Height="3"`).
fn bar_height(_kind: ToastKind) -> f32 {
    progress::HEIGHT_INDETERMINATE
}

fn message_style(t: &Theme) -> Style {
    Style::ui(t.font_base, t.fg_primary)
}

fn detail_style(t: &Theme) -> Style {
    Style::ui(t.font_s, t.fg_secondary)
}

fn parts<H: Copy + PartialEq>(p: &mut Painter<'_, H>, spec: &ToastSpec, expanded: bool) -> Parts {
    let t = p.theme;
    let width = if expanded { WIDTH_EXPANDED } else { WIDTH };
    let inner = width - 2.0 * (PAD + BORDER);
    let cancel = match spec.kind {
        ToastKind::Loading {
            cancellable: true, ..
        } => CANCEL_MARGIN + BUTTON,
        _ => 0.0,
    };
    let msg_w = (inner - TEXT_INSET - cancel - DISMISS_MARGIN - BUTTON).max(0.0);
    let msg_h = p
        .measure(&Spec::plain(spec.message.as_str(), message_style(&t)).wrapped(msg_w))
        .1;
    let row = msg_h.max(BUTTON);
    let detail_h = spec.detail.as_deref().map_or(0.0, |d| {
        DETAIL_TOP
            + p.measure(&Spec::plain(d, detail_style(&t)).wrapped((inner - TEXT_INSET).max(0.0)))
                .1
    });
    let bar_h = if matches!(spec.kind, ToastKind::Loading { .. }) {
        SECTION_TOP + bar_height(spec.kind)
    } else {
        0.0
    };
    let action_widths: Vec<f32> = spec
        .actions
        .iter()
        .map(|a| button::button_width(p, &a.label))
        .collect();
    let actions_h = if spec.actions.is_empty() {
        0.0
    } else {
        SECTION_TOP + action_height(&t)
    };
    let log = if expanded {
        SECTION_TOP + LOG_HEIGHT
    } else {
        0.0
    };
    Parts {
        width,
        inner,
        msg_w,
        msg_h,
        row,
        detail_h,
        bar_h,
        actions_h,
        action_widths,
        total: 2.0 * (PAD + BORDER) + row + detail_h + bar_h + log + actions_h,
    }
}

/// An action button's height: a line of the label and the padding.
fn action_height(t: &Theme) -> f32 {
    message_style(t).line + 2.0 * ACTION_PAD.1
}

/// The size of a toast with `spec` as it settles, collapsed or `expanded`.
pub fn toast_size<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    spec: &ToastSpec,
    expanded: bool,
) -> (f32, f32) {
    let m = parts(p, spec, expanded);
    (m.width, m.total)
}

fn glyph_of(kind: ToastKind) -> &'static str {
    match kind {
        ToastKind::Info => icon::INFO,
        ToastKind::Success => icon::CHECK,
        ToastKind::Warning => icon::WARNING,
        ToastKind::Error => icon::ERROR,
        ToastKind::Loading { .. } => icon::SYNC,
    }
}

fn accent_of(t: &Theme, kind: ToastKind) -> Rgba {
    match kind {
        ToastKind::Info | ToastKind::Loading { .. } => t.toast_info,
        ToastKind::Success => t.toast_success,
        ToastKind::Warning => t.toast_warning,
        ToastKind::Error => t.toast_error,
    }
}

/// What the user did to one toast this frame.
#[derive(Debug, Default)]
struct Outcome {
    toggle: bool,
    dismiss: bool,
    cancel: bool,
    action: Option<u64>,
}

/// What a toast remembers between frames.
#[derive(Debug, Default)]
struct Mem {
    /// Drawn before: its appear has begun.
    seen: bool,
    /// Where its slot was last frame, and the toasts below it then.
    slot_y: Option<f32>,
    below: Vec<u64>,
}

impl<'a> Ui<'a> {
    /// Draws `toasts` on the toast layer: the stack at the window's bottom right, newest at the
    /// bottom. Applies the frame's timers from `ui.time()`, asks for a frame at the next deadline,
    /// and animates arrivals, removals and expansion. Draws on the toast layer wherever it is
    /// called, outside any overlay and focus scope, and does not take part in a surrounding
    /// [`Ui::record`].
    ///
    /// Reports the action button or Cancel pressed this frame. Dismiss and the body press act on
    /// the model themselves. A toast is a `Status` node (`Alert` for an error) named by its
    /// message and described by its detail; its default action toggles the log, `Dismiss`
    /// dismisses it. Buttons are tab stops of the scope below; nothing takes focus on its own.
    pub fn toasts(&mut self, key: impl Hash, toasts: &mut Toasts) -> ToastResponse {
        toasts.tick(self.time());
        let mut resp = ToastResponse::default();
        let saved = self.painter().save_state();
        let paused = self.painter().pause_recording();
        self.painter().enter(LayerId::Toast, 0);
        if !self.is_enabled() {
            self.painter().push_xform(Xform {
                origin: (0.0, 0.0),
                scale: 1.0,
                dx: 0.0,
                dy: 0.0,
                alpha: crate::ui::DISABLED_ALPHA,
            });
        }
        self.scope(key, |ui| ui.toast_stack(toasts, &mut resp));
        self.painter().resume_recording(paused);
        self.painter().restore_state(saved);
        if let Some(t) = toasts.next_deadline() {
            self.request_repaint_at(t);
        }
        resp
    }

    fn toast_stack(&mut self, toasts: &mut Toasts, resp: &mut ToastResponse) {
        let tier = self.tier();
        let window = self.painter().window;
        // A toast that left before it was ever drawn has nothing to fade.
        let unseen: Vec<ToastId> = toasts
            .entries
            .iter()
            .filter(|e| e.leaving)
            .map(|e| e.id)
            .filter(|id| {
                let mem_id = self.id(id.0).with("mem");
                !self.transient::<Mem>(mem_id).seen
            })
            .collect();
        toasts.entries.retain(|e| !unseen.contains(&e.id));

        // Measure, and settle each height on its own transition.
        let mut sizes = Vec::with_capacity(toasts.entries.len());
        for e in &toasts.entries {
            let m = parts(self.painter(), &e.spec, e.expanded);
            let h_id = self.id(e.id.0).with("h");
            let h = self.animate_f32(h_id, m.total, &EXPAND);
            sizes.push((m, h));
        }
        // Lay out from the bottom edge up.
        let mut bottom = window.bottom() - MARGIN;
        let mut slots = vec![0.0f32; sizes.len()];
        for (slot, (_, h)) in slots.iter_mut().zip(&sizes).rev() {
            *slot = bottom - h;
            bottom = *slot - GAP;
        }

        let appear = self
            .theme()
            .motion
            .resolve(&APPEAR, tier)
            .is_some_and(|r| r.transforms);
        let mut outcomes: Vec<(ToastId, Outcome)> = Vec::new();
        let mut finished: Vec<ToastId> = Vec::new();
        for i in 0..toasts.entries.len() {
            let (id, leaving, expanded) = {
                let e = &toasts.entries[i];
                (e.id, e.leaving, e.expanded)
            };
            let (m, h) = &sizes[i];
            let slot_y = slots[i];
            let eid = self.id(id.0);
            let below: Vec<u64> = toasts.entries[i + 1..].iter().map(|e| e.id.0).collect();
            let mem_id = eid.with("mem");
            let (seen, prev_slot, prev_below) = {
                let mem = self.transient::<Mem>(mem_id);
                (mem.seen, mem.slot_y, std::mem::take(&mut mem.below))
            };
            // Opacity and the rise: a toast starts invisible and low; one that leaves fades from
            // where it is.
            let (alpha_id, rise_id, shift_id) =
                (eid.with("alpha"), eid.with("rise"), eid.with("shift"));
            if !seen {
                self.snap_f32(alpha_id, 0.0);
                if appear {
                    self.snap_f32(rise_id, APPEAR_RISE);
                }
            }
            let alpha = if leaving {
                self.animate_f32(alpha_id, 0.0, &LEAVE)
            } else {
                self.animate_f32(alpha_id, 1.0, &APPEAR)
            };
            let rise = if appear {
                self.animate_f32(rise_id, 0.0, &APPEAR)
            } else {
                0.0
            };
            // A slot that changed because a toast below it arrived or went keeps the toast where
            // it is and moves it to its new place: the offset takes the difference and settles.
            let mut shift = self.animate_f32(shift_id, 0.0, &SHIFT);
            if let Some(prev) = prev_slot
                && prev_below != below
            {
                self.snap_f32(shift_id, shift + prev - slot_y);
                shift = self.animate_f32(shift_id, 0.0, &SHIFT);
            }
            {
                let mem = self.transient::<Mem>(mem_id);
                mem.seen = true;
                mem.slot_y = Some(slot_y);
                mem.below = below;
            }
            if leaving && alpha <= 0.0 {
                finished.push(id);
                continue;
            }
            let card = Rect::new(window.right() - MARGIN - m.width, slot_y, m.width, *h);
            self.painter().push_xform(Xform {
                origin: (0.0, 0.0),
                scale: 1.0,
                dx: 0.0,
                dy: shift + rise,
                alpha,
            });
            let spec = toasts.entries[i].spec.clone();
            let outcome = self.scope(id.0, |ui| {
                ui.paint_toast(&spec, expanded, m, card, !leaving)
            });
            self.painter().pop_xform();
            outcomes.push((id, outcome));
        }

        for (id, o) in outcomes {
            let Some(i) = toasts.entries.iter().position(|e| e.id == id) else {
                continue;
            };
            if o.toggle {
                toasts.entries[i].expanded = !toasts.entries[i].expanded;
            }
            if o.cancel && resp.cancelled.is_none() {
                resp.cancelled = Some(id);
            }
            if let Some(a) = o.action
                && resp.action.is_none()
            {
                resp.action = Some((id, a));
            }
            if o.dismiss || o.action.is_some() {
                toasts.leave(i, true);
            }
            if o.toggle || o.dismiss || o.cancel || o.action.is_some() {
                self.request_catch_up();
            }
        }
        toasts.entries.retain(|e| !finished.contains(&e.id));
    }

    /// One toast at `card`. `live` is false while it fades out: it draws as it was, takes no
    /// input and emits no node.
    fn paint_toast(
        &mut self,
        spec: &ToastSpec,
        expanded: bool,
        m: &Parts,
        card: Rect,
        live: bool,
    ) -> Outcome {
        let t = *self.theme();
        let eid = self.scope_id();
        let mut out = Outcome::default();
        {
            let p = self.painter();
            p.fill(card, RADIUS, t.toast_bg);
            p.ring(card, RADIUS, BORDER, t.toast_border, false);
            p.push_clip_rounded(card, RADIUS);
        }
        let assertive = spec.kind == ToastKind::Error;
        let mut node = Node::new(
            eid,
            if assertive { Role::Alert } else { Role::Status },
            &spec.message,
            card,
        );
        node.description = spec.detail.clone();
        node.live = Some(if assertive {
            Live::Assertive
        } else {
            Live::Polite
        });
        if expanded {
            node.states.insert(State::Expanded);
        }
        node.actions.insert(Action::Default);
        node.actions.insert(Action::Dismiss);
        node.actions.insert(if expanded {
            Action::Collapse
        } else {
            Action::Expand
        });
        let by_access = live
            && self
                .previous_tree()
                .get(eid)
                .is_some_and(|n| n.actions.contains(Action::Dismiss))
            && self.input().events.iter().any(|ev| {
                matches!(ev, Event::Access(a) if a.target == eid && a.action == Action::Dismiss)
            });
        if by_access {
            out.dismiss = true;
        }
        if live {
            self.node_container(node, |ui| {
                ui.toast_content(spec, expanded, m, card, true, &mut out)
            });
        } else {
            self.toast_content(spec, expanded, m, card, false, &mut out);
        }
        self.painter().pop_clip();
        out
    }

    fn toast_content(
        &mut self,
        spec: &ToastSpec,
        expanded: bool,
        m: &Parts,
        card: Rect,
        live: bool,
        out: &mut Outcome,
    ) {
        let t = *self.theme();
        let eid = self.scope_id();
        let (x0, w) = (card.x + PAD + BORDER, m.inner);
        let mut y = card.y + PAD + BORDER;
        if live {
            let body = self.interact(eid, card, Sense::CLICK);
            if body.hovered {
                self.set_cursor(CursorIcon::Pointer);
            }
            out.toggle = body.clicked;
        }

        // Row 0: the glyph, the message, Cancel and Dismiss.
        let centre = (x0 + ICON / 2.0, y + m.row / 2.0);
        let accent = accent_of(&t, spec.kind);
        match spec.kind {
            ToastKind::Loading { .. } if live => {
                self.spinner(
                    "icon",
                    centre,
                    &SpinnerOptions {
                        busy: true,
                        size: ICON,
                        color: Some(accent),
                        name: "",
                    },
                );
            }
            ToastKind::Loading { .. } => {
                spinner::paint(self.painter(), centre, ICON, accent, 0.0);
            }
            kind => {
                let g = icon::glyph(glyph_of(kind), ICON, accent);
                self.painter()
                    .text_centered(&g, Rect::new(x0, y + (m.row - ICON) / 2.0, ICON, ICON));
            }
        }
        let msg = Spec::plain(spec.message.as_str(), message_style(&t)).wrapped(m.msg_w);
        self.painter()
            .text(&msg, x0 + TEXT_INSET, y + (m.row - m.msg_h) / 2.0);

        let dismiss = Rect::new(x0 + w - BUTTON, y + (m.row - BUTTON) / 2.0, BUTTON, BUTTON);
        let cancel = Rect::new(
            dismiss.x - DISMISS_MARGIN - BUTTON,
            dismiss.y,
            BUTTON,
            BUTTON,
        );
        let cancellable = matches!(
            spec.kind,
            ToastKind::Loading {
                cancellable: true,
                ..
            }
        );
        let button_of = |glyph, name| IconButton {
            glyph,
            glyph_size: BUTTON_GLYPH,
            color: None,
            radius: BUTTON_RADIUS,
            name,
        };
        if cancellable {
            let name = self.tr("luneui.dialog.cancel");
            let b = button_of(icon::CIRCLE_SLASH, name);
            if live {
                out.cancel = self.icon_button("cancel", cancel, &b).clicked;
            } else {
                let look = ButtonLook::of(&t, ButtonKind::Ghost, Default::default());
                button::paint_icon(self.painter(), cancel, &b, &look);
            }
        }
        let name = self.tr("luneui.toast-notifications.dismiss");
        let b = button_of(icon::CLOSE, name);
        if live {
            out.dismiss |= self.icon_button("dismiss", dismiss, &b).clicked;
        } else {
            let look = ButtonLook::of(&t, ButtonKind::Ghost, Default::default());
            button::paint_icon(self.painter(), dismiss, &b, &look);
        }
        y += m.row;

        // The detail, under the message.
        if let Some(d) = spec.detail.as_deref() {
            y += DETAIL_TOP;
            let s = Spec::plain(d, detail_style(&t)).wrapped((w - TEXT_INSET).max(0.0));
            let h = self.painter().measure(&s).1;
            self.painter().text(&s, x0 + TEXT_INSET, y);
            y += h;
        }

        // The bar of a running operation.
        if let ToastKind::Loading { progress, .. } = spec.kind {
            y += SECTION_TOP;
            let track = Rect::new(x0, y, w, 0.0);
            if live {
                self.progress_sized(
                    "progress",
                    track,
                    &ProgressOptions {
                        value: progress,
                        tone: Tone::Neutral,
                        head: None,
                        name: spec.message.as_str(),
                    },
                    bar_height(spec.kind),
                );
            } else {
                let bar = Rect::new(x0, y, w, bar_height(spec.kind));
                match progress {
                    Some(v) => progress::paint(self.painter(), bar, v, Tone::Neutral),
                    None => {
                        let (tier, now) = (self.tier(), self.time());
                        progress::paint_indeterminate(
                            self.painter(),
                            bar,
                            Tone::Neutral,
                            progress::indeterminate_at(tier, now),
                        );
                    }
                }
            }
            y += bar_height(spec.kind);
        }

        // The log block.
        if expanded {
            y += SECTION_TOP;
            self.toast_log(Rect::new(x0, y, w, LOG_HEIGHT), &spec.log, live);
            y += LOG_HEIGHT;
        }

        // The actions in the app's order (`MainWindow.axaml:373-392` sorts nothing), right-aligned.
        if !spec.actions.is_empty() {
            y += SECTION_TOP;
            let height = action_height(&t);
            let order: Vec<usize> = (0..spec.actions.len()).collect();
            let total: f32 = order.iter().map(|&i| m.action_widths[i]).sum::<f32>()
                + ACTION_GAP * (order.len() - 1) as f32;
            let mut x = x0 + w - total;
            for i in order {
                let a = &spec.actions[i];
                let r = Rect::new(x, y, m.action_widths[i], height);
                if live {
                    let key = format!("action-{i}");
                    if self.toast_action(&key, r, &a.label, a.primary) && out.action.is_none() {
                        out.action = Some(a.id);
                    }
                } else {
                    let look = action_look(&t, a.primary, false, false);
                    button::paint(self.painter(), r, &a.label, ButtonKind::Secondary, &look);
                }
                x += m.action_widths[i] + ACTION_GAP;
            }
        }
    }

    /// The log block at `block`: the lines in the mono face on the log fill, scrolled, following
    /// the tail. Only the lines in view are shaped.
    fn toast_log(&mut self, block: Rect, lines: &[String], live: bool) {
        let t = *self.theme();
        self.painter().fill(block, LOG_RADIUS, t.toast_log_bg);
        let inner = block.inset(LOG_PAD, LOG_PAD);
        let style = Style::mono(t.font_s, LOG_LINE, t.toast_log_fg);
        if !live {
            // The tail, as far as it fits.
            let fit = (inner.h / LOG_LINE).floor() as usize;
            let from = lines.len().saturating_sub(fit);
            let p = self.painter();
            p.push_clip(inner);
            for (k, line) in lines[from..].iter().enumerate() {
                p.text(
                    &Spec::plain(line.as_str(), style),
                    inner.x,
                    inner.y + k as f32 * LOG_LINE,
                );
            }
            p.pop_clip();
            return;
        }
        let opts = ScrollOptions {
            follow_tail: true,
            ..ScrollOptions::default()
        };
        self.scroll_area("log", inner, &opts, |ui, view| {
            let first =
                ((view.offset.1 / f64::from(LOG_LINE)).floor().max(0.0) as usize).min(lines.len());
            let last = (((view.offset.1 + f64::from(view.viewport.h)) / f64::from(LOG_LINE)).ceil()
                as usize
                + 1)
            .min(lines.len());
            let mut widest = view.viewport.w;
            let p = ui.painter();
            for (k, line) in lines[first..last].iter().enumerate() {
                let spec = Spec::plain(line.as_str(), style);
                widest = widest.max(p.measure(&spec).0);
                p.text(
                    &spec,
                    view.viewport.x,
                    view.viewport.y + (first + k) as f32 * LOG_LINE,
                );
            }
            (
                (),
                Extent::new(f64::from(widest), lines.len() as f64 * f64::from(LOG_LINE)),
            )
        });
    }

    /// An action button: the shipped app's `toast-action` look, `surface_input` for the primary
    /// one and a muted ghost for the rest. True when pressed this frame.
    fn toast_action(&mut self, key: &str, rect: Rect, label: &str, primary: bool) -> bool {
        let t = *self.theme();
        let id = self.id(key);
        let mut r = self.interact(id, rect, Sense::CLICK_FOCUS);
        if self.activation_keys(id, r.enabled, &[NamedKey::Enter, NamedKey::Space]) {
            r.clicked = true;
        }
        let target = action_look(&t, primary, r.hovered, r.pressed);
        let look = ButtonLook {
            fill: self.animate_color(id.with("fill"), target.fill, &HOVER),
            label: target.label,
            scale: self.animate_f32(
                id.with("press"),
                if r.pressed { button::PRESS_SCALE } else { 1.0 },
                &PRESS,
            ),
            ring: self.animate_f32(
                id.with("ring"),
                if r.focus_visible { 1.0 } else { 0.0 },
                &FOCUS,
            ),
        };
        button::paint(self.painter(), rect, label, ButtonKind::Secondary, &look);
        let mut node = Node::new(id, Role::Button, label, rect);
        if r.enabled {
            node.states.insert(State::Focusable);
            node.actions.insert(Action::Default);
        } else {
            node.states.insert(State::Disabled);
        }
        self.node(node);
        r.clicked
    }

    /// The notification history in `rect`: the header with its clear-all button, and the archived
    /// toasts newest first in a scrolling list. Returns the height it uses (`rect.h`). The list
    /// is a `List` of `ListItem` rows named by their messages; clear-all is a button.
    pub fn toast_history(&mut self, key: impl Hash, rect: Rect, toasts: &mut Toasts) -> f32 {
        let t = *self.theme();
        let header = self.tr("luneui.toast-notifications.history-header");
        let clear = self.tr("luneui.toast-notifications.clear-all");
        let id = self.id(&key);
        let top = rect.y + HISTORY_HEADER_TOP;
        let title = Spec::plain(header, Style::ui(t.font_m, t.on_accent).bold());
        let th = self.painter().measure(&title).1;
        self.painter()
            .text(&title, rect.x, top + (HISTORY_HEADER - th) / 2.0);

        let list_top = top + HISTORY_HEADER + HISTORY_HEADER_BOTTOM + HISTORY_LIST_TOP;
        let area = Rect::new(
            rect.x,
            list_top,
            rect.w,
            (rect.bottom() - list_top).max(0.0),
        );
        let msg_w = (rect.w - 2.0 * HISTORY_PAD.0 - HISTORY_ICON - HISTORY_GAP).max(0.0);
        let rows: Vec<(f32, f32)> = toasts
            .history
            .iter()
            .map(|s| {
                let p = self.painter();
                let msg = p
                    .measure(&Spec::plain(s.message.as_str(), message_style(&t)).wrapped(msg_w))
                    .1;
                let detail = s.detail.as_deref().map_or(0.0, |d| {
                    2.0 + p
                        .measure(&Spec::plain(d, detail_style(&t)).wrapped(
                            (msg_w + HISTORY_ICON + HISTORY_GAP - HISTORY_DETAIL_INSET).max(0.0),
                        ))
                        .1
                });
                let content = msg.max(HISTORY_ICON) + detail;
                (msg, 2.0 * (HISTORY_MARGIN + HISTORY_PAD.1) + content)
            })
            .collect();
        let total: f32 = rows.iter().map(|r| r.1).sum();

        let mut list = Node::new(id, Role::List, header, area);
        list.description = None;
        let history: Vec<ToastSpec> = toasts.history.iter().cloned().collect();
        self.node_container(list, |ui| {
            ui.scope(&key, |ui| {
                ui.scroll_area("list", area, &ScrollOptions::default(), |ui, view| {
                    let mut y = view.viewport.y;
                    for (n, (spec, (msg_h, h))) in history.iter().zip(&rows).enumerate() {
                        let row = Rect::new(view.viewport.x, y, view.viewport.w, *h);
                        let x = row.x + HISTORY_PAD.0;
                        let top = row.y + HISTORY_MARGIN + HISTORY_PAD.1;
                        let content = msg_h.max(HISTORY_ICON);
                        let g = icon::glyph(
                            glyph_of(spec.kind),
                            HISTORY_ICON,
                            accent_of(&t, spec.kind),
                        );
                        let p = ui.painter();
                        p.text_centered(
                            &g,
                            Rect::new(
                                x,
                                top + (content - HISTORY_ICON) / 2.0,
                                HISTORY_ICON,
                                HISTORY_ICON,
                            ),
                        );
                        let tx = x + HISTORY_ICON + HISTORY_GAP;
                        p.text(
                            &Spec::plain(spec.message.as_str(), message_style(&t)).wrapped(msg_w),
                            tx,
                            top + (content - msg_h) / 2.0,
                        );
                        if let Some(d) = spec.detail.as_deref() {
                            p.text(
                                &Spec::plain(d, detail_style(&t)).wrapped(
                                    (msg_w + HISTORY_ICON + HISTORY_GAP - HISTORY_DETAIL_INSET)
                                        .max(0.0),
                                ),
                                x + HISTORY_DETAIL_INSET,
                                top + content + 2.0,
                            );
                        }
                        let mut item =
                            Node::new(ui.id("row").with(n), Role::ListItem, &spec.message, row);
                        item.description = spec.detail.clone();
                        ui.node(item);
                        y += h;
                    }
                    (
                        (),
                        Extent::new(f64::from(view.viewport.w), f64::from(total)),
                    )
                });
            });
        });

        let button = IconButton {
            glyph: icon::CLEAR_ALL,
            glyph_size: CLEAR_GLYPH,
            color: Some(t.fg_dim),
            radius: BUTTON_RADIUS,
            name: clear,
        };
        let at = Rect::new(
            rect.right() - HISTORY_HEADER,
            top,
            HISTORY_HEADER,
            HISTORY_HEADER,
        );
        let cleared = self.scope(&key, |ui| ui.icon_button("clear", at, &button).clicked);
        if cleared {
            toasts.clear_history();
            self.request_catch_up();
        }
        rect.h
    }
}

/// The look of an action button at a state.
fn action_look(t: &Theme, primary: bool, hovered: bool, pressed: bool) -> ButtonLook {
    let [r, g, b, _] = t.surface_hover;
    let fill = match (pressed, hovered, primary) {
        (true, _, _) => t.surface_active,
        (false, true, _) => t.surface_hover,
        (false, false, true) => t.surface_input,
        (false, false, false) => [r, g, b, 0],
    };
    ButtonLook {
        fill,
        label: if primary {
            t.fg_primary
        } else {
            t.fg_secondary
        },
        scale: if pressed { button::PRESS_SCALE } else { 1.0 },
        ring: 0.0,
    }
}
