//! The kanban board (`data.kanban-board`): columns of cards, a free drag across columns and keyboard
//! moves (D167 to D169).
//!
//! The app owns the model: it passes columns and cards and [`Ui::kanban_board`] reports a move, an
//! open, a context request and the add and menu presses. [`CardMove::index`] counts the list as it
//! stands, the moved card still in it; [`landed_index`] is the same-column correction.
//!
//! One frame, in order: the layout (every slot, from the model and the text); what ended the drag
//! from outside (the source gone, the window lost); the window-space columns; the keys; then one
//! pass through the two scroll areas, each card reading its gesture before it draws; then what must
//! read the whole frame: the drop line and the ghosts.
//!
//! The areas take one closure each, so the columns a gesture hit-tests against come from the slots
//! and the scroll offsets the areas left last frame: a wheel step moves the picture one frame
//! before it moves the drop target.
//!
//! A card's drawn position has one owner, [`BoardMem`]. It is its `y` (a slide to its slot) while it
//! rests; the pointer while it drags, with its dimmed source held in place; and a [`Landing`] after
//! that, whatever ended the drag: the ghost runs from where it was to wherever the slot is now, and
//! replaces the body until it arrives. An accepted move, a refused one, a cancel and a keyboard move
//! across columns are that one path.

use std::collections::BTreeMap;
use std::hash::Hash;
use std::time::Duration;

use super::button::IconButton;
use super::focus_ring::{self, RingKind};
use super::scroll_area::{Bars, Extent, Reveal, ScrollOptions, ScrollView, Target};
use crate::access::{Action, Live, Node, Role, State};
use crate::backdrop::GlassLayer;
use crate::color::Rgba;
use crate::drag::{
    AUTO_SCROLL_TICK, Column, FreeDrag, FreeFrame, FreeResult, SETTLE, SLIDE, edge_step,
    paint_drop_line,
};
use crate::icon;
use crate::input::{Event, FocusEvent, Key, NamedKey};
use crate::motion::{Kind, Motion, Time};
use crate::paint::{Painter, Rect, Xform};
use crate::text::{Spec, Style};
use crate::theme::Theme;
use crate::ui::{Response, RoveAxis, Sense, Ui, WidgetId};

/// The board's margin around its header and columns.
pub const MARGIN: f32 = 12.0;
/// The board header's height and the gap under it.
pub const HEADER: f32 = 28.0;
pub const HEADER_GAP: f32 = 10.0;
/// A column's width, border included, and the gap after it.
pub const COLUMN_W: f32 = 272.0;
pub const COLUMN_GAP: f32 = 10.0;
/// A column's padding inside its border.
pub const COLUMN_PAD: f32 = 10.0;
/// A column header's height and the gap under it.
pub const COLUMN_HEADER: f32 = 24.0;
pub const COLUMN_HEADER_GAP: f32 = 8.0;
/// The tallest a column's card list gets before it scrolls.
pub const CARD_LIST_MAX: f32 = 640.0;
/// A card's padding inside its border, the gap under it and its corner radius.
pub const CARD_PAD: f32 = 10.0;
pub const CARD_GAP: f32 = 6.0;
pub const CARD_RADIUS: f32 = 6.0;
/// The spacing of a card's stack: title, tags, footer.
pub const STACK_GAP: f32 = 6.0;
/// A tag's text size.
pub const TAG_TEXT: f32 = 9.0;
/// The side of the board's icon buttons, and their corner radius.
pub const ICON_BUTTON: f32 = 24.0;
pub const ICON_RADIUS: f32 = 6.0;
/// The border of a column and of a card.
pub const BORDER: f32 = 1.0;
/// The corner radius of the board and of a column.
pub const RADIUS: f32 = 8.0;
/// The glyph in the board header's add button, and in a column's buttons.
pub const HEADER_GLYPH: f32 = 15.0;
pub const COLUMN_GLYPH: f32 = 14.0;
/// The gap after a column's card count.
pub const COUNT_GAP: f32 = 6.0;
/// A tag's corner radius, its padding, and the room above and to its right.
pub const TAG_RADIUS: f32 = 4.0;
pub const TAG_PAD_X: f32 = 6.0;
pub const TAG_PAD_Y: f32 = 1.0;
pub const TAG_TOP: f32 = 2.0;
pub const TAG_RIGHT: f32 = 4.0;
/// The footer's glyph size, height, and the gap between its glyph and the assignee.
pub const FOOTER_GLYPH: f32 = 11.0;
pub const FOOTER: f32 = 12.0;
pub const FOOTER_GAP: f32 = 4.0;
/// The empty hint's offset from the list's top left.
pub const HINT_LEFT: f32 = 2.0;
pub const HINT_TOP: f32 = 6.0;
/// The error state: the widest its stack gets, the stack's spacing and the warning glyph.
pub const ERROR_MAX_W: f32 = 420.0;
pub const ERROR_GAP: f32 = 10.0;
pub const ERROR_GLYPH: f32 = 40.0;

/// A card's width: the column's inside.
pub const CARD_W: f32 = COLUMN_W - 2.0 * (COLUMN_PAD + BORDER);
/// The width a card's title wraps at: the card's inside.
pub const CARD_INNER: f32 = CARD_W - 2.0 * (CARD_PAD + BORDER);

/// Card hover and the target column's fill.
pub const HOVER: Motion = Motion::of(Kind::Hover);
/// The focus ring of a card.
pub const FOCUS: Motion = Motion::of(Kind::Focus);

/// The horizontal area's key under the board's scope.
const COLUMNS: &str = "columns";
/// The fill of a column that is no drop target: white at alpha 0, so a fade keeps its colour.
const CLEAR: Rgba = [0xFF, 0xFF, 0xFF, 0x00];

/// One card, from the app's model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KanbanCard<'a> {
    /// Unique on the board: the card keeps its focus and its motion across columns by it.
    pub key: u64,
    pub title: &'a str,
    pub tags: &'a [&'a str],
    pub assignee: Option<&'a str>,
    /// Shows the note mark.
    pub note: bool,
}

/// One column, from the app's model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KanbanColumn<'a> {
    pub key: u64,
    pub title: &'a str,
    pub cards: &'a [KanbanCard<'a>],
}

/// What the board shows instead of its columns when it cannot be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardError<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub path: &'a str,
}

/// What [`Ui::kanban_board`] is given besides its rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardOptions<'a> {
    pub title: &'a str,
    pub columns: &'a [KanbanColumn<'a>],
    /// Shows only this, in place of the columns.
    pub error: Option<&'a BoardError<'a>>,
    /// The window is active: the glass shows. Inactive, the solid fallback replaces it.
    pub active: bool,
    /// The glass layer drawn while active.
    pub glass: GlassLayer,
    /// What the glass is composed over (the colour below it, premultiplied).
    pub under: [f32; 4],
    /// The accessible name; empty falls to `title`.
    pub name: &'a str,
}

/// A card the user moved, by a drop or a key. The app applies it, or does not: the card lands
/// wherever the model puts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardMove {
    pub card: u64,
    /// The key of the column it goes to.
    pub column: u64,
    /// Its place, counted on that column's list as it stands: in its own column, with the card
    /// still in it ([`landed_index`]).
    pub index: usize,
}

/// What [`Ui::kanban_board`] returns.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardResponse {
    /// The board's own region; `changed` when a card moved.
    pub response: Response,
    pub moved: Option<CardMove>,
    /// A card was clicked, activated or asked to open.
    pub opened: Option<u64>,
    /// A card asked for its menu, and the window-space rect to anchor it on.
    pub context: Option<(u64, Rect)>,
    pub add_column: bool,
    /// The add button of this column was pressed.
    pub add_card: Option<u64>,
    /// The menu button of this column was pressed, and the window-space rect to anchor it on.
    pub column_menu: Option<(u64, Rect)>,
}

/// The index a [`CardMove`] puts a card at once it left its old place: `from` is its index when the
/// move stays in its column, and past it the list is one shorter.
pub fn landed_index(from: Option<usize>, index: usize) -> usize {
    match from {
        Some(f) if index > f => index - 1,
        _ => index,
    }
}

/// The continuous look of one card.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardLook {
    pub fill: Rgba,
    /// Focus ring opacity; 0 draws no ring.
    pub focus: f32,
}

fn title_spec(card: &KanbanCard<'_>, theme: &Theme) -> Spec {
    Spec::plain(card.title, Style::ui(theme.font_base, theme.fg_primary)).wrapped(CARD_INNER)
}

/// Walks the tags as a flow `CARD_INNER` wide, handing `place` each chip's rect (from a top left of
/// the origin) and its text; returns the flow's height.
fn tag_flow<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    tags: &[&str],
    theme: &Theme,
    mut place: impl FnMut(&mut Painter<'_, H>, Rect, &Spec),
) -> f32 {
    let style = Style::ui(TAG_TEXT, theme.fg_secondary);
    let h = style.line + 2.0 * TAG_PAD_Y;
    let row = h + TAG_TOP;
    let (mut x, mut y) = (0.0, 0.0);
    for tag in tags {
        let spec = p.elide(&Spec::plain(*tag, style), CARD_INNER - 2.0 * TAG_PAD_X);
        let w = p.measure(&spec).0 + 2.0 * TAG_PAD_X;
        if x > 0.0 && x + w > CARD_INNER {
            x = 0.0;
            y += row;
        }
        place(p, Rect::new(x, y + TAG_TOP, w, h), &spec);
        x += w + TAG_RIGHT;
    }
    if tags.is_empty() { 0.0 } else { y + row }
}

fn footer_height(card: &KanbanCard<'_>) -> f32 {
    if card.assignee.is_some() || card.note {
        FOOTER
    } else {
        0.0
    }
}

/// A card's height at [`CARD_W`]. The footer always has its place in the stack, so a card with no
/// assignee and no note keeps the stack's gap under its last row.
pub fn card_height<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    card: &KanbanCard<'_>,
    theme: &Theme,
) -> f32 {
    let title = p.measure(&title_spec(card, theme)).1;
    let tags = if card.tags.is_empty() {
        0.0
    } else {
        STACK_GAP + tag_flow(p, card.tags, theme, |_, _, _| {})
    };
    2.0 * (BORDER + CARD_PAD) + title + tags + STACK_GAP + footer_height(card)
}

/// Draws one card in `rect`: fill, border, title, tags, footer and focus ring.
pub fn paint_card<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    rect: Rect,
    card: &KanbanCard<'_>,
    look: &CardLook,
    theme: &Theme,
) {
    p.fill(rect, CARD_RADIUS, look.fill);
    p.ring(rect, CARD_RADIUS, BORDER, theme.border, false);
    let x = rect.x + BORDER + CARD_PAD;
    let mut y = rect.y + BORDER + CARD_PAD;
    let title = title_spec(card, theme);
    y += p.text(&title, x, y).1;
    if !card.tags.is_empty() {
        y += STACK_GAP;
        let top = y;
        y += tag_flow(p, card.tags, theme, |p, r, spec| {
            let chip = Rect::new(x + r.x, top + r.y, r.w, r.h);
            p.fill(chip, TAG_RADIUS, theme.surface_selected);
            p.text_centered(spec, chip);
        });
    }
    y += STACK_GAP;
    let mut left = x;
    let mut room = CARD_INNER;
    if card.note {
        let at = Rect::new(x + CARD_INNER - FOOTER_GLYPH, y, FOOTER_GLYPH, FOOTER);
        p.text_centered(&icon::glyph(icon::NOTE, FOOTER_GLYPH, theme.fg_faint), at);
        room -= FOOTER_GLYPH + FOOTER_GAP;
    }
    if let Some(name) = card.assignee {
        let at = Rect::new(left, y, FOOTER_GLYPH, FOOTER);
        p.text_centered(
            &icon::glyph(icon::ACCOUNT, FOOTER_GLYPH, theme.fg_faint),
            at,
        );
        left += FOOTER_GLYPH + FOOTER_GAP;
        room -= FOOTER_GLYPH + FOOTER_GAP;
        let spec = Spec::plain(name, Style::ui(TAG_TEXT, theme.fg_tertiary));
        let spec = p.elide(&spec, room.max(0.0));
        let h = p.measure(&spec).1;
        p.text(&spec, left, y + (FOOTER - h) / 2.0);
    }
    focus_ring::paint(
        p,
        rect,
        RingKind::Edge {
            radius: CARD_RADIUS,
            on_accent: false,
        },
        look.focus,
    );
}

/// One card's place: `y` from the top of its list's content.
#[derive(Debug, Clone, Copy)]
struct Slot {
    key: u64,
    id: WidgetId,
    y: f32,
    h: f32,
}

/// One column's place: `x` from the left of the board's content.
#[derive(Debug)]
struct Lane {
    x: f32,
    /// The frame's height.
    h: f32,
    list_h: f32,
    /// The list's content.
    content_h: f32,
    hint: Option<Spec>,
    slots: Vec<Slot>,
}

impl Lane {
    /// The top of the gap in front of slot `index`, in list content space; past the last card, the
    /// gap under it.
    fn gap_y(&self, index: usize) -> f32 {
        match self.slots.get(index) {
            Some(s) => s.y - CARD_GAP,
            None => self.slots.last().map_or(-CARD_GAP, |s| s.y + s.h),
        }
    }
}

/// The frame's whole layout, from the model alone.
#[derive(Debug)]
struct Layout {
    /// The columns' viewport.
    viewport: Rect,
    content_w: f32,
    lanes: Vec<Lane>,
    /// Each card's column and index.
    at: BTreeMap<u64, (usize, usize)>,
}

impl Layout {
    fn of(ui: &mut Ui<'_>, rect: Rect, opts: &BoardOptions<'_>) -> Layout {
        let theme = *ui.theme();
        let top = MARGIN + HEADER + HEADER_GAP;
        let viewport = Rect::new(
            rect.x + MARGIN,
            rect.y + top,
            (rect.w - 2.0 * MARGIN).max(0.0),
            (rect.h - top - MARGIN).max(0.0),
        );
        let chrome = 2.0 * (COLUMN_PAD + BORDER) + COLUMN_HEADER + COLUMN_HEADER_GAP;
        let list_max = CARD_LIST_MAX.min(viewport.h - chrome).max(0.0);
        let hint_text = ui.tr("luneui.kanban-board.empty-column");
        let mut at = BTreeMap::new();
        let mut lanes = Vec::with_capacity(opts.columns.len());
        for (c, col) in opts.columns.iter().enumerate() {
            let mut slots = Vec::with_capacity(col.cards.len());
            let mut y = 0.0;
            for (i, card) in col.cards.iter().enumerate() {
                let h = card_height(ui.painter(), card, &theme);
                let id = ui.id(("card", card.key));
                at.entry(card.key).or_insert((c, i));
                slots.push(Slot {
                    key: card.key,
                    id,
                    y,
                    h,
                });
                y += h + CARD_GAP;
            }
            let hint = col.cards.is_empty().then(|| {
                Spec::plain(hint_text, Style::ui(theme.font_s, theme.fg_faint))
                    .wrapped(CARD_W - HINT_LEFT)
            });
            let content_h = match &hint {
                Some(spec) => HINT_TOP + ui.painter().measure(spec).1,
                None => y,
            };
            let list_h = content_h.min(list_max);
            lanes.push(Lane {
                x: c as f32 * (COLUMN_W + COLUMN_GAP),
                h: chrome + list_h,
                list_h,
                content_h,
                hint,
                slots,
            });
        }
        Layout {
            viewport,
            content_w: opts.columns.len() as f32 * (COLUMN_W + COLUMN_GAP),
            lanes,
            at,
        }
    }

    /// The top of the lists, in the board's space.
    fn list_y(&self) -> f32 {
        self.viewport.y + BORDER + COLUMN_PAD + COLUMN_HEADER + COLUMN_HEADER_GAP
    }

    fn slot(&self, key: u64) -> Option<(usize, &Slot)> {
        let (c, i) = *self.at.get(&key)?;
        Some((c, self.lanes.get(c)?.slots.get(i)?))
    }
}

/// A released card's ghost on its way to its slot.
#[derive(Debug, Clone, Copy)]
struct Landing {
    key: u64,
    /// Window-space top left it started from.
    from: (f32, f32),
    /// Where it was last drawn.
    pos: (f32, f32),
}

/// Everything the board keeps between frames: every source of a card's position but its `y`.
#[derive(Debug, Default)]
struct BoardMem {
    drag: FreeDrag,
    /// The pointer's place on the pressed card, from its drawn top left.
    grab: (f32, f32),
    /// Window-space top left of the dragged card's ghost.
    ghost: (f32, f32),
    landings: Vec<Landing>,
    /// The roving stop while no card has focus.
    cursor: Option<u64>,
    /// A card a key moved: revealed on the next frame, where the model has placed it.
    reveal: Option<u64>,
    /// The offsets the areas left last frame: the board's and each column's by key.
    board_off: f64,
    list_off: BTreeMap<u64, f64>,
    /// The column the drop line drew in last frame.
    line: Option<usize>,
    /// The time of the last auto-scroll frame: `None` while the pointer is in no zone, or the idle
    /// time before it arrived would count as a step.
    scrolled: Option<Time>,
}

impl BoardMem {
    fn list_offset(&self, key: u64) -> f64 {
        self.list_off.get(&key).copied().unwrap_or(0.0)
    }
}

/// What the frame's helpers share.
#[derive(Debug)]
struct Frame<'f, 'a> {
    id: WidgetId,
    rect: Rect,
    opts: &'f BoardOptions<'a>,
    lay: Layout,
    theme: Theme,
    /// Board space to window space.
    shift: (f32, f32),
}

impl Frame<'_, '_> {
    /// The window-space columns: cut to the viewport, and no column at all for a pointer above or
    /// below it. The midpoints are the slots', never animated positions.
    fn mids(&self, mem: &BoardMem) -> Vec<Vec<f64>> {
        let list_y = f64::from(self.lay.list_y() + self.shift.1);
        self.lay
            .lanes
            .iter()
            .zip(self.opts.columns)
            .map(|(lane, col)| {
                let off = mem.list_offset(col.key);
                lane.slots
                    .iter()
                    .map(|s| list_y - off + f64::from(s.y + s.h / 2.0))
                    .collect()
            })
            .collect()
    }

    fn columns<'m>(
        &self,
        mids: &'m [Vec<f64>],
        mem: &BoardMem,
        pointer: Option<(f32, f32)>,
    ) -> Vec<Column<'m>> {
        let vp = self.lay.viewport;
        let lo = f64::from(vp.x + self.shift.0);
        let hi = lo + f64::from(vp.w);
        let inside = pointer.is_none_or(|(_, y)| {
            let y = y - self.shift.1;
            y >= vp.y && y < vp.bottom()
        });
        self.lay
            .lanes
            .iter()
            .zip(mids)
            .map(|(lane, card_mids)| {
                let x = lo - mem.board_off + f64::from(lane.x);
                let (from, to) = (x.max(lo), (x + f64::from(COLUMN_W)).min(hi));
                Column {
                    x: from,
                    w: if inside { (to - from).max(0.0) } else { 0.0 },
                    card_mids,
                }
            })
            .collect()
    }

    /// Where slot `s` of column `c` is, in the board's space, at this frame's offsets.
    fn slot_rect(&self, mem: &BoardMem, c: usize, s: &Slot) -> Rect {
        let lane = &self.lay.lanes[c];
        let off = mem.list_offset(self.opts.columns[c].key);
        Rect::new(
            self.lay.viewport.x + lane.x - mem.board_off as f32 + BORDER + COLUMN_PAD,
            self.lay.list_y() + s.y - off as f32,
            CARD_W,
            s.h,
        )
    }

    fn to_window(&self, r: Rect) -> (f32, f32) {
        (r.x + self.shift.0, r.y + self.shift.1)
    }

    fn card(&self, key: u64) -> Option<&KanbanCard<'_>> {
        let (c, i) = *self.lay.at.get(&key)?;
        self.opts.columns.get(c)?.cards.get(i)
    }
}

impl Ui<'_> {
    /// A kanban board under `key` over `rect`. Cards take the pointer and the keys of D168 and D169;
    /// the app applies what the response reports before the next frame, and a card it leaves alone
    /// lands back in its slot. Inside a `with_enabled(false, ..)` scope it reports nothing and any
    /// drag ends.
    pub fn kanban_board(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &BoardOptions<'_>,
    ) -> BoardResponse {
        let id = self.id(&key);
        let mut mem = std::mem::take(self.persistent::<BoardMem>(id));
        let response = self.interact(id, rect, Sense::HOVER);
        let mut out = BoardResponse {
            response,
            moved: None,
            opened: None,
            context: None,
            add_column: false,
            add_card: None,
            column_menu: None,
        };
        self.scope(&key, |ui| {
            let fallback = ui.theme().window_fallback;
            let p = ui.painter();
            if opts.active {
                p.acrylic(rect, RADIUS, opts.glass.premultiplied(), opts.under, true);
            } else {
                p.fill(rect, RADIUS, fallback);
            }
            match opts.error {
                Some(error) => {
                    mem.drag.cancel();
                    mem.landings.clear();
                    ui.board_error(id, rect, error);
                }
                None => {
                    let name = if opts.name.is_empty() {
                        opts.title
                    } else {
                        opts.name
                    };
                    let node = Node::new(id, Role::List, name, rect);
                    ui.node_container(node, |ui| ui.board_body(id, rect, opts, &mut mem, &mut out));
                }
            }
        });
        if out.moved.is_some() {
            out.response.mark_changed();
        }
        *self.persistent::<BoardMem>(id) = mem;
        out
    }

    /// The error state: warning, title, message and path, a centred stack.
    fn board_error(&mut self, id: WidgetId, rect: Rect, error: &BoardError<'_>) {
        let theme = *self.theme();
        let w = ERROR_MAX_W.min(rect.w - 2.0 * MARGIN).max(0.0);
        let p = self.painter();
        let warning = icon::glyph(icon::WARNING, ERROR_GLYPH, theme.status_warning);
        let title = Spec::plain(
            error.title,
            Style::ui(theme.font_base, theme.fg_secondary).medium(),
        );
        let title = p.elide(&title, w);
        let message =
            Spec::plain(error.message, Style::ui(theme.font_s, theme.fg_tertiary)).wrapped(w);
        let line = (theme.font_s * 1.3).round();
        let path = Spec::plain(error.path, Style::mono(theme.font_s, line, theme.fg_faint));
        let path = path.wrapped(w);
        let specs = [title, message, path];
        let sizes = specs.clone().map(|s| p.measure(&s));
        let total = ERROR_GLYPH + sizes.iter().map(|s| s.1 + ERROR_GAP).sum::<f32>();
        let mut y = rect.y + (rect.h - total) / 2.0;
        let mid = rect.x + rect.w / 2.0;
        let box_of = Rect::new(mid - ERROR_GLYPH / 2.0, y, ERROR_GLYPH, ERROR_GLYPH);
        p.text_centered(&warning, box_of);
        y += ERROR_GLYPH + ERROR_GAP;
        for (spec, (tw, th)) in specs.iter().zip(sizes) {
            p.text(spec, mid - tw / 2.0, y);
            y += th + ERROR_GAP;
        }
        let bounds = Rect::new(mid - w / 2.0, rect.y, w, rect.h);
        let mut alert = Node::new(id, Role::Alert, error.title, bounds);
        alert.description = Some(error.message.to_owned());
        alert.live = Some(Live::Assertive);
        self.node_container(alert, |ui| {
            let node = Node::new(id.with("path"), Role::StaticText, error.path, bounds);
            ui.node(node);
        });
    }

    fn board_body(
        &mut self,
        id: WidgetId,
        rect: Rect,
        opts: &BoardOptions<'_>,
        mem: &mut BoardMem,
        out: &mut BoardResponse,
    ) {
        let lay = Layout::of(self, rect, opts);
        let window = self.painter().transformed(rect);
        let f = Frame {
            id,
            rect,
            opts,
            lay,
            theme: *self.theme(),
            shift: (window.x - rect.x, window.y - rect.y),
        };
        self.board_outside(&f, mem);
        let pointer = self.pointer_pos();
        let mids = f.mids(mem);
        let columns = f.columns(&mids, mem, pointer);
        // Before anything draws, so the frame shows the drag where this frame's pointer has it.
        if let Some((x, y)) = pointer
            && mem.drag.is_active()
        {
            mem.drag.update(f64::from(x), f64::from(y), &columns);
            if mem.drag.is_dragging() {
                mem.ghost = (x - mem.grab.0, y - mem.grab.1);
            }
        }
        if let Some(source) = mem.drag.source() {
            mem.landings.retain(|l| l.key != source);
        }
        self.board_auto_scroll(&f, mem);
        self.board_reveal(&f, mem);
        let items: Vec<(WidgetId, bool)> = f
            .lay
            .lanes
            .iter()
            .flat_map(|lane| lane.slots.iter())
            .map(|s| (s.id, self.is_enabled()))
            .collect();
        self.board_keys(&f, &items, mem, out);
        let cursor = mem
            .cursor
            .and_then(|k| f.lay.slot(k))
            .and_then(|(_, s)| items.iter().position(|i| i.0 == s.id));
        let stop = self.rove_stop(&items, cursor).map(|i| items[i].0);
        self.board_header(&f, out);
        self.board_columns(&f, &columns, stop, mem, out);
        self.board_drop_line(&f, mem);
        self.board_ghosts(&f, mem);
    }

    /// What ends a drag from outside the gesture, and the landings left with nothing to land on.
    fn board_outside(&mut self, f: &Frame<'_, '_>, mem: &mut BoardMem) {
        let keep = |k: &u64| f.opts.columns.iter().any(|c| c.key == *k);
        mem.list_off.retain(|k, _| keep(k));
        if !self.is_enabled() {
            mem.landings.clear();
        }
        mem.landings.retain(|l| f.lay.at.contains_key(&l.key));
        let Some(key) = mem.drag.pressed() else {
            return;
        };
        if !f.lay.at.contains_key(&key) {
            mem.drag.cancel();
        } else if !self.input().focused {
            let dragged = mem.drag.source();
            mem.drag.cancel();
            if let Some(key) = dragged {
                let from = mem.ghost;
                self.board_land(f, mem, key, from);
            }
        }
    }

    /// Starts `key`'s landing from the window-space `from`. A tier that snaps moves has none, and a
    /// disabled scope ends one at once: the card simply draws in its slot.
    fn board_land(&mut self, f: &Frame<'_, '_>, mem: &mut BoardMem, key: u64, from: (f32, f32)) {
        mem.landings.retain(|l| l.key != key);
        let moves = self.theme().motion.resolve(&SETTLE, self.tier()).is_some();
        if moves && self.is_enabled() {
            self.snap_f32(f.id.with(("land", key)), 0.0);
            mem.landings.push(Landing {
                key,
                from,
                pos: from,
            });
        }
    }

    /// The auto-scroll of a drag: a pointer in an edge zone of the board or of the target column
    /// scrolls it by `dt`, and asks for a frame until the content's end.
    fn board_auto_scroll(&mut self, f: &Frame<'_, '_>, mem: &mut BoardMem) {
        let pointer = self.pointer_pos().filter(|_| mem.drag.is_dragging());
        let Some((px, py)) = pointer else {
            mem.scrolled = None;
            return;
        };
        let vp = f.lay.viewport;
        let (px, py) = (f64::from(px - f.shift.0), f64::from(py - f.shift.1));
        let board_max = f64::from((f.lay.content_w - vp.w).max(0.0));
        let dx = edge_step(
            px,
            f64::from(vp.x),
            f64::from(vp.right()),
            mem.board_off > 0.0,
            mem.board_off < board_max - 0.5,
        );
        let target = mem.drag.target().map(|t| t.column);
        let lane = target.map(|c| (&f.lay.lanes[c], f.opts.columns[c].key));
        let (dy, list_room) = lane.map_or((0.0, (0.0, 0.0)), |(lane, key)| {
            let off = mem.list_offset(key);
            let max = f64::from((lane.content_h - lane.list_h).max(0.0));
            let top = f64::from(f.lay.list_y());
            let dy = edge_step(
                py,
                top,
                top + f64::from(lane.list_h),
                off > 0.0,
                off < max - 0.5,
            );
            (dy, (-off, max - off))
        });
        if dx == 0.0 && dy == 0.0 {
            mem.scrolled = None;
            return;
        }
        let now = self.time();
        let dt = mem
            .scrolled
            .map_or(Duration::ZERO, |t| now.saturating_sub(t));
        mem.scrolled = Some(now);
        let ticks = dt.as_secs_f64() / AUTO_SCROLL_TICK.as_secs_f64();
        let by_x = (dx * ticks).clamp(-mem.board_off, board_max - mem.board_off);
        let by_y = (dy * ticks).clamp(list_room.0, list_room.1);
        if by_x != 0.0 {
            self.scroll_shift_x(COLUMNS, by_x);
        }
        if let Some((_, key)) = lane.filter(|_| by_y != 0.0) {
            self.scroll_shift(("list", key), by_y);
        }
        self.mark_moving();
    }

    /// Reveals the card a key moved, now that the model has placed it.
    fn board_reveal(&mut self, f: &Frame<'_, '_>, mem: &mut BoardMem) {
        let Some((c, s)) = mem.reveal.take().and_then(|k| f.lay.slot(k)) else {
            return;
        };
        let (y, h) = (f64::from(s.y), f64::from(s.h));
        let x = f64::from(f.lay.lanes[c].x);
        let key = f.opts.columns[c].key;
        self.scroll_to(("list", key), Target::span_y(y, h), Reveal::Nearest);
        let w = f64::from(COLUMN_W);
        self.scroll_to(COLUMNS, Target::span_x(x, w), Reveal::Nearest);
    }

    /// The window-space top left card `key` shows at now, before this frame's drawing.
    fn board_drawn_at(
        &mut self,
        f: &Frame<'_, '_>,
        mem: &BoardMem,
        c: usize,
        s: &Slot,
    ) -> (f32, f32) {
        if let Some(l) = mem.landings.iter().find(|l| l.key == s.key) {
            return l.pos;
        }
        let r = f.slot_rect(mem, c, s);
        let y = self.animate_f32(s.id.with("y"), s.y, &SLIDE);
        f.to_window(Rect::new(r.x, r.y + y - s.y, r.w, r.h))
    }

    /// The cards' keys, the D169 loop: every key is taken before it is gated, and a Control, Alt or
    /// Command chord stays a shortcut.
    fn board_keys(
        &mut self,
        f: &Frame<'_, '_>,
        items: &[(WidgetId, bool)],
        mem: &mut BoardMem,
        out: &mut BoardResponse,
    ) {
        let enabled = self.is_enabled();
        let (focused, _) = self.focus_state();
        let at = focused.and_then(|id| {
            f.lay.at.iter().find_map(|(k, (c, i))| {
                let slot = f.lay.lanes[*c].slots[*i];
                (slot.id == id).then_some((*k, *c, *i, id))
            })
        });
        if let Some((key, col, index, id)) = at {
            mem.cursor = Some(key);
            let busy = mem.drag.is_active();
            let (mut c, mut i) = (col, index);
            for (event, ev) in self.focus_events(id) {
                let FocusEvent::Key {
                    key: Key::Named(k),
                    modifiers,
                    ..
                } = ev
                else {
                    continue;
                };
                let arrow = matches!(
                    k,
                    NamedKey::ArrowUp
                        | NamedKey::ArrowDown
                        | NamedKey::ArrowLeft
                        | NamedKey::ArrowRight
                );
                let ends = matches!(k, NamedKey::Home | NamedKey::End);
                if !(arrow || ends) || modifiers.control || modifiers.alt || modifiers.logo {
                    continue;
                }
                self.consume_focus_event(event);
                if !enabled || busy {
                    continue;
                }
                if arrow && modifiers.shift {
                    // One move a frame: the next one would count on a list the app has not changed.
                    if out.moved.is_none() {
                        self.board_key_move(f, mem, out, key, (col, index), k);
                    }
                } else {
                    (c, i) = board_step(&f.lay, (c, i), k);
                }
            }
            if (c, i) != (col, index) {
                let to = f.lay.lanes[c].slots[i];
                mem.cursor = Some(to.key);
                self.rove_focus(to.id);
            }
            while self.activation_keys(id, enabled, &[NamedKey::Enter, NamedKey::Space]) {
                if !busy {
                    out.opened = Some(key);
                }
            }
        }
        // The keys above are gone, so this only hands the Tab order the composite's list.
        self.rove_keys(items, RoveAxis::Both, false);
    }

    /// The move Shift with arrow `k` asks of the card at `at`, if it has anywhere to go.
    fn board_key_move(
        &mut self,
        f: &Frame<'_, '_>,
        mem: &mut BoardMem,
        out: &mut BoardResponse,
        key: u64,
        at: (usize, usize),
        k: NamedKey,
    ) {
        let (c, i) = at;
        let n = f.lay.lanes[c].slots.len();
        let across = |to: usize| (to, i.min(f.lay.lanes[to].slots.len()));
        let to = match k {
            NamedKey::ArrowUp if i > 0 => (c, i - 1),
            NamedKey::ArrowDown if i + 1 < n => (c, i + 2),
            NamedKey::ArrowLeft if c > 0 => across(c - 1),
            NamedKey::ArrowRight if c + 1 < f.lay.lanes.len() => across(c + 1),
            _ => return,
        };
        out.moved = Some(CardMove {
            card: key,
            column: f.opts.columns[to.0].key,
            index: to.1,
        });
        mem.reveal = Some(key);
        // A card's `y` lives inside its column's clip: across columns only the ghost can travel.
        if to.0 != c {
            let slot = f.lay.lanes[c].slots[i];
            let from = self.board_drawn_at(f, mem, c, &slot);
            self.board_land(f, mem, key, from);
        }
    }

    fn board_header(&mut self, f: &Frame<'_, '_>, out: &mut BoardResponse) {
        let theme = f.theme;
        let (x, y) = (f.rect.x + MARGIN, f.rect.y + MARGIN);
        let w = (f.rect.w - 2.0 * MARGIN).max(0.0);
        let add = Rect::new(
            x + w - ICON_BUTTON,
            y + (HEADER - ICON_BUTTON) / 2.0,
            ICON_BUTTON,
            ICON_BUTTON,
        );
        let p = self.painter();
        let title = Spec::plain(
            f.opts.title,
            Style::ui(theme.font_m, theme.fg_secondary).medium(),
        );
        let title = p.elide(&title, (w - ICON_BUTTON).max(0.0));
        let h = p.measure(&title).1;
        p.text(&title, x, y + (HEADER - h) / 2.0);
        let name = self.tr("luneui.kanban-board.add-column");
        let button = IconButton {
            glyph: icon::ADD,
            glyph_size: HEADER_GLYPH,
            color: Some(theme.fg_muted),
            radius: ICON_RADIUS,
            name,
        };
        out.add_column |= self.icon_button("add-column", add, &button).clicked;
    }

    fn board_columns(
        &mut self,
        f: &Frame<'_, '_>,
        columns: &[Column<'_>],
        stop: Option<WidgetId>,
        mem: &mut BoardMem,
        out: &mut BoardResponse,
    ) {
        let vp = f.lay.viewport;
        let opts = ScrollOptions {
            bars: (
                Bars::Auto.unless_fits(f64::from(f.lay.content_w), f64::from(vp.w)),
                Bars::Disabled,
            ),
            extent: Some((f64::from(f.lay.content_w), f64::from(vp.h))),
            wheel_x: true,
            ..ScrollOptions::default()
        };
        let area = self.scroll_area(COLUMNS, vp, &opts, |ui, view| {
            for c in 0..f.opts.columns.len() {
                ui.board_column(f, columns, stop, c, view, mem, out);
            }
            ((), Extent::new(f64::from(f.lay.content_w), f64::from(vp.h)))
        });
        mem.board_off = area.offset.0;
    }

    /// One column: frame, header, and its card list.
    #[allow(clippy::too_many_arguments)]
    fn board_column(
        &mut self,
        f: &Frame<'_, '_>,
        columns: &[Column<'_>],
        stop: Option<WidgetId>,
        c: usize,
        view: &ScrollView,
        mem: &mut BoardMem,
        out: &mut BoardResponse,
    ) {
        let (theme, col, lane) = (f.theme, &f.opts.columns[c], &f.lay.lanes[c]);
        let frame = Rect::new(view.viewport.x + lane.x, view.viewport.y, COLUMN_W, lane.h);
        let id = self.id(("column", col.key));
        let target = mem.drag.target().is_some_and(|t| t.column == c);
        let goal = if target { theme.surface_hover } else { CLEAR };
        let fill = self.animate_color(id.with("fill"), goal, &HOVER);
        let mut node = Node::new(id, Role::Group, col.title, frame);
        node.description = Some(col.cards.len().to_string());
        self.node_container(node, |ui| {
            let p = ui.painter();
            if fill[3] > 0 {
                p.fill(frame, RADIUS, fill);
            }
            p.ring(frame, RADIUS, BORDER, theme.border, false);
            let inner = frame.inset(BORDER + COLUMN_PAD, BORDER + COLUMN_PAD);
            ui.board_column_header(col, inner, &theme, out);
            let list = Rect::new(
                inner.x,
                inner.y + COLUMN_HEADER + COLUMN_HEADER_GAP,
                CARD_W,
                lane.list_h,
            );
            let opts = ScrollOptions {
                bars: (
                    Bars::Disabled,
                    Bars::Auto.unless_fits(f64::from(lane.content_h), f64::from(lane.list_h)),
                ),
                extent: Some((f64::from(CARD_W), f64::from(lane.content_h))),
                ..ScrollOptions::default()
            };
            let area = ui.scroll_area(("list", col.key), list, &opts, |ui, view| {
                for (i, (slot, card)) in lane.slots.iter().zip(col.cards).enumerate() {
                    ui.board_card(f, columns, stop, (c, i), slot, card, view, mem, out);
                }
                if let Some(spec) = &lane.hint {
                    let (x, y) = (view.viewport.x + HINT_LEFT, view.viewport.y + HINT_TOP);
                    let text = spec.runs.first().map_or("", |r| r.0.as_str()).to_owned();
                    let size = ui.painter().text(spec, x, y);
                    let at = Rect::new(x, y, size.0, size.1);
                    let hint = ui.id(("hint", col.key));
                    ui.node(Node::new(hint, Role::StaticText, &text, at));
                }
                let extent = Extent::new(f64::from(CARD_W), f64::from(lane.content_h));
                ((), extent)
            });
            mem.list_off.insert(col.key, area.offset.1);
        });
    }

    fn board_column_header(
        &mut self,
        col: &KanbanColumn<'_>,
        inner: Rect,
        theme: &Theme,
        out: &mut BoardResponse,
    ) {
        let menu = Rect::new(
            inner.right() - ICON_BUTTON,
            inner.y,
            ICON_BUTTON,
            ICON_BUTTON,
        );
        let add = Rect::new(menu.x - ICON_BUTTON, inner.y, ICON_BUTTON, ICON_BUTTON);
        let p = self.painter();
        let count = Spec::plain(
            col.cards.len().to_string(),
            Style::ui(theme.font_s, theme.fg_faint),
        );
        let (cw, ch) = p.measure(&count);
        let cx = add.x - COUNT_GAP - cw;
        p.text(&count, cx, inner.y + (COLUMN_HEADER - ch) / 2.0);
        let title = Spec::plain(
            col.title,
            Style::ui(theme.font_base, theme.fg_primary).medium(),
        );
        let title = p.elide(&title, (cx - inner.x).max(0.0));
        let th = p.measure(&title).1;
        p.text(&title, inner.x, inner.y + (COLUMN_HEADER - th) / 2.0);
        let mut button = IconButton {
            glyph: icon::ADD,
            glyph_size: COLUMN_GLYPH,
            color: Some(theme.fg_muted),
            radius: ICON_RADIUS,
            name: self.tr("luneui.kanban-board.add-card"),
        };
        if self.icon_button(("add", col.key), add, &button).clicked {
            out.add_card = Some(col.key);
        }
        button.glyph = icon::ELLIPSIS;
        button.name = self.tr("luneui.kanban-board.column-actions");
        let r = self.icon_button(("menu", col.key), menu, &button);
        if r.clicked {
            out.column_menu = Some((col.key, r.rect));
        }
    }

    /// One card: its region, node and gesture, then its body, so what it draws is this frame's.
    #[allow(clippy::too_many_arguments)]
    fn board_card(
        &mut self,
        f: &Frame<'_, '_>,
        columns: &[Column<'_>],
        stop: Option<WidgetId>,
        at: (usize, usize),
        slot: &Slot,
        card: &KanbanCard<'_>,
        view: &ScrollView,
        mem: &mut BoardMem,
        out: &mut BoardResponse,
    ) {
        let (id, key) = (slot.id, card.key);
        let rect = Rect::new(view.viewport.x, view.viewport.y + slot.y, CARD_W, slot.h);
        let sense = Sense {
            focusable: true,
            skip_tab: stop != Some(id),
            ..Sense::DRAG
        };
        let r = self.interact(id, rect, sense);
        let mut node = Node::new(id, Role::ListItem, card.title, rect);
        node.states.insert(State::Focusable);
        for action in [
            Action::Default,
            Action::Focus,
            Action::ScrollIntoView,
            Action::ShowMenu,
        ] {
            node.actions.insert(action);
        }
        self.node(node);

        let dragged = mem.drag.source() == Some(key);
        let frame = self.free_drag_gesture(id, &r, &mut mem.drag, key, columns);
        if r.press_started
            && let Some((px, py)) = r.press_pos
        {
            let top = self.painter().transformed(rect);
            let top = mem
                .landings
                .iter()
                .find(|l| l.key == key)
                .map_or((top.x, top.y), |l| l.pos);
            mem.grab = (px - top.0, py - top.1);
        }
        self.board_result(f, mem, out, at, key, dragged, &frame);
        if r.clicked && self.access_default(id) {
            out.opened = Some(key);
        }
        if r.secondary_clicked {
            out.context = Some((key, r.rect));
        }
        let source = mem.drag.source() == Some(key);
        if source {
            self.patch_node(id, |n| {
                n.states.insert(State::Dragging);
            });
        }

        // The body. A landing card has none, and its `y` waits at the slot for the ghost.
        let landing = mem.landings.iter().any(|l| l.key == key);
        let y = if landing {
            self.snap_f32(id.with("y"), slot.y)
        } else {
            self.animate_f32(id.with("y"), slot.y, &SLIDE)
        };
        let alpha = self.drag_dim(id, source);
        let theme = f.theme;
        let goal = if r.hovered {
            theme.surface_hover
        } else {
            theme.surface_input
        };
        let fill = self.animate_color(id.with("hover"), goal, &HOVER);
        let level = if r.focus_visible { 1.0 } else { 0.0 };
        let focus = self.animate_f32(id.with("focus"), level, &FOCUS);
        if landing {
            return;
        }
        let body = Rect::new(rect.x, view.viewport.y + y, rect.w, rect.h);
        let p = self.painter();
        if alpha < 1.0 {
            p.push_xform(Xform {
                origin: (0.0, 0.0),
                scale: 1.0,
                dx: 0.0,
                dy: 0.0,
                alpha,
            });
        }
        paint_card(p, body, card, &CardLook { fill, focus }, &theme);
        if alpha < 1.0 {
            p.pop_xform();
        }
    }

    /// Whether this frame's click on `id` came from the Default action: a pointer's arrives through
    /// the gesture, and one the gesture cancelled must stay nothing.
    fn access_default(&self, id: WidgetId) -> bool {
        self.input()
            .events
            .iter()
            .any(|e| matches!(e, Event::Access(a) if a.target == id && a.action == Action::Default))
    }

    /// What a gesture's frame means: an open, a reported move, and a landing for everything that
    /// ended a drag.
    #[allow(clippy::too_many_arguments)]
    fn board_result(
        &mut self,
        f: &Frame<'_, '_>,
        mem: &mut BoardMem,
        out: &mut BoardResponse,
        at: (usize, usize),
        key: u64,
        dragged: bool,
        frame: &FreeFrame,
    ) {
        match frame.result {
            FreeResult::Click => out.opened = Some(key),
            FreeResult::Drop { source, target } => {
                let own =
                    target.column == at.0 && (target.index == at.1 || target.index == at.1 + 1);
                if !own {
                    out.moved = Some(CardMove {
                        card: source,
                        column: f.opts.columns[target.column].key,
                        index: target.index,
                    });
                }
            }
            FreeResult::NoTarget | FreeResult::None => {}
        }
        if dragged && !frame.dragging {
            let from = mem.ghost;
            self.board_land(f, mem, key, from);
        }
    }

    /// The drop line, in the gap of the target slot: after the gestures, so it is this frame's.
    fn board_drop_line(&mut self, f: &Frame<'_, '_>, mem: &mut BoardMem) {
        let Some(t) = mem.drag.target() else {
            mem.line = None;
            return;
        };
        let lane = &f.lay.lanes[t.column];
        let gap = lane.gap_y(t.index);
        let id = f.id.with("drop-line");
        let y = if mem.line == Some(t.column) {
            self.animate_f32(id, gap, &SLIDE)
        } else {
            self.snap_f32(id, gap)
        };
        mem.line = Some(t.column);
        let off = mem.list_offset(f.opts.columns[t.column].key) as f32;
        let x = f.lay.viewport.x + lane.x - mem.board_off as f32;
        let top = f.lay.list_y();
        let clip = Rect::new(x, top - CARD_GAP, COLUMN_W, lane.list_h + 2.0 * CARD_GAP)
            .intersect(&f.lay.viewport);
        let at = Rect::new(x + BORDER + COLUMN_PAD, top + y - off, CARD_W, CARD_GAP);
        let theme = f.theme;
        let p = self.painter();
        p.push_clip(clip);
        paint_drop_line(p, at, &theme);
        p.pop_clip();
    }

    /// The dragged card's ghost at the pointer, and every landing at its place on the way to the
    /// card's slot of this frame. A press on a landing card holds it where it is: from there it
    /// becomes the drag, or goes on.
    fn board_ghosts(&mut self, f: &Frame<'_, '_>, mem: &mut BoardMem) {
        if let Some(key) = mem.drag.source()
            && let Some((_, slot)) = f.lay.slot(key)
        {
            let pos = mem.ghost;
            self.board_paint_ghost(f, key, pos, slot);
        }
        let mut landings = std::mem::take(&mut mem.landings);
        landings.retain_mut(|l| {
            let Some((c, slot)) = f.lay.slot(l.key) else {
                return false;
            };
            let id = f.id.with(("land", l.key));
            let t = if mem.drag.pressed() == Some(l.key) {
                l.from = l.pos;
                self.snap_f32(id, 0.0)
            } else {
                self.animate_f32(id, 1.0, &SETTLE)
            };
            let to = f.to_window(f.slot_rect(mem, c, slot));
            l.pos = (
                l.from.0 + (to.0 - l.from.0) * t,
                l.from.1 + (to.1 - l.from.1) * t,
            );
            self.board_paint_ghost(f, l.key, l.pos, slot);
            if t >= 1.0 {
                // The body takes over on the next frame, and nothing else would ask for one.
                self.request_catch_up();
            }
            t < 1.0
        });
        mem.landings = landings;
    }

    /// A card's ghost at the window-space `pos`, over an opaque fill: the card's own is 4% white.
    fn board_paint_ghost(&mut self, f: &Frame<'_, '_>, key: u64, pos: (f32, f32), slot: &Slot) {
        let Some(card) = f.card(key) else { return };
        let theme = f.theme;
        let level = if self.focus_state() == (Some(slot.id), true) {
            1.0
        } else {
            0.0
        };
        let focus = self.animate_f32(slot.id.with("focus"), level, &FOCUS);
        let rect = Rect::new(pos.0, pos.1, CARD_W, slot.h);
        let look = CardLook {
            fill: theme.surface_input,
            focus,
        };
        self.drag_ghost(|ui| {
            let p = ui.painter();
            p.fill(rect, CARD_RADIUS, theme.window_fallback);
            paint_card(p, rect, card, &look, &theme);
        });
    }
}

/// Where a plain `k` takes the cursor from `at`: Up, Down, Home and End inside the column; Left and
/// Right to the nearest index of the next column with cards.
fn board_step(lay: &Layout, at: (usize, usize), k: NamedKey) -> (usize, usize) {
    let (c, i) = at;
    let last = lay.lanes[c].slots.len().saturating_sub(1);
    let across = |mut cols: Box<dyn Iterator<Item = usize>>| {
        cols.find(|to| !lay.lanes[*to].slots.is_empty())
            .map_or(at, |to| (to, i.min(lay.lanes[to].slots.len() - 1)))
    };
    match k {
        NamedKey::ArrowUp => (c, i.saturating_sub(1)),
        NamedKey::ArrowDown => (c, (i + 1).min(last)),
        NamedKey::Home => (c, 0),
        NamedKey::End => (c, last),
        NamedKey::ArrowLeft => across(Box::new((0..c).rev())),
        NamedKey::ArrowRight => across(Box::new(c + 1..lay.lanes.len())),
        _ => at,
    }
}
