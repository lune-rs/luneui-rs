//! The code editor view (`text.code-editor-view`): a window of a [`CodeDoc`]'s lines over a
//! [scroll area](super::scroll_area), read-only or editable, with a gutter, app-supplied highlight
//! spans, line bands, panes that scroll together and anchors for overlays (D181 to D185).
//!
//! The frame is the [text area](super::text_area)'s, with the document replaced by the lines in
//! view: the engine's events, then a linked pane's offset, then the reveal, then the area, whose
//! content takes the press and draws. Nothing a frame does walks the document. [`DocHost`] is the
//! field's [`EditHost`] over the document, and [`DocLayout`] answers the engine by shaping the line
//! it is asked about, so a settled frame costs the same at 1,000 lines and at 1,000,000.
//!
//! The engine takes the text as `&mut` and a layout that reads it, which one document cannot be at
//! once. The engine never edits and asks for geometry in the same command, so [`DocHost::apply`]
//! splits them: a geometric command runs on a [`Frozen`] view with the [`DocLayout`], and every other
//! one on the document with a layout that only segments.
//!
//! Line tops are `f64`: at a million lines an `f32` cannot tell two pixels apart. Positions reach
//! the engine as `f32` from a nearby `base`, a top that the layout adds back.

use std::hash::Hash;
use std::ops::Range;

use super::field::{
    CARET_ROOM, EDIT_SENSE, EditHost, EditState, Shown, begin_edit_with, caret_at,
    draw_shown_inked, edit_node_states, edit_pointer_with, end_edit_with, screen_area,
};
use super::focus_ring::{self, RingKind};
use super::scroll_area::{Bars, Extent, Reveal, ScrollOptions, ScrollView, reveal_offset};
use super::text_lines::{Highlighter, PlainHighlighter, line_spec, range_boxes};
use super::virtual_list::{Heights, RowHeights};
use crate::access::{AccessValue, Node, Role, TextValue, Value};
use crate::code_doc::CodeDoc;
use crate::color::Rgba;
use crate::input::{FocusEvent, Key, NamedKey};
use crate::paint::Rect;
use crate::text::{CaretPos, LineSpan, Spec, Style, TextSystem, Unit};
use crate::text_edit::{
    EditBuffer, EditCommand, EditFlags, EditLayout, EditOutcome, EditText, SpecLayout, TextEdit,
};
use crate::ui::{Response, Sense, Ui, WidgetId};

/// The height of an unwrapped line, and a wrapped line's estimate until it is built.
pub const LINE: f32 = 20.0;
/// What an overlay pinned to a line keeps from the view's right edge.
pub const ANCHOR_INSET: f32 = 16.0;

/// From the view's left edge to the widest number.
const GUTTER_PAD: f32 = 4.0;
/// From the numbers to the text column.
const GUTTER_GAP: f32 = 8.0;
/// From the text column's edges to the text.
pub(super) const TEXT_PAD: f32 = 2.0;
/// The selection past a selected line's end.
const SLIVER: f32 = 4.0;

const AREA: &str = "area";

/// A full-row tint on one line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineBand {
    /// The line, from 1, as the gutter numbers it. Below 1 or past the last line it is skipped.
    pub line: usize,
    pub color: Rgba,
}

/// What [`Ui::code_editor`] is given besides its rect and document.
#[derive(Clone, Copy)]
pub struct CodeOptions<'a> {
    /// No caret and no edits: a Tab stop that selects, copies and scrolls (D181).
    pub read_only: bool,
    /// Wrap at the text column's width, and scroll only vertically.
    pub wrap: bool,
    /// The line numbers. On by default.
    pub gutter: bool,
    /// A plain Tab inserts a tab, and Shift+Tab still leaves. Off, Tab moves the focus.
    pub tab_inserts: bool,
    /// The accessible name.
    pub label: &'a str,
    /// The text's colour; `fg_primary` when `None`.
    pub color: Option<Rgba>,
    /// Asked for the lines in view, every frame: an app that tokenizes caches.
    pub highlighter: &'a dyn Highlighter,
    pub bands: &'a [LineBand],
    /// Panes of one scope with the same value here scroll together in y (D184).
    pub scroll_group: Option<&'a str>,
    /// The app's "same document" key: a new one starts at the top and forgets the measured
    /// heights.
    pub identity: Option<u64>,
}

impl Default for CodeOptions<'_> {
    fn default() -> Self {
        Self {
            read_only: false,
            wrap: false,
            gutter: true,
            tab_inserts: false,
            label: "",
            color: None,
            highlighter: &PlainHighlighter,
            bands: &[],
            scroll_group: None,
            identity: None,
        }
    }
}

impl std::fmt::Debug for CodeOptions<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeOptions")
            .field("read_only", &self.read_only)
            .field("wrap", &self.wrap)
            .field("gutter", &self.gutter)
            .field("tab_inserts", &self.tab_inserts)
            .field("label", &self.label)
            .field("color", &self.color)
            .field("bands", &self.bands.len())
            .field("scroll_group", &self.scroll_group)
            .field("identity", &self.identity)
            .finish_non_exhaustive()
    }
}

/// What [`Ui::code_editor`] returns.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeResponse {
    pub response: Response,
    /// The document changed this frame.
    pub changed: bool,
    /// The caret or the selected clause, in logical pixels, while the view takes composition.
    pub ime_area: Option<Rect>,
    /// The scroll offset, x then y.
    pub offset: (f64, f64),
    /// The rect the lines scroll in.
    pub viewport: Rect,
}

/// What a view keeps besides the engine's state.
#[derive(Debug, Default)]
struct CodeMem {
    heights: RowHeights,
    /// The widest line drawn: tabs and wide glyphs run past the mono advance.
    seen_w: f32,
    identity: Option<u64>,
    /// Where the last frame left the lines, for [`Ui::code_line_rect`].
    viewport: Rect,
    offset: f64,
}

/// How a host differs from the editor in the way it draws the frame: the code block (D10) sets a
/// size and a line of its own, scrolls only sideways and puts the ring round its box.
#[derive(Debug, Clone, Copy)]
pub(super) struct Look {
    /// The mono size and the line height.
    pub size: f32,
    pub line: f32,
    /// The lines scroll vertically. Off, the area has no vertical extent, so the vertical wheel
    /// goes past it.
    pub scroll_y: bool,
    /// Tab leaves the view without stopping on it.
    pub skip_tab: bool,
    /// Where the keyboard focus ring of a read-only view goes; the viewport when `None`.
    pub ring: Option<Rect>,
}

impl Look {
    /// The editor's own.
    pub(super) fn editor(size: f32) -> Look {
        Look {
            size,
            line: LINE,
            scroll_y: true,
            skip_tab: false,
            ring: None,
        }
    }
}

/// A usable length: finite and not negative.
fn usable(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

fn digits(lines: usize) -> u32 {
    lines.max(1).ilog10() + 1
}

/// The byte of char `at` of `text`; past the end, its length.
fn byte_at(text: &str, at: usize) -> usize {
    text.char_indices().nth(at).map_or(text.len(), |c| c.0)
}

/// The columns of a view, from its left edge.
#[derive(Debug, Clone, Copy)]
struct Columns {
    /// The gutter's width, and where the text column starts.
    gutter: f32,
    /// Where the text starts at scroll 0.
    text: f32,
}

impl Columns {
    fn of(opts: &CodeOptions<'_>, advance: f32, lines: usize) -> Columns {
        let gutter = if opts.gutter {
            GUTTER_PAD + digits(lines) as f32 * advance + GUTTER_GAP
        } else {
            0.0
        };
        Columns {
            gutter,
            text: gutter + TEXT_PAD,
        }
    }
}

/// The line heights while a frame measures them: a line above the first in view that changes height
/// shifts the area by as much, so what is on screen stays.
#[derive(Debug)]
struct LineTops<'a> {
    heights: &'a mut RowHeights,
    /// The height of a line of one row.
    line: f32,
    first: usize,
    shift: f64,
    relaid: bool,
}

impl LineTops<'_> {
    fn note(&mut self, line: usize, height: f32) {
        let change = self.heights.set(line, height.max(self.line));
        if change != 0.0 {
            self.relaid = true;
            if line < self.first {
                self.shift += change;
            }
        }
    }
}

/// A read of the document with the engine's write side: for the commands that never edit, whose
/// [`EditLayout`] reads the document at the same time. See [`DocHost::apply`].
struct Frozen<'a>(&'a CodeDoc);

impl EditText for Frozen<'_> {
    fn char_len(&self) -> usize {
        self.0.char_len()
    }

    fn fingerprint(&self) -> u64 {
        self.0.fingerprint()
    }

    fn window(&self, at: usize) -> (usize, &str) {
        self.0.window(at)
    }

    fn slice(&self, chars: Range<usize>) -> String {
        self.0.slice(chars)
    }
}

impl EditBuffer for Frozen<'_> {
    fn replace(&mut self, _chars: Range<usize>, _with: &str) {
        debug_assert!(false, "a geometric command edited the text");
    }
}

/// Whether `cmd` asks the layout about lines. These commands never edit.
fn geometric(cmd: &EditCommand) -> bool {
    matches!(
        cmd,
        EditCommand::Move { .. }
            | EditCommand::PlaceCaret(_)
            | EditCommand::ExtendTo(_)
            | EditCommand::SelectLineAt(_)
    )
}

/// The spec a line is measured and queried by: colour changes no position.
fn query_spec(text: &str, style: Style, wrap: Option<f32>) -> Spec {
    Spec {
        runs: vec![(text.to_owned(), style)],
        wrap,
    }
}

/// [`EditLayout`] over the document: it builds the spec of the one line each query is about.
struct DocLayout<'a, 'h> {
    system: &'a mut dyn TextSystem,
    scale: f32,
    doc: &'a CodeDoc,
    tops: &'a mut LineTops<'h>,
    style: Style,
    wrap: Option<f32>,
    page_rows: usize,
    /// The top every `y` here is measured from.
    base: f64,
}

impl DocLayout<'_, '_> {
    /// `f` on the layout of line `line`.
    fn on_line<R>(&mut self, line: usize, f: impl FnOnce(&mut SpecLayout<'_>) -> R) -> R {
        let spec = query_spec(self.doc.line(line), self.style, self.wrap);
        f(&mut SpecLayout::new(&mut *self.system, &spec, self.scale))
    }

    /// The visual rows of `line`: offsets of the document, `y` from the line's top. It measures a
    /// wrapped line, because a row's place must agree with [`RowHeights::row_at`] when the engine
    /// hits it. An unwrapped line has one row and shapes nothing.
    fn rows_of(&mut self, line: usize) -> Vec<LineSpan> {
        let start = self.doc.line_start(line);
        if self.wrap.is_none() {
            let chars = self.doc.line(line).chars().count();
            return vec![LineSpan {
                start,
                end: start + chars,
                y: 0.0,
                height: self.tops.line,
            }];
        }
        let mut rows = self.on_line(line, |l| l.lines());
        let height = rows.last().map_or(self.tops.line, |r| r.y + r.height);
        self.tops.note(line, height);
        for row in &mut rows {
            row.start += start;
            row.end += start;
        }
        rows
    }
}

impl EditLayout for DocLayout<'_, '_> {
    fn boundaries(&mut self, text: &str, unit: Unit) -> Vec<usize> {
        let spec = Spec {
            runs: Vec::new(),
            wrap: None,
        };
        SpecLayout::new(&mut *self.system, &spec, self.scale).boundaries(text, unit)
    }

    fn caret(&mut self, offset: usize) -> CaretPos {
        let line = self.doc.line_of(offset);
        let local = offset - self.doc.line_start(line).min(offset);
        let mut pos = self.on_line(line, |l| l.caret(local));
        pos.y += (self.tops.heights.top(line) - self.base) as f32;
        pos
    }

    /// A press in the gutter is on the text's left edge.
    fn hit(&mut self, pos: (f32, f32)) -> usize {
        let y = self.base + f64::from(pos.1);
        let line = self.tops.heights.row_at(y);
        let local_y = (y - self.tops.heights.top(line)) as f32;
        let local = self.on_line(line, |l| l.hit((pos.0.max(0.0), local_y)));
        self.doc.line_start(line) + local
    }

    /// Never asked: every engine query goes through [`Self::lines_near`], and a document's lines are
    /// not all built.
    fn lines(&mut self) -> Vec<LineSpan> {
        Vec::new()
    }

    fn lines_near(&mut self, at: usize, reach: isize) -> Vec<LineSpan> {
        let line = self.doc.line_of(at);
        let up = (-reach).max(0).unsigned_abs() + 1;
        let down = reach.max(0).unsigned_abs() + 1;
        let own = self.rows_of(line);
        let row = own.iter().rposition(|r| r.start <= at).unwrap_or(0);

        let (mut above, mut seen, mut l) = (Vec::new(), row, line);
        while seen < up && l > 0 {
            l -= 1;
            let rows = self.rows_of(l);
            seen += rows.len();
            above.push((l, rows));
        }
        let (mut below, mut seen, mut l) = (Vec::new(), own.len() - row - 1, line);
        while seen < down && l + 1 < self.doc.line_count() {
            l += 1;
            let rows = self.rows_of(l);
            seen += rows.len();
            below.push((l, rows));
        }

        // Measuring moved the tops, so they are read now.
        let heights = &*self.tops.heights;
        self.base = heights.top(line);
        let base = self.base;
        let lines = above.into_iter().rev().chain([(line, own)]).chain(below);
        lines
            .flat_map(|(l, rows)| {
                let top = (heights.top(l) - base) as f32;
                rows.into_iter()
                    .map(move |r| LineSpan { y: top + r.y, ..r })
            })
            .collect()
    }

    fn page_rows(&self) -> usize {
        self.page_rows
    }
}

/// The [`EditHost`] of a document.
struct DocHost<'a> {
    doc: &'a mut CodeDoc,
    tops: LineTops<'a>,
    heights: Heights,
    style: Style,
    wrap: Option<f32>,
    page_rows: usize,
    read_only: bool,
    tab_inserts: bool,
    /// What a pointer's `y` is measured from: the offset of the frame.
    base: f64,
}

impl DocHost<'_> {
    /// `f` on the layout that only segments, for what needs no line.
    fn segmenting<R>(ui: &mut Ui<'_>, f: impl FnOnce(&mut SpecLayout<'_>) -> R) -> R {
        let p = ui.painter();
        let scale = p.s;
        let system: &mut dyn TextSystem = &mut *p.text;
        let spec = Spec {
            runs: Vec::new(),
            wrap: None,
        };
        f(&mut SpecLayout::new(system, &spec, scale))
    }

    fn sync_heights(&mut self) {
        self.tops.heights.sync(self.doc.line_count(), self.heights);
    }

    /// Replaces the document with an assistive technology's `value`, under the rules a paste
    /// follows; the history goes and the caret lands at the end, as the field's set-value does.
    fn set_value(&mut self, ui: &mut Ui<'_>, flags: EditFlags, st: &mut EditState, value: String) {
        let (keymap, now) = (ui.keymap(), ui.time());
        let mut next = CodeDoc::default();
        let mut fresh = TextEdit::new();
        fresh.set_flags(flags);
        Self::segmenting(ui, |l| {
            fresh.apply(EditCommand::PasteText(value), &mut next, l, keymap, now)
        });
        let end = next.char_len();
        *self.doc = next;
        st.edit = TextEdit::new();
        st.edit.set_flags(flags);
        st.edit.set_selection(end, end);
        self.sync(ui, &mut st.edit);
        self.sync_heights();
    }
}

impl EditHost for DocHost<'_> {
    fn sync(&mut self, ui: &mut Ui<'_>, edit: &mut TextEdit) {
        Self::segmenting(ui, |l| edit.sync_text(&*self.doc, l));
    }

    fn apply(&mut self, ui: &mut Ui<'_>, edit: &mut TextEdit, cmd: EditCommand) -> EditOutcome {
        let (keymap, now) = (ui.keymap(), ui.time());
        if !geometric(&cmd) {
            let doc = &mut *self.doc;
            let out = Self::segmenting(ui, |l| edit.apply(cmd, doc, l, keymap, now));
            self.sync_heights();
            return out;
        }
        let p = ui.painter();
        let scale = p.s;
        let system: &mut dyn TextSystem = &mut *p.text;
        let doc = &*self.doc;
        let mut layout = DocLayout {
            system,
            scale,
            doc,
            tops: &mut self.tops,
            style: self.style,
            wrap: self.wrap,
            page_rows: self.page_rows,
            base: self.base,
        };
        edit.apply(cmd, &mut Frozen(doc), &mut layout, keymap, now)
    }

    fn hit(&mut self, ui: &mut Ui<'_>, pos: (f32, f32)) -> usize {
        let p = ui.painter();
        let scale = p.s;
        let system: &mut dyn TextSystem = &mut *p.text;
        let mut layout = DocLayout {
            system,
            scale,
            doc: &*self.doc,
            tops: &mut self.tops,
            style: self.style,
            wrap: self.wrap,
            page_rows: self.page_rows,
            base: self.base,
        };
        layout.hit(pos)
    }

    /// A document opens at its top.
    fn first_caret(&self) -> usize {
        0
    }

    /// Read-only, only Select all and Copy: the area has the Page keys, Home and End, and the
    /// arrows stay free.
    fn admits(&self, cmd: &EditCommand) -> bool {
        !self.read_only || matches!(cmd, EditCommand::SelectAll | EditCommand::Copy)
    }

    fn command(&self, event: &FocusEvent, flags: EditFlags) -> Option<EditCommand> {
        let plain_tab = matches!(
            event,
            FocusEvent::Key { key: Key::Named(NamedKey::Tab), modifiers, .. }
                if *modifiers == Default::default()
        );
        (plain_tab && self.tab_inserts && !flags.read_only && !flags.composing)
            .then(|| EditCommand::Insert("\t".to_owned()))
    }
}

/// A composition, as one line shows it (D181): a selection on the line is replaced in the drawing;
/// across lines, the first line is cut at the selection, and later lines stay until the commit.
#[derive(Debug)]
struct Compose {
    line: usize,
    shown: String,
    preedit: Range<usize>,
    clause: Option<Range<usize>>,
    caret: Option<usize>,
    /// What the view follows: the caret, or the clause or preedit that hides it.
    focus_at: usize,
}

fn compose(doc: &CodeDoc, edit: &TextEdit) -> Option<Compose> {
    let text = edit.preedit()?;
    let sel = edit.selection();
    let line = doc.line_of(sel.start);
    let body = doc.line(line);
    let from = sel.start - doc.line_start(line).min(sel.start);
    let cut = byte_at(body, from);
    let rest = if doc.line_of(sel.end) == line {
        &body[byte_at(body, sel.end - doc.line_start(line).min(sel.end))..]
    } else {
        ""
    };
    let shown = format!("{}{text}{rest}", &body[..cut]);
    let start = body[..cut].chars().count();
    let preedit = start..start + text.chars().count();
    let (clause, caret, focus_at) = match edit.preedit_cursor() {
        Some((a, b)) if a == b => (None, Some(start + a), start + a),
        Some((a, b)) => (Some(start + a..start + b), None, start + b),
        None => (None, None, preedit.end),
    };
    Some(Compose {
        line,
        shown,
        preedit,
        clause,
        caret,
        focus_at,
    })
}

/// The lines `lines` joined by `\n`, and `sel` clipped to them, in chars of that text.
fn view_value(doc: &CodeDoc, lines: Range<usize>, sel: &Range<usize>) -> (String, (usize, usize)) {
    let mut text = String::new();
    let (mut count, mut from, mut to) = (0, 0, 0);
    let last = lines.end;
    for i in lines {
        let line = doc.line(i);
        let (start, chars) = (doc.line_start(i), line.chars().count());
        let place = |at: usize| count + at.saturating_sub(start).min(chars);
        if sel.start >= start {
            from = place(sel.start);
        }
        if sel.end >= start {
            to = place(sel.end);
        }
        text.push_str(line);
        count += chars;
        if i + 1 < last {
            text.push('\n');
            count += 1;
        }
    }
    (text, (from, to))
}

/// One line about to draw.
#[derive(Debug)]
struct Built {
    line: usize,
    /// From the viewport's top, in the current space.
    y: f32,
    height: f32,
    spec: Spec,
}

/// What [`draw_lines`] draws from.
struct Draw<'a, 'o> {
    opts: &'a CodeOptions<'o>,
    viewport: Rect,
    cols: Columns,
    view: &'a ScrollView,
    selection: Range<usize>,
    caret: usize,
    compose: Option<&'a Compose>,
    /// The caret shows.
    caret_on: bool,
}

/// The chars of `line`'s text that `sel` covers, and whether it runs past the line's end.
fn line_selection(
    doc: &CodeDoc,
    line: usize,
    chars: usize,
    sel: &Range<usize>,
) -> (Range<usize>, bool) {
    let start = doc.line_start(line);
    let end = start + chars;
    if sel.is_empty() || sel.end < start || sel.start > end {
        return (0..0, false);
    }
    let local = sel.start.saturating_sub(start)..sel.end.min(end) - start;
    (local, sel.end > end)
}

/// Builds and draws the lines in view; the selection, the composition and the caret on them.
/// Returns the lines built and the box the input method avoids.
fn draw_lines(
    ui: &mut Ui<'_>,
    host: &mut DocHost<'_>,
    d: &Draw<'_, '_>,
) -> (Range<usize>, f32, Option<Rect>) {
    let t = *ui.theme();
    let (offset, vp) = (d.view.offset.1, d.viewport);
    let bottom = offset + f64::from(vp.h);
    let first = host.tops.heights.row_at(offset);
    host.tops.first = first;

    // The specs first: bands go under the selection, and a line's band needs its height.
    let mut built = Vec::new();
    let (mut top, mut line, mut seen_w) = (host.tops.heights.top(first), first, 0.0f32);
    let count = host.doc.line_count();
    while line < count && top < bottom {
        let text = match d.compose.filter(|c| c.line == line) {
            Some(c) => c.shown.as_str(),
            None => host.doc.line(line),
        };
        let spans = d.opts.highlighter.spans(line, text);
        let spec = Spec {
            runs: line_spec(text, host.style, &spans),
            wrap: host.wrap,
        };
        let (w, h) = ui.painter().measure(&spec);
        if host.wrap.is_some() {
            host.tops.note(line, h);
        }
        seen_w = seen_w.max(w);
        let height = host.tops.heights.height(line);
        built.push(Built {
            line,
            y: (top - offset) as f32 + vp.y,
            height: height as f32,
            spec,
        });
        top += height;
        line += 1;
    }
    let shown = first..line;

    for band in d.opts.bands {
        let at = band.line.wrapping_sub(1);
        if shown.contains(&at)
            && let Some(b) = built.get(at - first)
        {
            ui.painter()
                .fill(Rect::new(vp.x, b.y, vp.w, b.height), 0.0, band.color);
        }
    }

    let clip = Rect::new(
        vp.x + d.cols.gutter,
        vp.y,
        (vp.w - d.cols.gutter).max(0.0),
        vp.h,
    );
    let x = vp.x + d.cols.text - d.view.offset.0 as f32;
    ui.painter().push_clip(clip);
    let mut bar = None;
    let caret_line = host.doc.line_of(d.caret);
    for b in &built {
        let composing = d.compose.filter(|c| c.line == b.line);
        let chars = host.doc.line(b.line).chars().count();
        let (range, past_end) = if d.compose.is_some() {
            (0..0, false)
        } else {
            line_selection(host.doc, b.line, chars, &d.selection)
        };
        if past_end {
            let pos = caret_at(ui, &b.spec, chars);
            let sliver = Rect::new(x + pos.x, b.y + pos.y, SLIVER, pos.height);
            ui.painter().fill(sliver, 0.0, t.text_selection);
        }
        let own_caret = d.caret_on && d.compose.is_none() && b.line == caret_line;
        if composing.is_none() && !own_caret {
            for r in range_boxes(ui, &b.spec, &range) {
                let r = Rect::new(x + r.x, b.y + r.y, r.w, r.h);
                ui.painter().fill(r, 0.0, t.text_selection);
            }
            ui.painter().text(&b.spec, x, b.y);
            continue;
        }
        let local = d.caret - host.doc.line_start(b.line).min(d.caret);
        let shown = match composing {
            Some(c) => Shown::line(
                b.spec.clone(),
                0..0,
                Some(c.preedit.clone()),
                c.clause.clone(),
                c.caret,
                c.focus_at,
            ),
            None => Shown::line(b.spec.clone(), range, None, None, Some(local), local),
        };
        let area = draw_shown_inked(ui, &shown, (x, b.y), d.caret_on, t.fg_primary);
        bar = Some(screen_area(ui, area, clip));
    }
    ui.painter().pop_clip();

    if d.opts.gutter {
        let style = Style::mono(host.style.size, host.style.line, t.fg_faint);
        let right = vp.x + d.cols.gutter - GUTTER_GAP;
        ui.painter()
            .push_clip(Rect::new(vp.x, vp.y, d.cols.gutter.min(vp.w), vp.h));
        for b in &built {
            let number = Spec::plain((b.line + 1).to_string(), style);
            let w = ui.painter().measure(&number).0;
            ui.painter().text(&number, right - w, b.y);
        }
        ui.painter().pop_clip();
    }

    // The lines just outside are measured and not drawn, so one that comes into view from above
    // has shifted the area already.
    if let Some(wrap) = host.wrap {
        for near in host.tops.heights.window(offset, f64::from(vp.h)) {
            if !shown.contains(&near) {
                let spec = query_spec(host.doc.line(near), host.style, Some(wrap));
                let h = ui.painter().measure(&spec).1;
                host.tops.note(near, h);
            }
        }
    }
    (shown, seen_w, bar)
}

impl Ui<'_> {
    /// A code view under `key` over `rect` on `doc`: lines of mono text with a gutter, read-only or
    /// editable as `opts` says. Only the lines in view are built, so its frame costs the same at
    /// any line count.
    ///
    /// Editable, it behaves as [`Self::text_area`] without the box: keys, typed text, clipboard
    /// answers and IME events edit `doc`, and the caret comes into view at once. Read-only, it is
    /// a Tab stop with no caret: a press focuses and selects, Select all and Copy work, the Page
    /// keys, Home and End scroll, and the arrows stay free for a binding.
    pub fn code_editor(
        &mut self,
        key: impl Hash,
        rect: Rect,
        doc: &mut CodeDoc,
        opts: &CodeOptions<'_>,
    ) -> CodeResponse {
        let group = opts.scroll_group.map(|g| self.id(("code-scroll", g)));
        let look = Look::editor(self.theme().font_base);
        self.scope(key, |ui| ui.code_frame(rect, doc, opts, group, &look))
    }

    /// The rect of line `line` (from 1, as the gutter numbers it) of the view under `key`, in the
    /// space the view was drawn in, as the last call left it: as wide as the view, for an overlay
    /// pinned to the line ([`ANCHOR_INSET`] from the right edge). `None` out of view or out of
    /// range, so an overlay that is not there registers nothing.
    pub fn code_line_rect(&mut self, key: impl Hash, line: usize) -> Option<Rect> {
        let mem = self.persistent::<CodeMem>(self.id(key));
        let index = line.checked_sub(1)?;
        if index >= mem.heights.count() {
            return None;
        }
        let top = mem.heights.top(index) - mem.offset;
        let height = mem.heights.height(index);
        let vp = mem.viewport;
        let shows = top + height > 0.0 && top < f64::from(vp.h);
        shows.then(|| Rect::new(vp.x, vp.y + top as f32, vp.w, height as f32))
    }

    /// The frame of [`Self::code_editor`] under the current scope, drawn as `look` says.
    pub(super) fn code_frame(
        &mut self,
        rect: Rect,
        doc: &mut CodeDoc,
        opts: &CodeOptions<'_>,
        group: Option<WidgetId>,
        look: &Look,
    ) -> CodeResponse {
        let t = *self.theme();
        let id = self.scope_id();
        let viewport = Rect::new(rect.x, rect.y, usable(rect.w), usable(rect.h));
        let (vw, vh) = (f64::from(viewport.w), f64::from(viewport.h));
        let style = Style::mono(look.size, look.line, opts.color.unwrap_or(t.fg_primary));
        let advance = self.painter().measure(&Spec::plain("0", style)).0;

        let mut mem = std::mem::take(self.persistent::<CodeMem>(id));
        if mem.identity != opts.identity {
            mem = CodeMem {
                identity: opts.identity,
                ..CodeMem::default()
            };
        }
        let number_width = digits(doc.line_count());
        let cols = Columns::of(opts, advance, doc.line_count());
        let wrap_w = (viewport.w - cols.text - TEXT_PAD - CARET_ROOM).max(1.0);
        let heights = if opts.wrap {
            Heights::Measured {
                estimate: look.line,
            }
        } else {
            Heights::Fixed(look.line)
        };
        mem.heights.sync(doc.line_count(), heights);
        let drawn = self.persistent::<EditState>(id).offset;
        let first = mem.heights.row_at(drawn.1);
        let flags = EditFlags {
            read_only: opts.read_only,
            multiline: true,
            ..EditFlags::default()
        };
        let mut host = DocHost {
            doc,
            tops: LineTops {
                heights: &mut mem.heights,
                line: look.line,
                first,
                shift: 0.0,
                relaid: false,
            },
            heights,
            style,
            wrap: opts.wrap.then_some(wrap_w),
            page_rows: (viewport.h / look.line).floor().max(1.0) as usize,
            read_only: opts.read_only,
            tab_inserts: opts.tab_inserts,
            base: drawn.1,
        };

        let (mut st, frame) = begin_edit_with(self, id, flags, &mut host);
        let enabled = self.is_enabled();
        let focused = enabled && self.focus_state().0 == Some(id);
        let composing = compose(host.doc, &st.edit);

        // The caret's line is measured before anything reads where it is.
        let reveal = std::mem::take(&mut st.reveal) && focused && !opts.read_only;
        let target = reveal.then(|| {
            let (line, text, at) = match &composing {
                Some(c) => (c.line, c.shown.as_str(), c.focus_at),
                None => {
                    let caret = st.edit.caret();
                    let line = host.doc.line_of(caret);
                    let at = caret - host.doc.line_start(line).min(caret);
                    (line, host.doc.line(line), at)
                }
            };
            let spec = query_spec(text, style, host.wrap);
            let pos = caret_at(self, &spec, at);
            if let Some(wrap) = host.wrap {
                let h = self.painter().measure(&spec).1;
                host.tops.note(line, h);
                // What fills the view above it too: only then is where it lands the line's place.
                let (mut l, mut above) = (line, 0.0);
                while l > 0 && above < vh {
                    l -= 1;
                    let spec = query_spec(host.doc.line(l), style, Some(wrap));
                    let h = self.painter().measure(&spec).1;
                    host.tops.note(l, h);
                    above += host.tops.heights.height(l);
                }
            }
            (line, pos)
        });
        if host.tops.shift != 0.0 {
            self.scroll_shift(AREA, std::mem::take(&mut host.tops.shift));
        }

        // A linked pane takes the group's offset before its area runs. Its own queued shift moves
        // it first, so the group's value is asked of where that takes it (D184).
        let pending = self.scroll_pending_shift(AREA);
        let pulled = group.map_or(0.0, |g| self.scroll_link_pull(g, drawn.1 + pending));
        if pulled != 0.0 {
            self.scroll_shift(AREA, pulled);
        }

        if let Some((line, pos)) = target {
            let y = host.tops.heights.top(line) + f64::from(pos.y);
            let oy = drawn.1 + pending + pulled;
            let height = f64::from(pos.height.max(look.line));
            let by_y = reveal_offset(y, height, vh, oy, Reveal::Nearest) - oy;
            self.scroll_shift(AREA, by_y);
            if !opts.wrap {
                let x = f64::from(pos.x - TEXT_PAD);
                let w = f64::from(CARET_ROOM + 2.0 * TEXT_PAD);
                let room = (vw - f64::from(cols.text - TEXT_PAD)).max(0.0);
                let by_x = reveal_offset(x, w, room, drawn.0, Reveal::Nearest) - drawn.0;
                self.scroll_shift_x(AREA, by_x);
            }
        }

        let line_w = f64::from(advance) * host.doc.widest_chars() as f64;
        let text_w = line_w.max(f64::from(mem.seen_w));
        let extent = (
            if opts.wrap {
                vw
            } else {
                f64::from(cols.text + CARET_ROOM + TEXT_PAD) + text_w
            },
            if look.scroll_y {
                host.tops.heights.total()
            } else {
                vh
            },
        );
        let scroll = ScrollOptions {
            bars: (
                if opts.wrap {
                    Bars::Disabled
                } else {
                    Bars::Auto.unless_fits(extent.0, vw)
                },
                if look.scroll_y {
                    Bars::Auto.unless_fits(extent.1, vh)
                } else {
                    Bars::Disabled
                },
            ),
            extent: Some(extent),
            identity: opts.identity,
            ..ScrollOptions::default()
        };

        let mut response = None;
        let mut area = None;
        let mut seen = 0.0;
        let out = self.scroll_area_untranslated(AREA, viewport, &scroll, |ui, view| {
            // The region registers inside the content, because the area's own would block one
            // under it.
            let sense = Sense {
                skip_tab: look.skip_tab,
                ..EDIT_SENSE
            };
            let mut r = ui.interact(id, viewport, sense);
            if frame.changed {
                r.mark_changed();
            }
            if let Some(AccessValue::Text(value)) = r.set_value.clone()
                && !opts.read_only
            {
                host.set_value(ui, flags, &mut st, value);
                r.mark_changed();
            }
            host.base = view.offset.1;
            let origin = (viewport.x + cols.text - view.offset.0 as f32, viewport.y);
            edit_pointer_with(ui, &mut host, &mut st, &r, origin);

            let focused = enabled && ui.focus_state().0 == Some(id);
            let draw = Draw {
                opts,
                viewport,
                cols,
                view,
                selection: if focused { st.edit.selection() } else { 0..0 },
                caret: st.edit.caret(),
                compose: composing.as_ref(),
                caret_on: focused && !opts.read_only,
            };
            let (shown, widest, bar) = draw_lines(ui, &mut host, &draw);
            seen = widest;
            if focused {
                area = bar;
            }
            if host.tops.shift != 0.0 {
                ui.scroll_shift(AREA, std::mem::take(&mut host.tops.shift));
            }

            let mut node = Node::new(id, Role::MultilineText, opts.label, viewport);
            let (text, selection) = view_value(host.doc, shown, &st.edit.selection());
            node.value = Some(Value::Text(text.clone()));
            node.text = Some(TextValue { text, selection });
            edit_node_states(&mut node, opts.read_only, enabled);
            ui.node(node);
            response = Some(r);
            ((), Extent::default())
        });

        st.offset = out.offset;
        // A read-only view follows nothing: Select all must not jump to the end.
        if opts.read_only {
            st.reveal = false;
        }
        // A far line's top was an estimate: the lines drawn around it may have moved it.
        if reveal && host.tops.relaid {
            st.reveal = true;
        }
        if st.reveal || host.tops.relaid {
            self.request_catch_up();
        }
        // The gutter was laid out before the frame's edits.
        if digits(host.doc.line_count()) != number_width {
            self.request_catch_up();
        }
        if !opts.wrap && seen > mem.seen_w {
            mem.seen_w = seen;
            self.request_catch_up();
        }
        mem.viewport = viewport;
        mem.offset = out.offset.1;
        *self.persistent::<CodeMem>(id) = mem;

        // Only a move of its own publishes; a pane that was just pulled follows, and says nothing.
        if let Some(g) = group
            && pulled == 0.0
        {
            self.scroll_link_push(g, drawn.1, out.offset.1);
        }

        let (now_focused, ring) = self.focus_state();
        let holds = enabled && now_focused == Some(id);
        if holds && ring && opts.read_only {
            focus_ring::paint(
                self.painter(),
                look.ring.unwrap_or(viewport),
                RingKind::Button,
                1.0,
            );
        }
        if holds && opts.tab_inserts && !opts.read_only {
            self.take_tab_next_frame(id);
        }

        let ime_area = end_edit_with(self, id, !opts.read_only, st, area);
        let response = response.unwrap_or(out.response);
        CodeResponse {
            changed: response.changed,
            response,
            ime_area,
            offset: out.offset,
            viewport,
        }
    }
}
