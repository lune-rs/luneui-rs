//! Button control and its label width.
//!
//! The pattern every widget follows (D2):
//! 1. [`ButtonLook`], plain data for the continuous look of one frame, and [`ButtonLook::of`], the
//!    look a discrete [`ButtonState`] settles to;
//! 2. [`paint`], which draws a look (specimen sheets call it directly);
//! 3. [`button`], the Painter-API function: the discrete look, `paint`, then the hit region;
//! 4. [`Ui::button`], which interacts, animates each field of the look toward its settled value
//!    through the rows of [`HOVER`], [`PRESS`] and [`FOCUS`], paints and emits the semantic node.
//!
//! At rest the animated look equals `ButtonLook::of`, so the `Ui` draws exactly what `button` does.

use std::hash::Hash;

use super::focus_ring::{self, RingKind};
use super::label;
use crate::access::{Action, Node, Role, State};
use crate::color::Rgba;
use crate::icon;
use crate::input::NamedKey;
use crate::motion::{Kind, Motion};
use crate::paint::{Painter, Rect, Xform};
use crate::theme::Theme;
use crate::ui::{Response, Sense, Ui, WidgetId};

/// The press scale about the rect's centre.
pub const PRESS_SCALE: f32 = 0.98;

/// A small button's height, corner radius and horizontal padding.
pub const SMALL_HEIGHT: f32 = 24.0;
pub const SMALL_RADIUS: f32 = 4.0;
pub const SMALL_PADDING: f32 = 12.0;
/// The glyph of a labelled icon button, and the gap between it and the label.
pub const ICON_SIZE: f32 = 16.0;
pub const ICON_GAP: f32 = 8.0;

/// Fill colour change on hover and press.
pub const HOVER: Motion = Motion::of(Kind::Hover);
/// Press scale.
pub const PRESS: Motion = Motion::of(Kind::Press);
/// Focus ring opacity.
pub const FOCUS: Motion = Motion::of(Kind::Focus);

/// Which button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    /// No fill at rest, `surface_hover` on hover, `surface_active` pressed; secondary text.
    Ghost,
}

impl ButtonKind {
    /// The ring at corner `radius`: the original look at the theme's radius, else on the edge.
    fn ring(self, theme: &Theme, radius: f32) -> RingKind {
        let on_accent = self == ButtonKind::Primary;
        if radius == theme.item_radius {
            if on_accent {
                RingKind::PrimaryButton
            } else {
                RingKind::Button
            }
        } else {
            RingKind::Edge { radius, on_accent }
        }
    }
}

/// What [`Ui::button_with`] is given besides its rect and text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonOptions<'a> {
    pub kind: ButtonKind,
    /// 24 high whatever the rect says (the top stays), radius 4, padding 0 12.
    pub small: bool,
    /// A glyph before the label: [`ICON_SIZE`] px, [`ICON_GAP`] from the label.
    pub icon: Option<&'a str>,
}

impl Default for ButtonOptions<'_> {
    fn default() -> Self {
        ButtonOptions {
            kind: ButtonKind::Secondary,
            small: false,
            icon: None,
        }
    }
}

/// A button that is only a glyph: ghost behaviour, the glyph centred in the caller's rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconButton<'a> {
    pub glyph: &'a str,
    pub glyph_size: f32,
    /// The glyph's colour; `None` is the ghost label colour (`fg_secondary`).
    pub color: Option<Rgba>,
    pub radius: f32,
    /// The accessible name.
    pub name: &'a str,
}

/// What the pointer and keyboard are doing to a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ButtonState {
    pub hovered: bool,
    pub pressed: bool,
    pub focus_visible: bool,
}

/// The continuous look of a button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonLook {
    pub fill: Rgba,
    pub label: Rgba,
    /// Scale about the rect's centre; 1.0 pushes no transform.
    pub scale: f32,
    /// Focus ring opacity; 0.0 draws no ring.
    pub ring: f32,
}

impl ButtonLook {
    /// The look `state` settles to.
    pub fn of(theme: &Theme, kind: ButtonKind, state: ButtonState) -> ButtonLook {
        let fill = match (kind, state.pressed, state.hovered) {
            (ButtonKind::Primary, false, false) => theme.accent,
            (ButtonKind::Primary, _, _) => theme.accent_hover,
            (ButtonKind::Secondary, true, _) => theme.surface_active,
            (ButtonKind::Secondary, false, true) => theme.surface_selected,
            (ButtonKind::Secondary, false, false) => theme.surface_hover,
            (ButtonKind::Ghost, true, _) => theme.surface_active,
            (ButtonKind::Ghost, false, true) => theme.surface_hover,
            // Transparent, in the hover colour so the fade has no dark middle.
            (ButtonKind::Ghost, false, false) => {
                let [r, g, b, _] = theme.surface_hover;
                [r, g, b, 0]
            }
        };
        ButtonLook {
            fill,
            label: match kind {
                ButtonKind::Primary => theme.on_accent_text,
                ButtonKind::Secondary => theme.fg_primary,
                ButtonKind::Ghost => theme.fg_secondary,
            },
            scale: if state.pressed { PRESS_SCALE } else { 1.0 },
            ring: if state.focus_visible { 1.0 } else { 0.0 },
        }
    }
}

/// What a button shows in its rect.
#[derive(Clone, Copy)]
enum Content<'a> {
    Label {
        text: &'a str,
        icon: Option<&'a str>,
    },
    Glyph {
        glyph: &'a str,
        size: f32,
    },
}

/// Draws a button in `r` with `look`.
pub fn paint<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    text: &str,
    kind: ButtonKind,
    look: &ButtonLook,
) {
    let radius = p.theme.item_radius;
    paint_content(
        p,
        r,
        radius,
        Content::Label { text, icon: None },
        kind,
        look,
    );
}

/// [`paint`] for [`Ui::button_with`]'s variants: `r` is cut to 24 high when `options.small`, the
/// corner radius, the glyph and the ghost fill follow `options`. For specimen sheets.
pub fn paint_with<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    text: &str,
    options: &ButtonOptions<'_>,
    look: &ButtonLook,
) {
    let (r, radius) = if options.small {
        (
            Rect {
                h: SMALL_HEIGHT,
                ..r
            },
            SMALL_RADIUS,
        )
    } else {
        (r, p.theme.item_radius)
    };
    let content = Content::Label {
        text,
        icon: options.icon,
    };
    paint_content(p, r, radius, content, options.kind, look);
}

/// [`paint`] for [`Ui::icon_button`]: the ghost look of `button` over the caller's rect.
pub fn paint_icon<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    button: &IconButton<'_>,
    look: &ButtonLook,
) {
    let content = Content::Glyph {
        glyph: button.glyph,
        size: button.glyph_size,
    };
    let look = ButtonLook {
        label: button.color.unwrap_or(look.label),
        ..*look
    };
    paint_content(p, r, button.radius, content, ButtonKind::Ghost, &look);
}

fn paint_content<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    radius: f32,
    content: Content<'_>,
    kind: ButtonKind,
    look: &ButtonLook,
) {
    let t = p.theme;
    let scaled = look.scale != 1.0;
    if scaled {
        p.push_xform(Xform {
            origin: (r.x + r.w / 2.0, r.y + r.h / 2.0),
            scale: look.scale,
            dx: 0.0,
            dy: 0.0,
            alpha: 1.0,
        });
    }
    // A ghost at rest has no fill to draw.
    if look.fill[3] > 0 {
        p.fill(r, radius, look.fill);
    }
    match content {
        Content::Label { text, icon: None } => {
            p.text_centered(&label(&t, text, look.label), r);
        }
        Content::Label {
            text,
            icon: Some(glyph),
        } => {
            let glyph = icon::glyph(glyph, ICON_SIZE, look.label);
            let text = label(&t, text, look.label);
            let (gw, tw) = (p.measure(&glyph).0, p.measure(&text).0);
            let x = r.x + (r.w - (gw + ICON_GAP + tw)) / 2.0;
            p.text_centered(&glyph, Rect::new(x, r.y, gw, r.h));
            p.text_centered(&text, Rect::new(x + gw + ICON_GAP, r.y, tw, r.h));
        }
        Content::Glyph { glyph, size } => {
            p.text_centered(&icon::glyph(glyph, size, look.label), r);
        }
    }
    focus_ring::paint(p, r, kind.ring(&t, radius), look.ring);
    if scaled {
        p.pop_xform();
    }
}

/// Draws a primary or secondary button in `r` and registers `hit` over it. The Painter form: a
/// [`Ui`] has [`Ui::button`], which also handles focus, keys and the accessibility node.
pub fn button<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    text: &str,
    primary: bool,
    hit: H,
) {
    let kind = if primary {
        ButtonKind::Primary
    } else {
        ButtonKind::Secondary
    };
    let state = ButtonState {
        hovered: p.hovered(hit),
        ..ButtonState::default()
    };
    let look = ButtonLook::of(&p.theme, kind, state);
    paint(p, r, text, kind, &look);
    p.hit(r, hit);
}

/// The width [`button`] needs for `text`: the label and 16 px each side.
pub fn button_width<H: Copy + PartialEq>(p: &mut Painter<'_, H>, text: &str) -> f32 {
    let t = p.theme;
    p.measure(&label(&t, text, t.fg_primary)).0 + 32.0
}

/// The width [`Ui::button_with`] fits: the label, the glyph and its gap when there is one, and
/// the padding (16 each side, 12 for a small button).
pub fn button_width_with<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    text: &str,
    options: &ButtonOptions<'_>,
) -> f32 {
    let t = p.theme;
    let pad = if options.small { SMALL_PADDING } else { 16.0 };
    let glyph = match options.icon {
        Some(g) => p.measure(&icon::glyph(g, ICON_SIZE, t.fg_primary)).0 + ICON_GAP,
        None => 0.0,
    };
    p.measure(&label(&t, text, t.fg_primary)).0 + glyph + 2.0 * pad
}

impl Ui<'_> {
    /// A button under `key` in `rect`. `clicked` on a pointer click, Enter or Space while focused,
    /// or an accessibility default action. Disabled (see [`Ui::with_enabled`]) it reports nothing.
    pub fn button(&mut self, key: impl Hash, rect: Rect, text: &str, kind: ButtonKind) -> Response {
        let radius = self.theme().item_radius;
        let content = Content::Label { text, icon: None };
        self.button_inner(key, rect, radius, content, kind, text)
    }

    /// [`Ui::button`] with the options of [`ButtonOptions`]: a ghost, a small button (24 high,
    /// the top of `rect` kept) and a glyph before the label.
    pub fn button_with(
        &mut self,
        key: impl Hash,
        rect: Rect,
        text: &str,
        options: &ButtonOptions<'_>,
    ) -> Response {
        let (rect, radius) = if options.small {
            (
                Rect {
                    h: SMALL_HEIGHT,
                    ..rect
                },
                SMALL_RADIUS,
            )
        } else {
            (rect, self.theme().item_radius)
        };
        let content = Content::Label {
            text,
            icon: options.icon,
        };
        self.button_inner(key, rect, radius, content, options.kind, text)
    }

    /// A ghost button holding only a glyph, centred in `rect` (the caller's rect is its size: 28
    /// by 28, 24 by 24, ...). Role button named `button.name`; Enter and Space click.
    pub fn icon_button(&mut self, key: impl Hash, rect: Rect, button: &IconButton<'_>) -> Response {
        self.icon_button_sensed(key, rect, button, Sense::CLICK_FOCUS)
    }

    /// [`Self::icon_button`] registered with `sense`, for a composite that decides which of its
    /// buttons is a tab stop.
    pub(crate) fn icon_button_sensed(
        &mut self,
        key: impl Hash,
        rect: Rect,
        button: &IconButton<'_>,
        sense: Sense,
    ) -> Response {
        let content = Content::Glyph {
            glyph: button.glyph,
            size: button.glyph_size,
        };
        self.button_inner_colored(
            key,
            rect,
            button.radius,
            content,
            ButtonKind::Ghost,
            button.name,
            button.color,
            sense,
        )
    }

    /// Consumes the activation `keys` of `id` and says whether one activates it. Enabled, the
    /// first key found is taken and the rest stay for later handlers. Disabled, every one is taken
    /// and nothing activates, so a key the widget would handle reaches no shortcut. A Control, Alt
    /// or Command chord on one of them is not the widget's: it stays a shortcut.
    pub(crate) fn activation_keys(
        &mut self,
        id: WidgetId,
        enabled: bool,
        keys: &[NamedKey],
    ) -> bool {
        if enabled {
            return keys.iter().any(|k| self.consume_plain_key(id, *k));
        }
        for k in keys {
            while self.consume_plain_key(id, *k) {}
        }
        false
    }

    fn button_inner(
        &mut self,
        key: impl Hash,
        rect: Rect,
        radius: f32,
        content: Content<'_>,
        kind: ButtonKind,
        name: &str,
    ) -> Response {
        self.button_inner_colored(
            key,
            rect,
            radius,
            content,
            kind,
            name,
            None,
            Sense::CLICK_FOCUS,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn button_inner_colored(
        &mut self,
        key: impl Hash,
        rect: Rect,
        radius: f32,
        content: Content<'_>,
        kind: ButtonKind,
        name: &str,
        color: Option<Rgba>,
        sense: Sense,
    ) -> Response {
        let id = self.id(key);
        let mut r = self.interact(id, rect, sense);
        if self.activation_keys(id, r.enabled, &[NamedKey::Enter, NamedKey::Space]) {
            r.clicked = true;
        }

        let state = ButtonState {
            hovered: r.hovered,
            pressed: r.pressed,
            focus_visible: r.focus_visible,
        };
        let target = ButtonLook::of(self.theme(), kind, state);
        let look = ButtonLook {
            fill: self.animate_color(id.with("fill"), target.fill, &HOVER),
            label: color.unwrap_or(target.label),
            scale: self.animate_f32(id.with("press"), target.scale, &PRESS),
            ring: self.animate_f32(id.with("ring"), target.ring, &FOCUS),
        };
        paint_content(self.painter(), rect, radius, content, kind, &look);

        let mut node = Node::new(id, Role::Button, name, rect);
        if r.enabled {
            if sense.focusable {
                node.states.insert(State::Focusable);
            }
            node.actions.insert(Action::Default);
        } else {
            node.states.insert(State::Disabled);
        }
        self.node(node);
        r
    }
}
