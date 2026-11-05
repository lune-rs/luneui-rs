//! The text box on the editing engine ([`TextEdit`](crate::text_edit::TextEdit)), on the same four
//! layers as the button (see `button.rs`): a [`FieldLook`] and its settled values, [`paint`], the
//! Painter-API [`field`] and [`Ui::text_field_with`] with [`Ui::text_field`] over it.
//!
//! The Painter API draws a static box. The `Ui` form owns an [`EditState`] per id, and a frame runs
//! in this order: sync the engine with the app's text, run the focus events through it, then the
//! pointer, then draw. [`begin_edit`], [`edit_pointer`], [`Shown`] and [`draw_shown`] are shared
//! with the [text area](super::text_area).
//!
//! Offsets are the engine's chars; `SpecLayout` is the one place they meet the text system's bytes.
//! While composing, what is drawn is `TextEdit::display`'s string, which the selection offsets do
//! not index. A password's real text reaches no `Spec`: everything is drawn, measured and hit on
//! the fixed mask, which has no positions, so [`MaskLayout`] answers for it.

use std::hash::Hash;
use std::ops::Range;

use super::input_box::{self, InputBoxLook};
use super::label;
use crate::access::{AccessValue, Action, Node, Role, State, TextValue, Value};
use crate::color::{Rgba, premul};
use crate::icon;
use crate::input::{CursorIcon, FocusEvent, ImeEvent};
use crate::motion::{Kind, Lerp, Motion};
use crate::paint::{Painter, Rect};
use crate::text::{CaretPos, Font, LineSpan, Spec, Style, TextSystem, Unit};
use crate::text_edit::{EditCommand, EditFlags, EditLayout, EditOutcome, SpecLayout, TextEdit};
use crate::theme::Theme;
use crate::ui::{Response, Sense, Ui, WidgetId};

/// Box fill on hover (not while focused).
pub const HOVER: Motion = Motion::of(Kind::Hover);
/// Ring colour: `border` to `accent` on focus.
pub const FOCUS: Motion = Motion::of(Kind::Focus);
/// Ring colour: to `err` on an error.
pub const ERROR: Motion = Motion::of(Kind::Selection);

/// The height of a `small` field.
pub const SMALL_HEIGHT: f32 = 24.0;
/// How many dots a password shows, whatever its length.
pub const MASK_DOTS: usize = 16;

/// From the box's left edge to the text.
const TEXT_INSET: f32 = 10.0;
/// From the text's left edge back to the clip.
const CLIP_GAP: f32 = 4.0;
/// From the box's right edge to the clip.
const CLIP_RIGHT: f32 = 6.0;
/// The search variant's text inset.
const SEARCH_INSET: f32 = 33.0;
/// The date variant's right padding, which holds its icon.
const DATE_PADDING: f32 = 28.0;
/// The icons of the search and date variants.
const ICON: f32 = 16.0;
/// From the box's left edge to the search icon: the text inset less the icon and a margin of 8.
const SEARCH_ICON_X: f32 = 9.0;
/// The clear button's side and its icon.
const CLEAR: f32 = 18.0;
const CLEAR_ICON: f32 = 12.0;
/// Between the text's clip and the clear button.
const CLEAR_GAP: f32 = 2.0;
/// The caret's width, its distance from the glyph edge, and its height with no text to measure.
const CARET_WIDTH: f32 = 1.5;
const CARET_GAP: f32 = 1.0;
const EMPTY_CARET: f32 = 16.0;
/// What the horizontal scroll keeps between the caret and the clip.
const CARET_MARGIN: f32 = 2.0;
/// What the caret takes past the text's end.
pub(super) const CARET_ROOM: f32 = CARET_GAP + CARET_WIDTH;

/// An edit takes hover, clicks, drags and focus.
pub(super) const EDIT_SENSE: Sense = Sense {
    hover: true,
    click: true,
    drag: true,
    focusable: true,
    scroll: false,
    skip_tab: false,
};

/// What the app says about the field: an error flag (the form around it sets it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FieldState {
    pub error: bool,
}

/// The look and behaviour a field takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Variant {
    #[default]
    Plain,
    /// The mono face, for ids, URLs and paths.
    Mono,
    /// A fixed mask, never the real length; the value cannot be copied and takes no composition.
    Password,
    /// A search icon at the left and a clear button while the text is non-empty.
    Search,
    /// A calendar icon at the right and the mono face. Only the look: there is no picker.
    Date,
}

/// The chrome a field draws around its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FieldChrome {
    /// The box: fill, border and the focus and error rings.
    #[default]
    Box,
    /// None: no fill, border or ring, the caret in `fg_tertiary`. The text, the placeholder,
    /// editing, focus, composition and the node are the box field's. For a bar that draws its own
    /// frame (the filter bar).
    Bare,
}

/// What [`Ui::text_field_with`] is given besides its rect and text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FieldOptions<'a> {
    /// Shown while the text is empty; the accessible name when there is no `label`.
    pub placeholder: &'a str,
    pub error: bool,
    pub variant: Variant,
    /// The box (default) or no chrome at all.
    pub chrome: FieldChrome,
    /// 24 high, whatever the rect says; the top stays.
    pub small: bool,
    /// The caret and selection work, edits do not.
    pub read_only: bool,
    /// The most chars an edit may leave in the text.
    pub max_chars: Option<usize>,
    /// The accessible name.
    pub label: Option<&'a str>,
    /// The accessible description.
    pub description: Option<&'a str>,
}

/// What [`Ui::text_field_with`] and [`Ui::text_area`] return.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldResponse {
    /// `changed` tells that the text changed.
    pub response: Response,
    /// Enter was pressed in a one-line field.
    pub submitted: bool,
    /// The caret or the selected clause, in logical pixels, while the widget takes composition.
    pub ime_area: Option<Rect>,
}

/// The continuous look of a text field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldLook {
    /// Box fill: `surface_hover` when hovered and not focused, else `surface_input`.
    pub fill: Rgba,
    /// Ring from `border` (0) to `accent` (1).
    pub focus: f32,
    /// Ring from the colour the focus gives to `err` (1); 0 keeps it.
    pub error: f32,
    /// The static caret, drawn while focused.
    pub caret: bool,
}

impl FieldLook {
    /// The look a field settles to for these flags.
    pub fn of(theme: &Theme, hovered: bool, focused: bool, state: FieldState) -> FieldLook {
        FieldLook {
            fill: if hovered && !focused {
                theme.surface_hover
            } else {
                theme.surface_input
            },
            focus: if focused { 1.0 } else { 0.0 },
            error: if state.error { 1.0 } else { 0.0 },
            caret: focused,
        }
    }
}

/// Draws the box. The ring is `border` at focus 0 and `accent` at 1; `err` replaces it at error 1.
/// Each end is exact, so the rest look is the old `input_box` look.
fn paint_box<H: Copy + PartialEq>(p: &mut Painter<'_, H>, r: Rect, look: &FieldLook) {
    let t = p.theme;
    let ring = t
        .border
        .map(f32::from)
        .lerp(t.accent.map(f32::from), look.focus)
        .map(|c| c.round() as u8);
    let border = ring
        .map(f32::from)
        .lerp(t.err.map(f32::from), look.error)
        .map(|c| c.round() as u8);
    input_box::paint(
        p,
        r,
        &InputBoxLook {
            fill: look.fill,
            border,
        },
    );
}

/// [`paint_box`] for the text area, which draws its text in a scroll area.
pub(super) fn paint_frame(ui: &mut Ui<'_>, r: Rect, look: &FieldLook) {
    paint_box(ui.painter(), r, look);
}

/// Draws a field showing `text` (or `placeholder` while empty) in `r` with `look`.
pub fn paint<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    text: &str,
    placeholder: &str,
    look: &FieldLook,
) {
    let t = p.theme;
    paint_box(p, r, look);
    p.push_clip(r.inset(CLIP_RIGHT, 0.0));
    let (w, h) = if text.is_empty() {
        p.text_in(&label(&t, placeholder, t.fg_tertiary), r, TEXT_INSET);
        (0.0, 0.0)
    } else {
        p.text_in(&label(&t, text, t.fg_primary), r, TEXT_INSET)
    };
    if look.caret {
        let ch = if h > 0.0 { h } else { EMPTY_CARET };
        let caret = Rect::new(
            r.x + TEXT_INSET + w + CARET_GAP,
            r.y + (r.h - ch) / 2.0,
            CARET_WIDTH,
            ch,
        );
        p.fill_over(caret, 0.0, premul(t.fg_primary));
    }
    p.pop_clip();
}

/// The Painter-API field: the discrete look (`focused` given, hover from the painter), then its
/// hit region.
pub fn field<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    text: &str,
    placeholder: &str,
    focused: bool,
    hit: H,
) {
    let look = FieldLook::of(&p.theme, p.hovered(hit), focused, FieldState::default());
    paint(p, r, text, placeholder, &look);
    p.hit(r, hit);
}

/// How one edit widget runs its engine and shapes its text.
#[derive(Debug, Clone, Copy)]
pub(super) struct EditConfig {
    pub(super) style: Style,
    pub(super) password: bool,
    pub(super) read_only: bool,
    pub(super) multiline: bool,
    pub(super) max_chars: Option<usize>,
    /// The width the text wraps at.
    pub(super) wrap: Option<f32>,
    /// How many lines a page motion crosses.
    pub(super) page_rows: usize,
}

impl EditConfig {
    fn flags(&self) -> EditFlags {
        EditFlags {
            read_only: self.read_only,
            password: self.password,
            multiline: self.multiline,
            max_chars: self.max_chars,
            composing: false,
        }
    }

    /// The spec that draws `shown`; for a password, the mask, or nothing while it is empty.
    fn spec(&self, shown: &str) -> Spec {
        let text = if self.password && !shown.is_empty() {
            "\u{2022}".repeat(MASK_DOTS)
        } else {
            shown.to_owned()
        };
        Spec {
            runs: vec![(text, self.style)],
            wrap: self.wrap,
        }
    }

    /// The placeholder's spec.
    pub(super) fn placeholder(&self, text: &str, color: Rgba) -> Spec {
        Spec {
            runs: vec![(
                text.to_owned(),
                Style {
                    color,
                    ..self.style
                },
            )],
            wrap: self.wrap,
        }
    }
}

/// The retained state of one edit widget.
#[derive(Debug, Default)]
pub(super) struct EditState {
    pub(super) edit: TextEdit,
    /// A one-line field's horizontal scroll, in logical px.
    scroll: f32,
    /// A text area's offset as its scroll area last gave it.
    pub(super) offset: (f64, f64),
    /// A clipboard read this widget asked for is unanswered.
    paste_pending: bool,
    /// The engine has its first caret: the text's end.
    started: bool,
    /// The press in progress extends the selection as it drags (a word or line press keeps what
    /// it selected).
    drag_extends: bool,
    /// A command moved the caret, so a text area brings it into view.
    pub(super) reveal: bool,
}

/// Set on every frame that draws the widget: a fresh one means the last frame did not, and a
/// composition or a clipboard answer does not outlive that.
#[derive(Debug, Default)]
struct Drawn(bool);

/// A password's layout: the mask has no positions, so the caret and any point are after it, and
/// its one line spans the real text.
struct MaskLayout<'a> {
    inner: SpecLayout<'a>,
    chars: usize,
}

impl EditLayout for MaskLayout<'_> {
    fn boundaries(&mut self, text: &str, unit: Unit) -> Vec<usize> {
        self.inner.boundaries(text, unit)
    }

    fn caret(&mut self, _offset: usize) -> CaretPos {
        self.inner.caret(usize::MAX)
    }

    fn hit(&mut self, _pos: (f32, f32)) -> usize {
        self.chars
    }

    fn lines(&mut self) -> Vec<LineSpan> {
        let chars = self.chars;
        let mut lines = self.inner.lines();
        lines.truncate(1);
        for line in &mut lines {
            line.start = 0;
            line.end = chars;
        }
        lines
    }
}

/// Runs `f` on the layout of `spec`, which shows a text of `chars` chars.
fn with_layout<R>(
    ui: &mut Ui<'_>,
    cfg: &EditConfig,
    spec: &Spec,
    chars: usize,
    f: impl FnOnce(&mut dyn EditLayout) -> R,
) -> R {
    let p = ui.painter();
    let scale = p.s;
    let system: &mut dyn TextSystem = &mut *p.text;
    let mut inner = SpecLayout::new(system, spec, scale).with_page_rows(cfg.page_rows);
    if cfg.password {
        f(&mut MaskLayout { inner, chars })
    } else {
        f(&mut inner)
    }
}

fn sync(ui: &mut Ui<'_>, cfg: &EditConfig, edit: &mut TextEdit, text: &str) {
    let spec = cfg.spec(text);
    let chars = text.chars().count();
    with_layout(ui, cfg, &spec, chars, |l| edit.sync(text, l));
}

/// Runs one command. The layout is built from `text` as it stands: an earlier command of the frame
/// may have changed it.
fn apply(
    ui: &mut Ui<'_>,
    cfg: &EditConfig,
    edit: &mut TextEdit,
    text: &mut String,
    cmd: EditCommand,
) -> EditOutcome {
    let (keymap, now) = (ui.keymap(), ui.time());
    let spec = cfg.spec(text);
    let chars = text.chars().count();
    with_layout(ui, cfg, &spec, chars, |l| {
        edit.apply(cmd, text, l, keymap, now)
    })
}

/// What the input of a frame did to the text.
#[derive(Debug, Default)]
pub(super) struct EditFrame {
    pub(super) changed: bool,
    pub(super) submitted: bool,
}

/// Starts an edit's frame: takes the state of `id` (hand it back with [`end_edit`]), syncs it with
/// the app's text, ends what focus loss ends, and runs this frame's focus events through the
/// engine, consuming what it consumed. A widget that holds the key focus in a disabled scope takes
/// the keys it takes enabled and runs none of them (D140, D177).
pub(super) fn begin_edit(
    ui: &mut Ui<'_>,
    id: WidgetId,
    cfg: &EditConfig,
    text: &mut String,
) -> (EditState, EditFrame) {
    let mut st = std::mem::take(ui.persistent::<EditState>(id));
    let drawn = std::mem::replace(&mut ui.transient::<Drawn>(id).0, true);
    let key_focus = ui.focus_state().0 == Some(id);
    let focused = ui.is_enabled() && key_focus;
    st.edit.set_flags(cfg.flags());
    if !st.started {
        st.started = true;
        let end = text.chars().count();
        st.edit.set_selection(end, end);
    }
    sync(ui, cfg, &mut st.edit, text);
    if !focused || !drawn {
        st.paste_pending = false;
        if st.edit.preedit().is_some() {
            apply(ui, cfg, &mut st.edit, text, EditCommand::ImeCancel);
        }
    }
    let mut frame = EditFrame::default();
    if key_focus {
        run_events(ui, id, cfg, &mut st, text, &mut frame, focused);
    }
    (st, frame)
}

/// Runs the focus events of `id` through the engine. With `act` false (a disabled scope) the engine
/// only says what it takes: those events are consumed and nothing else happens, no command, no
/// clipboard call, no change to the pending paste.
fn run_events(
    ui: &mut Ui<'_>,
    id: WidgetId,
    cfg: &EditConfig,
    st: &mut EditState,
    text: &mut String,
    frame: &mut EditFrame,
    act: bool,
) {
    let keymap = ui.keymap();
    let events = ui.focus_events(id);
    for (n, (index, event)) in events.iter().enumerate() {
        // Not acting: a clipboard answer is nobody's to insert, and the pending paste stays as it is.
        if !act
            && matches!(
                event,
                FocusEvent::ClipboardText(_) | FocusEvent::ClipboardUnavailable
            )
        {
            continue;
        }
        match event {
            // Another widget's answer, or a stale one.
            FocusEvent::ClipboardText(_) | FocusEvent::ClipboardUnavailable
                if !st.paste_pending =>
            {
                continue;
            }
            FocusEvent::ClipboardText(_) => st.paste_pending = false,
            FocusEvent::ClipboardUnavailable => {
                st.paste_pending = false;
                ui.consume_focus_event(*index);
                continue;
            }
            // A password shows no composition: a preedit would draw the real characters.
            FocusEvent::Ime(ImeEvent::Preedit { .. }) if cfg.password => {
                ui.consume_focus_event(*index);
                continue;
            }
            _ => {}
        }
        let flags = st.edit.flags();
        let Some(cmd) = EditCommand::from_event(event, keymap, flags) else {
            // The press behind typed text is the field's too: no binding on it may also fire.
            let types = matches!(event, FocusEvent::Key { .. })
                && !flags.read_only
                && !flags.composing
                && matches!(events.get(n + 1), Some((_, FocusEvent::Text(_))));
            if types {
                ui.consume_focus_event(*index);
            }
            continue;
        };
        if !act {
            if st.edit.takes(&cmd) {
                ui.consume_focus_event(*index);
            }
            continue;
        }
        let out = apply(ui, cfg, &mut st.edit, text, cmd);
        if out.consumed {
            ui.consume_focus_event(*index);
            st.reveal = true;
        }
        frame.changed |= out.changed;
        frame.submitted |= out.submitted;
        if let Some(write) = out.clipboard_write {
            ui.clipboard_write(write);
        }
        if out.clipboard_read {
            ui.clipboard_read();
            st.paste_pending = true;
        }
    }
}

/// Replaces the text with an assistive technology's `value`, under the rules a paste follows. The
/// history goes, even for an equal text, and the caret lands at the end.
pub(super) fn set_value(
    ui: &mut Ui<'_>,
    cfg: &EditConfig,
    st: &mut EditState,
    text: &mut String,
    value: String,
) {
    let mut next = String::new();
    let mut fresh = TextEdit::new();
    fresh.set_flags(cfg.flags());
    apply(
        ui,
        cfg,
        &mut fresh,
        &mut next,
        EditCommand::PasteText(value),
    );
    let end = next.chars().count();
    *text = next;
    st.edit = TextEdit::new();
    st.edit.set_flags(cfg.flags());
    st.edit.set_selection(end, end);
    sync(ui, cfg, &mut st.edit, text);
}

/// Runs a response's press and drag through the engine. `origin` is the text's top-left corner in
/// the space `r`'s widget draws in.
pub(super) fn edit_pointer(
    ui: &mut Ui<'_>,
    cfg: &EditConfig,
    st: &mut EditState,
    r: &Response,
    text: &mut String,
    origin: (f32, f32),
) {
    let at = ui
        .painter()
        .transformed(Rect::new(origin.0, origin.1, 0.0, 0.0));
    let local = |p: (f32, f32)| (p.0 - at.x, p.1 - at.y);
    if r.press_started {
        let composing = st.edit.preedit().is_some();
        let shift = r.press_modifiers.shift;
        st.drag_extends = !composing && (r.press_count <= 1 || shift);
        if let Some(pos) = r.press_pos.filter(|_| !composing) {
            let pos = local(pos);
            let offset = |ui: &mut Ui<'_>, text: &str| {
                let spec = cfg.spec(text);
                with_layout(ui, cfg, &spec, text.chars().count(), |l| l.hit(pos))
            };
            let cmd = match r.press_count {
                _ if shift => EditCommand::ExtendTo(pos),
                0 | 1 => EditCommand::PlaceCaret(pos),
                _ if cfg.password => EditCommand::SelectAll,
                2 => EditCommand::SelectWordAt(offset(ui, text)),
                _ => EditCommand::SelectLineAt(offset(ui, text)),
            };
            apply(ui, cfg, &mut st.edit, text, cmd);
            st.reveal = true;
        }
    }
    if r.dragged
        && st.drag_extends
        && let Some(pos) = ui.pointer_pos()
    {
        // The mask has no positions: dragging over it selects it.
        let cmd = if cfg.password {
            EditCommand::SelectAll
        } else {
            EditCommand::ExtendTo(local(pos))
        };
        apply(ui, cfg, &mut st.edit, text, cmd);
        st.reveal = true;
    }
    if r.hovered || r.pressed {
        ui.set_cursor(CursorIcon::Text);
    }
}

/// What an edit draws this frame. Offsets are chars of `spec`'s text, which is the app's text with
/// the preedit in place of the selection, or the mask.
#[derive(Debug)]
pub(super) struct Shown {
    pub(super) spec: Spec,
    /// Nothing to show: the placeholder takes its place.
    pub(super) empty: bool,
    selection: Range<usize>,
    preedit: Option<Range<usize>>,
    /// The selected clause of the preedit.
    clause: Option<Range<usize>>,
    /// Where the caret is drawn, if it is.
    caret: Option<usize>,
    /// What the view follows: the caret, or the clause or preedit that hides it.
    pub(super) focus_at: usize,
}

impl Shown {
    pub(super) fn new(cfg: &EditConfig, edit: &TextEdit, text: &str, focused: bool) -> Shown {
        if cfg.password {
            let spec = cfg.spec(text);
            let dots = if text.is_empty() { 0 } else { MASK_DOTS };
            let selected = focused && !edit.selection().is_empty();
            return Shown {
                spec,
                empty: text.is_empty(),
                selection: if selected { 0..dots } else { 0..0 },
                preedit: None,
                clause: None,
                caret: Some(dots),
                focus_at: dots,
            };
        }
        let (shown, preedit) = edit.display(text);
        let (selection, clause, caret, focus_at) = match (&preedit, edit.preedit_cursor()) {
            (None, _) => {
                let sel = if focused { edit.selection() } else { 0..0 };
                (sel, None, Some(edit.caret()), edit.caret())
            }
            (Some(p), Some((a, b))) if a == b => (0..0, None, Some(p.start + a), p.start + a),
            (Some(p), Some((a, b))) => (0..0, Some(p.start + a..p.start + b), None, p.start + b),
            (Some(p), None) => (0..0, None, None, p.end),
        };
        Shown {
            empty: shown.is_empty(),
            spec: cfg.spec(&shown),
            selection,
            preedit,
            clause,
            caret,
            focus_at,
        }
    }
}

/// Runs `f` on the plain layout of `spec`.
fn drawn_layout<R>(ui: &mut Ui<'_>, spec: &Spec, f: impl FnOnce(&mut SpecLayout<'_>) -> R) -> R {
    let p = ui.painter();
    let scale = p.s;
    let system: &mut dyn TextSystem = &mut *p.text;
    f(&mut SpecLayout::new(system, spec, scale))
}

/// Where the caret sits at `offset` of `spec`.
pub(super) fn caret_at(ui: &mut Ui<'_>, spec: &Spec, offset: usize) -> CaretPos {
    drawn_layout(ui, spec, |l| l.caret(offset))
}

/// The boxes of chars `range` of `spec`, one per visual line it touches, from the text's top-left.
pub(super) fn range_boxes(ui: &mut Ui<'_>, spec: &Spec, range: &Range<usize>) -> Vec<Rect> {
    let Some((text, style)) = spec.runs.first().filter(|_| !range.is_empty()) else {
        return Vec::new();
    };
    let parts = drawn_layout(ui, spec, |l| {
        let rows = l.lines();
        let spans = rows.iter().enumerate().filter_map(|(i, row)| {
            let (a, b) = (range.start.max(row.start), range.end.min(row.end));
            (a < b).then_some((i, *row, a, b))
        });
        let spans: Vec<_> = spans.collect();
        spans
            .into_iter()
            .map(|(i, row, a, b)| (i, row, a, b, l.caret(a), l.caret(b)))
            .collect::<Vec<_>>()
    });
    let mut boxes = Vec::with_capacity(parts.len());
    for (i, row, a, b, from, to) in parts {
        // A soft wrap's end offset shows on the next row, so this row's part is measured.
        let to_x = if to.line == i {
            to.x
        } else {
            let part: String = text.chars().skip(a).take(b - a).collect();
            from.x + ui.painter().measure(&Spec::plain(part, *style)).0
        };
        let (x0, x1) = (from.x.min(to_x), from.x.max(to_x));
        boxes.push(Rect::new(x0, row.y, x1 - x0, row.height));
    }
    boxes
}

/// Draws `shown` with its text's top-left at `(x, y)`: the selection and the clause under the
/// text, then the text, the preedit's underlines and, with `caret`, the caret. Returns the box the
/// input method's candidate window avoids, in the same space.
pub(super) fn draw_shown(ui: &mut Ui<'_>, shown: &Shown, at: (f32, f32), caret: bool) -> Rect {
    let ink = ui.theme().fg_primary;
    draw_shown_inked(ui, shown, at, caret, ink)
}

/// [`draw_shown`] with the caret in `caret_ink`.
pub(super) fn draw_shown_inked(
    ui: &mut Ui<'_>,
    shown: &Shown,
    (x, y): (f32, f32),
    caret: bool,
    caret_ink: Rgba,
) -> Rect {
    let t = *ui.theme();
    let at = |b: &Rect| Rect::new(x + b.x, y + b.y, b.w, b.h);
    for b in range_boxes(ui, &shown.spec, &shown.selection) {
        ui.painter().fill(at(&b), 0.0, t.text_selection);
    }
    let clause = shown
        .clause
        .as_ref()
        .map(|c| range_boxes(ui, &shown.spec, c))
        .unwrap_or_default();
    for b in &clause {
        ui.painter().fill(at(b), 0.0, t.text_selection);
    }
    if !shown.empty {
        ui.painter().text(&shown.spec, x, y);
    }
    let preedit = shown
        .preedit
        .as_ref()
        .map(|p| range_boxes(ui, &shown.spec, p))
        .unwrap_or_default();
    let ink = premul(t.fg_primary);
    let underline = |b: &Rect, h: f32| {
        let r = at(b);
        Rect::new(r.x, r.bottom() - h, r.w, h)
    };
    for b in &preedit {
        ui.painter().fill_over(underline(b, 1.0), 0.0, ink);
    }
    for b in &clause {
        ui.painter().fill_over(underline(b, 2.0), 0.0, ink);
    }
    let pos = caret_at(ui, &shown.spec, shown.caret.unwrap_or(shown.focus_at));
    let ch = if !shown.empty && pos.height > 0.0 {
        pos.height
    } else {
        EMPTY_CARET
    };
    let bar = Rect::new(x + pos.x + CARET_GAP, y + pos.y, CARET_WIDTH, ch);
    if caret && shown.caret.is_some() {
        ui.painter().fill_over(bar, 0.0, premul(caret_ink));
    }
    clause
        .first()
        .or(preedit.first().filter(|_| shown.caret.is_none()))
        .map_or(bar, at)
}

/// Hands the state back, follows the composition, and reports `area`, the widget's IME area when
/// it holds focus, if it takes composition: a read-only text and a password do not.
pub(super) fn end_edit(
    ui: &mut Ui<'_>,
    id: WidgetId,
    cfg: &EditConfig,
    st: EditState,
    area: Option<Rect>,
) -> Option<Rect> {
    ui.set_composing(id, st.edit.preedit().is_some());
    *ui.persistent::<EditState>(id) = st;
    let takes_composition = !cfg.read_only && !cfg.password;
    let area = area.filter(|_| takes_composition);
    if area.is_some() {
        ui.set_ime_area(area);
    }
    area
}

/// The IME area of `bar`, both in the current space: where it is on screen, cut to `clip`.
pub(super) fn screen_area(ui: &mut Ui<'_>, bar: Rect, clip: Rect) -> Rect {
    let p = ui.painter();
    p.transformed(bar).intersect(&p.transformed(clip))
}

/// What an edit's node says beyond its role.
#[derive(Debug, Clone, Copy)]
pub(super) struct NodeFacts<'a> {
    pub(super) role: Role,
    pub(super) name: &'a str,
    pub(super) description: Option<&'a str>,
    pub(super) error: bool,
    pub(super) enabled: bool,
}

/// Emits the node of an edit in `rect`. The text and the selection exclude the preedit; a password
/// has neither.
pub(super) fn edit_node(
    ui: &mut Ui<'_>,
    id: WidgetId,
    rect: Rect,
    cfg: &EditConfig,
    edit: &TextEdit,
    text: &str,
    facts: NodeFacts<'_>,
) {
    let mut node = Node::new(id, facts.role, facts.name, rect);
    node.description = facts.description.map(str::to_owned);
    if !cfg.password {
        let sel = edit.selection();
        node.value = Some(Value::Text(text.to_owned()));
        node.text = Some(TextValue {
            text: text.to_owned(),
            selection: (sel.start, sel.end),
        });
    }
    if facts.error {
        node.states.insert(State::Invalid);
    }
    if cfg.read_only {
        node.states.insert(State::ReadOnly);
    }
    if facts.enabled {
        node.states.insert(State::Focusable);
        node.actions.insert(Action::Focus);
        if !cfg.read_only {
            node.actions.insert(Action::SetValue);
        }
    } else {
        node.states.insert(State::Disabled);
    }
    ui.node(node);
}

/// The look a focused or hovered edit has this frame, through the three motion rows.
pub(super) fn animated_look(
    ui: &mut Ui<'_>,
    id: WidgetId,
    hovered: bool,
    focused: bool,
    error: bool,
) -> FieldLook {
    let target = FieldLook::of(ui.theme(), hovered, focused, FieldState { error });
    FieldLook {
        fill: ui.animate_color(id.with("fill"), target.fill, &HOVER),
        focus: ui.animate_f32(id.with("focus"), target.focus, &FOCUS),
        error: ui.animate_f32(id.with("error"), target.error, &ERROR),
        caret: target.caret,
    }
}

/// The scroll that keeps the caret at text x `caret_x` inside `clip` with its margin, for text of
/// width `text_w` starting at `left` at scroll 0. Space past the text's end is given back.
fn keep_caret(scroll: f32, caret_x: f32, text_w: f32, left: f32, clip: Rect) -> f32 {
    let last = clip.right() - CARET_MARGIN - CARET_ROOM;
    let right = left + caret_x - last;
    let room = left + text_w - last;
    let first = left + caret_x + CARET_GAP - (clip.x + CARET_MARGIN);
    if caret_x <= 0.0 {
        return 0.0;
    }
    scroll.min(room).max(right).min(first).max(0.0)
}

impl Ui<'_> {
    /// A text field under `key` in `rect` showing `text`. See [`Self::text_field_with`]; this one
    /// has a placeholder and an error flag, and no other option.
    pub fn text_field(
        &mut self,
        key: impl Hash,
        rect: Rect,
        text: &mut String,
        placeholder: &str,
        state: FieldState,
    ) -> Response {
        let opts = FieldOptions {
            placeholder,
            error: state.error,
            ..FieldOptions::default()
        };
        self.text_field_with(key, rect, text, &opts).response
    }

    /// A one-line text field under `key` in `rect` editing `text`. A press focuses it and places
    /// the caret; while it is focused, keys, typed text, clipboard answers and IME events edit
    /// `text` through the engine, and an edit marks the response `changed`. A set-value access
    /// request replaces `text`. Disabled (see [`Ui::with_enabled`]) it ignores all of that.
    pub fn text_field_with(
        &mut self,
        key: impl Hash,
        rect: Rect,
        text: &mut String,
        opts: &FieldOptions<'_>,
    ) -> FieldResponse {
        let t = *self.theme();
        let id = self.id(key);
        let rect = if opts.small {
            Rect {
                h: SMALL_HEIGHT,
                ..rect
            }
        } else {
            rect
        };
        let mono = matches!(opts.variant, Variant::Mono | Variant::Date);
        let base = Style::ui(t.font_base, t.fg_primary);
        let cfg = EditConfig {
            style: if mono {
                Style {
                    font: Font::Mono,
                    ..base
                }
            } else {
                base
            },
            password: opts.variant == Variant::Password,
            read_only: opts.read_only,
            multiline: false,
            max_chars: opts.max_chars,
            wrap: None,
            page_rows: 1,
        };
        let mut r = self.interact(id, rect, EDIT_SENSE);
        let (mut st, frame) = begin_edit(self, id, &cfg, text);
        if frame.changed {
            r.mark_changed();
        }
        if let Some(AccessValue::Text(value)) = r.set_value.clone()
            && !opts.read_only
        {
            set_value(self, &cfg, &mut st, text, value);
            r.mark_changed();
        }
        let left = rect.x
            + if opts.variant == Variant::Search {
                SEARCH_INSET
            } else {
                TEXT_INSET
            };
        let origin = (left - st.scroll, rect.y);
        edit_pointer(self, &cfg, &mut st, &r, text, origin);

        // The clear button sits over the field, so it registers after it.
        let side = (rect.h - CLEAR) / 2.0;
        let clear_rect = Rect::new(rect.right() - side - CLEAR, rect.y + side, CLEAR, CLEAR);
        let clearable = opts.variant == Variant::Search && r.enabled && !opts.read_only;
        if clearable && !text.is_empty() {
            let c = self.interact(id.with("clear"), clear_rect, Sense::CLICK);
            if c.press_started || c.clicked {
                // While composing the engine refuses both, as it does any press.
                apply(self, &cfg, &mut st.edit, text, EditCommand::SelectAll);
                let out = apply(
                    self,
                    &cfg,
                    &mut st.edit,
                    text,
                    EditCommand::Delete { word: false },
                );
                if out.changed {
                    r.mark_changed();
                }
            }
        }
        let clearing = clearable && !text.is_empty();

        let look = animated_look(self, id, r.hovered, r.focused, opts.error);
        let bare = opts.chrome == FieldChrome::Bare;
        if !bare {
            paint_box(self.painter(), rect, &look);
        }

        let shown = Shown::new(&cfg, &st.edit, text, r.focused);
        let right = match opts.variant {
            Variant::Date => DATE_PADDING,
            _ if clearing => side + CLEAR + CLEAR_GAP,
            _ => CLIP_RIGHT,
        };
        let inset = left - rect.x - CLIP_GAP;
        let clip = Rect::new(
            rect.x + inset,
            rect.y,
            (rect.w - (inset + right)).max(0.0),
            rect.h,
        );
        let (w, h) = self.painter().measure(&shown.spec);
        let caret_x = caret_at(self, &shown.spec, shown.focus_at).x;
        st.scroll = if r.focused {
            keep_caret(st.scroll, caret_x, w, left, clip)
        } else {
            keep_caret(st.scroll, w, w, left, clip).min(st.scroll)
        };
        let scroll = st.scroll;

        self.painter().push_clip(clip);
        if shown.empty {
            let hint = cfg.placeholder(opts.placeholder, t.fg_tertiary);
            self.painter().text_in(&hint, rect, left - rect.x);
        }
        let text_h = if h > 0.0 && !shown.empty {
            h
        } else {
            EMPTY_CARET
        };
        let top = rect.y + (rect.h - text_h) / 2.0;
        let caret_ink = if bare { t.fg_tertiary } else { t.fg_primary };
        let bar = draw_shown_inked(self, &shown, (left - scroll, top), look.caret, caret_ink);
        let area = screen_area(self, bar, clip);
        self.painter().pop_clip();

        let (name, color) = (opts.label.unwrap_or(opts.placeholder), t.fg_secondary);
        match opts.variant {
            Variant::Search => {
                let at = Rect::new(rect.x + SEARCH_ICON_X, rect.y, ICON, rect.h);
                let glyph = icon::glyph(icon::SEARCH, ICON, color);
                self.painter().text_centered(&glyph, at);
            }
            Variant::Date => {
                let at = Rect::new(rect.right() - DATE_PADDING, rect.y, DATE_PADDING, rect.h);
                let glyph = icon::glyph(icon::CALENDAR, ICON, color);
                self.painter().text_centered(&glyph, at);
            }
            _ => {}
        }
        if clearing {
            let glyph = icon::glyph(icon::CHROME_CLOSE, CLEAR_ICON, color);
            self.painter().text_centered(&glyph, clear_rect);
        }

        let facts = NodeFacts {
            role: Role::TextInput,
            name,
            description: opts.description,
            error: opts.error,
            enabled: r.enabled,
        };
        edit_node(self, id, rect, &cfg, &st.edit, text, facts);
        if clearing {
            let mut node = Node::new(
                id.with("clear"),
                Role::Button,
                self.tr("luneui.text-field.clear"),
                clear_rect,
            );
            node.states.insert(State::Focusable);
            node.actions.insert(Action::Default);
            self.node(node);
        }
        let ime_area = end_edit(self, id, &cfg, st, r.focused.then_some(area));
        FieldResponse {
            response: r,
            submitted: frame.submitted,
            ime_area,
        }
    }
}
