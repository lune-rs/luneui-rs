//! A tinted block of text with an icon and a tone (`widget.callout`).
//!
//! Radius 6, a 1 px border, padding 8 10 inside it; a 14 px icon centred in a 20 px slot, 8 px, then
//! the text (13 on a 20 px line, wrapping). At the right, an optional figure (mono, bright, Medium;
//! never gold) and an optional small action button. Neutral is `surface_input` with `border_subtle`;
//! every other tone is the badge recipe from [`Tone`] (fill 1A, border 44, a
//! tone-coloured icon). A callout is static: it takes no input of its own and is not dismissible.

use std::hash::Hash;

use super::button::{ButtonOptions, button_width_with};
use super::tone::Tone;
use crate::access::{Live, Node, Role, Value};
use crate::icon;
use crate::paint::Rect;
use crate::text::{Spec, Style};
use crate::theme::Theme;
use crate::ui::{Response, Ui};

/// The corner radius and the border's width.
pub const RADIUS: f32 = 6.0;
pub const BORDER: f32 = 1.0;
/// Padding inside the border: vertical and horizontal.
pub const PAD_Y: f32 = 8.0;
pub const PAD_X: f32 = 10.0;
/// The icon's size, the slot it is centred in (as wide as the icon, one line high), and the gap
/// after the slot.
pub const ICON: f32 = 14.0;
pub const SLOT_W: f32 = 14.0;
pub const SLOT_H: f32 = 20.0;
pub const GAP: f32 = 8.0;
/// A text line.
pub const LINE: f32 = 20.0;
/// Between the text, the figure and the action.
pub const SIDE_GAP: f32 = 8.0;
/// Before the action: the side gap and the action's own margin.
pub const ACTION_GAP: f32 = 16.0;

/// What a callout shows.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CalloutOptions<'a> {
    pub tone: Tone,
    /// The glyph (an [`icon`] constant).
    pub icon: &'a str,
    pub text: &'a str,
    /// The one number the notice is about.
    pub figure: Option<&'a str>,
    /// The action button's label.
    pub action: Option<&'a str>,
    /// The notice appears after an action: a polite live region.
    pub announce: bool,
}

/// What [`Ui::callout`] returns.
#[derive(Debug, Clone, PartialEq)]
pub struct CalloutResponse {
    /// The height drawn.
    pub height: f32,
    /// The action button's response, when there is one.
    pub action: Option<Response>,
}

fn fill(theme: &Theme, tone: Tone) -> crate::color::Rgba {
    match tone {
        Tone::Neutral => theme.surface_input,
        t => t.fill(theme),
    }
}

impl Ui<'_> {
    /// A callout under `key` with its top left and width of `rect` (its height is ignored). The
    /// node is a `Note` named by the text, valued by the figure, a polite live region when
    /// `announce`; the action button's node is its child.
    pub fn callout(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &CalloutOptions<'_>,
    ) -> CalloutResponse {
        let id = self.id(&key);
        let t = *self.theme();
        let button = ButtonOptions {
            small: true,
            ..ButtonOptions::default()
        };
        let figure_style = Style {
            line: LINE,
            ..Style::mono(t.font_base, LINE, t.fg_bright).medium()
        };
        let figure = opts.figure.map(|f| Spec::plain(f, figure_style));
        let figure_w = figure.as_ref().map_or(0.0, |f| self.painter().measure(f).0);
        let action_w = opts
            .action
            .map_or(0.0, |a| button_width_with(self.painter(), a, &button));

        let left = rect.x + BORDER + PAD_X;
        let right = rect.right() - BORDER - PAD_X;
        let side = |w: f32, gap: f32| if w > 0.0 { w + gap } else { 0.0 };
        let text_x = left + SLOT_W + GAP;
        let text_w =
            (right - text_x - side(figure_w, SIDE_GAP) - side(action_w, ACTION_GAP)).max(0.0);
        let text_style = Style {
            line: LINE,
            ..Style::ui(t.font_base, t.fg_primary)
        };
        let text = Spec::plain(opts.text, text_style).wrapped(text_w);
        let text_h = self.painter().measure(&text).1.max(LINE);
        let action_h = if opts.action.is_some() {
            super::button::SMALL_HEIGHT
        } else {
            0.0
        };
        let content_h = text_h.max(action_h);
        let height = BORDER + PAD_Y + content_h + PAD_Y + BORDER;
        let bounds = Rect::new(rect.x, rect.y, rect.w, height);
        let top = rect.y + BORDER + PAD_Y;
        // A text shorter than the content (the action is taller) is centred with its icon.
        let text_top = top + (content_h - text_h) / 2.0;

        let p = self.painter();
        p.fill(bounds, RADIUS, fill(&t, opts.tone));
        p.ring(bounds, RADIUS, BORDER, opts.tone.border(&t), false);
        let icon_color = match opts.tone {
            Tone::Neutral => t.fg_tertiary,
            Tone::Data => t.data_hi,
            tone => tone.color(&t),
        };
        p.text_centered(
            &icon::glyph(opts.icon, ICON, icon_color),
            Rect::new(left, text_top, SLOT_W, SLOT_H),
        );
        p.text(&text, text_x, text_top);
        let mut x = right;
        let mut action_at = None;
        if let Some(label) = opts.action {
            x -= action_w;
            action_at = Some((
                label,
                Rect::new(x, top + (content_h - action_h) / 2.0, action_w, action_h),
            ));
            x -= ACTION_GAP;
        }
        if let Some(f) = &figure {
            x -= figure_w;
            p.text(f, x, top + (content_h - LINE) / 2.0);
        }

        let mut node = Node::new(id, Role::Note, opts.text, bounds);
        node.value = opts.figure.map(|f| Value::Text(f.to_owned()));
        node.live = opts.announce.then_some(Live::Polite);
        let action = self.scope(&key, |ui| {
            ui.node_container(node, |ui| {
                action_at.map(|(label, r)| ui.button_with("action", r, label, &button))
            })
        });
        CalloutResponse { height, action }
    }
}
