//! The settings page frame (`layout.settings-page`, D152): glass with a top-left radius, a 20 px
//! content host that scrolls, and a restart banner docked below it.
//!
//! [`Ui::settings_page`] draws the glass over the rect it is given (the app's glass layer, whatever the
//! window's state, with the corner radius of 8 at the top left only: R13, the widget names no
//! preset), then runs `content` once
//! in the host: a vertical [`scroll_area`](super::scroll_area) inset [`HOST_PAD`] from the page's
//! left, top and right, and from the banner (or the page's bottom) below. `content` draws in the
//! viewport's rect and returns its height. The rows are [`Ui::setting_row`]; the section nav is the
//! app's.
//!
//! The banner is docked at the bottom and shown while [`SettingsPage::restart`] is set: padding 20
//! by 10 on `status_warning_dark`, a 14 px warning glyph and the message in `status_warning_light`,
//! and the action button at the right (padding 12 by 6, radius 6, a 1 px `status_warning_light`
//! border). It fades in with [`REVEAL`]; when the flag is cleared it fades back out and keeps its
//! space until it is gone, then the content's height snaps. A hidden banner takes no height and
//! has no node. The action is the app's restart command: [`SettingsResponse::restart`] is true on
//! the frame it was pressed (once).
//!
//! The page is a container, so a disabled page is `with_enabled(false)` around the call, as for
//! any group; the action then takes no focus and reports nothing. The page moves no focus on its
//! own: the app knows the first control.

use std::hash::Hash;

use crate::access::{Action, Live, Node, Role, State};
use crate::backdrop::GlassLayer;
use crate::color::Rgba;
use crate::icon;
use crate::input::NamedKey;
use crate::motion::{Easing, Kind, Motion, Reduce, Token};
use crate::paint::{Rect, Xform};
use crate::text::{Spec, Style};
use crate::ui::{Sense, Ui};

use super::button::HOVER;
use super::focus_ring::{self, RingKind};
use super::scroll_area::{Bars, Extent, ScrollOptions};

/// The page's top-left corner radius; the other three corners are square.
pub const RADIUS: f32 = 8.0;
/// The content host's margin on every side.
pub const HOST_PAD: f32 = 20.0;
/// The banner's padding: sides, then above and below.
pub const BANNER_PAD_X: f32 = 20.0;
pub const BANNER_PAD_Y: f32 = 10.0;
/// The banner's glyph size, and its gap to the message.
pub const ICON_SIZE: f32 = 14.0;
pub const ICON_GAP: f32 = 8.0;
/// The room kept between the message and the action.
pub const ACTION_GAP: f32 = 8.0;
/// The action's padding (sides, then above and below), corner radius and border width.
pub const ACTION_PAD_X: f32 = 12.0;
pub const ACTION_PAD_Y: f32 = 6.0;
pub const ACTION_RADIUS: f32 = 6.0;
pub const ACTION_BORDER: f32 = 1.0;

/// The banner's reveal and hide: `appear`, `base`, quad-out, opacity only; Reduced keeps the fade
/// at `fast`, Off snaps.
pub const REVEAL: Motion = Motion {
    kind: Kind::Appear,
    token: Token::Base,
    easing: Easing::QuadOut,
    reduced: Reduce::Keep,
    reduced_token: Some(Token::Fast),
};

/// What [`Ui::settings_page`] is given besides its rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SettingsPage<'a> {
    /// The glass layer the page draws.
    pub glass: GlassLayer,
    /// What the page is composed over (premultiplied); see
    /// [`ToolbarOptions::under`](super::toolbar::ToolbarOptions::under).
    pub under: [f32; 4],
    /// A restart is needed: the banner shows. Cleared by the app once it has dealt with it.
    pub restart: bool,
    /// The group's accessible name (the section's title; app text).
    pub name: &'a str,
}

/// What [`Ui::settings_page`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SettingsResponse {
    /// The banner's action was pressed this frame.
    pub restart: bool,
}

/// `c` with alpha 0, so a fade keeps its hue.
fn clear(c: Rgba) -> Rgba {
    [c[0], c[1], c[2], 0]
}

/// What the pointer and keyboard are doing to the action.
#[derive(Debug, Clone, Copy, Default)]
struct ActionLook {
    hovered: bool,
    pressed: bool,
    focus_visible: bool,
}

impl Ui<'_> {
    /// A settings page under `key` over `rect`. `content` draws the current section into the
    /// host's viewport rect and returns its height, which is what scrolls. Returns whether the
    /// banner's action was pressed this frame. The node is a `Group` named `page.name` over the
    /// content's `ScrollArea` and, while `restart` is set, a polite `Status` banner holding the
    /// action `Button`.
    pub fn settings_page(
        &mut self,
        key: impl Hash,
        rect: Rect,
        page: &SettingsPage<'_>,
        content: impl FnOnce(&mut Ui<'_>, Rect) -> f32,
    ) -> SettingsResponse {
        let id = self.id(&key);
        let t = *self.theme();
        let radii = [RADIUS, 0.0, 0.0, 0.0];
        let p = self.painter();
        p.acrylic_radii(rect, radii, page.glass.premultiplied(), page.under, true);

        // The reveal runs every frame so a hide has somewhere to start from; the banner is on
        // screen while it is set or still fading.
        let reveal = self
            .animate_f32(
                id.with("reveal"),
                if page.restart { 1.0 } else { 0.0 },
                &REVEAL,
            )
            .clamp(0.0, 1.0);
        let shown = page.restart || reveal > 0.0;

        let message = self.tr("luneui.settings-page.restart-message").to_owned();
        let action = self.tr("luneui.settings-page.restart-action").to_owned();
        let style = Style::ui(t.font_base, t.status_warning_light);
        let (action_w, line_h, bar_h) = if shown {
            let p = self.painter();
            let (lw, lh) = p.measure(&Spec::plain(&action, style));
            let action_h = lh + 2.0 * (ACTION_PAD_Y + ACTION_BORDER);
            (
                lw + 2.0 * (ACTION_PAD_X + ACTION_BORDER),
                lh,
                action_h.max(lh).max(ICON_SIZE) + 2.0 * BANNER_PAD_Y,
            )
        } else {
            (0.0, 0.0, 0.0)
        };

        let host = Rect::new(
            rect.x + HOST_PAD,
            rect.y + HOST_PAD,
            (rect.w - 2.0 * HOST_PAD).max(0.0),
            (rect.h - bar_h - 2.0 * HOST_PAD).max(0.0),
        );
        let bar = Rect::new(rect.x, rect.bottom() - bar_h, rect.w, bar_h);

        let group = Node::new(id, Role::Group, page.name, rect);
        let mut response = SettingsResponse::default();
        self.scope(&key, |ui| {
            ui.node_container(group, |ui| {
                let opts = ScrollOptions {
                    bars: (Bars::Disabled, Bars::Auto),
                    ..ScrollOptions::default()
                };
                ui.scroll_area("content", host, &opts, |ui, view| {
                    let vp = view.viewport;
                    let h = content(ui, vp);
                    let h = if h.is_finite() { h.max(0.0) } else { 0.0 };
                    ((), Extent::new(f64::from(vp.w), f64::from(h)))
                });
                if shown {
                    let geo = Banner {
                        bar,
                        action_w,
                        line_h,
                        reveal,
                        live: page.restart,
                    };
                    response.restart = ui.restart_banner(&geo, &message, &action, style);
                }
            });
        });
        response
    }

    /// Draws the banner in `geo.bar` and returns whether its action was pressed. A banner that is
    /// not `live` is only fading out: it draws, but has no node and takes no input.
    fn restart_banner(&mut self, geo: &Banner, message: &str, action: &str, style: Style) -> bool {
        let t = *self.theme();
        let bar = geo.bar;
        let action_h = geo.line_h + 2.0 * (ACTION_PAD_Y + ACTION_BORDER);
        let at = Rect::new(
            bar.right() - BANNER_PAD_X - geo.action_w,
            bar.y + (bar.h - action_h) / 2.0,
            geo.action_w,
            action_h,
        );
        let banner_id = self.id("banner");
        let action_id = self.id("restart");

        let mut look = ActionLook::default();
        let mut clicked = false;
        let mut enabled = true;
        if geo.live {
            let r = self.interact(action_id, at, Sense::CLICK_FOCUS);
            enabled = r.enabled;
            clicked = r.clicked;
            if self.activation_keys(action_id, r.enabled, &[NamedKey::Enter, NamedKey::Space]) {
                clicked = true;
            }
            look = ActionLook {
                hovered: r.hovered,
                pressed: r.pressed,
                focus_visible: r.focus_visible,
            };
        }
        let fill_target = if look.pressed {
            t.surface_active
        } else if look.hovered {
            t.surface_hover
        } else {
            clear(t.surface_hover)
        };
        let fill = self.animate_color(action_id.with("fill"), fill_target, &HOVER);
        let ring = self.animate_f32(
            action_id.with("ring"),
            if look.focus_visible { 1.0 } else { 0.0 },
            &super::button::FOCUS,
        );

        let faded = geo.reveal < 1.0;
        let p = self.painter();
        if faded {
            p.push_xform(Xform {
                alpha: geo.reveal,
                ..Xform::translate(0.0, 0.0)
            });
        }
        p.fill(bar, 0.0, t.status_warning_dark);
        let glyph = icon::glyph(icon::WARNING, ICON_SIZE, t.status_warning_light);
        let glyph_x = bar.x + BANNER_PAD_X;
        p.text_centered(&glyph, Rect::new(glyph_x, bar.y, ICON_SIZE, bar.h));
        let text_x = glyph_x + ICON_SIZE + ICON_GAP;
        let room = (at.x - ACTION_GAP - text_x).max(0.0);
        let spec = p.elide(&Spec::plain(message, style), room);
        p.text(&spec, text_x, bar.y + (bar.h - geo.line_h) / 2.0);
        if fill[3] > 0 {
            p.fill(at, ACTION_RADIUS, fill);
        }
        p.ring(
            at,
            ACTION_RADIUS,
            ACTION_BORDER,
            t.status_warning_light,
            false,
        );
        p.text_centered(&Spec::plain(action, style), at);
        focus_ring::paint(
            p,
            at,
            RingKind::Edge {
                radius: ACTION_RADIUS,
                on_accent: false,
            },
            ring,
        );
        if faded {
            p.pop_xform();
        }

        if geo.live {
            let mut node = Node::new(banner_id, Role::Status, message, bar);
            node.live = Some(Live::Polite);
            self.node_container(node, |ui| {
                let mut button = Node::new(action_id, Role::Button, action, at);
                if enabled {
                    button.states.insert(State::Focusable);
                    button.actions.insert(Action::Default);
                } else {
                    button.states.insert(State::Disabled);
                }
                ui.node(button);
            });
        }
        clicked && enabled
    }
}

/// Where the banner sits and what it is made of.
struct Banner {
    bar: Rect,
    action_w: f32,
    line_h: f32,
    /// The fade, 0 to 1.
    reveal: f32,
    /// The flag is set: the banner takes input and has a node.
    live: bool,
}
