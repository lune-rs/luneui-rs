//! Codicons, as mapped into the Nerd Font (the house look uses the Codicons icon pack). Icons are drawn
//! with the mono font role.

use crate::color::Rgba;
use crate::text::{Spec, Style};

pub const ADD: &str = "\u{ea60}";
/// `cod-remove`, a minus.
pub const REMOVE: &str = "\u{eb3b}";
pub const GEAR: &str = "\u{eb51}";
pub const ELLIPSIS: &str = "\u{ea7c}";
pub const CHROME_CLOSE: &str = "\u{eab8}";
pub const EDIT: &str = "\u{ea73}";
pub const COPY: &str = "\u{ebcc}";
pub const INFO: &str = "\u{ea74}";
pub const CHEVRON_DOWN: &str = "\u{eab4}";
pub const ARROW_LEFT: &str = "\u{ea9b}";
pub const CHECK: &str = "\u{eab2}";
pub const CHROME_MINIMIZE: &str = "\u{eaba}";
pub const CHROME_MAXIMIZE: &str = "\u{eab9}";
pub const CHROME_RESTORE: &str = "\u{eabb}";
pub const SEARCH: &str = "\u{ea6d}";
pub const CALENDAR: &str = "\u{eab0}";
pub const SYNC: &str = "\u{ea77}";
pub const STAR_FULL: &str = "\u{eb59}";
pub const STAR_EMPTY: &str = "\u{ea6a}";
pub const CLOSE: &str = "\u{ea76}";
pub const CHEVRON_RIGHT: &str = "\u{eab6}";
pub const WARNING: &str = "\u{ea6c}";
pub const ERROR: &str = "\u{ea87}";
pub const CLEAR_ALL: &str = "\u{eabf}";
pub const CIRCLE_SLASH: &str = "\u{eabd}";
pub const ARROW_UP: &str = "\u{eaa1}";
pub const ARROW_DOWN: &str = "\u{ea9a}";
pub const ACCOUNT: &str = "\u{eb99}";
pub const NOTE: &str = "\u{eb26}";

/// Text spec for one icon glyph at `size`.
pub fn glyph(glyph: &str, size: f32, color: Rgba) -> Spec {
    Spec::plain(glyph, Style::mono(size, size * 1.25, color))
}
