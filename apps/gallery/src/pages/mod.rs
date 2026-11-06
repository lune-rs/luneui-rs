//! The page registry. Legacy pages first (they keep the Painter API and the pixel gate's 15
//! scenes), then the Ui pages in the order they are built. Adding a page is its own file plus one
//! line in [`PAGES`].

use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;

use crate::page::Page;
use crate::scenes::background_in;

mod a11y;
mod badge;
mod banner;
mod bar_chart;
mod button;
mod callout;
mod checkbox;
mod code_block;
mod code_editor;
mod combo;
mod command_palette;
mod controls_ui;
mod dialog;
mod diff_view;
mod drag_drop;
mod dropdown;
pub(crate) mod effects;
mod file_drop;
mod filter_bar;
mod flyout;
mod focus;
mod form_field;
mod icon_rail;
mod image;
mod ime;
mod kanban_board;
mod kv_list;
mod label;
mod lane_graph;
mod layout_page;
mod list_row;
mod locale;
mod markdown;
mod menu;
mod motion;
mod nav_item;
mod panel;
mod pie_chart;
mod popover;
mod primitives;
mod progress;
mod radio_group;
pub(crate) mod reference;
mod row;
mod scroll_area;
mod scrollbar;
mod search_picker;
mod settings_page;
mod shortcuts;
mod side_panel;
mod sidebar;
mod slider;
mod spinner;
mod split_panes;
mod stack;
mod star_rating;
mod stat;
mod status_dot;
mod stepper;
mod steps;
mod tab_strip;
mod table;
mod text_edit;
mod text_field;
mod theme;
mod toast_notifications;
mod toggle;
mod toolbar;
mod tooltip;
mod tray;
mod tree_view;
mod ui;
mod verdict_row;
mod virtual_list;
mod window;

pub(crate) use tray::{SurfaceFrame, draw_surface, surface_size};

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
    &stack::PAGE,
    &image::PAGE,
    &shortcuts::PAGE,
    &primitives::PAGE,
    &scroll_area::PAGE,
    &scrollbar::PAGE,
    &virtual_list::PAGE,
    &popover::PAGE,
    &tooltip::PAGE,
    &text_edit::PAGE,
    &ime::PAGE,
    &drag_drop::PAGE,
    &badge::PAGE,
    &status_dot::PAGE,
    &verdict_row::PAGE,
    &progress::PAGE,
    &spinner::PAGE,
    &callout::PAGE,
    &stat::PAGE,
    &kv_list::PAGE,
    &row::PAGE,
    &list_row::PAGE,
    &form_field::PAGE,
    &steps::PAGE,
    &nav_item::PAGE,
    &radio_group::PAGE,
    &star_rating::PAGE,
    &slider::PAGE,
    &filter_bar::PAGE,
    &search_picker::PAGE,
    &tab_strip::PAGE,
    &toolbar::PAGE,
    &banner::PAGE,
    &flyout::PAGE,
    &toast_notifications::PAGE,
    &command_palette::PAGE,
    &layout_page::PAGE,
    &panel::PAGE,
    &sidebar::PAGE,
    &side_panel::PAGE,
    &icon_rail::PAGE,
    &split_panes::PAGE,
    &tree_view::PAGE,
    &settings_page::PAGE,
    &table::PAGE,
    &bar_chart::PAGE,
    &pie_chart::PAGE,
    &kanban_board::PAGE,
    &lane_graph::PAGE,
    &code_block::PAGE,
    &code_editor::PAGE,
    &markdown::PAGE,
    &diff_view::PAGE,
    &theme::PAGE,
    &tray::PAGE,
    &file_drop::PAGE,
    &window::PAGE,
];

/// The window background of a Ui page (glass, focused) and its card. Returns the card.
pub(crate) fn card(ui: &mut Ui<'_>, title: &str) -> Rect {
    let theme = *ui.theme();
    background_in(ui.painter(), &theme, true, true, title)
}

/// A small section heading at (`x`, `y`).
pub(crate) fn heading(ui: &mut Ui<'_>, text: &str, x: f32, y: f32) {
    let t = *ui.theme();
    let spec = Spec::plain(text, Style::ui(t.font_s, t.fg_tertiary).medium());
    ui.painter().text(&spec, x, y);
}

/// A small caption at (`x`, `y`).
pub(crate) fn caption(ui: &mut Ui<'_>, text: &str, x: f32, y: f32) {
    let t = *ui.theme();
    let spec = Spec::plain(text, Style::ui(t.font_s, t.fg_secondary));
    ui.painter().text(&spec, x, y);
}
