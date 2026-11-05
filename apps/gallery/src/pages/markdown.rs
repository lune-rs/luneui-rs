//! The `markdown-preview` page: the block model drawn in a window. A stub until its step: it draws the empty card.

use lune_ui::ui::Ui;

use super::card;
use crate::page::{Page, PageKind, PageState, UiPage};

pub(crate) static PAGE: Page = Page {
    id: "markdown-preview",
    title: "Markdown preview",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: &[],
        bench: &[],
    }),
};

struct Empty;

fn live() -> Box<dyn PageState> {
    Box::new(Empty)
}

impl PageState for Empty {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        card(ui, "Markdown preview");
    }
}
