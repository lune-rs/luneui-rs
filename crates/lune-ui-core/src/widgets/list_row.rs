//! A list row with a trailing badge, drawn inside the virtual list (`widget.list-row-badge`).
//!
//! The list owns selection, keys, the highlight fill and the focus ring; [`Ui::list_row`] draws
//! the content of one row from inside the list's `draw`: a head line (an optional status dot, the
//! name in 13 Medium on a 20 px line, an optional badge, then the value, mono 13, at the right
//! edge, 8 px apart), a sub-line (mono 11 on a 16 px line) and the meta (11 on a 16 px line), 4 px
//! between lines. The name is `fg_secondary` at rest and `fg_bright` once selected, with the
//! `selection` motion; a row never animates while the list scrolls. [`paint`] draws the same
//! content for a specimen.

use crate::access::{Node, Role, Value};
use crate::color::{Rgba, mix};
use crate::motion::{Kind, Motion};
use crate::paint::{Painter, Rect};
use crate::text::{Spec, Style};
use crate::ui::Ui;

use super::badge;
use super::status_dot;
use super::tone::Tone;
use super::virtual_list::{HIGHLIGHT_INSET, RowCtx};

/// The name's colour between unselected and selected.
pub const SELECTION: Motion = Motion::of(Kind::Selection);

/// The side-panel list's padding: horizontal, vertical.
pub const SIDE_PANEL_PADDING: (f32, f32) = (8.0, 6.0);
/// A master list's padding, wider than the side panel's.
pub const MASTER_PADDING: (f32, f32) = (12.0, 6.0);
/// The head line (name, badge and value), and the small lines.
pub const NAME_LINE: f32 = 20.0;
pub const SMALL_LINE: f32 = 16.0;
/// Between the lines of a row.
pub const LINE_GAP: f32 = 4.0;
/// Between the dot, the name, the badge and the value.
pub const BADGE_GAP: f32 = 8.0;

/// What a list row shows.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ListRow<'a> {
    pub name: &'a str,
    pub value: Option<&'a str>,
    pub sub: Option<&'a str>,
    pub meta: Option<&'a str>,
    pub badge: Option<(&'a str, Tone)>,
    /// A status dot before the name.
    pub dot: Option<Tone>,
}

/// The height of a row slot with `padding` (what `Heights::Fixed` wants): the highlight's inset,
/// the padding and the lines.
pub fn height(row: &ListRow<'_>, padding: (f32, f32)) -> f32 {
    let mut lines = vec![NAME_LINE];
    if row.sub.is_some() {
        lines.push(SMALL_LINE);
    }
    if row.meta.is_some() {
        lines.push(SMALL_LINE);
    }
    let content: f32 = lines.iter().sum::<f32>() + LINE_GAP * (lines.len() - 1) as f32;
    2.0 * HIGHLIGHT_INSET.1 + 2.0 * padding.1 + content
}

/// Draws the content of a row in the slot `rect` with the name in `name_color`; returns the
/// slot's height. No fill and no ring: those are the list's (a specimen draws them itself).
pub fn paint<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    rect: Rect,
    row: &ListRow<'_>,
    padding: (f32, f32),
    name_color: Rgba,
) -> f32 {
    let t = p.theme;
    let x = rect.x + HIGHLIGHT_INSET.0 + padding.0;
    let w = (rect.w - 2.0 * (HIGHLIGHT_INSET.0 + padding.0)).max(0.0);
    let mut y = rect.y + HIGHLIGHT_INSET.1 + padding.1;

    // The head: dot, name, badge, value at the right edge.
    let badge_w = row
        .badge
        .map_or(0.0, |(text, _)| badge::width(p, text, false));
    let lead = if row.dot.is_some() {
        status_dot::SIZE + status_dot::GAP
    } else {
        0.0
    };
    if let Some(tone) = row.dot {
        status_dot::paint(p, (x + status_dot::SIZE / 2.0, y + NAME_LINE / 2.0), tone);
    }
    let value = row.value.map(|v| {
        let style = Style::mono(t.font_base, NAME_LINE, t.fg_secondary);
        p.elide(&Spec::plain(v, style), (w - lead).max(0.0))
    });
    let value_w = value.as_ref().map_or(0.0, |v| p.measure(v).0);
    let side = |w: f32| if w > 0.0 { w + BADGE_GAP } else { 0.0 };
    let name_style = Style {
        line: NAME_LINE,
        ..Style::ui(t.font_base, name_color).medium()
    };
    let name = p.elide(
        &Spec::plain(row.name, name_style),
        (w - lead - side(badge_w) - side(value_w)).max(0.0),
    );
    p.text(&name, x + lead, y);
    let mut right = x + w;
    if let Some(v) = &value {
        right -= value_w;
        p.text(v, right, y);
        right -= BADGE_GAP;
    }
    if let Some((text, tone)) = row.badge {
        badge::paint(
            p,
            (right - badge_w, y + (NAME_LINE - badge::HEIGHT) / 2.0),
            text,
            tone,
            false,
        );
    }
    y += NAME_LINE + LINE_GAP;

    // The sub-line, then the meta.
    if let Some(s) = row.sub {
        let style = Style::mono(t.font_s, SMALL_LINE, t.fg_tertiary);
        let spec = p.elide(&Spec::plain(s, style), w);
        p.text(&spec, x, y);
        y += SMALL_LINE + LINE_GAP;
    }
    if let Some(m) = row.meta {
        let style = Style {
            line: SMALL_LINE,
            ..Style::ui(t.font_s, t.fg_tertiary)
        };
        let spec = p.elide(&Spec::plain(m, style), w);
        p.text(&spec, x, y);
    }
    height(row, padding)
}

/// What a row remembers between frames: where it was drawn.
#[derive(Debug, Default)]
struct Mem {
    y: Option<f32>,
}

impl Ui<'_> {
    /// The content of `row` inside the virtual list's `draw` for `ctx`; returns the height to
    /// report. The list has painted the fill and the ring and made the `ListItem` node; this adds
    /// the name colour (`selection` motion, snapped while the row moves, which is while the list
    /// scrolls) and the row's `StaticText` nodes for the name and the value.
    pub fn list_row(&mut self, ctx: &RowCtx, row: &ListRow<'_>, padding: (f32, f32)) -> f32 {
        let t = *self.theme();
        let id = self.id("name");
        let moved = {
            let mem = self.transient::<Mem>(id);
            let moved = mem.y.is_some_and(|y| y != ctx.rect.y);
            mem.y = Some(ctx.rect.y);
            moved
        };
        let want = if ctx.selected { 1.0 } else { 0.0 };
        let amount = if moved {
            self.snap_f32(id, want)
        } else {
            self.animate_f32(id, want, &SELECTION)
        };
        let color = mix(t.fg_secondary, t.fg_bright, amount);
        let h = paint(self.painter(), ctx.rect, row, padding, color);

        let mut name = Node::new(id.with("text"), Role::StaticText, row.name, ctx.rect);
        name.value = Some(Value::Text(row.name.to_owned()));
        self.node(name);
        if let Some(v) = row.value {
            let mut value = Node::new(id.with("value"), Role::StaticText, row.name, ctx.rect);
            value.value = Some(Value::Text(v.to_owned()));
            self.node(value);
        }
        h
    }
}
