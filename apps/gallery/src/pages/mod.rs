//! The page registry. Legacy pages first (they keep the Painter API and the pixel gate's 15
//! scenes), then the Ui pages in the order they are built. Adding a page is its own file plus one
//! line in [`PAGES`].

use lune_ui::lunegit_dark::THEME;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;

use crate::page::Page;
use crate::scenes::background;

mod a11y;
mod button;
mod checkbox;
mod combo;
mod controls_ui;
mod dialog;
mod dropdown;
pub(crate) mod effects;
mod focus;
mod label;
mod locale;
mod menu;
mod motion;
pub(crate) mod reference;
mod stepper;
mod text_field;
mod toggle;
mod ui;

/// Every page, in listing and window order.
pub(crate) static PAGES: &[&Page] = &[
    &reference::PAGE,
    &effects::PAGE,
    &button::PAGE,
    &toggle::PAGE,
    &checkbox::PAGE,
    &label::PAGE,
    &stepper::PAGE,
    &combo::PAGE,
    &text_field::PAGE,
    &dialog::PAGE,
    &dropdown::PAGE,
    &menu::PAGE,
    &motion::PAGE,
    &ui::PAGE,
    &focus::PAGE,
    &a11y::PAGE,
    &locale::PAGE,
];

/// The window background of a Ui page (glass, focused) and its card. Returns the card.
pub(crate) fn card(ui: &mut Ui<'_>, title: &str) -> Rect {
    background(ui.painter(), true, true, title)
}

/// A small section heading at (`x`, `y`).
pub(crate) fn heading(ui: &mut Ui<'_>, text: &str, x: f32, y: f32) {
    let spec = Spec::plain(text, Style::ui(THEME.font_s, THEME.fg_tertiary).medium());
    ui.painter().text(&spec, x, y);
}

/// A small caption at (`x`, `y`).
pub(crate) fn caption(ui: &mut Ui<'_>, text: &str, x: f32, y: f32) {
    let spec = Spec::plain(text, Style::ui(THEME.font_s, THEME.fg_secondary));
    ui.painter().text(&spec, x, y);
}
