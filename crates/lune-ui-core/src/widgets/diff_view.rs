//! The diff view (`text.diff-view`, D15): a read-only diff of an app-supplied line model, unified or
//! split, on the list frame ([`ListStyle`], D161).
//!
//! The toolkit never runs a diff algorithm and never reads a repository (D45): the app passes
//! [`DiffLine`]s typed added, removed, context or hunk, with their numbers, and for the split form the
//! row pairing [`pair_rows`] makes (called when its lines change, so no frame is O(rows)).
//!
//! One `pane` function draws every form. Unified is one pane over the lines. Split is the same pane
//! run twice, over the left and right halves of each [`SplitRow`], each with its own list state,
//! horizontal offset and scope, linked in y (D8). Only that link and the clear-on-press join them: a
//! press in one pane clears the other's selection, and a wrapped row is as tall as the taller half in
//! both, so rows stay aligned.
//!
//! Row order: fill, marks, selection, text. The list paints no fill of its own (`ListStyle::fill`),
//! and there is no row hover. The number and prefix columns stay put while the text scrolls
//! sideways (the design source moves them; a gutter that leaves is worse, D15).
//!
//! Keys and copy are the list's: Up, Down, Page, Home and End move the cursor, Shift extends,
//! Command+C writes the selected lines of the pane that has a selection, in row order, joined by
//! `\n`, the end trimmed. A secondary click is reported ([`DiffResponse::context`]); the app opens
//! its menu and may use [`Ui::diff_selected_text`] for its "Copy selected lines" row.
//!
//! Hunk rows carry the app's actions as 18 px buttons (hosts of `icon_button`, one Tab stop each);
//! a press on one reports `(hunk_index, action)` and selects no row.

use std::cell::Cell as Flag;
use std::hash::Hash;

use crate::access::{Node, Role};
use crate::color::Rgba;
use crate::paint::Rect;
use crate::text::{Spec, Style};
use crate::theme::Theme;
use crate::ui::{Response, Sense, Ui, WidgetId};

use super::button::IconButton;
use super::text_lines::{HighlightSpan, line_spec, range_boxes};
use super::virtual_list::{
    Heights, ListOptions, ListResponse, ListStyle, RowCtx, RowKey, SelectionMode,
};

/// A row's least height, and the line height of the text.
pub const ROW: f32 = 20.0;
/// The width of a line-number column.
pub const NUMBER_W: f32 = 48.0;
/// The prefix column's width, and the margin on each side of it.
pub const PREFIX_W: f32 = 10.0;
pub const PREFIX_MARGIN: f32 = 4.0;
/// The margin right of the text.
pub const TEXT_PAD: f32 = 8.0;
/// The split's separator.
pub const SEPARATOR: f32 = 1.0;
/// A hunk action button's side, its radius and glyph size.
pub const BUTTON: f32 = 18.0;
pub const BUTTON_RADIUS: f32 = 5.0;
pub const BUTTON_GLYPH: f32 = 10.0;

/// The hunk text's size and line height.
const HUNK_SIZE: f32 = 9.0;
const HUNK_LINE: f32 = 12.0;
/// The pill's side and top padding, and its radius.
const PILL_PAD: (f32, f32) = (12.0, 2.0);
const PILL_RADIUS: f32 = 10.0;
/// The hunk row's rules: margin to the edge, gap to the pill, and gap before the first action.
const RULE_EDGE: f32 = 12.0;
const RULE_GAP: f32 = 8.0;
/// Between two actions.
const ACTION_GAP: f32 = 4.0;
/// The selection overlay's alpha (the accent over the row).
const SELECTION_ALPHA: u8 = 0x33;

/// What a line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineKind {
    Added,
    Removed,
    Context,
    /// A hunk header; its `text` is the header's.
    Hunk,
}

/// A changed stretch of a line: chars `start..end` (D16), clamped to the line like a highlight span;
/// empty or inverted ones draw nothing. The line's kind picks the tint (`diff_added_mark` on an
/// added line, `diff_removed_mark` on a removed one); on any other line it draws nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffMark {
    pub start: usize,
    pub end: usize,
}

/// One line of the diff, as the app supplies it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffLine<'a> {
    pub kind: LineKind,
    /// The line's number in the old file.
    pub old: Option<u32>,
    /// The line's number in the new file.
    pub new: Option<u32>,
    /// The text, without its terminator.
    pub text: &'a str,
    /// A hunk row's index, reported with its action.
    pub hunk: Option<u32>,
    pub marks: &'a [DiffMark],
    /// Syntax colours, in chars of the line (the same spans as the code view's).
    pub spans: &'a [HighlightSpan],
}

impl<'a> DiffLine<'a> {
    /// A line of `kind` with `text` and no numbers, marks or spans.
    pub fn new(kind: LineKind, text: &'a str) -> Self {
        DiffLine {
            kind,
            old: None,
            new: None,
            text,
            hunk: None,
            marks: &[],
            spans: &[],
        }
    }
}

/// One row of the split form: indices into the lines. An empty half is `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitRow {
    pub left: Option<usize>,
    pub right: Option<usize>,
}

/// Pairs `lines` into split rows: a removed run and the added run after it pair in order, the longer
/// one's rest against `None`; context and hunk lines pair with themselves. An added run with no
/// removed run before it pairs against `None`. O(lines).
pub fn pair_rows(lines: &[DiffLine<'_>]) -> Vec<SplitRow> {
    let mut rows = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        match lines[i].kind {
            LineKind::Context | LineKind::Hunk => {
                rows.push(SplitRow {
                    left: Some(i),
                    right: Some(i),
                });
                i += 1;
            }
            LineKind::Removed | LineKind::Added => {
                let run = |from: usize, kind: LineKind| {
                    from + lines[from..].iter().take_while(|l| l.kind == kind).count()
                };
                let removed_end = run(i, LineKind::Removed);
                let added_end = run(removed_end, LineKind::Added);
                let (removed, added) = (i..removed_end, removed_end..added_end);
                for k in 0..removed.len().max(added.len()) {
                    rows.push(SplitRow {
                        left: (k < removed.len()).then(|| removed.start + k),
                        right: (k < added.len()).then(|| added.start + k),
                    });
                }
                i = added_end;
            }
        }
    }
    rows
}

/// The longest line's length in chars: the app calls this beside [`pair_rows`] and passes it as
/// [`DiffOptions::widest_chars`], so the horizontal bar is right before the lines are built.
pub fn widest_chars(lines: &[DiffLine<'_>]) -> usize {
    lines
        .iter()
        .map(|l| l.text.chars().count())
        .max()
        .unwrap_or(0)
}

/// How the lines show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffMode<'a> {
    /// One column of lines with two number columns.
    Unified,
    /// Old and new side by side, over the rows [`pair_rows`] made.
    Split(&'a [SplitRow]),
}

/// An action a hunk row offers: a glyph and the name the button carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HunkAction<'a> {
    pub glyph: &'a str,
    pub label: &'a str,
}

/// What a diff is given besides its rect and lines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffOptions<'a> {
    pub mode: DiffMode<'a>,
    /// Long lines wrap (and there is no horizontal bar); off, they run on and the bar shows.
    pub wrap: bool,
    /// The accessible name.
    pub label: &'a str,
    /// Buttons on every hunk row that has a [`DiffLine::hunk`]; empty: none.
    pub actions: &'a [HunkAction<'a>],
    /// The app's "same document" key: the same key keeps the offsets (clamped) and the selection, a
    /// new one starts at the top with none.
    pub identity: u64,
    /// [`widest_chars`] of the lines (0: the widest line built so far decides).
    pub widest_chars: usize,
}

/// What [`Ui::diff_view`] returns.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffResponse {
    /// The first pane's region (the left one in split), the Tab stop.
    pub response: Response,
    /// A hunk action pressed this frame: the hunk's index and the action's index in
    /// [`DiffOptions::actions`].
    pub hunk: Option<(u32, usize)>,
    /// The line a secondary click landed on, as an index into the lines; `None` for an empty half.
    pub context: Option<usize>,
    /// The selection changed this frame.
    pub changed: bool,
}

/// Which pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Unified,
    Left,
    Right,
}

impl Side {
    /// The pane's key under the diff's scope.
    fn key(self) -> &'static str {
        match self {
            Side::Unified => "u",
            Side::Left => "l",
            Side::Right => "r",
        }
    }

    /// Where the text starts, from the pane's left edge.
    fn gutter(self) -> f32 {
        let prefix = PREFIX_W + 2.0 * PREFIX_MARGIN;
        match self {
            Side::Unified => 2.0 * NUMBER_W + prefix,
            Side::Left | Side::Right => NUMBER_W + prefix,
        }
    }
}

/// What the panes share in a frame.
struct View<'a> {
    lines: &'a [DiffLine<'a>],
    rows: Option<&'a [SplitRow]>,
    opts: &'a DiffOptions<'a>,
    theme: Theme,
    /// One mono char's width.
    advance: f32,
    /// The text column's width when wrapping.
    wrap_w: f32,
}

impl<'a> View<'a> {
    fn count(&self) -> usize {
        self.rows.map_or(self.lines.len(), <[SplitRow]>::len)
    }

    /// The line a pane shows at `row`, if any.
    fn line_at(&self, side: Side, row: usize) -> Option<usize> {
        let at = match (side, self.rows) {
            (Side::Unified, None) => Some(row),
            (Side::Left, Some(rows)) => rows.get(row)?.left,
            (Side::Right, Some(rows)) => rows.get(row)?.right,
            _ => None,
        }?;
        (at < self.lines.len()).then_some(at)
    }

    fn line(&self, side: Side, row: usize) -> Option<&'a DiffLine<'a>> {
        self.line_at(side, row).map(|i| &self.lines[i])
    }
}

/// A line's text colour.
fn kind_color(theme: &Theme, kind: LineKind) -> Rgba {
    match kind {
        LineKind::Added => theme.ok,
        LineKind::Removed => theme.err,
        LineKind::Context => theme.fg_primary,
        LineKind::Hunk => theme.fg_quaternary,
    }
}

/// The row fill of a line (transparent for context and hunk).
fn kind_fill(theme: &Theme, kind: LineKind) -> Option<Rgba> {
    match kind {
        LineKind::Added => Some(theme.diff_added),
        LineKind::Removed => Some(theme.diff_removed),
        LineKind::Context | LineKind::Hunk => None,
    }
}

/// The tint of a mark on a line of `kind`.
fn mark_fill(theme: &Theme, kind: LineKind) -> Option<Rgba> {
    match kind {
        LineKind::Added => Some(theme.diff_added_mark),
        LineKind::Removed => Some(theme.diff_removed_mark),
        LineKind::Context | LineKind::Hunk => None,
    }
}

fn prefix_of(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Added => "+",
        LineKind::Removed => "-",
        LineKind::Context => " ",
        LineKind::Hunk => "",
    }
}

fn code_style(theme: &Theme, color: Rgba) -> Style {
    Style::mono(theme.font_base, ROW, color)
}

/// The spec that draws `line`'s text: its runs coloured by the kind and the spans, wrapped to `wrap`.
fn text_spec(theme: &Theme, line: &DiffLine<'_>, wrap: Option<f32>) -> Spec {
    let base = code_style(theme, kind_color(theme, line.kind));
    let runs = line_spec(line.text, base, line.spans);
    match wrap {
        Some(w) => Spec { runs, wrap: None }.wrapped(w),
        None => Spec { runs, wrap: None },
    }
}

/// What the panes of one frame report.
#[derive(Default)]
struct Acc {
    hunk: Option<(u32, usize)>,
}

/// A hunk button's last look, so the next frame's colours follow it.
#[derive(Debug, Default, Clone, Copy)]
struct ButtonMem {
    hovered: bool,
    pressed: bool,
}

/// What a pane keeps: the widest line it has drawn (for the bar), and which document that was.
#[derive(Debug, Default)]
struct PaneMem {
    widest: f32,
    identity: Option<u64>,
}

/// A usable length: finite and not negative.
fn usable(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

impl Ui<'_> {
    /// A diff under `key` over `rect` of `lines`, shown as `opts.mode` says (D15). Only the rows in
    /// view are built. Selection, keys and copy are the list's ([`Ui::virtual_list`]); Command+C
    /// writes the clipboard here. Unified has one Tab stop, split two, and each visible hunk action
    /// one more, in call order.
    pub fn diff_view(
        &mut self,
        key: impl Hash,
        rect: Rect,
        lines: &[DiffLine<'_>],
        opts: &DiffOptions<'_>,
    ) -> DiffResponse {
        self.scope(key, |ui| ui.diff_frame(rect, lines, opts))
    }

    /// The text of the lines selected in the diff under `key` (of the current scope), as Command+C
    /// writes it: the selected lines of the pane that has a selection, in row order, joined by `\n`,
    /// the end trimmed. An app's "Copy selected lines" menu row writes this to the clipboard.
    pub fn diff_selected_text(
        &mut self,
        key: impl Hash,
        lines: &[DiffLine<'_>],
        mode: DiffMode<'_>,
    ) -> String {
        self.scope(key, |ui| {
            let sides: &[Side] = match mode {
                DiffMode::Unified => &[Side::Unified],
                DiffMode::Split(_) => &[Side::Left, Side::Right],
            };
            for &side in sides {
                let keys: Vec<RowKey> = ui
                    .list_state(side.key())
                    .selection
                    .iter()
                    .copied()
                    .collect();
                if !keys.is_empty() {
                    return copy_text(lines, mode, side, &keys);
                }
            }
            String::new()
        })
    }

    fn diff_frame(
        &mut self,
        rect: Rect,
        lines: &[DiffLine<'_>],
        opts: &DiffOptions<'_>,
    ) -> DiffResponse {
        let theme = *self.theme();
        let rows = match opts.mode {
            DiffMode::Unified => None,
            DiffMode::Split(rows) => Some(rows),
        };
        let advance = {
            let probe = Spec::plain("0", code_style(&theme, theme.fg_primary));
            self.painter().measure(&probe).0
        };
        let (w, h) = (usable(rect.w), usable(rect.h));
        let sides: &[Side] = if rows.is_some() {
            &[Side::Left, Side::Right]
        } else {
            &[Side::Unified]
        };
        let pane_w = if rows.is_some() {
            ((w - SEPARATOR) / 2.0).floor().max(0.0)
        } else {
            w
        };
        let wrap_w = (pane_w - sides[0].gutter() - TEXT_PAD).max(1.0);
        let view = View {
            lines,
            rows,
            opts,
            theme,
            advance,
            wrap_w,
        };

        let link = rows.map(|_| self.id("link"));
        let mut acc = Acc::default();
        let mut response = None;
        let (mut changed, mut context) = (false, None);
        for &side in sides {
            let x = match side {
                Side::Right => rect.x + pane_w + SEPARATOR,
                _ => rect.x,
            };
            let width = match side {
                Side::Left => pane_w,
                Side::Right => (w - pane_w - SEPARATOR).max(0.0),
                Side::Unified => w,
            };
            let pane = Rect::new(x, rect.y, width, h);
            let list = self.diff_pane(side, pane, &view, link, &mut acc);
            if let Some(i) = list.context.and_then(|k| view.line_at(side, k.0 as usize)) {
                context = Some(i);
            }
            if let Some(keys) = &list.copy {
                let text = copy_text(lines, opts.mode, side, keys);
                self.clipboard_write(text);
            }
            changed |= list.response.changed;
            if list.response.changed {
                changed |= self.clear_other_pane(side);
            }
            response.get_or_insert(list.response);
        }
        if rows.is_some() {
            self.painter().fill(
                Rect::new(rect.x + pane_w, rect.y, SEPARATOR, h),
                0.0,
                theme.surface_separator,
            );
        }
        let mut response = response.unwrap_or_else(|| Response::inert(self.id("none"), rect, true));
        if changed {
            response.mark_changed();
        }
        DiffResponse {
            response,
            hunk: acc.hunk,
            context,
            changed,
        }
    }

    /// A selection change in `side`, by any input, empties the other pane's: only one pane holds a
    /// selection. Whether that changed it. When the other pane drew before this one, it drew the
    /// old selection: a frame is asked.
    fn clear_other_pane(&mut self, side: Side) -> bool {
        let other = match side {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
            Side::Unified => return false,
        };
        let state = self.list_state(other.key());
        let had = !state.selection.is_empty() || state.anchor.is_some();
        state.selection.clear();
        state.anchor = None;
        if had && other == Side::Left {
            self.request_catch_up();
        }
        had
    }

    /// One pane: a list over `rect` of the rows `side` shows.
    fn diff_pane(
        &mut self,
        side: Side,
        rect: Rect,
        view: &View<'_>,
        link: Option<WidgetId>,
        acc: &mut Acc,
    ) -> ListResponse {
        // The two panes are two tables: each says which half it is.
        let label = match side {
            Side::Unified => view.opts.label.to_owned(),
            Side::Left => format!(
                "{} ({})",
                view.opts.label,
                self.tr("luneui.diff-view.pane-old")
            ),
            Side::Right => format!(
                "{} ({})",
                view.opts.label,
                self.tr("luneui.diff-view.pane-new")
            ),
        };
        let opts = view.opts;
        let mem_id = self.id(side.key()).with("pane");
        let mem = self.persistent::<PaneMem>(mem_id);
        if mem.identity != Some(opts.identity) {
            *mem = PaneMem {
                widest: 0.0,
                identity: Some(opts.identity),
            };
        }
        let widest_text = view.advance * opts.widest_chars as f32;
        let content_w =
            (!opts.wrap).then(|| f64::from(side.gutter() + widest_text.max(mem.widest) + TEXT_PAD));
        let style = ListStyle {
            role: Role::Table,
            row_role: Role::Row,
            flat: true,
            content_w,
            fill: false,
            identity: Some(opts.identity),
            link,
        };
        let list = ListOptions {
            count: view.count(),
            heights: if opts.wrap {
                Heights::Measured { estimate: ROW }
            } else {
                Heights::Fixed(ROW)
            },
            selection: SelectionMode::Multi,
            index_of: None,
            label: &label,
        };
        let widest = Flag::new(0.0f32);
        let resp = self.list_styled(
            side.key(),
            rect,
            &list,
            style,
            |i| RowKey(i as u64),
            |ui, row| ui.diff_row(view, side, row, &widest, acc),
        );
        let mem = self.persistent::<PaneMem>(mem_id);
        mem.widest = mem.widest.max(widest.get());
        resp
    }

    /// One row inside its slot; returns its height.
    fn diff_row(
        &mut self,
        v: &View<'_>,
        side: Side,
        row: &RowCtx,
        widest: &Flag<f32>,
        acc: &mut Acc,
    ) -> f32 {
        let theme = v.theme;
        let slot = row.rect;
        let line = v.line(side, row.index);
        let wrap = v.opts.wrap.then_some(v.wrap_w);

        // A wrapped row is as tall as the taller half, in both panes.
        let mut height = ROW;
        if let Some(w) = wrap {
            let halves = match v.rows {
                None => [line.copied(), None],
                Some(_) => [
                    v.line(Side::Left, row.index).copied(),
                    v.line(Side::Right, row.index).copied(),
                ],
            };
            for half in halves.into_iter().flatten() {
                if half.kind != LineKind::Hunk {
                    let spec = text_spec(&theme, &half, Some(w));
                    height = height.max(self.painter().measure(&spec).1);
                }
            }
        }
        let area = Rect::new(slot.x, slot.y, slot.w, height);

        let Some(line) = line else {
            // An empty half: no text, but the row can still be selected.
            if row.selected {
                self.painter().fill(area, 0.0, selection_fill(&theme));
            }
            return height;
        };
        if let Some(fill) = kind_fill(&theme, line.kind) {
            self.painter().fill(area, 0.0, fill);
        }
        if line.kind == LineKind::Hunk {
            self.hunk_row(v, line, row, area, acc);
        } else {
            self.code_row(v, side, line, row, area, wrap, widest);
        }
        if let Some(text) = description(self, line.kind) {
            let id = self.scope_id();
            self.patch_node(id, |n| n.description = Some(text));
        }
        height
    }

    /// The numbers, prefix and text of an added, removed or context line.
    #[allow(clippy::too_many_arguments)]
    fn code_row(
        &mut self,
        v: &View<'_>,
        side: Side,
        line: &DiffLine<'_>,
        row: &RowCtx,
        area: Rect,
        wrap: Option<f32>,
        widest: &Flag<f32>,
    ) {
        let theme = v.theme;
        let gutter = side.gutter();
        let text_x = area.x + gutter;
        let column = Rect::new(text_x, area.y, (area.right() - text_x).max(0.0), area.h);
        let spec = text_spec(&theme, line, wrap);
        let (text_w, text_h) = self.painter().measure(&spec);
        widest.set(widest.get().max(text_w));
        let origin = (text_x - row.scroll_x, area.y + (area.h - text_h) / 2.0);

        // Marks, under the text.
        let tint = mark_fill(&theme, line.kind);
        let mut boxes = Vec::new();
        if let Some(tint) = tint {
            let chars = line.text.chars().count();
            for mark in line.marks {
                let (a, b) = (mark.start.min(chars), mark.end.min(chars));
                if a < b {
                    boxes.extend(range_boxes(self, &spec, &(a..b)));
                }
            }
            let p = self.painter();
            p.push_clip(column);
            for b in boxes {
                p.fill(
                    Rect::new(origin.0 + b.x, origin.1 + b.y, b.w, b.h),
                    0.0,
                    tint,
                );
            }
            p.pop_clip();
        }
        if row.selected {
            self.painter().fill(area, 0.0, selection_fill(&theme));
        }

        // The numbers and the prefix stay put; the text scrolls under its column's clip.
        let mut nodes = Vec::new();
        let numbers: &[(f32, Option<u32>)] = match side {
            Side::Unified => &[(0.0, line.old), (NUMBER_W, line.new)],
            Side::Left => &[(0.0, line.old)],
            Side::Right => &[(0.0, line.new)],
        };
        for (i, (dx, number)) in numbers.iter().enumerate() {
            let Some(n) = number else { continue };
            let text = n.to_string();
            let num = Spec::plain(text.as_str(), code_style(&theme, theme.fg_faint));
            let p = self.painter();
            let (w, h) = p.measure(&num);
            let at = (
                area.x + dx + NUMBER_W - TEXT_PAD - w,
                area.y + (area.h - h) / 2.0,
            );
            p.text(&num, at.0, at.1);
            nodes.push((i, text, Rect::new(area.x + dx, area.y, NUMBER_W, area.h)));
        }
        let prefix_x = area.x + gutter - PREFIX_W - PREFIX_MARGIN;
        let prefix = Spec::plain(
            prefix_of(line.kind),
            code_style(&theme, kind_color(&theme, line.kind)),
        );
        let p = self.painter();
        let ph = p.measure(&prefix).1;
        p.text(&prefix, prefix_x, area.y + (area.h - ph) / 2.0);
        p.push_clip(column);
        p.text(&spec, origin.0, origin.1);
        p.pop_clip();

        for (i, name, bounds) in nodes {
            let id = self.id(("n", i));
            self.node(Node::new(id, Role::Cell, &name, bounds));
        }
        let id = self.id("text");
        self.node(Node::new(id, Role::Cell, line.text, column));
    }

    /// A hunk row: a rule, the pill with the hunk's text, the actions, a rule.
    fn hunk_row(
        &mut self,
        v: &View<'_>,
        line: &DiffLine<'_>,
        row: &RowCtx,
        area: Rect,
        acc: &mut Acc,
    ) {
        let theme = v.theme;
        let actions = if line.hunk.is_some() {
            v.opts.actions
        } else {
            &[]
        };
        let n = actions.len() as f32;
        let buttons = if actions.is_empty() {
            0.0
        } else {
            RULE_GAP + n * BUTTON + (n - 1.0) * ACTION_GAP
        };
        let style = Style::mono(HUNK_SIZE, HUNK_LINE, theme.fg_quaternary);
        let room = (area.w - 2.0 * (RULE_EDGE + RULE_GAP) - buttons - 2.0 * PILL_PAD.0).max(0.0);
        let p = self.painter();
        let spec = p.elide(&Spec::plain(line.text, style), room);
        let (text_w, text_h) = p.measure(&spec);
        let pill = (text_w + 2.0 * PILL_PAD.0, text_h + 2.0 * PILL_PAD.1);
        let group_w = pill.0 + buttons;
        let gx = area.x + ((area.w - group_w) / 2.0).max(0.0);
        let mid = area.y + (area.h - 1.0) / 2.0;

        let left = (area.x + RULE_EDGE, gx - RULE_GAP);
        if left.1 > left.0 {
            p.fill(
                Rect::new(left.0, mid, left.1 - left.0, 1.0),
                0.0,
                theme.surface_separator,
            );
        }
        let right = (gx + group_w + RULE_GAP, area.right() - RULE_EDGE);
        if right.1 > right.0 {
            p.fill(
                Rect::new(right.0, mid, right.1 - right.0, 1.0),
                0.0,
                theme.surface_separator,
            );
        }
        let pill_rect = Rect::new(gx, area.y + (area.h - pill.1) / 2.0, pill.0, pill.1);
        p.fill(pill_rect, PILL_RADIUS, theme.surface_selected);
        p.text(&spec, pill_rect.x + PILL_PAD.0, pill_rect.y + PILL_PAD.1);
        if row.selected {
            p.fill(area, 0.0, selection_fill(&theme));
        }

        let name = line.text;
        let id = self.id("text");
        self.node(Node::new(id, Role::Cell, name, pill_rect));
        let (Some(hunk), false) = (line.hunk, actions.is_empty()) else {
            return;
        };
        for (a, action) in actions.iter().enumerate() {
            let at = gx + pill.0 + RULE_GAP + a as f32 * (BUTTON + ACTION_GAP);
            let rect = Rect::new(at, area.y + (area.h - BUTTON) / 2.0, BUTTON, BUTTON);
            if self.hunk_button(a, rect, action, &theme).clicked {
                acc.hunk = Some((hunk, a));
            }
        }
    }

    /// One hunk action: an 18 px button (overlay fill and border at rest, hover and press
    /// lift them) on an `icon_button` host.
    fn hunk_button(
        &mut self,
        index: usize,
        rect: Rect,
        action: &HunkAction<'_>,
        theme: &Theme,
    ) -> Response {
        let mem_id = self.id(("act", index)).with("look");
        let look = *self.transient::<ButtonMem>(mem_id);
        let lifted = look.hovered || look.pressed;
        let p = self.painter();
        if !lifted {
            p.fill(rect, BUTTON_RADIUS, theme.surface_overlay);
        }
        let edge = if lifted {
            theme.surface_divider
        } else {
            theme.border
        };
        p.ring(rect, BUTTON_RADIUS, 1.0, edge, false);
        let color = if look.pressed {
            theme.fg_bright
        } else if look.hovered {
            theme.fg_primary
        } else {
            theme.fg_secondary
        };
        let button = IconButton {
            glyph: action.glyph,
            glyph_size: BUTTON_GLYPH,
            color: Some(color),
            radius: BUTTON_RADIUS,
            name: action.label,
        };
        let r = self.icon_button_sensed(("act", index), rect, &button, Sense::CLICK_FOCUS);
        *self.transient::<ButtonMem>(mem_id) = ButtonMem {
            hovered: r.hovered,
            pressed: r.pressed,
        };
        r
    }
}

/// The row node's description for a line of `kind`: added, removed or unchanged.
fn description(ui: &Ui<'_>, kind: LineKind) -> Option<String> {
    let key = match kind {
        LineKind::Added => "luneui.diff-view.row-added",
        LineKind::Removed => "luneui.diff-view.row-removed",
        LineKind::Context => "luneui.diff-view.row-context",
        LineKind::Hunk => return None,
    };
    Some(ui.tr(key).to_owned())
}

/// The accent over the row, at alpha 0x33.
fn selection_fill(theme: &Theme) -> Rgba {
    let [r, g, b, _] = theme.accent;
    [r, g, b, SELECTION_ALPHA]
}

/// The text of the selected rows `keys` of pane `side`, in row order, joined by `\n`, the end
/// trimmed. An empty half adds nothing.
fn copy_text(lines: &[DiffLine<'_>], mode: DiffMode<'_>, side: Side, keys: &[RowKey]) -> String {
    let mut out = String::new();
    for key in keys {
        let row = key.0 as usize;
        let at = match (mode, side) {
            (DiffMode::Unified, _) => Some(row),
            (DiffMode::Split(rows), Side::Left) => rows.get(row).and_then(|r| r.left),
            (DiffMode::Split(rows), _) => rows.get(row).and_then(|r| r.right),
        };
        if let Some(line) = at.and_then(|i| lines.get(i)) {
            out.push_str(line.text);
            out.push('\n');
        }
    }
    out.truncate(out.trim_end().len());
    out
}
