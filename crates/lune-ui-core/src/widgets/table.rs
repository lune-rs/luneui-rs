//! The data table (`data.table`, D162): a sortable header over a virtualized body on the list frame
//! ([`ListStyle`], D161). 28 px rows with hairlines, numeric columns in mono, tones for the cell
//! text, controls in cells.
//!
//! The app owns the data and the sorting: it passes the columns and a closure per visible cell,
//! and the table reports a sort request ([`TableResponse::sort`]) when a header was pressed. The
//! table keeps the selection (a [`ListState`], by row key, so a re-sort keeps it) and the sort key
//! ([`TableState`]) under the persistent rule.
//!
//! Keys: the sortable headers are one roving stop (Left, Right, Home, End move, Enter and Space
//! sort), the body is the list frame's one stop. A table with a sortable column therefore has two
//! stops. There is no cell focus; a control in a cell is its own stop.
//!
//! There is no horizontal scroll: `Fill` columns shrink, then the last columns clip, and every
//! text elides ([`columns`]).

use std::hash::Hash;

use crate::access::{Action, Node, Role, Sort, State};
use crate::color::{Rgba, with_alpha};
use crate::icon;
use crate::input::NamedKey;
use crate::motion::{Kind, Motion};
use crate::paint::Rect;
use crate::text::{Spec, Style};
use crate::theme::Theme;
use crate::ui::{Response, RoveAxis, Sense, Ui, WidgetId};

use super::focus_ring::{self, RingKind};
use super::virtual_list::{
    Heights, ListOptions, ListState, ListStyle, RowCtx, RowKey, SelectionMode,
};

/// The sorted header's text colour and arrow opacity, between sorted and not.
pub const HEADER_FADE: Motion = Motion::of(Kind::Selection);

/// The header's height, hairline included.
pub const HEADER: f32 = 28.0;
/// A row's height, hairline included; the least height of a wrapping row.
pub const ROW: f32 = 28.0;
/// Horizontal cell padding.
pub const PAD: f32 = 8.0;
/// The first cell's left and the last cell's right padding.
pub const EDGE_PAD: f32 = 16.0;
/// Vertical padding of a wrapping cell.
pub const WRAP_PAD: f32 = 4.0;
/// The sort arrow's size.
pub const ARROW: f32 = 12.0;
/// Between the header text and the arrow.
pub const ARROW_GAP: f32 = 4.0;

/// The body text's line height.
const BODY_LINE: f32 = 20.0;
/// The header text's line height.
const HEAD_LINE: f32 = 16.0;
/// The header's id scope and the body list's key, under the table's scope.
const HEAD: &str = "head";
const BODY: &str = "body";

/// How wide a column is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnWidth {
    /// This many logical pixels.
    Fixed(f32),
    /// A share of what the fixed columns leave, by this weight.
    Fill(f32),
}

/// One column. `key` names it for the sort and the ids; it must be unique in the table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableColumn<'a> {
    pub key: u64,
    pub title: &'a str,
    pub width: ColumnWidth,
    /// Numeric: the cell text is mono and right-aligned, and so is the header (in the sans face).
    pub num: bool,
    /// A header press sorts by this column.
    pub sortable: bool,
}

/// A sort: the column's key and the direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortKey {
    pub column: u64,
    pub descending: bool,
}

/// What a table keeps besides its selection: persistent. The app may set or clear `sort`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TableState {
    pub sort: Option<SortKey>,
}

/// What a table is given besides its rect.
#[derive(Clone, Copy)]
pub struct TableOptions<'a> {
    pub columns: &'a [TableColumn<'a>],
    /// The number of rows.
    pub count: usize,
    pub selection: SelectionMode,
    /// A cell may wrap: a row is as tall as its tallest cell (at least [`ROW`]), measured.
    pub wrap: bool,
    /// Finds a key's row after the data changed (see [`ListOptions::index_of`]).
    pub index_of: Option<&'a dyn Fn(RowKey) -> Option<usize>>,
    /// The table's accessible name.
    pub label: &'a str,
}

impl std::fmt::Debug for TableOptions<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TableOptions")
            .field("columns", &self.columns)
            .field("count", &self.count)
            .field("selection", &self.selection)
            .field("wrap", &self.wrap)
            .field("index_of", &self.index_of.is_some())
            .field("label", &self.label)
            .finish()
    }
}

/// The colour family of a cell's text; a selected row's text is `fg_bright` whatever the tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellTone {
    /// `fg_primary`.
    #[default]
    Default,
    /// `fg_secondary`.
    Muted,
    /// `warn`.
    Warn,
    /// `err`.
    Err,
    /// `fg_tertiary`: no value, or a reference figure.
    Dim,
    /// `fg_bright`: the total a table builds up to.
    Strong,
}

impl CellTone {
    /// The tone's text colour.
    pub fn color(self, theme: &Theme) -> Rgba {
        match self {
            CellTone::Default => theme.fg_primary,
            CellTone::Muted => theme.fg_secondary,
            CellTone::Warn => theme.warn,
            CellTone::Err => theme.err,
            CellTone::Dim => theme.fg_tertiary,
            CellTone::Strong => theme.fg_bright,
        }
    }
}

/// What the app's closure answers for a cell.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// Text the table draws (elided to the cell, or wrapped when `wrap`).
    Text {
        text: String,
        tone: CellTone,
        mono: bool,
        wrap: bool,
    },
    /// The closure drew its own widget in [`CellCtx::rect`].
    Control,
}

impl Cell {
    /// Plain text in the default tone.
    pub fn text(text: impl Into<String>) -> Cell {
        Cell::Text {
            text: text.into(),
            tone: CellTone::Default,
            mono: false,
            wrap: false,
        }
    }

    /// This text cell in `tone` (a control cell stays as it is).
    #[must_use]
    pub fn tone(mut self, to: CellTone) -> Cell {
        if let Cell::Text { tone, .. } = &mut self {
            *tone = to;
        }
        self
    }

    /// This text cell in the mono face.
    #[must_use]
    pub fn mono(mut self) -> Cell {
        if let Cell::Text { mono, .. } = &mut self {
            *mono = true;
        }
        self
    }

    /// This text cell wrapping onto more lines (honoured when [`TableOptions::wrap`]).
    #[must_use]
    pub fn wrapped(mut self) -> Cell {
        if let Cell::Text { wrap, .. } = &mut self {
            *wrap = true;
        }
        self
    }
}

/// What the closure is told about a cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellCtx {
    pub row: usize,
    pub key: RowKey,
    /// The column's index in [`TableOptions::columns`].
    pub column: usize,
    /// The padded box: the cell less its side padding, as high as the row's slot.
    pub rect: Rect,
    pub selected: bool,
    /// The pointer is over the row (also while it is over a control in the row).
    pub hovered: bool,
}

/// What [`Ui::table`] returns.
#[derive(Debug, Clone, PartialEq)]
pub struct TableResponse {
    /// The body's region, the Tab stop; `changed` when the selection changed this frame.
    pub response: Response,
    /// The sort a header press asked for, on the frame it did. The table has already stored it in
    /// [`TableState`]; the app reorders its rows.
    pub sort: Option<SortKey>,
    /// The row a secondary click landed on.
    pub context: Option<RowKey>,
    /// The primary modifier with C: the selected keys in key order, for the app to format.
    pub copy: Option<Vec<RowKey>>,
    /// The body's offset after this frame's input.
    pub offset: f64,
    /// The height of all rows this frame used.
    pub extent: f64,
}

/// A usable length: finite and not negative.
fn usable(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

/// The columns' places across a table `w` wide, as `(x, width)` from its left edge. Fixed columns
/// take their width first, the `Fill` columns share the rest by weight; a column that no longer
/// fits shrinks and the ones after it clip, so the last edge is never past `w`. A `Fill` of weight
/// 0 gets no width. Never negative, never NaN.
pub fn columns(cols: &[TableColumn<'_>], w: f32) -> Vec<(f32, f32)> {
    let w = usable(w);
    let fixed: f32 = cols
        .iter()
        .map(|c| match c.width {
            ColumnWidth::Fixed(f) => usable(f),
            ColumnWidth::Fill(_) => 0.0,
        })
        .sum();
    let weights: f32 = cols
        .iter()
        .map(|c| match c.width {
            ColumnWidth::Fill(k) => usable(k),
            ColumnWidth::Fixed(_) => 0.0,
        })
        .sum();
    let rest = (w - fixed).max(0.0);
    let mut x = 0.0f32;
    cols.iter()
        .map(|c| {
            let want = match c.width {
                ColumnWidth::Fixed(f) => usable(f),
                ColumnWidth::Fill(k) if weights > 0.0 => rest * usable(k) / weights,
                ColumnWidth::Fill(_) => 0.0,
            };
            let at = x.min(w);
            let width = want.min(w - at).max(0.0);
            x += want;
            (at, width)
        })
        .collect()
}

/// The padding of the cell of column `i` of `n`: left, then right.
fn pads(i: usize, n: usize) -> (f32, f32) {
    (
        if i == 0 { EDGE_PAD } else { PAD },
        if i + 1 == n { EDGE_PAD } else { PAD },
    )
}

/// The cell box `(x, w)` less its padding.
fn padded(at: (f32, f32), i: usize, n: usize) -> (f32, f32) {
    let (l, r) = pads(i, n);
    (at.0 + l, (at.1 - l - r).max(0.0))
}

/// The direction a sorted header's arrow last showed, so that it fades out the way it came in.
#[derive(Debug, Default)]
struct ArrowMem {
    descending: bool,
}

/// What the header's frame needs.
struct Header<'r> {
    id: WidgetId,
    rect: Rect,
    layout: &'r [(f32, f32)],
    opts: &'r TableOptions<'r>,
}

impl Ui<'_> {
    /// The retained sort of the table under `key` (of the current scope). The app may change it.
    pub fn table_state(&mut self, key: impl Hash) -> &mut TableState {
        let id = self.id(key);
        self.persistent::<TableState>(id)
    }

    /// The retained selection of the table under `key` (of the current scope): the body list's,
    /// see [`Ui::list_state`].
    pub fn table_selection(&mut self, key: impl Hash) -> &mut ListState {
        let id = self.id(key).with(BODY);
        self.persistent::<ListState>(id)
    }

    /// A table under `key` over `rect`: the header on top, the rows under it in a scrolling body.
    /// `row_key` names the row at an index; `cell` is called for each cell of each row in view and
    /// either answers the text the table draws or draws a widget of its own in the cell's rect
    /// and answers [`Cell::Control`]. Widgets in a cell have their ids under the cell's scope.
    pub fn table(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &TableOptions<'_>,
        row_key: impl Fn(usize) -> RowKey,
        mut cell: impl FnMut(&mut Self, &CellCtx) -> Cell,
    ) -> TableResponse {
        self.scope(key, |ui| ui.table_frame(rect, opts, &row_key, &mut cell))
    }

    fn table_frame(
        &mut self,
        rect: Rect,
        opts: &TableOptions<'_>,
        row_key: &dyn Fn(usize) -> RowKey,
        cell: &mut dyn FnMut(&mut Self, &CellCtx) -> Cell,
    ) -> TableResponse {
        let id = self.scope_id();
        let layout = columns(opts.columns, rect.w);
        let head = Rect::new(rect.x, rect.y, rect.w, HEADER.min(usable(rect.h)));
        let body = Rect::new(rect.x, head.bottom(), rect.w, usable(rect.h - head.h));
        let list = ListOptions {
            count: opts.count,
            heights: if opts.wrap {
                Heights::Measured { estimate: ROW }
            } else {
                Heights::Fixed(ROW)
            },
            selection: opts.selection,
            index_of: opts.index_of,
            label: opts.label,
        };
        let style = ListStyle {
            role: Role::Group,
            row_role: Role::Row,
            flat: true,
            content_w: None,
            fill: true,
            identity: None,
            link: None,
        };
        let header = Header {
            id,
            rect: head,
            layout: &layout,
            opts,
        };
        let node = Node::new(id, Role::Table, opts.label, rect);
        let (sort, list) = self.node_container(node, |ui| {
            let sort = ui.table_header(&header);
            let list = ui.list_styled(BODY, body, &list, style, row_key, |ui, row| {
                ui.table_row(row, &layout, opts, cell)
            });
            (sort, list)
        });
        TableResponse {
            response: list.response,
            sort,
            context: list.context,
            copy: list.copy,
            offset: list.offset,
            extent: list.extent,
        }
    }

    /// The header: registers the sortable cells, applies their input, draws, and emits the nodes.
    fn table_header(&mut self, h: &Header<'_>) -> Option<SortKey> {
        let (opts, head) = (h.opts, h.rect);
        let theme = *self.theme();
        let enabled = self.is_enabled();
        let head_id = h.id.with(HEAD);
        let n = opts.columns.len();
        let kid_of = |i: usize| head_id.with(opts.columns[i].key);

        // The sortable columns are the roving group; the sorted one is its stop, else the first.
        let sortable: Vec<usize> = (0..n).filter(|&i| opts.columns[i].sortable).collect();
        let items: Vec<(WidgetId, bool)> = sortable.iter().map(|&i| (kid_of(i), enabled)).collect();
        let current = self.persistent::<TableState>(h.id).sort;
        let active = current.and_then(|s| {
            sortable
                .iter()
                .position(|&i| opts.columns[i].key == s.column)
        });
        let stop = self.rove_stop(&items, active);

        let mut requested = None;
        let mut responses: Vec<Option<Response>> = vec![None; n];
        for (slot, &i) in sortable.iter().enumerate() {
            let col = &opts.columns[i];
            let (x, w) = h.layout[i];
            let sense = if stop == Some(slot) {
                Sense::CLICK_FOCUS
            } else {
                Sense::CLICK_ROVE
            };
            let kid = items[slot].0;
            let r = self.interact(kid, Rect::new(head.x + x, head.y, w, head.h), sense);
            let key = self.activation_keys(kid, r.enabled, &[NamedKey::Enter, NamedKey::Space]);
            if r.enabled && (r.clicked || key) {
                let state = self.persistent::<TableState>(h.id);
                let next = match state.sort {
                    Some(s) if s.column == col.key => SortKey {
                        column: col.key,
                        descending: !s.descending,
                    },
                    _ => SortKey {
                        column: col.key,
                        descending: false,
                    },
                };
                state.sort = Some(next);
                requested = Some(next);
            }
            responses[i] = Some(r);
        }
        if !items.is_empty() {
            self.rove_keys(&items, RoveAxis::Horizontal, false);
        }

        let sorted = self.persistent::<TableState>(h.id).sort;
        let row = Node::new(head_id, Role::Row, "", head);
        self.node_container(row, |ui| {
            for (i, col) in opts.columns.iter().enumerate() {
                let cell = Rect::new(head.x + h.layout[i].0, head.y, h.layout[i].1, head.h);
                let this = sorted.filter(|s| s.column == col.key);
                let r = responses[i].as_ref();
                ui.header_cell(&theme, col, i, n, cell, this, kid_of(i), r);
                let mut node = Node::new(kid_of(i), Role::ColumnHeader, col.title, cell);
                node.sort = match (col.sortable, this) {
                    (false, _) => None,
                    (true, None) => Some(Sort::None),
                    (true, Some(s)) if s.descending => Some(Sort::Descending),
                    (true, Some(_)) => Some(Sort::Ascending),
                };
                if col.sortable {
                    node.states.insert(State::Focusable);
                    node.actions.insert(Action::Default);
                }
                ui.node(node);
            }
        });
        ui_hairline(self, head);
        requested
    }

    /// One header cell: the title, the arrow when sorted, the focus ring.
    #[allow(clippy::too_many_arguments)]
    fn header_cell(
        &mut self,
        theme: &Theme,
        col: &TableColumn<'_>,
        i: usize,
        n: usize,
        cell: Rect,
        sorted: Option<SortKey>,
        kid: WidgetId,
        response: Option<&Response>,
    ) {
        let level = |on: bool| if on { 1.0 } else { 0.0 };
        let (color, arrow) = if col.sortable {
            let to = if sorted.is_some() {
                theme.fg_secondary
            } else {
                theme.fg_tertiary
            };
            (
                self.animate_color(kid.with("color"), to, &HEADER_FADE),
                self.animate_f32(kid.with("arrow"), level(sorted.is_some()), &HEADER_FADE),
            )
        } else {
            (theme.fg_tertiary, 0.0)
        };
        let mem = self.transient::<ArrowMem>(kid.with("dir"));
        if let Some(s) = sorted {
            mem.descending = s.descending;
        }
        let descending = mem.descending;
        let reserve = if sorted.is_some() || arrow > 0.0 {
            ARROW + ARROW_GAP
        } else {
            0.0
        };

        let (x, w) = padded((cell.x, cell.w), i, n);
        let style = Style {
            line: HEAD_LINE,
            ..Style::ui(theme.font_s, color).medium()
        };
        let p = self.painter();
        let spec = p.elide(&Spec::plain(col.title, style), (w - reserve).max(0.0));
        let tw = p.measure(&spec).0;
        let text_x = if col.num { x + w - reserve - tw } else { x };
        if w > 0.0 {
            p.text_in(&spec, Rect::new(text_x, cell.y, tw, cell.h), 0.0);
            if arrow > 0.0 && reserve > 0.0 {
                let at = if col.num {
                    x + w - ARROW
                } else {
                    text_x + tw + ARROW_GAP
                };
                let glyph = if descending {
                    icon::ARROW_DOWN
                } else {
                    icon::ARROW_UP
                };
                let g = icon::glyph(glyph, ARROW, with_alpha(theme.fg_secondary, arrow));
                p.text_centered(&g, Rect::new(at, cell.y, ARROW, cell.h));
            }
        }
        if response.is_some_and(|r| r.focus_visible) {
            let ring = RingKind::Edge {
                radius: theme.item_radius,
                on_accent: false,
            };
            focus_ring::paint(p, cell.inset(1.0, 2.0), ring, 1.0);
        }
    }

    /// One body row, inside its slot: the cells, then the hairline. Returns the row's height.
    fn table_row(
        &mut self,
        row: &RowCtx,
        layout: &[(f32, f32)],
        opts: &TableOptions<'_>,
        cell: &mut dyn FnMut(&mut Self, &CellCtx) -> Cell,
    ) -> f32 {
        let theme = *self.theme();
        let n = opts.columns.len();
        let slot = row.rect;

        // Pass one: the closure answers for every cell (a control draws itself here).
        let mut cells = Vec::with_capacity(n);
        for (i, col) in opts.columns.iter().enumerate() {
            let (x, w) = layout[i];
            let (px, pw) = padded((x, w), i, n);
            let ctx = CellCtx {
                row: row.index,
                key: row.key,
                column: i,
                rect: Rect::new(slot.x + px, slot.y, pw, slot.h),
                selected: row.selected,
                hovered: row.hovered,
            };
            let cell_id = self.id(col.key);
            let bounds = Rect::new(slot.x + x, slot.y, w, slot.h);
            let answer = self.scope(col.key, |ui| {
                ui.node_container(Node::new(cell_id, Role::Cell, "", bounds), |ui| {
                    cell(ui, &ctx)
                })
            });
            if let Cell::Text { text, .. } = &answer {
                self.patch_node(cell_id, |n| n.name.clone_from(text));
            }
            cells.push((ctx.rect, answer));
        }

        // The row is as tall as its tallest wrapping cell.
        let mut height = ROW;
        let mut shaped = Vec::with_capacity(n);
        for (i, (rect, answer)) in cells.iter().enumerate() {
            let Cell::Text {
                text,
                tone,
                mono,
                wrap,
            } = answer
            else {
                continue;
            };
            let col = &opts.columns[i];
            let color = if row.selected {
                theme.fg_bright
            } else {
                tone.color(&theme)
            };
            let style = if *mono || col.num {
                Style::mono(theme.font_base, BODY_LINE, color)
            } else {
                Style {
                    line: BODY_LINE,
                    ..Style::ui(theme.font_base, color)
                }
            };
            let wraps = *wrap && opts.wrap;
            let spec = Spec::plain(text.as_str(), style);
            let p = self.painter();
            if wraps {
                let spec = spec.wrapped(rect.w);
                let text_h = p.measure(&spec).1;
                height = height.max(text_h + 2.0 * WRAP_PAD);
                shaped.push((i, *rect, spec, true));
            } else {
                shaped.push((i, *rect, p.elide(&spec, rect.w), false));
            }
        }

        // Pass two: the text, centred on the row, and the hairline.
        for (i, rect, spec, wraps) in shaped {
            if rect.w <= 0.0 {
                continue;
            }
            let p = self.painter();
            let (tw, th) = p.measure(&spec);
            let y = slot.y + (height - th) / 2.0;
            let x = if opts.columns[i].num && !wraps {
                rect.right() - tw
            } else {
                rect.x
            };
            p.text(&spec, x, y);
        }
        if row.index + 1 < opts.count {
            self.painter().fill(
                Rect::new(slot.x, slot.y + height - 1.0, slot.w, 1.0),
                0.0,
                theme.border_subtle,
            );
        }
        height
    }
}

/// The 1 px hairline along the bottom of the header.
fn ui_hairline(ui: &mut Ui<'_>, head: Rect) {
    let c = ui.theme().border_subtle;
    ui.painter()
        .fill(Rect::new(head.x, head.bottom() - 1.0, head.w, 1.0), 0.0, c);
}
