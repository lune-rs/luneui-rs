//! The one tone recipe of the status widgets: badge, status dot, verdict row, callout and
//! progress read their colours here, so a tone looks the same everywhere. Which domain status
//! maps to which tone is the app's choice.

use crate::color::Rgba;
use crate::theme::Theme;

/// Alpha of a tone's fill (the badge recipe: fill at 1A).
pub const FILL_ALPHA: u8 = 0x1A;
/// Alpha of a tone's border (the badge recipe: border at 44).
pub const BORDER_ALPHA: u8 = 0x44;

/// A status meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tone {
    /// No status: the overlay fill, the subtle border, secondary text.
    #[default]
    Neutral,
    Ok,
    Warn,
    Err,
    Data,
    /// The neutral status dot (`paused`).
    Paused,
}

impl Tone {
    /// The tone's own colour: a dot's fill, a bar's fill, a glyph.
    pub fn color(self, theme: &Theme) -> Rgba {
        match self {
            Tone::Neutral => theme.fg_secondary,
            Tone::Ok => theme.ok,
            Tone::Warn => theme.warn,
            Tone::Err => theme.err,
            Tone::Data => theme.data,
            Tone::Paused => theme.paused,
        }
    }

    /// The fill of a tag in this tone: the colour at alpha 0x1A; neutral is `surface_overlay`.
    pub fn fill(self, theme: &Theme) -> Rgba {
        match self {
            Tone::Neutral => theme.surface_overlay,
            _ => with_alpha_byte(self.color(theme), FILL_ALPHA),
        }
    }

    /// The border of a tag in this tone: the colour at alpha 0x44; neutral is `border_subtle`.
    pub fn border(self, theme: &Theme) -> Rgba {
        match self {
            Tone::Neutral => theme.border_subtle,
            _ => with_alpha_byte(self.color(theme), BORDER_ALPHA),
        }
    }

    /// The text of a tag in this tone: the colour, except `Data`, which reads `data_hi`.
    pub fn text(self, theme: &Theme) -> Rgba {
        match self {
            Tone::Data => theme.data_hi,
            _ => self.color(theme),
        }
    }
}

fn with_alpha_byte(c: Rgba, a: u8) -> Rgba {
    [c[0], c[1], c[2], a]
}
