//! Label text: the [`label`] spec helper for the widgets, and [`LabelRole`] with [`Ui::label`] for
//! static text (`label.md`). Static text takes no input and animates nothing.

use std::hash::Hash;

use crate::access::{Node, Role, Value};
use crate::color::Rgba;
use crate::paint::Rect;
use crate::text::{Font, Spec, Style};
use crate::theme::Theme;
use crate::ui::Ui;

/// UI-font text at the theme's base size.
pub fn label(theme: &Theme, text: &str, color: Rgba) -> Spec {
    Spec::plain(text, Style::ui(theme.font_base, color))
}

/// The text roles of `label.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelRole {
    /// 13 px, primary text.
    Base,
    /// 11 px, tertiary text.
    Small,
    /// 14 px Medium, primary text.
    Section,
    /// 18 px on a 24 px line, Medium, bright text.
    Title,
    /// The base size in the mono face.
    Mono,
    /// The base size, Medium, bright text.
    Strong,
}

impl LabelRole {
    /// The style the role draws with under `theme`.
    pub fn style(self, theme: &Theme) -> Style {
        match self {
            LabelRole::Base => Style::ui(theme.font_base, theme.fg_primary),
            LabelRole::Small => Style::ui(theme.font_s, theme.fg_tertiary),
            LabelRole::Section => Style::ui(theme.font_m, theme.fg_primary).medium(),
            LabelRole::Title => Style {
                line: theme.line_title,
                ..Style::ui(theme.font_title, theme.fg_bright)
            }
            .medium(),
            LabelRole::Mono => Style {
                font: Font::Mono,
                ..Style::ui(theme.font_base, theme.fg_primary)
            },
            LabelRole::Strong => Style::ui(theme.font_base, theme.fg_bright).medium(),
        }
    }
}

impl Ui<'_> {
    /// `text` in `role` with its top left at `at`, through the painter. Returns its size. The
    /// static-text node is named and valued by `text`.
    pub fn label(
        &mut self,
        key: impl Hash,
        at: (f32, f32),
        text: &str,
        role: LabelRole,
    ) -> (f32, f32) {
        let id = self.id(key);
        let style = role.style(self.theme());
        let (w, h) = self.painter().text(&Spec::plain(text, style), at.0, at.1);
        let mut node = Node::new(id, Role::StaticText, text, Rect::new(at.0, at.1, w, h));
        node.value = Some(Value::Text(text.to_owned()));
        self.node(node);
        (w, h)
    }
}
