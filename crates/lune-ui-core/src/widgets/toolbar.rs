//! A strip of actions over a glass layer the app passes in (`widget.toolbar`).
//!
//! [`Ui::toolbar`] draws the bar and hands its three columns to the caller, who places the
//! actions: [`Ui::toolbar_action`] (a 40 by 40 glyph button with a tooltip) and
//! [`Ui::toolbar_label`] (a glyph and an elided text, the branch button's shape). The commands and
//! their names are the app's. The bar is the app's glass layer while the window is active and the
//! theme's `window_fallback` fill when it is not (D128): the widget names no preset, the caller
//! passes the layer.
//!
//! The columns are the entry's `*, Auto, *` grid. The middle column's width is the caller's
//! content, which this widget cannot know before the closure runs, so [`ToolbarCols::center`] is
//! the zero-width rect at the middle: a centred group is placed around `center.x`, and the left
//! and right columns each run from the edge to that middle.

use std::hash::Hash;

use super::button::{self, ButtonLook, ButtonState, FOCUS, HOVER, IconButton, PRESS};
use super::focus_ring::{self, RingKind};
use crate::access::{Action, Node, Role, State};
use crate::backdrop::GlassLayer;
use crate::icon;
use crate::input::NamedKey;
use crate::paint::{Rect, Xform};
use crate::ui::{Response, Sense, Ui};

/// The padding of the bar: nothing on the left, 6 above and below, 8 on the right.
pub const PAD_LEFT: f32 = 0.0;
pub const PAD_TOP: f32 = 6.0;
pub const PAD_RIGHT: f32 = 8.0;
pub const PAD_BOTTOM: f32 = 6.0;
/// The spacing between the groups of a column; the buttons of a group touch.
pub const GROUP_GAP: f32 = 8.0;
/// An action button's side, and its glyph's size.
pub const ACTION_SIZE: f32 = 40.0;
pub const ACTION_GLYPH: f32 = 20.0;
/// The labelled button's glyph size, gap to the text, padding, and the widest its text gets.
pub const LABEL_GLYPH: f32 = 14.0;
pub const LABEL_GAP: f32 = 4.0;
pub const LABEL_PAD_X: f32 = 6.0;
pub const LABEL_PAD_Y: f32 = 4.0;
pub const LABEL_MAX_WIDTH: f32 = 420.0;

/// What [`Ui::toolbar`] is given besides its rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolbarOptions<'a> {
    /// The window is active: the glass shows. Inactive, the solid fallback replaces it.
    pub active: bool,
    /// The glass layer drawn while active.
    pub glass: GlassLayer,
    /// What the glass layer is composed over (the colour below it, premultiplied).
    /// Pass the layer the bar sits on: acrylic replaces what is under it, and the pixel just outside
    /// the rect takes this colour, so a zero `under` leaves a 1 px transparent rim over anything but
    /// the window edge.
    pub under: [f32; 4],
    /// The accessible name of the bar.
    pub name: &'a str,
}

/// The bar's columns, inside its padding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolbarCols {
    /// From the left edge to the middle.
    pub left: Rect,
    /// Zero wide, at the middle.
    pub center: Rect,
    /// From the middle to the right edge.
    pub right: Rect,
}

impl ToolbarCols {
    /// The columns of a bar at `rect`.
    pub fn of(rect: Rect) -> ToolbarCols {
        let inner = Rect::new(
            rect.x + PAD_LEFT,
            rect.y + PAD_TOP,
            (rect.w - PAD_LEFT - PAD_RIGHT).max(0.0),
            (rect.h - PAD_TOP - PAD_BOTTOM).max(0.0),
        );
        let half = inner.w / 2.0;
        ToolbarCols {
            left: Rect::new(inner.x, inner.y, half, inner.h),
            center: Rect::new(inner.x + half, inner.y, 0.0, inner.h),
            right: Rect::new(inner.x + half, inner.y, half, inner.h),
        }
    }
}

impl Ui<'_> {
    /// A toolbar under `key` over `rect`: draws the glass (or the fallback), then runs `f` with
    /// the columns, inside the bar's scope and semantic node (role `Toolbar`, named
    /// `opts.name`, the actions its children).
    pub fn toolbar<R>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &ToolbarOptions<'_>,
        f: impl FnOnce(&mut Ui<'_>, ToolbarCols) -> R,
    ) -> R {
        let id = self.id(&key);
        let fallback = self.theme().window_fallback;
        let p = self.painter();
        if opts.active {
            p.acrylic(rect, 0.0, opts.glass.premultiplied(), opts.under, true);
        } else {
            p.fill(rect, 0.0, fallback);
        }
        let node = Node::new(id, Role::Toolbar, opts.name, rect);
        self.scope(&key, |ui| {
            ui.node_container(node, |ui| f(ui, ToolbarCols::of(rect)))
        })
    }

    /// An action under `key`: a 40 by 40 ghost button at the top left of `rect` with a 20 px
    /// `fg_muted` glyph, and a tooltip of `name` (also the node's name). Disabled it is inert and
    /// drawn at 0.4.
    pub fn toolbar_action(
        &mut self,
        key: impl Hash,
        rect: Rect,
        glyph: &str,
        name: &str,
        enabled: bool,
    ) -> Response {
        let rect = Rect::new(rect.x, rect.y, ACTION_SIZE, ACTION_SIZE);
        let theme = *self.theme();
        let button = IconButton {
            glyph,
            glyph_size: ACTION_GLYPH,
            color: Some(theme.fg_muted),
            radius: theme.item_radius,
            name,
        };
        let r = self.with_enabled(enabled, |ui| ui.icon_button(key, rect, &button));
        self.tooltip(&r, name);
        r
    }

    /// A button under `key` in `rect` with a 14 px glyph and a text: padding 6 and 4, the text
    /// elided to at most 420 px (less when `rect` is narrower), `fg_subtle` at `font_m` (D133). The look is the ghost
    /// button's. Role `Button` named `name`. Disabled through [`Ui::with_enabled`].
    pub fn toolbar_label(
        &mut self,
        key: impl Hash,
        rect: Rect,
        glyph: &str,
        text: &str,
        name: &str,
    ) -> Response {
        let id = self.id(key);
        let mut r = self.interact(id, rect, Sense::CLICK_FOCUS);
        if self.activation_keys(id, r.enabled, &[NamedKey::Enter, NamedKey::Space]) {
            r.clicked = true;
        }
        let theme = *self.theme();
        let state = ButtonState {
            hovered: r.hovered,
            pressed: r.pressed,
            focus_visible: r.focus_visible,
        };
        let target = ButtonLook::of(&theme, button::ButtonKind::Ghost, state);
        let fill = self.animate_color(id.with("fill"), target.fill, &HOVER);
        let scale = self.animate_f32(id.with("press"), target.scale, &PRESS);
        let ring = self.animate_f32(id.with("ring"), target.ring, &FOCUS);

        let color = theme.fg_subtle;
        let p = self.painter();
        let scaled = scale != 1.0;
        if scaled {
            p.push_xform(Xform {
                origin: (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0),
                scale,
                dx: 0.0,
                dy: 0.0,
                alpha: 1.0,
            });
        }
        if fill[3] > 0 {
            p.fill(rect, theme.item_radius, fill);
        }
        let glyph = icon::glyph(glyph, LABEL_GLYPH, color);
        let gw = p.measure(&glyph).0;
        let room = rect.w - 2.0 * LABEL_PAD_X - gw - LABEL_GAP;
        let spec = p.elide(
            &label_spec(&theme, text, color),
            room.clamp(0.0, LABEL_MAX_WIDTH),
        );
        let x = rect.x + LABEL_PAD_X;
        p.text_centered(&glyph, Rect::new(x, rect.y, gw, rect.h));
        let tw = p.measure(&spec).0;
        p.text_centered(&spec, Rect::new(x + gw + LABEL_GAP, rect.y, tw, rect.h));
        focus_ring::paint(p, rect, RingKind::Button, ring.min(1.0));
        if scaled {
            p.pop_xform();
        }

        let mut node = Node::new(id, Role::Button, name, rect);
        if r.enabled {
            node.states.insert(State::Focusable);
            node.actions.insert(Action::Default);
        } else {
            node.states.insert(State::Disabled);
        }
        self.node(node);
        r
    }
}

/// The label's text: the theme's medium size (`font_m`).
fn label_spec(
    theme: &crate::theme::Theme,
    text: &str,
    color: crate::color::Rgba,
) -> crate::text::Spec {
    crate::text::Spec::plain(text, crate::text::Style::ui(theme.font_m, color))
}

/// The width [`Ui::toolbar_label`] wants for `text`: padding, glyph, gap and the text (at most
/// [`LABEL_MAX_WIDTH`]).
pub fn label_width<H: Copy + PartialEq>(
    p: &mut crate::paint::Painter<'_, H>,
    glyph: &str,
    text: &str,
) -> f32 {
    let theme = p.theme;
    let gw = p
        .measure(&icon::glyph(glyph, LABEL_GLYPH, theme.fg_subtle))
        .0;
    let tw = p.measure(&label_spec(&theme, text, theme.fg_subtle)).0;
    2.0 * LABEL_PAD_X + gw + LABEL_GAP + tw.min(LABEL_MAX_WIDTH)
}
