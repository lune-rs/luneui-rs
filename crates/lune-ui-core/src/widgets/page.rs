//! The page card (`layout.page`, D148): header, a vertically scrolling body and an optional pinned
//! footer, with a keyed swap of the body.
//!
//! [`Ui::page`] draws the card in the space it is given (margin 0 10 10 6 applied inside), then runs
//! one closure per slot, in tab order: the header actions, the body, the footer. Like the toolbar
//! it takes the glass layer from the caller and draws it whatever the window's state is (R13): the
//! widget names no preset.
//!
//! The footer is fixed at [`FOOTER_H`] (12 + a 32 px button row + 12) so the body's room is known
//! before the closure runs; its closure's return is ignored. A page is a container, so a disabled
//! page is `with_enabled(false)` around the call, as for any group.
//!
//! The body is a [`scroll_area`](super::scroll_area) clipped to the card's rounded rect
//! ([`Painter::push_clip_rounded`](crate::paint::Painter::push_clip_rounded)). Text cannot be cut
//! by a rounded clip (it takes the bounding rect), so the card keeps it out of the corners (D12):
//! its own title, subtitle and padded body are inset by 16, which is more than the radius of 8.
//!
//! A change of [`PageOptions::swap_key`] swaps the body: it fades in ([`SWAP_OPACITY`]) while it
//! slides up [`SWAP_DY`] px ([`SWAP_SLIDE`], dropped under Reduced). The card, the header and the
//! footer stay put. The first frame of a page does not animate; a swap in flight restarts from
//! the look it has. A new key also puts the body back at the top.

use std::hash::Hash;

use crate::access::{Node, Role};
use crate::backdrop::GlassLayer;
use crate::motion::{Easing, Kind, Motion, Reduce};
use crate::paint::{Rect, Xform};
use crate::text::{Spec, Style};
use crate::ui::{Ui, WidgetId};

use super::scroll_area::{Bars, Extent, ScrollOptions, ScrollResponse, ScrollView};

/// The card's margin inside the rect it is given: top, right, bottom, left.
pub const MARGIN: (f32, f32, f32, f32) = (0.0, 10.0, 10.0, 6.0);
/// The card's corner radius.
pub const RADIUS: f32 = 8.0;
/// The padding of the header and the footer's sides, and of the body's sides and bottom.
pub const PAD: f32 = 16.0;
/// The title's line (its size is the theme's `font_title`) and the subtitle's.
pub const TITLE_LINE: f32 = 24.0;
pub const SUBTITLE_LINE: f32 = 20.0;
/// Between the title column and the actions.
pub const ACTIONS_GAP: f32 = 12.0;
/// The footer: padding above and below its 32 px button row, and its total height.
pub const FOOTER_PAD_Y: f32 = 12.0;
pub const FOOTER_ROW: f32 = 32.0;
pub const FOOTER_H: f32 = FOOTER_PAD_Y * 2.0 + FOOTER_ROW;
/// How far below its place a swapped-in body starts.
pub const SWAP_DY: f32 = 6.0;

/// The swap's opacity: `page-swap`, `moderate`, quad-out; it keeps its tween under Reduced.
pub const SWAP_OPACITY: Motion = Motion {
    reduced: Reduce::Keep,
    ..Motion::of(Kind::PageSwap)
};
/// The swap's slide: `page-swap`, `moderate`, sine-in-out; dropped under Reduced.
pub const SWAP_SLIDE: Motion = Motion {
    easing: Easing::SineInOut,
    reduced: Reduce::Snap,
    ..Motion::of(Kind::PageSwap)
};

/// What [`Ui::page`] is given besides its rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageOptions<'a> {
    /// The title (18 on a 24 px line, `fg_bright`, medium), elided to the room left of the actions.
    pub title: &'a str,
    /// The line under the title (13 on 20, `fg_tertiary`); it is also the group's description.
    pub subtitle: Option<&'a str>,
    /// The glass layer the card draws.
    pub glass: GlassLayer,
    /// What the card is composed over (premultiplied); see [`ToolbarOptions::under`](super::toolbar::ToolbarOptions::under).
    pub under: [f32; 4],
    /// The width of the header's actions, at the right edge. 0 draws and calls none.
    pub actions_w: f32,
    /// A pinned footer of [`FOOTER_H`].
    pub footer: bool,
    /// Which page the body holds: a change swaps it and scrolls to the top.
    pub swap_key: u64,
    /// The group's accessible name; empty falls back to the title.
    pub name: &'a str,
}

/// The slot a [`Ui::page`] closure is called for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageSlot {
    /// The header's actions: the right-aligned box, as wide as `actions_w` and as tall as the
    /// heading. Called only when `actions_w` is above 0. The header aligns its actions to the top,
    /// so the app puts them at the slot's top, not its middle.
    Actions(Rect),
    /// The body: the scroll view and the padded box content is drawn in (0 16 16, so the box is
    /// the viewport less 16 each side and 16 below). The closure returns the content's height.
    Body(ScrollView, Rect),
    /// The footer's content row. Called only with `footer`.
    Footer(Rect),
}

/// The card inside `rect`: the margin 0 10 10 6 taken off.
pub fn card_rect(rect: Rect) -> Rect {
    Rect::new(
        rect.x + MARGIN.3,
        rect.y + MARGIN.0,
        (rect.w - MARGIN.1 - MARGIN.3).max(0.0),
        (rect.h - MARGIN.0 - MARGIN.2).max(0.0),
    )
}

/// The header's height: padding, the title's line, the subtitle's if any.
pub fn header_height(has_subtitle: bool) -> f32 {
    2.0 * PAD + TITLE_LINE + if has_subtitle { SUBTITLE_LINE } else { 0.0 }
}

/// The retained key of the body that is shown.
#[derive(Debug, Default)]
struct Shown(Option<u64>);

impl Ui<'_> {
    /// A page card under `key` in `rect`. `content` is called for each slot in tab order and
    /// returns the body's height for [`PageSlot::Body`] (its other slots' returns are ignored).
    /// Returns the body's scroll area response. The node is a `Group` named by the title
    /// (`opts.name` when set) described by the subtitle, over the actions, a `ScrollArea` body and
    /// a `Group` footer.
    pub fn page(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &PageOptions<'_>,
        mut content: impl FnMut(&mut Ui<'_>, PageSlot) -> f32,
    ) -> ScrollResponse<()> {
        let id = self.id(&key);
        let t = *self.theme();
        let card = card_rect(rect);
        let header_h = header_height(opts.subtitle.is_some());
        let footer_h = if opts.footer { FOOTER_H } else { 0.0 };
        let body = Rect::new(
            card.x,
            card.y + header_h,
            card.w,
            (card.h - header_h - footer_h).max(0.0),
        );
        let actions_w = if opts.actions_w.is_finite() {
            opts.actions_w.max(0.0)
        } else {
            0.0
        };

        let p = self.painter();
        p.acrylic(card, RADIUS, opts.glass.premultiplied(), opts.under, true);

        let name = if opts.name.is_empty() {
            opts.title
        } else {
            opts.name
        };
        let mut group = Node::new(id, Role::Group, name, card);
        group.description = opts.subtitle.map(str::to_owned);
        let (alpha, dy) = self.swap_look(id.with("swap"), opts.swap_key);

        self.scope(&key, |ui| {
            ui.node_container(group, |ui| {
                // The header: the heading, elided to the room left of the actions.
                let room = (card.w
                    - 2.0 * PAD
                    - actions_w
                    - if actions_w > 0.0 { ACTIONS_GAP } else { 0.0 })
                .max(0.0);
                let p = ui.painter();
                if !opts.title.is_empty() {
                    let style = Style {
                        line: TITLE_LINE,
                        ..Style::ui(t.font_title, t.fg_bright).medium()
                    };
                    let spec = p.elide(&Spec::plain(opts.title, style), room);
                    p.text(&spec, card.x + PAD, card.y + PAD);
                }
                if let Some(sub) = opts.subtitle {
                    let style = Style {
                        line: SUBTITLE_LINE,
                        ..Style::ui(t.font_base, t.fg_tertiary)
                    };
                    let spec = p.elide(&Spec::plain(sub, style), room);
                    p.text(&spec, card.x + PAD, card.y + PAD + TITLE_LINE);
                }
                if actions_w > 0.0 {
                    let heading_h = header_h - 2.0 * PAD;
                    let at = Rect::new(
                        card.right() - PAD - actions_w,
                        card.y + PAD,
                        actions_w,
                        heading_h,
                    );
                    content(ui, PageSlot::Actions(at));
                }

                // The body, cut by the card's rounded rect.
                ui.painter().push_clip_rounded(card, RADIUS);
                let scroll = ScrollOptions {
                    bars: (Bars::Disabled, Bars::Auto),
                    identity: Some(opts.swap_key),
                    ..ScrollOptions::default()
                };
                let res = ui.scroll_area("body", body, &scroll, |ui, view| {
                    let vp = view.viewport;
                    let inner = Rect::new(
                        vp.x + PAD,
                        vp.y,
                        (vp.w - 2.0 * PAD).max(0.0),
                        (vp.h - PAD).max(0.0),
                    );
                    let faded = alpha < 1.0;
                    let moved = dy != 0.0;
                    let p = ui.painter();
                    if faded {
                        p.push_xform(Xform {
                            alpha,
                            ..Xform::translate(0.0, 0.0)
                        });
                    }
                    if moved {
                        p.push_translate(0.0, dy);
                    }
                    let h = content(ui, PageSlot::Body(*view, inner));
                    let p = ui.painter();
                    if moved {
                        p.pop_translate();
                    }
                    if faded {
                        p.pop_xform();
                    }
                    let h = if h.is_finite() { h.max(0.0) } else { 0.0 };
                    ((), Extent::new(f64::from(vp.w), f64::from(h + PAD)))
                });
                ui.painter().pop_clip();

                // The footer: a hairline over a row the caller fills.
                if opts.footer {
                    let at = Rect::new(card.x, card.bottom() - FOOTER_H, card.w, FOOTER_H);
                    ui.painter().fill(
                        Rect::new(card.x + 1.0, at.y, (card.w - 2.0).max(0.0), 1.0),
                        0.0,
                        t.border_subtle,
                    );
                    let row = Rect::new(
                        card.x + PAD,
                        at.y + FOOTER_PAD_Y,
                        (card.w - 2.0 * PAD).max(0.0),
                        FOOTER_ROW,
                    );
                    let footer = Node::new(ui.id("footer"), Role::Group, "", at);
                    ui.node_container(footer, |ui| content(ui, PageSlot::Footer(row)));
                }

                // The outline last, so nothing the body drew covers it.
                ui.painter().ring(card, RADIUS, 1.0, t.border_subtle, true);
                res
            })
        })
    }

    /// The body's opacity and downward offset this frame, and the swap they belong to: a changed
    /// `swap_key` starts both from where they are now (from 0 and [`SWAP_DY`] when at rest).
    fn swap_look(&mut self, id: WidgetId, swap_key: u64) -> (f32, f32) {
        let shown = self.persistent::<Shown>(id);
        let changed = shown.0.is_some_and(|k| k != swap_key);
        shown.0 = Some(swap_key);
        let (op, slide) = (id.with("opacity"), id.with("slide"));
        if changed {
            let now = self.animate_f32(op, 1.0, &SWAP_OPACITY);
            self.snap_f32(op, if now < 1.0 { now } else { 0.0 });
            let now = self.animate_f32(slide, 0.0, &SWAP_SLIDE);
            self.snap_f32(slide, if now != 0.0 { now } else { SWAP_DY });
        }
        (
            self.animate_f32(op, 1.0, &SWAP_OPACITY),
            self.animate_f32(slide, 0.0, &SWAP_SLIDE),
        )
    }
}
