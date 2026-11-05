//! The command palette (`overlay.command-palette`, D147): a 640 px modal card with one search
//! box, the app's grouped results and a footer, on the dialog's modal host.
//!
//! The app owns the search: it passes sections of rows already filtered by the query (prefixes,
//! debounce and date parsing are its logic) and reads [`PaletteResponse`]. Focus stays in the
//! search box; Up, Down and Enter are taken before the box draws, so the caret does not move, and
//! the rows have no tab stops of their own. The selected row is kept by key: when the rows change
//! the first row is selected unless the selected key is still there. A "show more" control is a
//! row of its section, in the same order.
//!
//! A frame draws every row it is given (a few hundred at most: the app caps its sections), in a
//! [`scroll_area`](Ui::scroll_area) whose extent is known before the content, so the selected row
//! is revealed in the frame it is chosen. Row keys must be unique within a palette.

use std::cell::RefCell;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

use super::dialog::{DialogGlass, ModalHost, modal_host};
use super::filter_bar::FilterOptions;
use super::scroll_area::{Bars, Extent, Reveal, ScrollOptions, Target};
use crate::access::{Action, Live, Node, Role, State, Value};
use crate::icon;
use crate::input::{FocusEvent, Key, NamedKey};
use crate::paint::{Ops, Painter, Rect};
use crate::text::{Spec, Style};
use crate::theme::Theme;
use crate::ui::{Sense, Ui, WidgetId};

/// The card's width.
pub const CARD_W: f32 = 640.0;
/// The card's greatest height.
pub const MAX_H: f32 = 720.0;
/// The results area's greatest height.
pub const RESULTS_MAX_H: f32 = 560.0;
/// The search row: padding left, top, right and bottom, and the bar's height (a 28 px dock with
/// the filter bar's 8 by 6 padding around it: `GlobalSearchDialogView.axaml:17-18`,
/// `App.axaml:302-306`).
pub const SEARCH_PAD: [f32; 4] = [18.0, 18.0, 18.0, 10.0];
pub const SEARCH_H: f32 = 40.0;
/// The results area spans the card, with no margin and no gap under it (`GlobalSearchDialogView.axaml:
/// 59-67`: the `ScrollViewer` has `Padding="10,0,10,0"`; the `18,0,18,10` margin at `:60` is the
/// indexing banner's). Its content is inset by `LIST_PAD`, so rows sit 10 in and their text 18.
pub const LIST_PAD: f32 = 10.0;
/// The gap under the error line in the footer (the stack's spacing, `axaml:523`).
const ERROR_GAP: f32 = 6.0;
/// The destructive mark: a dot of this diameter at the row's right edge, 8 from what is left of
/// it (`axaml:489-496`).
const DOT: f32 = 6.0;
const DOT_GAP: f32 = 8.0;
/// A row's padding (horizontal, vertical), corner radius, and the space between its icon and text.
pub const ROW_PAD: (f32, f32) = (8.0, 6.0);
pub const ROW_RADIUS: f32 = 6.0;
pub const ICON: f32 = 14.0;
pub const ICON_GAP: f32 = 8.0;
/// The space between two sections, and between a header (or the rows) and what follows it.
pub const SECTION_GAP: f32 = 10.0;
pub const HEADER_GAP: f32 = 4.0;
/// A header's horizontal margin.
pub const HEADER_INSET: f32 = 8.0;
/// The footer's padding: left, top, right and bottom.
pub const FOOTER_PAD: [f32; 4] = [18.0, 10.0, 18.0, 12.0];
/// Between a row's text and its right column (a chord hint, the destructive glyph).
const RIGHT_GAP: f32 = 12.0;
/// The card keeps this much window around it when the window is short.
const WINDOW_MARGIN: f32 = 40.0;

/// One result row. `key` identifies it (unique in the palette) and is what [`PaletteResponse`]
/// reports; `hint` is the right-hand text, such as `Chord::hint`'s.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteRow<'a> {
    pub key: u64,
    pub text: &'a str,
    /// A second, smaller line under the text.
    pub detail: Option<&'a str>,
    /// A codicon glyph (`icon::*`) before the text.
    pub icon: Option<&'a str>,
    pub hint: Option<&'a str>,
    /// Activating it needs the app's confirmation: the row shows the warning glyph and, on hover,
    /// the string table's `luneui.command-palette.destructive-tooltip`. It still reports
    /// activation.
    pub destructive: bool,
}

/// A titled run of rows. A section with no rows is not drawn. `more` adds the "show more" row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteSection<'a> {
    pub title: &'a str,
    pub rows: &'a [PaletteRow<'a>],
    pub more: bool,
}

/// What [`Ui::command_palette`] shows. The app filters by the query: it passes the rows to show.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette<'a> {
    pub sections: &'a [PaletteSection<'a>],
    /// Shown while the query is empty; also the card's accessible name when `name` is empty.
    pub placeholder: &'a str,
    /// Left of the footer; the right is the string table's `luneui.command-palette.footer-right`.
    pub footer_left: &'a str,
    /// One line in the footer above its text, in `status_error`, announced politely.
    pub error: Option<&'a str>,
    pub name: &'a str,
}

/// What a frame of [`Ui::command_palette`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PaletteResponse {
    /// The query changed this frame (typing, the clear button, a paste).
    pub changed: bool,
    /// The key of the row activated by a click or by Enter.
    pub activated: Option<u64>,
    /// The index of the section whose "show more" row was activated.
    pub show_more: Option<usize>,
    /// Escape, a scrim click or an assistive technology's dismiss.
    pub dismissed: bool,
}

/// What the palette remembers while it is open.
#[derive(Debug, Clone, Copy, Default)]
struct PaletteMem {
    /// The selected row's key (a show-more row has its own derived key).
    selected: Option<u64>,
    /// A hash of the row keys last seen, to tell when the rows changed.
    shown: u64,
}

/// Line heights of the two text sizes the palette uses.
#[derive(Debug, Clone, Copy)]
struct Lines {
    main: f32,
    small: f32,
}

impl Lines {
    fn of(t: &Theme) -> Self {
        Self {
            main: Style::ui(t.font_base, t.fg_primary).line,
            small: Style::ui(t.font_s, t.fg_primary).line,
        }
    }

    fn row(self, row: &PaletteRow<'_>) -> f32 {
        2.0 * ROW_PAD.1 + self.main + row.detail.map_or(0.0, |_| self.small)
    }

    fn more(self) -> f32 {
        2.0 * ROW_PAD.1 + self.small
    }

    /// The footer's height: its padding around the key line, and the error line with its gap
    /// above them when there is one.
    fn footer(self, has_error: bool) -> f32 {
        let error = if has_error {
            self.small + ERROR_GAP
        } else {
            0.0
        };
        FOOTER_PAD[1] + error + self.small + FOOTER_PAD[3]
    }
}

/// The key a section's "show more" row selects by.
fn more_key(section: usize) -> u64 {
    let mut h = DefaultHasher::new();
    ("palette-more-row", section).hash(&mut h);
    h.finish()
}

/// One selectable entry of the list: a row or a section's "show more".
#[derive(Debug, Clone, Copy)]
struct Entry {
    key: u64,
    section: usize,
    /// The row's index in its section; `None` for "show more".
    row: Option<usize>,
    /// Top and height, in content coordinates.
    y: f32,
    h: f32,
    /// What to reveal for it: the entry, and the section's header too for the first one.
    reveal: (f32, f32),
}

/// A drawn section: its top (the header's), height and its entries' range.
#[derive(Debug, Clone)]
struct Block {
    section: usize,
    y: f32,
    h: f32,
    entries: std::ops::Range<usize>,
}

#[derive(Debug, Clone, Default)]
struct List {
    entries: Vec<Entry>,
    blocks: Vec<Block>,
    height: f32,
}

impl List {
    fn of(palette: &Palette<'_>, lines: Lines) -> Self {
        let mut list = List::default();
        let mut y = 0.0;
        for (si, sec) in palette.sections.iter().enumerate() {
            if sec.rows.is_empty() {
                continue;
            }
            if !list.blocks.is_empty() {
                y += SECTION_GAP;
            }
            let top = y;
            let first = list.entries.len();
            y += lines.small + HEADER_GAP;
            for (ri, row) in sec.rows.iter().enumerate() {
                let h = lines.row(row);
                let from = if ri == 0 { top } else { y };
                list.entries.push(Entry {
                    key: row.key,
                    section: si,
                    row: Some(ri),
                    y,
                    h,
                    reveal: (from, y + h - from),
                });
                y += h;
            }
            if sec.more {
                y += HEADER_GAP;
                let h = lines.more();
                list.entries.push(Entry {
                    key: more_key(si),
                    section: si,
                    row: None,
                    y,
                    h,
                    reveal: (y, h),
                });
                y += h;
            }
            list.blocks.push(Block {
                section: si,
                y: top,
                h: y - top,
                entries: first..list.entries.len(),
            });
        }
        list.height = y;
        list
    }

    fn index_of(&self, key: Option<u64>) -> Option<usize> {
        let key = key?;
        self.entries.iter().position(|e| e.key == key)
    }

    /// A hash of the entries' keys, in order.
    fn shown(&self) -> u64 {
        let mut h = DefaultHasher::new();
        for e in &self.entries {
            e.key.hash(&mut h);
        }
        h.finish()
    }
}

/// Where the parts of the card sit.
#[derive(Debug, Clone, Copy)]
struct Geometry {
    card: Rect,
    search: Rect,
    error: Option<Rect>,
    /// The scroll area's viewport; `None` with no rows.
    results: Option<Rect>,
    footer: Rect,
}

impl Geometry {
    fn of(win: Rect, list: &List, has_error: bool, lines: Lines) -> Self {
        let search_block = SEARCH_PAD[1] + SEARCH_H + SEARCH_PAD[3];
        let footer_h = lines.footer(has_error);
        let max_h = MAX_H.min((win.h - 2.0 * WINDOW_MARGIN).max(0.0));
        let room = (max_h - search_block - footer_h).max(0.0);
        let results_h = if list.entries.is_empty() {
            0.0
        } else {
            list.height.min(RESULTS_MAX_H).min(room)
        };
        let h = search_block + results_h + footer_h;
        let card = Rect::new(
            (win.w - CARD_W) / 2.0,
            ((win.h - h) / 2.0).max(20.0),
            CARD_W,
            h,
        );
        let search = Rect::new(
            card.x + SEARCH_PAD[0],
            card.y + SEARCH_PAD[1],
            CARD_W - SEARCH_PAD[0] - SEARCH_PAD[2],
            SEARCH_H,
        );
        let results =
            (results_h > 0.0).then(|| Rect::new(card.x, card.y + search_block, CARD_W, results_h));
        let footer = Rect::new(card.x, card.bottom() - footer_h, CARD_W, footer_h);
        let error = has_error.then(|| {
            Rect::new(
                footer.x + FOOTER_PAD[0],
                footer.y + FOOTER_PAD[1],
                CARD_W - FOOTER_PAD[0] - FOOTER_PAD[2],
                lines.small,
            )
        });
        Geometry {
            card,
            search,
            error,
            results,
            footer,
        }
    }
}

/// What a frame of the body decided.
#[derive(Debug, Clone, Copy, Default)]
struct Outcome {
    changed: bool,
    /// The entry activated: a row's key or a section's "show more".
    act: Option<Act>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Act {
    Row(u64),
    More(usize),
}

/// What the body needs besides the `Ui`.
struct Body<'p, 'q> {
    palette: &'p Palette<'p>,
    list: &'q List,
    geo: Geometry,
    lines: Lines,
    id: WidgetId,
    search_key: (WidgetId, &'static str),
    search_id: WidgetId,
}

/// The fill of a row from what the pointer and the selection do to it.
fn row_fill(t: &Theme, hovered: bool, pressed: bool, selected: bool) -> Option<[u8; 4]> {
    if pressed || (selected && hovered) {
        Some(t.surface_active)
    } else if selected {
        Some(t.surface_selected)
    } else if hovered {
        Some(t.surface_hover)
    } else {
        None
    }
}

impl<'p, 'q> Body<'p, 'q> {
    /// The keys: Up, Down and Enter, taken before the search box draws. Returns the activation.
    /// Taken even when disabled (so no shortcut sees them), but then they do nothing; and left to
    /// the input method while it composes.
    fn keys(&self, ui: &mut Ui<'_>, mem: &mut PaletteMem) -> Option<Act> {
        if ui.composing() {
            return None;
        }
        let live = ui.is_enabled();
        let n = self.list.entries.len();
        let mut at = self.list.index_of(mem.selected);
        for (index, event) in ui.focus_events(self.search_id) {
            let FocusEvent::Key {
                key: Key::Named(key),
                modifiers,
                ..
            } = event
            else {
                continue;
            };
            if modifiers.control || modifiers.alt || modifiers.logo {
                continue;
            }
            match key {
                NamedKey::ArrowDown | NamedKey::ArrowUp => {
                    ui.consume_focus_event(index);
                    if live && n > 0 {
                        let step = if key == NamedKey::ArrowDown { 1 } else { n - 1 };
                        let to = at.map_or(if step == 1 { 0 } else { n - 1 }, |i| (i + step) % n);
                        at = Some(to);
                        mem.selected = Some(self.list.entries[to].key);
                    }
                }
                NamedKey::Enter => {
                    ui.consume_focus_event(index);
                    if live && let Some(i) = at {
                        return Some(self.act_of(&self.list.entries[i]));
                    }
                    return None;
                }
                _ => {}
            }
        }
        None
    }

    fn act_of(&self, e: &Entry) -> Act {
        match e.row {
            Some(_) => Act::Row(e.key),
            None => Act::More(e.section),
        }
    }

    /// Everything on the card after the acrylic.
    fn draw<'a>(&self, ui: &mut Ui<'a>, query: &mut String, mem: &mut PaletteMem) -> Outcome {
        let t = *ui.theme();
        let palette = self.palette;
        let geo = &self.geo;
        let mut out = Outcome::default();
        ui.painter()
            .fill_radii(geo.footer, [0.0, 0.0, 8.0, 8.0], t.dialog_footer);

        // Selection: valid and, when the rows changed, the first row unless it is still there.
        let before = mem.selected;
        if self.list.index_of(mem.selected).is_none() {
            mem.selected = self.list.entries.first().map(|e| e.key);
        }
        let shown = self.list.shown();
        let rows_changed = mem.shown != shown;
        mem.shown = shown;
        let enter = self.keys(ui, mem);

        let field = ui.filter_bar(
            self.search_key,
            geo.search,
            query,
            &FilterOptions {
                placeholder: palette.placeholder,
                label: None,
            },
        );
        out.changed = field.response.changed;

        if let (Some(text), Some(r)) = (palette.error, geo.error) {
            let eid = self.id.with("error");
            let spec = Spec::plain(text, Style::ui(t.font_s, t.status_error));
            let spec = ui.painter().elide(&spec, r.w);
            ui.painter().text(&spec, r.x, r.y);
            let mut node = Node::new(eid, Role::Status, text, r);
            node.live = Some(Live::Polite);
            ui.node(node);
        }

        let mut act = enter;
        if let Some(view) = geo.results {
            let key = (self.id, "results");
            if mem.selected != before || rows_changed {
                if let Some(e) = self
                    .list
                    .index_of(mem.selected)
                    .map(|i| self.list.entries[i])
                {
                    ui.scroll_to(
                        key,
                        Target::span_y(f64::from(e.reveal.0), f64::from(e.reveal.1)),
                        Reveal::Nearest,
                    );
                }
            }
            let opts = ScrollOptions {
                bars: (Bars::Disabled, Bars::Auto),
                extent: Some((f64::from(view.w), f64::from(self.list.height))),
                ..ScrollOptions::default()
            };
            let selected = mem.selected;
            let clicked = ui
                .scroll_area(key, view, &opts, |ui, scroll| {
                    (
                        self.results(ui, scroll.viewport, selected),
                        Extent::new(f64::from(view.w), f64::from(self.list.height)),
                    )
                })
                .value;
            if let Some(key) = clicked {
                mem.selected = Some(key);
                act = act.or(self
                    .list
                    .index_of(Some(key))
                    .map(|i| self.act_of(&self.list.entries[i])));
            }
        }

        self.footer(ui, &t);
        out.act = act;
        out
    }

    /// The sections, in content coordinates; returns the key of the entry a click activated.
    fn results<'a>(&self, ui: &mut Ui<'a>, vp: Rect, selected: Option<u64>) -> Option<u64> {
        let t = *ui.theme();
        let palette = self.palette;
        let x = vp.x + LIST_PAD;
        let w = vp.w - 2.0 * LIST_PAD;
        let rows_id = self.id.with("rows");
        let mut clicked = None;
        for block in &self.list.blocks {
            let sec = &palette.sections[block.section];
            let group = Node::new(
                rows_id.with(("section", block.section)),
                Role::Group,
                sec.title,
                Rect::new(vp.x, vp.y + block.y, vp.w, block.h),
            );
            ui.node_container(group, |ui| {
                let style = Style {
                    weight: 600,
                    ..Style::ui(t.font_s, t.fg_tertiary)
                };
                let head = ui
                    .painter()
                    .elide(&Spec::plain(sec.title, style), w - 2.0 * HEADER_INSET);
                ui.painter().text(&head, x + HEADER_INSET, vp.y + block.y);
                for e in &self.list.entries[block.entries.clone()] {
                    let rect = Rect::new(x, vp.y + e.y, w, e.h);
                    let is_selected = selected == Some(e.key);
                    let hit = match e.row {
                        Some(ri) => self.row(ui, rows_id, rect, &sec.rows[ri], is_selected),
                        None => self.more(ui, rows_id, rect, e.section, is_selected),
                    };
                    if hit {
                        clicked = Some(e.key);
                    }
                }
            });
        }
        clicked
    }

    /// One result row; true when it was clicked.
    fn row(
        &self,
        ui: &mut Ui<'_>,
        rows_id: WidgetId,
        rect: Rect,
        row: &PaletteRow<'_>,
        selected: bool,
    ) -> bool {
        let t = *ui.theme();
        let id = rows_id.with(("row", row.key));
        let r = ui.interact(id, rect, Sense::CLICK);
        if let Some(fill) = row_fill(&t, r.hovered, r.pressed, selected) {
            ui.painter().fill(rect, ROW_RADIUS, fill);
        }
        let mut left = rect.x + ROW_PAD.0;
        let mut right = rect.right() - ROW_PAD.0;
        if let Some(glyph) = row.icon {
            let spec = icon::glyph(glyph, ICON, t.fg_dim);
            ui.painter()
                .text_centered(&spec, Rect::new(left, rect.y, ICON, rect.h));
            left += ICON + ICON_GAP;
        }
        if row.destructive {
            // A 6 px dot in `status_error`, 8 from the text (`axaml:489-496`).
            let dot = Rect::new(right - DOT, rect.y + (rect.h - DOT) / 2.0, DOT, DOT);
            ui.painter().fill(dot, DOT / 2.0, t.status_error);
            right -= DOT + DOT_GAP;
        }
        let top = rect.y + ROW_PAD.1;
        if let Some(hint) = row.hint {
            let spec = Spec::plain(hint, Style::ui(t.font_s, t.fg_dim));
            let (hw, _) = ui.painter().measure(&spec);
            ui.painter().text(
                &spec,
                right - hw,
                top + (self.lines.main - self.lines.small) / 2.0,
            );
            right -= hw + RIGHT_GAP;
        }
        let main = Spec::plain(row.text, Style::ui(t.font_base, t.fg_primary));
        let main = ui.painter().elide(&main, (right - left).max(0.0));
        ui.painter().text(&main, left, top);
        if let Some(detail) = row.detail {
            let spec = Spec::plain(detail, Style::ui(t.font_s, t.fg_secondary));
            let spec = ui
                .painter()
                .elide(&spec, (rect.right() - ROW_PAD.0 - left).max(0.0));
            ui.painter().text(&spec, left, top + self.lines.main);
        }

        let mut node = Node::new(id, Role::ListItem, row.text, rect);
        node.actions.insert(Action::Default);
        if selected {
            node.states.insert(State::Selected);
        }
        node.description = row.detail.map(str::to_owned);
        node.value = row.hint.map(|h| Value::Text(h.to_owned()));
        ui.node(node);
        if row.destructive {
            let tip = ui.tr("luneui.command-palette.destructive-tooltip");
            // Also sets the row's description to the tooltip text.
            ui.tooltip(&r, tip);
        }
        r.clicked
    }

    /// A section's "show more" row; true when it was clicked.
    fn more(
        &self,
        ui: &mut Ui<'_>,
        rows_id: WidgetId,
        rect: Rect,
        section: usize,
        selected: bool,
    ) -> bool {
        let t = *ui.theme();
        let id = rows_id.with(("more", section));
        let r = ui.interact(id, rect, Sense::CLICK);
        if let Some(fill) = row_fill(&t, r.hovered, r.pressed, selected) {
            ui.painter().fill(rect, ROW_RADIUS, fill);
        }
        let label = ui.tr("luneui.command-palette.show-more");
        let spec = Spec::plain(label, Style::ui(t.font_s, t.fg_secondary));
        let spec = ui
            .painter()
            .elide(&spec, (rect.w - 2.0 * ROW_PAD.0).max(0.0));
        ui.painter()
            .text(&spec, rect.x + ROW_PAD.0, rect.y + ROW_PAD.1);
        let mut node = Node::new(id, Role::Button, label, rect);
        node.actions.insert(Action::Default);
        if selected {
            node.states.insert(State::Selected);
        }
        ui.node(node);
        r.clicked
    }

    /// The footer's two columns.
    fn footer(&self, ui: &mut Ui<'_>, t: &Theme) {
        let f = self.geo.footer;
        let style = Style::ui(t.font_s, t.fg_secondary);
        // The error line, when there is one, comes first, so the key line sits under it.
        let y = f.y + FOOTER_PAD[1] + self.geo.error.map_or(0.0, |_| self.lines.small + ERROR_GAP);
        let right = Spec::plain(ui.tr("luneui.command-palette.footer-right"), style);
        let (rw, _) = ui.painter().measure(&right);
        let right_x = f.right() - FOOTER_PAD[2] - rw;
        ui.painter().text(&right, right_x, y);
        if !self.palette.footer_left.is_empty() {
            let left = Spec::plain(self.palette.footer_left, style);
            let room = (right_x - RIGHT_GAP - (f.x + FOOTER_PAD[0])).max(0.0);
            let left = ui.painter().elide(&left, room);
            ui.painter().text(&left, f.x + FOOTER_PAD[0], y);
        }
    }
}

impl<'a> Ui<'a> {
    /// The command palette under `key`; call it every frame it is open. `query` is the search
    /// text (edited in place); `palette` holds the rows the app found for it. Stop calling to
    /// close: the card then plays the dialog's close motion from what it last drew.
    ///
    /// Focus opens in the search box and stays there: Up and Down move the selection (wrapping,
    /// over rows and "show more" rows alike), Enter activates it, a click activates a row, and
    /// Escape or a click on the scrim reports `dismissed`. Tab never leaves the card. A
    /// destructive row reports its activation as any other: the app asks to confirm (a dialog it
    /// opens meanwhile draws above the palette and takes the keys).
    pub fn command_palette(
        &mut self,
        key: impl Hash,
        query: &mut String,
        palette: &Palette<'_>,
        glass: &DialogGlass,
    ) -> PaletteResponse {
        let id = self.id(&key);
        let t = *self.theme();
        let lines = Lines::of(&t);
        let list = List::of(palette, lines);
        let win = self.painter().window;
        let geo = Geometry::of(win, &list, palette.error.is_some(), lines);
        let search_key = (id, "search");
        let search_id = self.id(search_key);
        let mem_id = id.with("mem");
        let body = Body {
            palette,
            list: &list,
            geo,
            lines,
            id,
            search_key,
            search_id,
        };
        let host = ModalHost {
            card: geo.card,
            name: if palette.name.is_empty() {
                palette.placeholder
            } else {
                palette.name
            },
            description: None,
            scrim_blur: false,
            glass,
            initial_focus: Some(search_id),
            default_action: false,
        };
        let drawn: Rc<RefCell<Ops>> = Rc::default();
        let exit_parts = {
            let drawn = Rc::clone(&drawn);
            move |p: &mut Painter<'_, WidgetId>| p.replay(&drawn.borrow())
        };
        let mut mem = *self.transient::<PaletteMem>(mem_id);
        let report = modal_host(self, &key, &host, exit_parts, |ui| {
            let (out, ops) = ui.record(|ui| body.draw(ui, query, &mut mem));
            *drawn.borrow_mut() = ops;
            out
        });
        *self.transient::<PaletteMem>(mem_id) = mem;
        let out = report.content;
        PaletteResponse {
            changed: out.changed,
            activated: match out.act {
                Some(Act::Row(key)) => Some(key),
                _ => None,
            },
            show_more: match out.act {
                Some(Act::More(section)) => Some(section),
                _ => None,
            },
            dismissed: report.scrim_clicked || report.frame.dismissed.is_some(),
        }
    }
}
