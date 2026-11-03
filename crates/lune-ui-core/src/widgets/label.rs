//! Label text spec.

use crate::color::Rgba;
use crate::text::{Spec, Style};
use crate::theme::Theme;

/// UI-font text at the theme's base size.
pub fn label(theme: &Theme, text: &str, color: Rgba) -> Spec {
    Spec::plain(text, Style::ui(theme.font_base, color))
}
