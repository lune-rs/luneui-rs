//! Luneium's glass context menu (`ContextMenu.css`, `Popup.css`) and its motion, with the row
//! variants (header, danger, disabled, chevron), the keys and the one highlight they share with
//! the pointer, and anchored placement.

use std::hash::Hash;

use super::popover::{PlaceAlign, PlaceRequest, Side, glass_panel, place};
use super::status_dot;
use super::tone::Tone;
use crate::access::{Action, Node, Role, State};
use crate::color::Rgba;
use crate::icon;
use crate::input::{FocusEvent, Key, NamedKey};
use crate::motion::{Easing, Kind, Motion, Reduce, Token};
use crate::paint::{LayerId, Painter, Rect, Xform};
use crate::text::{Spec, Style};
use crate::ui::{OverlayOptions, OverlayPhase, Response, RoveAxis, Sense, Ui, WidgetId};

/// One row of a menu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuEntry<'s, H> {
    Item {
        glyph: Option<&'s str>,
        text: &'s str,
        shortcut: &'s str,
        hit: H,
    },
    Separator,
}

/// Luneium's menu motion (`Popup.css`): in 0.18 s ease-out from 4 px lower at 97 %; out 0.12 s
/// ease-in to 2 px lower at 98 %. Both about the top-left corner.
pub const MENU_IN: f32 = 0.18;
pub const MENU_OUT: f32 = 0.12;

/// Menu opacity, vertical offset and scale at one moment of its animation.
pub type MenuAnim = (f32, f32, f32);

/// CSS `cubic-bezier(x1, y1, x2, y2)` at time `t`; [`motion::cubic_bezier`](crate::motion::cubic_bezier).
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    crate::motion::cubic_bezier(x1, y1, x2, y2, t)
}

/// Opening motion at `t` = elapsed / [`MENU_IN`].
pub fn menu_open_anim(t: f32) -> MenuAnim {
    let e = cubic_bezier(0.215, 0.61, 0.355, 1.0, t);
    (e, 4.0 * (1.0 - e), 0.97 + 0.03 * e)
}

/// Closing motion at `t` = elapsed / [`MENU_OUT`]; `None` once it has finished (`t` ≥ 1).
pub fn menu_close_anim(t: f32) -> Option<MenuAnim> {
    if t >= 1.0 {
        return None;
    }
    let e = cubic_bezier(0.42, 0.0, 1.0, 1.0, t);
    Some((1.0 - e, 2.0 * e, 1.0 - 0.02 * e))
}

/// An item row with its variants, for [`MenuRow::Entry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuItem<'s> {
    pub glyph: Option<&'s str>,
    pub text: &'s str,
    /// The hint at the row's right end; a `chevron` takes its place.
    pub shortcut: &'s str,
    /// Not picked, skipped by the keys and never lit; it still blocks what is under it.
    pub disabled: bool,
    /// Text and glyph in the error colour.
    pub danger: bool,
    /// A hint that the row leads further. It opens nothing: the app acts on the pick.
    pub chevron: bool,
}

/// One row of a menu as the layout and the paint see it (no hit value).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuRow<'s> {
    /// An [`Self::Entry`] with its flags off.
    Item {
        glyph: Option<&'s str>,
        text: &'s str,
        shortcut: &'s str,
    },
    Entry(MenuItem<'s>),
    /// Two lines of text, the first behind a status dot; no hit target and no stop.
    Header {
        name: &'s str,
        status: &'s str,
        tone: Tone,
    },
    Separator,
}

impl<'s> MenuRow<'s> {
    /// The item this row is, if it is one.
    fn item(&self) -> Option<MenuItem<'s>> {
        match *self {
            MenuRow::Item {
                glyph,
                text,
                shortcut,
            } => Some(MenuItem {
                glyph,
                text,
                shortcut,
                ..MenuItem::default()
            }),
            MenuRow::Entry(item) => Some(item),
            MenuRow::Header { .. } | MenuRow::Separator => None,
        }
    }

    pub(crate) fn height(&self) -> f32 {
        match self {
            MenuRow::Item { .. } | MenuRow::Entry(_) => ROW,
            MenuRow::Header { .. } => HEADER,
            MenuRow::Separator => SEP,
        }
    }
}

impl<'s, H> From<&MenuEntry<'s, H>> for MenuRow<'s> {
    fn from(e: &MenuEntry<'s, H>) -> Self {
        match e {
            MenuEntry::Item {
                glyph,
                text,
                shortcut,
                ..
            } => MenuRow::Item {
                glyph: *glyph,
                text,
                shortcut,
            },
            MenuEntry::Separator => MenuRow::Separator,
        }
    }
}

/// The inset of a menu's rows from its panel.
pub(crate) const PAD: f32 = 6.0;
const ROW: f32 = 28.0;
const SEP: f32 = 11.0;
/// A header row's height: 6 px of padding around a 20 px and a 16 px line.
pub const HEADER: f32 = 48.0;
const HEADER_PAD: f32 = 6.0;
const HEADER_NAME_LINE: f32 = 20.0;
const HEADER_STATUS_LINE: f32 = 16.0;
/// The side of a chevron's glyph.
const CHEVRON: f32 = 12.0;

/// How one row is drawn this frame; the default draws it at rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RowLook {
    /// The highlight's fill at this moment; no alpha draws none.
    pub hover: Rgba,
    /// The pointer holds the row: the pressed fill replaces `hover`.
    pub pressed: bool,
}

/// Where a menu opens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuPlace {
    /// Its top left at a point, as a context menu at the pointer.
    At((f32, f32)),
    /// Next to `rect`, in window coordinates, through [`place`].
    Anchor {
        rect: Rect,
        side: Side,
        align: PlaceAlign,
        gap: f32,
    },
}

/// What [`Ui::menu_with`] needs besides its rows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuOptions<'s> {
    pub place: MenuPlace,
    /// The widget focus returns to on close when the one focused at open is gone.
    pub anchor_id: Option<WidgetId>,
    /// The menu's accessible name; empty uses `luneui.menu.name`.
    pub name: &'s str,
}

/// Where a menu's panel and rows are.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuLayout {
    pub panel: Rect,
    /// One rect per row, in order: an item's 28 px hover and hit rect, or a separator's 11 px strip.
    pub rows: Vec<Rect>,
}

/// The header's name and status specs.
fn header_specs(theme: &crate::theme::Theme, name: &str, status: &str) -> (Spec, Spec) {
    (
        Spec::plain(name, Style::ui(12.0, theme.menu_text_primary).medium()),
        Spec::plain(status, Style::ui(theme.font_s, theme.menu_text_tertiary)),
    )
}

/// Sizes a menu of `rows` and places it at `at`, kept inside the window (flipped above `at` near
/// the bottom edge).
pub fn menu_layout<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    rows: &[MenuRow<'_>],
    at: (f32, f32),
) -> MenuLayout {
    menu_layout_at(p, rows, &MenuPlace::At(at))
}

/// Sizes a menu of `rows` and places it through [`place`] as `at` says, kept inside the window.
pub fn menu_layout_at<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    rows: &[MenuRow<'_>],
    at: &MenuPlace,
) -> MenuLayout {
    let t = p.theme;
    let label_style = Style::ui(12.0, t.menu_text_primary);
    let hint_style = Style::ui(t.font_s, t.menu_text_quaternary);
    let mut w: f32 = 190.0;
    let mut h = 2.0 * PAD;
    for e in rows {
        h += e.height();
        if let Some(item) = e.item() {
            let tw = p.measure(&Spec::plain(item.text, label_style)).0;
            let sw = if item.chevron {
                16.0 + CHEVRON
            } else if item.shortcut.is_empty() {
                0.0
            } else {
                16.0 + p.measure(&Spec::plain(item.shortcut, hint_style)).0
            };
            w = w.max(2.0 * PAD + 8.0 + 14.0 + 8.0 + tw + sw + 8.0);
        } else if let MenuRow::Header { name, status, .. } = e {
            let (name, status) = header_specs(&t, name, status);
            let nw = status_dot::SIZE + status_dot::GAP + p.measure(&name).0;
            let content = nw.max(p.measure(&status).0);
            w = w.max(2.0 * PAD + 8.0 + content + 8.0);
        }
    }
    let (anchor, side, align, gap) = match *at {
        MenuPlace::At((x, y)) => (
            Rect::new(x, y, 0.0, 0.0),
            Side::Bottom,
            PlaceAlign::Start,
            0.0,
        ),
        MenuPlace::Anchor {
            rect,
            side,
            align,
            gap,
        } => (rect, side, align, gap),
    };
    let panel = place(&PlaceRequest {
        anchor,
        size: (w, h),
        side,
        align,
        gap,
        margin: 8.0,
        shift: true,
        window: p.window,
    })
    .rect;
    let mut cy = panel.y + PAD;
    let rects = rows
        .iter()
        .map(|e| {
            let r = Rect::new(panel.x + PAD, cy, panel.w - 2.0 * PAD, e.height());
            cy += e.height();
            r
        })
        .collect();
    MenuLayout { panel, rows: rects }
}

/// Draws one item in `r`.
fn paint_item<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    item: &MenuItem<'_>,
    r: Rect,
    look: RowLook,
) {
    let t = p.theme;
    if look.pressed {
        p.fill(r, 4.0, t.menu_item_pressed);
    } else if look.hover[3] > 0 {
        p.fill(r, 4.0, look.hover);
    }
    let (label, glyph, hint, chevron) = if item.disabled {
        let c = t.menu_text_disabled;
        (c, c, c, c)
    } else if item.danger {
        (t.err, t.err, t.menu_text_quaternary, t.menu_text_tertiary)
    } else {
        (
            t.menu_text_primary,
            t.menu_text_tertiary,
            t.menu_text_quaternary,
            t.menu_text_tertiary,
        )
    };
    let icon_slot = Rect::new(r.x + 8.0, r.y, 14.0, r.h);
    if let Some(g) = item.glyph {
        p.text_centered(&icon::glyph(g, 13.0, glyph), icon_slot);
    }
    p.text_in(
        &Spec::plain(item.text, Style::ui(12.0, label)),
        Rect::new(icon_slot.right() + 8.0, r.y, 0.0, r.h),
        0.0,
    );
    if item.chevron {
        let slot = Rect::new(r.right() - 8.0 - CHEVRON, r.y, CHEVRON, r.h);
        p.text_centered(&icon::glyph(icon::CHEVRON_RIGHT, CHEVRON, chevron), slot);
    } else if !item.shortcut.is_empty() {
        let spec = Spec::plain(item.shortcut, Style::ui(t.font_s, hint));
        let sw = p.measure(&spec).0;
        p.text_in(&spec, Rect::new(r.right() - 8.0 - sw, r.y, sw, r.h), 0.0);
    }
}

/// Draws a header in `r`: the dot and the name, the status line under them.
fn paint_header<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    name: &str,
    status: &str,
    tone: Tone,
) {
    let t = p.theme;
    let (name, status) = header_specs(&t, name, status);
    let x = r.x + 8.0;
    let line = Rect::new(x, r.y + HEADER_PAD, 0.0, HEADER_NAME_LINE);
    status_dot::paint(p, (x + status_dot::SIZE / 2.0, line.y + line.h / 2.0), tone);
    let text_x = x + status_dot::SIZE + status_dot::GAP;
    p.text_in(&name, Rect::new(text_x, line.y, 0.0, line.h), 0.0);
    let below = Rect::new(x, line.y + line.h, 0.0, HEADER_STATUS_LINE);
    p.text_in(&status, below, 0.0);
}

/// Draws a separator row `r` of a menu whose panel is `panel`.
pub(crate) fn paint_separator<H: Copy + PartialEq>(p: &mut Painter<'_, H>, panel: Rect, r: Rect) {
    let t = p.theme;
    p.fill(
        Rect::new(
            panel.x + PAD + 2.0,
            r.y + 5.0,
            panel.w - 2.0 * PAD - 4.0,
            1.0,
        ),
        0.0,
        t.menu_separator,
    );
}

/// Draws the rows of a menu whose panel is `panel`, at `rects` (the surface is the caller's).
/// `looks[i]` is row `i`'s look (a missing entry is the look at rest).
pub(crate) fn menu_paint_rows<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    rows: &[MenuRow<'_>],
    panel: Rect,
    rects: &[Rect],
    looks: &[RowLook],
) {
    for (i, (e, r)) in rows.iter().zip(rects).enumerate() {
        match e {
            MenuRow::Item { .. } | MenuRow::Entry(_) => {
                if let Some(item) = e.item() {
                    let look = looks.get(i).copied().unwrap_or_default();
                    paint_item(p, &item, *r, look);
                }
            }
            MenuRow::Header { name, status, tone } => paint_header(p, *r, name, status, *tone),
            MenuRow::Separator => paint_separator(p, panel, *r),
        }
    }
}

/// Draws the glass panel and its rows, untransformed and without hit regions. `looks[i]` is row
/// `i`'s look (a missing entry is the look at rest).
pub fn menu_paint<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    rows: &[MenuRow<'_>],
    layout: &MenuLayout,
    looks: &[RowLook],
) {
    glass_panel(p, layout.panel);
    menu_paint_rows(p, rows, layout.panel, &layout.rows, looks);
}

/// A glass menu at `at` (kept inside the window), the content under it blurred (30 px,
/// saturate 1.8), black 45 % over grey 165 @ 30 %, radius 9, inset 1 px ring, 6 px padding,
/// 28 px rows with radius 4, 12 px labels, 14 px icon slot, 11 px hints. `interactive` is off
/// while the menu fades out.
pub fn menu<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    entries: &[MenuEntry<'_, H>],
    at: (f32, f32),
    anim: MenuAnim,
    interactive: bool,
) {
    let rows: Vec<MenuRow<'_>> = entries.iter().map(MenuRow::from).collect();
    let layout = menu_layout(p, &rows, at);
    let hover = p.theme.menu_item_hover;
    let looks: Vec<RowLook> = entries
        .iter()
        .map(|e| match e {
            MenuEntry::Item { hit, .. } if interactive && p.hovered(*hit) => RowLook {
                hover,
                pressed: false,
            },
            _ => RowLook::default(),
        })
        .collect();
    let (alpha, dy, scale) = anim;
    p.set_xform(Some(Xform {
        origin: (layout.panel.x, layout.panel.y),
        scale,
        dx: 0.0,
        dy,
        alpha,
    }));
    menu_paint(p, &rows, &layout, &looks);
    if interactive {
        for (e, r) in entries.iter().zip(&layout.rows) {
            if let MenuEntry::Item { hit, .. } = e {
                p.hit(*r, *hit);
            }
        }
    }
    p.set_xform(None);
}

/// Open: pop, fast, cubic-out; Reduced fades opacity only, still at Fast (menu.md's override).
pub const OPEN: Motion = Motion {
    kind: Kind::Pop,
    token: Token::Fast,
    easing: Easing::CUBIC_OUT,
    reduced: Reduce::Fade,
    reduced_token: Some(Token::Fast),
};
/// Close: dismiss, instant (the moved 120 ms), ease-in; Reduced fades opacity only.
pub const CLOSE: Motion = Motion {
    kind: Kind::Dismiss,
    token: Token::Instant,
    easing: Easing::EASE_IN,
    reduced: Reduce::Fade,
    reduced_token: Some(Token::Instant),
};
/// A row's fill as the pointer enters and leaves it; a key moves the highlight at once.
pub const HOVER: Motion = Motion::of(Kind::Hover);

/// What a menu frame reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuResponse {
    /// The index in `rows` of the item picked this frame.
    pub picked: Option<usize>,
    /// The Ui asks the app to close the menu (Escape, an outside press, an accessibility dismiss).
    pub dismissed: bool,
}

/// An owned [`MenuRow`], for the exit painter.
#[derive(Debug, Clone)]
enum OwnedRow {
    Item {
        glyph: Option<String>,
        text: String,
        shortcut: String,
        disabled: bool,
        danger: bool,
        chevron: bool,
    },
    Header {
        name: String,
        status: String,
        tone: Tone,
    },
    Separator,
}

impl OwnedRow {
    fn of(r: &MenuRow<'_>) -> OwnedRow {
        match (r, r.item()) {
            (_, Some(item)) => OwnedRow::Item {
                glyph: item.glyph.map(str::to_owned),
                text: item.text.to_owned(),
                shortcut: item.shortcut.to_owned(),
                disabled: item.disabled,
                danger: item.danger,
                chevron: item.chevron,
            },
            (MenuRow::Header { name, status, tone }, _) => OwnedRow::Header {
                name: (*name).to_owned(),
                status: (*status).to_owned(),
                tone: *tone,
            },
            _ => OwnedRow::Separator,
        }
    }

    fn borrow(&self) -> MenuRow<'_> {
        match self {
            OwnedRow::Item {
                glyph,
                text,
                shortcut,
                disabled,
                danger,
                chevron,
            } => MenuRow::Entry(MenuItem {
                glyph: glyph.as_deref(),
                text,
                shortcut,
                disabled: *disabled,
                danger: *danger,
                chevron: *chevron,
            }),
            OwnedRow::Header { name, status, tone } => MenuRow::Header {
                name,
                status,
                tone: *tone,
            },
            OwnedRow::Separator => MenuRow::Separator,
        }
    }
}

/// The transform of a menu at one moment: the moved curves, or opacity only when `transforms`
/// is off (the Reduced fade).
pub(crate) fn xform_of(anim: MenuAnim, origin: (f32, f32), transforms: bool) -> Xform {
    let (alpha, dy, scale) = anim;
    if transforms {
        Xform {
            origin,
            scale,
            dx: 0.0,
            dy,
            alpha,
        }
    } else {
        Xform {
            origin,
            scale: 1.0,
            dx: 0.0,
            dy: 0.0,
            alpha,
        }
    }
}

/// A menu's highlight between frames.
#[derive(Debug, Clone, Copy, Default)]
struct RowsMem {
    /// The row the last input lit; never a disabled one.
    lit: Option<usize>,
    /// The last input to move it was a key, so the pointer leaving a row puts nothing out.
    keyboard: bool,
    /// The enabled row under the pointer last frame: only a change of it is an input, so a pointer
    /// resting on a row does not relight it.
    hovered: Option<usize>,
    /// Focus was on a row last frame.
    focus_in: bool,
}

/// One frame of a menu's rows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RowsFrame {
    /// The row picked this frame.
    pub(crate) picked: Option<usize>,
    /// One look per row, for [`menu_paint`].
    pub(crate) looks: Vec<RowLook>,
}

/// The enabled row `key` reaches from `from`, wrapping; with nothing lit the arrows start at the
/// ends. `items` has one entry per row.
fn next_lit(items: &[(WidgetId, bool)], from: Option<usize>, key: NamedKey) -> Option<usize> {
    let n = items.len();
    let on = |i: &usize| items[*i].1;
    match (key, from) {
        (NamedKey::Home, _) | (NamedKey::ArrowDown, None) => (0..n).find(on),
        (NamedKey::End, _) | (NamedKey::ArrowUp, None) => (0..n).rev().find(on),
        (NamedKey::ArrowDown, Some(at)) => (1..n).map(|d| (at + d) % n).find(on),
        (NamedKey::ArrowUp, Some(at)) => (1..n).map(|d| (at + n - d) % n).find(on),
        _ => None,
    }
}

/// Registers the rows of the menu `id` at `rects` and their nodes, and runs the highlight: at most
/// one row is lit, the row of the last input (the pointer entering or leaving one, a press, an
/// arrow, Home or End, a keyboard open). A row disabled while lit puts it out. Up and Down wrap,
/// Enter and Space pick the lit row, else the focused one. Rows rove: one tab stop. The caller
/// draws [`RowsFrame::looks`].
pub(crate) fn menu_rows(
    ui: &mut Ui<'_>,
    id: WidgetId,
    rows: &[MenuRow<'_>],
    rects: &[Rect],
) -> RowsFrame {
    let t = *ui.theme();
    let group_on = ui.is_enabled();
    // One entry per row, so an index here is a row's index; only an enabled item is `true`.
    let items: Vec<(WidgetId, bool)> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let on = row.item().is_some_and(|item| !item.disabled);
            (id.with(i), on && group_on)
        })
        .collect();
    let stop = ui.rove_stop(&items, None);
    let mut responses: Vec<Option<Response>> = Vec::with_capacity(rows.len());
    for (i, (row, r)) in rows.iter().zip(rects).enumerate() {
        let row_id = items[i].0;
        let Some(item) = row.item() else {
            if let MenuRow::Header { name, status, .. } = row {
                let mut node = Node::new(row_id, Role::StaticText, name, *r);
                node.description = (!status.is_empty()).then(|| (*status).to_owned());
                ui.node(node);
            }
            responses.push(None);
            continue;
        };
        let sense = match (items[i].1, stop == Some(i)) {
            (false, _) => Sense::HOVER,
            (true, true) => Sense::CLICK_FOCUS,
            (true, false) => Sense::CLICK_ROVE,
        };
        let resp = ui.interact(row_id, *r, sense);
        let mut node = Node::new(row_id, Role::MenuItem, item.text, *r);
        if !item.shortcut.is_empty() {
            node.description = Some(item.shortcut.to_owned());
        }
        if item.disabled {
            node.states.insert(State::Disabled);
        } else {
            node.states.insert(State::Focusable);
            node.actions.insert(Action::Default);
        }
        ui.node(node);
        // A disabled row's response can still carry an accessibility action's click.
        responses.push(items[i].1.then_some(resp));
    }
    let find =
        |test: fn(&Response) -> bool| responses.iter().position(|r| r.as_ref().is_some_and(test));
    let hovered = find(|r| r.hovered);
    let held = find(|r| r.pressed);
    let press = find(|r| r.press_started);
    let clicked = find(|r| r.clicked);

    let (focused, ring) = ui.focus_state();
    let focused_row = focused.and_then(|f| items.iter().position(|c| c.0 == f));
    let mut m = *ui.transient::<RowsMem>(id);
    // Focus arriving with the ring is a keyboard open: its row is lit, and a pointer already
    // resting on another row has not moved.
    if focused_row.is_some() && !m.focus_in && ring {
        m.lit = focused_row;
        m.keyboard = true;
        m.hovered = hovered;
    }
    m.focus_in = focused_row.is_some();
    if hovered != m.hovered {
        m.hovered = hovered;
        if hovered.is_some() {
            m.lit = hovered;
            m.keyboard = false;
        } else if !m.keyboard {
            m.lit = None;
        }
    }
    if press.is_some() {
        m.lit = press;
        m.keyboard = false;
    }
    m.lit = m.lit.filter(|i| items.get(*i).is_some_and(|c| c.1));

    // The keys come to the focused row, but move from the lit one: focus can sit on the first row
    // with nothing lit, and the first Down must light that row, not the second. The lit row
    // changes here, not when the focus move lands, so it is lit the frame the key arrives.
    let mut picked = clicked;
    if let Some(row) = focused_row {
        let row_id = items[row].0;
        let mut moved = None;
        for (index, event) in ui.focus_events(row_id) {
            let FocusEvent::Key {
                key: Key::Named(key),
                modifiers,
                ..
            } = event
            else {
                continue;
            };
            let chord = modifiers.control || modifiers.alt || modifiers.logo;
            let moves = matches!(
                key,
                NamedKey::ArrowDown | NamedKey::ArrowUp | NamedKey::Home | NamedKey::End
            );
            if moves && !chord {
                ui.consume_focus_event(index);
                if let Some(to) = next_lit(&items, m.lit, key).or(m.lit) {
                    m.lit = Some(to);
                    m.keyboard = true;
                    moved = Some(to);
                }
            }
        }
        if let Some(to) = moved {
            ui.rove_focus(items[to].0);
        }
        let target = m.lit.or(focused_row);
        let enabled = target.is_some_and(|i| items[i].1);
        if ui.activation_keys(row_id, enabled, &[NamedKey::Enter, NamedKey::Space]) {
            picked = picked.or(target);
        }
    }
    // The keys above are taken, so this only names the group for the Tab order.
    ui.rove_keys(&items, RoveAxis::Vertical, true);
    *ui.transient::<RowsMem>(id) = m;

    let [r, g, b, _] = t.menu_item_hover;
    let looks = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            if row.item().is_none() {
                return RowLook::default();
            }
            let lit = m.lit == Some(i);
            let target = if lit { t.menu_item_hover } else { [r, g, b, 0] };
            let faded = ui.animate_color(items[i].0.with("fill"), target, &HOVER);
            RowLook {
                // A key moves the highlight at once, so one row shows it the frame the key
                // arrives; the fade is the pointer's. A disabled row has no fill.
                hover: if m.keyboard || !items[i].1 {
                    target
                } else {
                    faded
                },
                // Held with the pointer off it, nothing is lit and the press still shows.
                pressed: held == Some(i) && (lit || m.lit.is_none()),
            }
        })
        .collect();
    RowsFrame { picked, looks }
}

impl<'a> Ui<'a> {
    /// A glass menu of `rows` at `at` on the popup layer: [`Self::menu_with`] with no anchor and
    /// the default name.
    pub fn menu(&mut self, key: impl Hash, rows: &[MenuRow<'_>], at: (f32, f32)) -> MenuResponse {
        let options = MenuOptions {
            place: MenuPlace::At(at),
            anchor_id: None,
            name: "",
        };
        self.menu_with(key, rows, &options)
    }

    /// A glass menu of `rows` on the popup layer, placed as `options` says, non-modal: Escape and
    /// a press outside dismiss it (`dismissed`), and that press reaches nothing, so a press on the
    /// trigger closes without reopening. A click on an item, or Enter or Space on the lit item
    /// (else the focused one), picks it (`picked`); Up, Down, Home and End move the highlight over
    /// the enabled items. Call it every frame it is open; stop calling to close it, and it fades
    /// out through its exit painter. The look follows D7: opening at progress `p` is
    /// [`menu_open_anim`]`(p)`, open is `menu_open_anim(1.0)`, closing at `p` is
    /// [`menu_close_anim`]`(p)`.
    pub fn menu_with(
        &mut self,
        key: impl Hash,
        rows: &[MenuRow<'_>],
        options: &MenuOptions<'_>,
    ) -> MenuResponse {
        let layout = menu_layout_at(self.painter(), rows, &options.place);
        let name = match options.name {
            "" => self.tr("luneui.menu.name"),
            name => name,
        };
        let overlay = OverlayOptions {
            layer: LayerId::Popup,
            modal: false,
            dismiss_on_escape: true,
            dismiss_on_outside_press: true,
            dismiss_on_focus_loss: false,
            anchor: options.anchor_id,
            initial_focus: None,
            open: OPEN,
            close: CLOSE,
        };
        let owned: Vec<OwnedRow> = rows.iter().map(OwnedRow::of).collect();
        let exit_layout = layout.clone();
        let exit = move |p: &mut Painter<'_, WidgetId>, progress: f32, transforms: bool| {
            let Some(anim) = menu_close_anim(progress) else {
                return;
            };
            let rows: Vec<MenuRow<'_>> = owned.iter().map(OwnedRow::borrow).collect();
            let origin = (exit_layout.panel.x, exit_layout.panel.y);
            p.push_xform(xform_of(anim, origin, transforms));
            // No looks: the fill snaps off as the menu leaves.
            menu_paint(p, &rows, &exit_layout, &[]);
            p.pop_xform();
        };
        let (frame, picked) = self.overlay(&key, &overlay, exit, |ui, frame| {
            let origin = (layout.panel.x, layout.panel.y);
            let anim = match frame.phase {
                OverlayPhase::Opening(p) => menu_open_anim(p),
                OverlayPhase::Open => menu_open_anim(1.0),
            };
            let menu_id = ui.id(&key);
            let mut menu_node = Node::new(menu_id, Role::Menu, name, layout.panel);
            menu_node.states.insert(State::Expanded);
            let rows_frame = ui.node_container(menu_node, |ui| {
                ui.painter()
                    .push_xform(xform_of(anim, origin, frame.transforms));
                // The panel blocks what is under it, so a press on a separator or the padding is
                // not an outside press.
                ui.interact(menu_id.with("panel"), layout.panel, Sense::HOVER);
                menu_rows(ui, menu_id, rows, &layout.rows)
            });
            let p = ui.painter();
            menu_paint(p, rows, &layout, &rows_frame.looks);
            p.pop_xform();
            rows_frame.picked
        });
        MenuResponse {
            picked,
            dismissed: frame.dismissed.is_some(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubic_bezier_endpoints() {
        for (x1, y1, x2, y2) in [(0.215, 0.61, 0.355, 1.0), (0.42, 0.0, 1.0, 1.0)] {
            assert!(cubic_bezier(x1, y1, x2, y2, -1.0).abs() < 1e-5);
            assert!(cubic_bezier(x1, y1, x2, y2, 0.0).abs() < 1e-5);
            assert!((cubic_bezier(x1, y1, x2, y2, 1.0) - 1.0).abs() < 1e-5);
            assert!((cubic_bezier(x1, y1, x2, y2, 2.0) - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn open_curve_ends_at_rest() {
        let (alpha, dy, scale) = menu_open_anim(1.0);
        assert!((alpha - 1.0).abs() < 1e-5);
        assert!(dy.abs() < 1e-4);
        assert!((scale - 1.0).abs() < 1e-5);
    }

    #[test]
    fn close_curve_ends_at_none() {
        assert!(menu_close_anim(1.0).is_none());
        assert!(menu_close_anim(3.0).is_none());
        let (alpha, ..) = menu_close_anim(0.0).expect("closing at t=0");
        assert!((alpha - 1.0).abs() < 1e-5);
    }
}
