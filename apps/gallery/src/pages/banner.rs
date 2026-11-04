//! The `banner` page: a stub the widget step fills (live state, scenes and bench).

use lune_ui::ui::Ui;

use super::card;
use crate::page::{Page, PageKind, PageState, UiPage};

pub(crate) static PAGE: Page = Page {
    id: "banner",
    title: "Banner and empty state",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Empty),
        scenes: &[],
        bench: &[],
    }),
};

/// Draws the empty card until the widget lands.
struct Empty;

impl PageState for Empty {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        card(ui, "Banner and empty state");
    }
}
