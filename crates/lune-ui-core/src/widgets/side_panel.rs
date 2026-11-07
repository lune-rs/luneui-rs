//! The side panel host (`layout.side-panel`, D149): one column on the app's glass layer that shows
//! one of several panels by key. The panels' contents are the app's.
//!
//! [`Ui::side_panel`] draws the card (the app's glass layer, whatever the window's state: R13, the
//! widget names no preset), then runs `content`
//! once, for the shown panel, in the rect inside the 1 px [`RING`] and the [`PAD`] of 10. The shown panel runs under
//! [`Ui::keep_alive`]; every other key of [`SidePanelOptions::keys`] is only marked alive, so a
//! hidden panel's scroll offset, text edits and other retained state survive however many frames
//! it is not drawn. Hidden panels draw nothing and emit no node. A panel stays alive once it has
//! been shown or listed, until the app ends it with [`Ui::end_keep_alive`].
//!
//! A change of [`SidePanelOptions::panel`] shows the new panel with the drawer motion: its
//! content fades in ([`SHOW_OPACITY`]) while it slides in from [`SHOW_SLIDE_FROM`] px to its place
//! ([`SHOW_SLIDE`], dropped under Reduced). The first frame of a host does not animate; a show in
//! flight restarts from the look it has. The card does not move (the source animates the panel's
//! own view, not the card: [`SidePanelOptions::card_moves`] makes the card and ring take the slide
//! and fade too), and a hide snaps: the old panel is gone on the frame of the change. A show moves
//! no focus (the app knows the first control).
//!
//! The card's text limit (D12): a text cannot be cut by a rounded clip, so the host keeps the
//! content [`RING`] + [`PAD`] (11) from the edge, more than the radius of 8.

use std::hash::Hash;

use crate::access::{Node, Role};
use crate::backdrop::GlassLayer;
use crate::motion::{Easing, Kind, Motion, Reduce};
use crate::paint::{Rect, Xform};
use crate::ui::{Ui, WidgetId};

/// The card's corner radius.
pub const RADIUS: f32 = 8.0;
/// The padding between the card's ring and the content.
pub const PAD: f32 = 10.0;
/// The ring: 1 px in `border`, inside the rect (the inner `glass-panel` border,
/// `SidePanelView.axaml:20`, `App.axaml:294-298`), so the content starts `RING + PAD` in.
pub const RING: f32 = 1.0;
/// Where a panel's slide starts (x offset, px); it ends at 0.
pub const SHOW_SLIDE_FROM: f32 = -6.0;

/// The show's opacity: `drawer`, `moderate`, quad-out; it keeps its tween under Reduced.
pub const SHOW_OPACITY: Motion = Motion {
    easing: Easing::QuadOut,
    reduced: Reduce::Keep,
    ..Motion::of(Kind::Drawer)
};
/// The show's slide: `drawer`, `moderate`, sine-in-out; dropped under Reduced.
pub const SHOW_SLIDE: Motion = Motion {
    reduced: Reduce::Snap,
    ..Motion::of(Kind::Drawer)
};

/// What [`Ui::side_panel`] is given besides its rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SidePanelOptions<'a> {
    /// The key of the panel that is shown.
    pub panel: u64,
    /// Every panel's key, so the hidden ones stay alive. `panel` need not be repeated.
    pub keys: &'a [u64],
    /// The glass layer the card draws.
    pub glass: GlassLayer,
    /// What the card is composed over (premultiplied); see
    /// [`ToolbarOptions::under`](super::toolbar::ToolbarOptions::under).
    pub under: [f32; 4],
    /// The accessible name of the group (the shown panel's title).
    pub name: &'a str,
    /// The card and its ring take the show's slide and fade with the content. Off by default: the
    /// source's animation is on the panel's own view, and the card stays still.
    pub card_moves: bool,
}

/// The rect `content` is given in a host at `rect`.
pub fn content_rect(rect: Rect) -> Rect {
    Rect::new(
        rect.x + RING + PAD,
        rect.y + RING + PAD,
        (rect.w - 2.0 * (RING + PAD)).max(0.0),
        (rect.h - 2.0 * (RING + PAD)).max(0.0),
    )
}

/// The key of the panel that is shown.
#[derive(Debug, Default)]
struct Shown(Option<u64>);

impl Ui<'_> {
    /// A side panel host under `key` in `rect`; `content` draws the shown panel into
    /// [`content_rect`]. The node is a `Group` named `opts.name` over the content's nodes.
    pub fn side_panel(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &SidePanelOptions<'_>,
        content: impl FnOnce(&mut Ui<'_>, Rect),
    ) {
        let id = self.id(&key);
        let border = self.theme().border;
        let (alpha, dx) = self.show_look(id.with("show"), opts.panel);
        let (faded, moved) = (alpha < 1.0, dx != 0.0);
        // The card and its ring move with the show only when asked.
        let (card_faded, card_moved) = (opts.card_moves && faded, opts.card_moves && moved);
        let p = self.painter();
        if card_faded {
            p.push_xform(Xform {
                alpha,
                ..Xform::translate(0.0, 0.0)
            });
        }
        if card_moved {
            p.push_translate(dx, 0.0);
        }
        p.acrylic(rect, RADIUS, opts.glass.premultiplied(), opts.under, true);
        p.ring(rect, RADIUS, RING, border, false);
        if card_moved {
            p.pop_translate();
        }
        if card_faded {
            p.pop_xform();
        }
        for k in opts.keys.iter().filter(|k| **k != opts.panel) {
            self.mark_alive((&key, *k));
        }
        let group = Node::new(id, Role::Group, opts.name, rect);
        self.node_container(group, |ui| {
            ui.keep_alive((&key, opts.panel), |ui| {
                let p = ui.painter();
                if faded {
                    p.push_xform(Xform {
                        alpha,
                        ..Xform::translate(0.0, 0.0)
                    });
                }
                if moved {
                    p.push_translate(dx, 0.0);
                }
                content(ui, content_rect(rect));
                let p = ui.painter();
                if moved {
                    p.pop_translate();
                }
                if faded {
                    p.pop_xform();
                }
            });
        });
    }

    /// The content's opacity and x offset this frame: a changed `panel` starts both from where
    /// they are now (from 0 and [`SHOW_SLIDE_FROM`] when at rest).
    fn show_look(&mut self, id: WidgetId, panel: u64) -> (f32, f32) {
        let shown = self.persistent::<Shown>(id);
        let changed = shown.0.is_some_and(|k| k != panel);
        shown.0 = Some(panel);
        let (op, slide) = (id.with("opacity"), id.with("slide"));
        if changed {
            let now = self.animate_f32(op, 1.0, &SHOW_OPACITY);
            self.snap_f32(op, if now < 1.0 { now } else { 0.0 });
            let now = self.animate_f32(slide, 0.0, &SHOW_SLIDE);
            self.snap_f32(slide, if now != 0.0 { now } else { SHOW_SLIDE_FROM });
        }
        (
            self.animate_f32(op, 1.0, &SHOW_OPACITY),
            self.animate_f32(slide, 0.0, &SHOW_SLIDE),
        )
    }
}
