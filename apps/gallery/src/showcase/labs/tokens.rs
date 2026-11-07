//! The tokens lab: every colour of the running theme with its name and value, and its font sizes
//! and radii. The theme is the frame's, so a theme chosen in the settings pane (or an edit of a
//! glass layer) shows here at once.

use lune_ui::color::Rgba;
use lune_ui::paint::Rect;
use lune_ui::theme::Theme;
use lune_ui::ui::Ui;

use super::{hex, text};
use crate::page::{Page, PageKind, PageState, Stage, UiPage};
use crate::pages::{card, heading};

pub(crate) static PAGE: Page = Page {
    id: "lab-tokens",
    title: "Tokens lab",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: &[],
        bench: &[],
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Tokens)
}

/// Every colour field of [`Theme`], by name (a test holds the list to the struct).
macro_rules! colours {
    ($($field:ident),+ $(,)?) => {
        /// The theme's colour fields with their values.
        fn colours(t: &Theme) -> Vec<(&'static str, Rgba)> {
            vec![$((stringify!($field), t.$field)),+]
        }
    };
}

colours! {
    fg_primary, fg_secondary, fg_tertiary, surface_input, surface_hover, surface_selected,
    border, border_subtle, accent, accent_hover, scroll_thumb, scroll_thumb_hover,
    text_selection, on_accent, on_accent_text, dialog_backdrop, dialog_footer,
    menu_text_primary, menu_text_tertiary, menu_text_quaternary, menu_popup_border,
    menu_item_hover, menu_separator, menu_glass, menu_item_pressed, menu_text_disabled,
    toast_bg, toast_border, toast_info, toast_success, toast_warning, toast_error,
    toast_log_bg, toast_log_fg, dirty_indicator, count_badge, caption_close_hover,
    caption_close_fg, fg_muted, fg_subtle, fg_bright, fg_quaternary, fg_dim, fg_faint,
    surface_overlay, surface_active, surface_separator, surface_divider, ok, warn, err,
    status_warning, status_warning_light, status_error_light, status_error,
    status_warning_dark, data, data_hi, gold, paused, window_fallback, link, code_inline,
    diff_added, diff_removed, diff_added_mark, diff_removed_mark,
}

/// Every colour of `t` in the order shown: the colour fields, then the tints of the glass layers.
fn rows(t: &Theme) -> Vec<(String, Rgba)> {
    let mut out: Vec<(String, Rgba)> = colours(t)
        .into_iter()
        .map(|(name, c)| (name.to_owned(), c))
        .collect();
    out.push(("glass.window.tint".to_owned(), t.glass.window.tint));
    out.push(("glass.panel.tint".to_owned(), t.glass.panel.tint));
    out.push(("glass.card.tint".to_owned(), t.glass.card.tint));
    out
}

/// The colours' columns: how many, how wide, how tall a row is.
const COLS: usize = 4;
const COL_W: f32 = 238.0;
const ROW_H: f32 = 22.0;
/// From a column's left edge to the hex value.
const VALUE_AT: f32 = 170.0;
const SWATCH: (f32, f32) = (28.0, 16.0);
/// Where the colours begin, below the sizes.
const COLOURS_AT: f32 = 136.0;

struct Tokens;

impl PageState for Tokens {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Tokens lab");
        let t = *ui.theme();
        let (x, y) = (card.x + 24.0, card.y + 20.0);

        heading(ui, "FONT SIZES AND LINE", x, y);
        let sizes = [
            ("font_s", t.font_s),
            ("font_base", t.font_base),
            ("font_m", t.font_m),
            ("font_title", t.font_title),
        ];
        for (i, (name, size)) in sizes.iter().enumerate() {
            let line = format!("{name} {size}");
            text(
                ui,
                &line,
                x + i as f32 * 190.0,
                y + 22.0,
                *size,
                t.fg_primary,
            );
        }
        text(
            ui,
            &format!("line_title {}", t.line_title),
            x + 4.0 * 190.0,
            y + 22.0,
            t.font_s,
            t.fg_secondary,
        );

        heading(ui, "RADII", x, y + 58.0);
        let radii = [
            ("item_radius", t.item_radius),
            ("card radius", lune_ui::chrome::CARD_RADIUS),
        ];
        for (i, (name, radius)) in radii.iter().enumerate() {
            let rx = x + i as f32 * 190.0;
            let r = Rect::new(rx, y + 78.0, 56.0, 28.0);
            let p = ui.painter();
            p.fill(r, *radius, t.surface_selected);
            p.ring(r, *radius, 1.0, t.border, false);
            text(
                ui,
                &format!("{name} {radius}"),
                rx + 64.0,
                y + 85.0,
                t.font_s,
                t.fg_secondary,
            );
        }

        heading(ui, "COLOURS", x, y + COLOURS_AT - 22.0);
        let all = rows(&t);
        let per_col = all.len().div_ceil(COLS);
        for (i, (name, colour)) in all.iter().enumerate() {
            let (col, row) = (i / per_col, i % per_col);
            let cx = x + col as f32 * COL_W;
            let cy = y + COLOURS_AT + row as f32 * ROW_H;
            let swatch = Rect::new(cx, cy, SWATCH.0, SWATCH.1);
            let p = ui.painter();
            p.fill(swatch, 3.0, *colour);
            p.ring(swatch, 3.0, 1.0, t.border_subtle, false);
            text(ui, name, cx + SWATCH.0 + 8.0, cy, t.font_s, t.fg_primary);
            text(
                ui,
                &hex(*colour),
                cx + VALUE_AT,
                cy,
                t.font_s,
                t.fg_tertiary,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::rig::{Themed, player};
    use super::*;
    use lune_ui::lunegit_dark::THEME;
    use lune_ui::scene::LayerId;
    use lune_ui::testing::FakeText;
    use std::collections::BTreeSet;

    /// The `Rgba` fields of `pub struct Theme`, read from its source.
    fn colour_fields_of_the_struct() -> Vec<String> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/lune-ui-core/src/theme.rs");
        let source = std::fs::read_to_string(path).expect("theme.rs");
        let body = source
            .split("pub struct Theme {")
            .nth(1)
            .and_then(|rest| rest.split("\n}").next())
            .expect("the struct");
        body.lines()
            .filter_map(|l| {
                let l = l.trim();
                let rest = l.strip_prefix("pub ")?;
                let (name, ty) = rest.split_once(": ")?;
                (ty.trim_end_matches(',') == "Rgba").then(|| name.to_owned())
            })
            .collect()
    }

    /// The list is every colour of the struct, none twice and none made up. The reader finds the
    /// struct's 67 colours (a count that holds the instrument, not only the comparison).
    #[test]
    fn every_colour_of_the_theme_is_listed() {
        let in_struct = colour_fields_of_the_struct();
        assert_eq!(in_struct.len(), 67, "the reader of the struct");
        let listed: Vec<&str> = colours(&THEME).iter().map(|(n, _)| *n).collect();
        let (a, b): (BTreeSet<&str>, BTreeSet<&str>) = (
            in_struct.iter().map(String::as_str).collect(),
            listed.iter().copied().collect(),
        );
        assert_eq!(listed.len(), b.len(), "a colour twice");
        assert_eq!(a, b);
    }

    #[test]
    fn the_values_are_the_running_themes() {
        let mut theme = THEME;
        theme.accent = [1, 2, 3, 255];
        theme.glass.card.tint = [9, 9, 9, 255];
        let all = rows(&theme);
        let value = |name: &str| all.iter().find(|(n, _)| n == name).map(|(_, c)| *c);
        assert_eq!(value("accent"), Some([1, 2, 3, 255]));
        assert_eq!(value("glass.card.tint"), Some([9, 9, 9, 255]));
        assert_eq!(value("fg_primary"), Some(THEME.fg_primary));
    }

    #[test]
    fn the_columns_fit_the_card_and_every_row_is_drawn() {
        let per_col = rows(&THEME).len().div_ceil(COLS);
        // The last row ends above the card's bottom edge (616 high, 20 of margin).
        assert!(
            20.0 + COLOURS_AT + per_col as f32 * ROW_H < 616.0,
            "{per_col} rows"
        );
        assert!(COLS as f32 * COL_W <= 960.0);
        let mut p = player("lab-tokens", Box::new(Themed(live(), THEME)));
        p.frame(&mut FakeText, 0, &[]).expect("frame");
        let (scene, _) = p.frame(&mut FakeText, 16, &[]).expect("frame");
        let page = &scene.layers[LayerId::Base as usize].geometry;
        let n = rows(&THEME).len();
        assert!(
            page.under.len() >= n,
            "{} fills for {n} swatches",
            page.under.len()
        );
        assert!(
            page.over.len() + page.under.len() >= 2 * n,
            "swatches and their rings"
        );
    }
}
