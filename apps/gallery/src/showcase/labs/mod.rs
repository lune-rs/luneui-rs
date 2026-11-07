//! The labs: live pages for what no component page shows. Each is an ordinary Ui page, listed
//! in the sidebar's last group (`nav::GROUPS`) and in `PAGES`, so it draws on a stage, runs
//! headless and follows every global setting through the frame's theme, tier and clock.
//!
//! - glass: the window, panel and card layers over high-contrast content;
//! - blur: the backdrop blur over moving content;
//! - effects: the custom-draw slot, glass off, an unfocused window and the tab outline;
//! - motion: the fifteen kinds and the seven easings, each replayed on demand;
//! - tokens: every theme colour, font size and radius;
//! - clipboard: text copy and paste, on a press only;
//! - json: the bounded JSON reader on text typed in.
//!
//! Only the blur lab asks for a backdrop; the shell asks for none.

pub(crate) mod blur;
pub(crate) mod clipboard;
pub(crate) mod effects;
pub(crate) mod glass;
pub(crate) mod json;
pub(crate) mod motion;
pub(crate) mod tokens;

use lune_ui::color::Rgba;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;

/// `#rrggbb`, with the alpha as two more digits when the colour is not opaque.
pub(crate) fn hex(c: Rgba) -> String {
    if c[3] == 255 {
        format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
    } else {
        format!("#{:02x}{:02x}{:02x}{:02x}", c[0], c[1], c[2], c[3])
    }
}

/// `text` at (`x`, `y`) in `size` and `color`.
pub(crate) fn text(ui: &mut Ui<'_>, text: &str, x: f32, y: f32, size: f32, color: Rgba) {
    ui.painter()
        .text(&Spec::plain(text, Style::ui(size, color)), x, y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_reads_as_hex_with_its_alpha_only_when_it_has_one() {
        assert_eq!(hex([0x1f, 0xa0, 0x05, 255]), "#1fa005");
        assert_eq!(hex([0, 0, 0, 0x72]), "#00000072");
    }
}

/// What the labs' draw tests share: a lab drawn on a stage the size of the shell's canvas.
#[cfg(test)]
pub(crate) mod rig {
    use lune_ui::motion::MotionTier;
    use lune_ui::paint::Rect;
    use lune_ui::theme::Theme;
    use lune_ui::ui::Ui;

    use crate::page::{PageState, Player, Stage};

    /// The canvas of a hosted page: the card of a page alone in a 1,040 x 660 window.
    pub(crate) const CANVAS: Rect = Rect::new(10.0, 34.0, 1020.0, 616.0);

    /// A lab on the canvas of a 1,600 x 900 window.
    pub(crate) fn player(id: &'static str, state: Box<dyn PageState>) -> Player {
        let mut p = Player::new(id, state, (1600.0, 900.0), 1.0, MotionTier::Full);
        p.set_stage(Stage::at(CANVAS));
        p
    }

    /// A page that runs under `theme`, as the shell's settings make it.
    pub(crate) struct Themed(pub(crate) Box<dyn PageState>, pub(crate) Theme);

    impl PageState for Themed {
        fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
            self.0.draw(ui, stage);
        }

        fn theme(&self) -> Option<Theme> {
            Some(self.1)
        }
    }
}
