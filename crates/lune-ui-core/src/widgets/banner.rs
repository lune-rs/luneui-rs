//! A banner and an empty state that take the caller's own text (`widget.banner-empty-state`).
//!
//! [`Ui::banner`] is a warning strip: a solid dark fill, a 1 px light border, a spinning glyph and
//! a wrapped line, all in the warning light colour. [`Ui::empty_state`] is a centred column: a
//! 100 by 100 picture the app registered, a title, a centred subtitle and a row of secondary
//! buttons. Texts and commands are the app's (D128); the toolkit bundles no picture.

use std::hash::Hash;

use super::button::{ButtonKind, button_width};
use super::spinner::SpinnerOptions;
use crate::access::{Live, Node, Role, State};
use crate::image::{Fit, ImageAlign, ImageId};
use crate::paint::{Painter, Rect};
use crate::text::{Spec, Style};
use crate::ui::Ui;

/// The banner's corner radius and border width.
pub const BANNER_RADIUS: f32 = 6.0;
pub const BANNER_BORDER: f32 = 1.0;
/// The banner's padding, inside the border.
pub const BANNER_PAD_X: f32 = 10.0;
pub const BANNER_PAD_Y: f32 = 8.0;
/// The banner's glyph size and the gap between it and the text.
pub const BANNER_ICON: f32 = 14.0;
pub const BANNER_GAP: f32 = 8.0;

/// The empty view's column: the widest it gets, the space between its parts, the picture's side,
/// the action buttons' height and the space between them.
pub const EMPTY_MAX_WIDTH: f32 = 520.0;
pub const EMPTY_SPACING: f32 = 24.0;
pub const EMPTY_IMAGE: f32 = 100.0;
pub const EMPTY_ACTION_HEIGHT: f32 = 32.0;
pub const EMPTY_ACTION_GAP: f32 = 12.0;

/// What [`Ui::empty_state`] shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmptyState<'a> {
    /// A picture registered with the renderer; `None` draws no picture.
    pub image: Option<ImageId>,
    pub title: &'a str,
    /// Wrapped and centred; an empty one is left out.
    pub subtitle: &'a str,
    /// The action buttons' labels, left to right.
    pub actions: &'a [&'a str],
}

/// The text width left in a banner of `width`.
fn banner_text_width(width: f32) -> f32 {
    let inset = 2.0 * (BANNER_BORDER + BANNER_PAD_X) + BANNER_ICON + BANNER_GAP;
    (width - inset).max(0.0)
}

fn banner_style(p: &Painter<'_, impl Copy + PartialEq>) -> Style {
    Style::ui(p.theme.font_base, p.theme.status_warning_light)
}

/// The height [`Ui::banner`] takes for `text` at `width`.
pub fn banner_height<H: Copy + PartialEq>(p: &mut Painter<'_, H>, width: f32, text: &str) -> f32 {
    let spec = Spec::plain(text, banner_style(p)).wrapped(banner_text_width(width));
    let text_h = p.measure(&spec).1;
    text_h.max(BANNER_ICON) + 2.0 * (BANNER_PAD_Y + BANNER_BORDER)
}

/// Draws `text` wrapped to `w`, each visual line centred in `x..x + w`, from `y`; returns the
/// height. The text system has no alignment, so every row is drawn on its own.
fn centered_lines<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    text: &str,
    style: Style,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let spec = Spec::plain(text, style).wrapped(w);
    let height = p.measure(&spec).1;
    for line in p.text_lines(&spec) {
        let Some(row) = text.get(line.start..line.end).map(str::trim) else {
            continue;
        };
        if row.is_empty() {
            continue;
        }
        let row = Spec::plain(row, style);
        let rw = p.measure(&row).0;
        p.text(&row, x + (w - rw) / 2.0, y + line.y);
    }
    height
}

impl Ui<'_> {
    /// A banner under `key` at the top left of `rect`, `rect.w` wide: the warning strip with a
    /// glyph (turning while `busy`) and `text` wrapped beside it. Returns its height. The node is
    /// `Status`, a polite live region, named `text`; it takes no input.
    pub fn banner(&mut self, key: impl Hash, rect: Rect, text: &str, busy: bool) -> f32 {
        let id = self.id(&key);
        let theme = *self.theme();
        let height = banner_height(self.painter(), rect.w, text);
        let bar = Rect::new(rect.x, rect.y, rect.w, height);
        let inset = BANNER_BORDER + BANNER_PAD_X;
        let spec =
            Spec::plain(text, banner_style(self.painter())).wrapped(banner_text_width(rect.w));
        let text_h = self.painter().measure(&spec).1;
        let p = self.painter();
        p.fill(bar, BANNER_RADIUS, theme.status_warning_dark);
        p.ring(
            bar,
            BANNER_RADIUS,
            BANNER_BORDER,
            theme.status_warning_light,
            false,
        );
        let text_x = bar.x + inset + BANNER_ICON + BANNER_GAP;
        p.text(&spec, text_x, bar.y + (height - text_h) / 2.0);

        let mut node = Node::new(id, Role::Status, text, bar);
        node.live = Some(Live::Polite);
        let centre = (bar.x + inset + BANNER_ICON / 2.0, bar.y + height / 2.0);
        self.node_container(node, |ui| {
            ui.scope(&key, |ui| {
                ui.spinner(
                    "spinner",
                    centre,
                    &SpinnerOptions {
                        busy,
                        size: BANNER_ICON,
                        color: Some(theme.status_warning_light),
                        name: "",
                    },
                );
            });
        });
        height
    }

    /// The empty view under `key`, centred in `rect`: the column of `state`, at most 520 wide.
    /// Returns the index of the action clicked this frame, if any. The picture's node is `Image`
    /// and hidden (the view carries no name for it); the title and subtitle are `StaticText`;
    /// the actions are secondary buttons named by their labels.
    pub fn empty_state(
        &mut self,
        key: impl Hash,
        rect: Rect,
        state: &EmptyState<'_>,
    ) -> Option<usize> {
        let id = self.id(&key);
        let theme = *self.theme();
        let col_w = rect.w.min(EMPTY_MAX_WIDTH);
        let x = rect.x + (rect.w - col_w) / 2.0;

        let title_style = Style {
            line: theme.line_title,
            ..Style::ui(theme.font_title, theme.fg_primary)
        };
        let sub_style = Style::ui(theme.font_base, theme.fg_secondary);
        let p = self.painter();
        let title_h = p
            .measure(&Spec::plain(state.title, title_style).wrapped(col_w))
            .1;
        let sub_h = if state.subtitle.is_empty() {
            0.0
        } else {
            p.measure(&Spec::plain(state.subtitle, sub_style).wrapped(col_w))
                .1
        };
        let widths: Vec<f32> = state.actions.iter().map(|a| button_width(p, a)).collect();

        let mut parts = vec![title_h];
        if state.image.is_some() {
            parts.insert(0, EMPTY_IMAGE);
        }
        if sub_h > 0.0 {
            parts.push(sub_h);
        }
        if !widths.is_empty() {
            parts.push(EMPTY_ACTION_HEIGHT);
        }
        let total = parts.iter().sum::<f32>() + EMPTY_SPACING * (parts.len() - 1) as f32;
        let mut y = rect.y + ((rect.h - total) / 2.0).max(0.0);

        let mut clicked = None;
        self.scope(&key, |ui| {
            if let Some(image) = state.image {
                let dst = Rect::new(x + (col_w - EMPTY_IMAGE) / 2.0, y, EMPTY_IMAGE, EMPTY_IMAGE);
                ui.painter()
                    .image(dst, image, Fit::Contain, ImageAlign::CENTER, 1.0);
                let mut node = Node::new(id.with("image"), Role::Image, "", dst);
                node.states.insert(State::Hidden);
                ui.node(node);
                y += EMPTY_IMAGE + EMPTY_SPACING;
            }
            let bounds = Rect::new(x, y, col_w, title_h);
            centered_lines(ui.painter(), state.title, title_style, x, y, col_w);
            ui.node(Node::new(
                id.with("title"),
                Role::StaticText,
                state.title,
                bounds,
            ));
            y += title_h + EMPTY_SPACING;
            if sub_h > 0.0 {
                let bounds = Rect::new(x, y, col_w, sub_h);
                centered_lines(ui.painter(), state.subtitle, sub_style, x, y, col_w);
                ui.node(Node::new(
                    id.with("subtitle"),
                    Role::StaticText,
                    state.subtitle,
                    bounds,
                ));
                y += sub_h + EMPTY_SPACING;
            }
            let row = widths.iter().sum::<f32>()
                + EMPTY_ACTION_GAP * widths.len().saturating_sub(1) as f32;
            let mut bx = x + (col_w - row) / 2.0;
            for (i, (label, w)) in state.actions.iter().zip(&widths).enumerate() {
                let r = Rect::new(bx, y, *w, EMPTY_ACTION_HEIGHT);
                let resp = ui.button(format!("action-{i}"), r, label, ButtonKind::Secondary);
                if resp.clicked && clicked.is_none() {
                    clicked = Some(i);
                }
                bx += w + EMPTY_ACTION_GAP;
            }
        });
        clicked
    }
}
