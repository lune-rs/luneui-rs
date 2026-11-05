//! The tray flyout (`overlay.flyout`, D145): a glass popover drawn from the app's anchor. A header
//! (mark, wordmark, state), a strip of compact stat cells, a list of routes, a separator and the
//! menu's rows, with the menu's keys. M10 places it from the tray icon's rect; here it is placed
//! from whatever anchor the app gives.
//!
//! Geometry (the consuming app's design): 340 wide, 5 px of padding inside a 1 px ring (so content starts
//! 6 px in, as a menu's rows do), the header 6 8 around a 20 px line, the stats and the routes
//! each 8 px apart from what is above and below them (neighbouring margins collapse, as in the
//! design), the routes inset 8 more. Empty `stats` or `routes` take no height; the separator and
//! the rows follow when there are rows. Text is at least 9 px from the card's edge.

use std::hash::Hash;

use super::kv_list::{self, KvMode, KvOptions};
use super::menu::{MenuRow, PAD, menu_paint_rows, menu_rows, paint_separator};
use super::popover::{PlaceAlign, PopoverOptions, Side, Surface};
use super::stat::{self, StatOptions};
use super::status_dot;
use super::tone::Tone;
use crate::access::{Live, Node, Role, State, Value};
use crate::icon;
use crate::paint::{Painter, Rect};
use crate::text::{Spec, Style};
use crate::ui::{Sense, Ui, WidgetId};

/// The card's width.
const WIDTH: f32 = 340.0;
/// The space above and below the stats and the routes.
const BLOCK_MARGIN: f32 = 8.0;
/// The header's padding: vertical and horizontal.
const HEADER_PAD: (f32, f32) = (6.0, 8.0);
/// The header's line.
const HEADER_LINE: f32 = 20.0;
/// The header's height.
const HEADER_H: f32 = 2.0 * HEADER_PAD.0 + HEADER_LINE;
/// The mark's side, and its distance from the wordmark.
const MARK: f32 = 16.0;
const MARK_GAP: f32 = 8.0;
/// The state's line, and the distance of its dot from its text.
const STATE_LINE: f32 = 16.0;
const STATE_DOT_GAP: f32 = 6.0;
/// The stats strip's outline radius, and its width (the cells sit this far inside it).
const STRIP_RADIUS: f32 = 8.0;
const STRIP_RING: f32 = 1.0;
/// The routes' key size is `font_s`, as in the consuming app's design.
/// The routes' horizontal inset.
const ROUTES_INSET: f32 = 8.0;
/// A separator row's height.
const SEPARATOR: f32 = 11.0;
/// Between the flyout and its anchor, and between it and the window's edge.
const GAP: f32 = 12.0;

/// What a flyout shows and where it opens.
#[derive(Debug, Clone, Copy)]
pub struct Flyout<'a> {
    /// The tray icon's rect (or the app's stand-in), in window coordinates.
    pub anchor: Rect,
    /// The widget focus returns to on close when the one focused at open is gone.
    pub anchor_id: Option<WidgetId>,
    pub side: Side,
    pub align: PlaceAlign,
    /// The header's mark: a glyph of the icon font, drawn 16 px in `fg_primary`.
    pub mark: Option<&'a str>,
    /// The app's name, 14 px bold in `gold`.
    pub wordmark: &'a str,
    /// The state's text at the header's right, behind a dot in `tone`.
    pub state: &'a str,
    pub tone: Tone,
    /// The stat cells, drawn compact.
    pub stats: &'a [StatOptions<'a>],
    /// Key and value pairs, the values mono.
    pub routes: &'a [(&'a str, &'a str)],
    /// The action rows; a pick reports the index in this slice.
    pub rows: &'a [MenuRow<'a>],
    /// The rows' accessible name; empty uses `luneui.menu.name`.
    pub name: &'a str,
}

/// What a flyout frame reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlyoutResponse {
    /// The index in [`Flyout::rows`] of the row picked this frame.
    pub picked: Option<usize>,
    /// The Ui asks the app to close the flyout (Escape, an outside press, focus loss, an
    /// accessibility dismiss).
    pub dismissed: bool,
}

/// The content heights of the stats and the routes (0 for an empty one).
fn block_heights<H: Copy + PartialEq>(p: &mut Painter<'_, H>, f: &Flyout<'_>) -> (f32, f32) {
    let w = WIDTH - 2.0 * PAD;
    let stats = if f.stats.is_empty() {
        0.0
    } else {
        let cells = compact(f.stats);
        stat::strip_height(p, &cells, w - 2.0 * STRIP_RING) + 2.0 * STRIP_RING
    };
    let routes = if f.routes.is_empty() {
        0.0
    } else {
        kv_list::height_keyed(p, f.routes, &ROUTES, w - 2.0 * ROUTES_INSET, p.theme.font_s)
    };
    (stats, routes)
}

/// The space the stats and the routes take with their margins: each block has 8 px above it, the
/// last has 8 below, and two neighbours share theirs.
fn blocks_height(stats: f32, routes: f32, f: &Flyout<'_>) -> f32 {
    let n = usize::from(!f.stats.is_empty()) + usize::from(!f.routes.is_empty());
    if n == 0 {
        0.0
    } else {
        (n + 1) as f32 * BLOCK_MARGIN + stats + routes
    }
}

/// `cells` drawn compact.
fn compact<'a>(cells: &[StatOptions<'a>]) -> Vec<StatOptions<'a>> {
    cells
        .iter()
        .map(|c| StatOptions {
            compact: true,
            ..*c
        })
        .collect()
}

const ROUTES: KvOptions = KvOptions {
    mode: KvMode::Default,
    mono: true,
};

/// The card's size: 340 wide, and as tall as its padding, header, blocks, separator and rows.
pub fn flyout_size<H: Copy + PartialEq>(p: &mut Painter<'_, H>, f: &Flyout<'_>) -> (f32, f32) {
    let (stats, routes) = block_heights(p, f);
    let rows: f32 = f.rows.iter().map(MenuRow::height).sum();
    let tail = if f.rows.is_empty() {
        0.0
    } else {
        SEPARATOR + rows
    };
    (
        WIDTH,
        2.0 * PAD + HEADER_H + blocks_height(stats, routes, f) + tail,
    )
}

/// The names of `parts` in one line, for a group's accessible name.
fn joined<'s>(parts: impl Iterator<Item = &'s str>) -> String {
    parts.collect::<Vec<_>>().join(", ")
}

/// Draws the flyout's content in `rect`, the card the surface is drawn behind, and returns the
/// index of the row picked this frame. The window M10 opens runs this on its own `Ui`.
pub(crate) fn flyout_body(ui: &mut Ui<'_>, rect: Rect, f: &Flyout<'_>) -> Option<usize> {
    let t = *ui.theme();
    let content = Rect::new(rect.x + PAD, rect.y + PAD, rect.w - 2.0 * PAD, 0.0);
    // The card blocks what is under it, so a press on the header or a margin is not an outside
    // press.
    let card = ui.id("card");
    ui.interact(card, rect, Sense::HOVER);

    // Header: mark, wordmark, and the state at the right.
    let top = content.y + HEADER_PAD.0;
    let mut x = content.x + HEADER_PAD.1;
    if let Some(mark) = f.mark {
        let slot = Rect::new(x, top, MARK, HEADER_LINE);
        ui.painter()
            .text_centered(&icon::glyph(mark, MARK, t.fg_primary), slot);
        x += MARK + MARK_GAP;
    }
    let name = Spec::plain(
        f.wordmark,
        Style {
            line: HEADER_LINE,
            ..Style::ui(t.font_m, t.gold).bold()
        },
    );
    ui.painter().text(&name, x, top);
    let state = Spec::plain(
        f.state,
        Style {
            line: STATE_LINE,
            ..Style::ui(t.font_s, t.fg_secondary)
        },
    );
    let (sw, sh) = ui.painter().measure(&state);
    let state_x = content.right() - HEADER_PAD.1 - sw;
    let state_y = top + (HEADER_LINE - STATE_LINE) / 2.0;
    let dot_x = state_x - STATE_DOT_GAP - status_dot::SIZE / 2.0;
    status_dot::paint(ui.painter(), (dot_x, top + HEADER_LINE / 2.0), f.tone);
    ui.painter().text(&state, state_x, state_y);
    let mut node = Node::new(
        ui.id("state"),
        Role::Status,
        f.state,
        Rect::new(state_x, state_y, sw, sh),
    );
    node.value = Some(Value::Text(f.state.to_owned()));
    node.live = Some(Live::Polite);
    ui.node(node);

    // The stats and the routes, 8 px apart from each other and from the header and the rows.
    let (stats_h, routes_h) = block_heights(ui.painter(), f);
    let mut y = top + HEADER_LINE + HEADER_PAD.0;
    if !f.stats.is_empty() {
        y += BLOCK_MARGIN;
        let at = Rect::new(content.x, y, content.w, stats_h);
        let group = Node::new(
            ui.id("stats-group"),
            Role::Group,
            &joined(f.stats.iter().map(|s| s.label)),
            at,
        );
        let cells = compact(f.stats);
        let inner = Rect::new(
            at.x + STRIP_RING,
            at.y + STRIP_RING,
            at.w - 2.0 * STRIP_RING,
            at.h - 2.0 * STRIP_RING,
        );
        ui.node_container(group, |ui| ui.stat_strip("stats", inner, &cells));
        // The outline of the strip panel, after the cells.
        ui.painter()
            .ring(at, STRIP_RADIUS, STRIP_RING, t.border_subtle, false);
        y += stats_h;
    }
    if !f.routes.is_empty() {
        y += BLOCK_MARGIN;
        let at = Rect::new(
            content.x + ROUTES_INSET,
            y,
            content.w - 2.0 * ROUTES_INSET,
            routes_h,
        );
        let group = Node::new(
            ui.id("routes-group"),
            Role::Group,
            &joined(f.routes.iter().map(|(k, _)| *k)),
            at,
        );
        ui.node_container(group, |ui| {
            ui.kv_list_keyed("routes", at, f.routes, &ROUTES, t.font_s)
        });
        y += routes_h;
    }
    if !f.stats.is_empty() || !f.routes.is_empty() {
        y += BLOCK_MARGIN;
    }
    if f.rows.is_empty() {
        return None;
    }

    // The separator, then the rows.
    paint_separator(
        ui.painter(),
        rect,
        Rect::new(content.x, y, content.w, SEPARATOR),
    );
    y += SEPARATOR;
    let rows_top = y;
    let rects: Vec<Rect> = f
        .rows
        .iter()
        .map(|row| {
            let r = Rect::new(content.x, y, content.w, row.height());
            y += row.height();
            r
        })
        .collect();
    let menu_name = match f.name {
        "" => ui.tr("luneui.menu.name"),
        name => name,
    };
    let menu_id = ui.id("rows");
    let mut menu = Node::new(
        menu_id,
        Role::Menu,
        menu_name,
        Rect::new(content.x, rows_top, content.w, y - rows_top),
    );
    menu.states.insert(State::Expanded);
    let frame = ui.node_container(menu, |ui| menu_rows(ui, menu_id, f.rows, &rects));
    menu_paint_rows(ui.painter(), f.rows, rect, &rects, &frame.looks);
    frame.picked
}

impl<'a> Ui<'a> {
    /// The flyout `f` on the popup layer, a glass popover: Escape, a press outside it and focus
    /// loss dismiss it (`dismissed`), and a click on a row, or Enter or Space on the lit one,
    /// picks it (`picked`). The rows are the menu's: Up, Down, Home and End move one highlight
    /// over the enabled rows, which are one tab stop. Opens and closes with the menu's curves:
    /// call it every frame it is open.
    pub fn flyout(&mut self, key: impl Hash, f: &Flyout<'_>) -> FlyoutResponse {
        let size = flyout_size(self.painter(), f);
        let options = PopoverOptions {
            anchor: f.anchor,
            anchor_id: f.anchor_id,
            size,
            side: f.side,
            align: f.align,
            gap: GAP,
            margin: GAP,
            shift: true,
            surface: Surface::Glass,
        };
        let (frame, _, picked) = self.popover(&key, &options, |ui, rect| {
            ui.scope(&key, |ui| flyout_body(ui, rect, f))
        });
        FlyoutResponse {
            picked,
            dismissed: frame.dismissed.is_some(),
        }
    }
}
