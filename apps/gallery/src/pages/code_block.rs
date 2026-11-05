//! The `code-block` page: the read-only code view with a copy button. A stub until its step: it draws the empty card.

use lune_ui::ui::Ui;

use super::card;
use crate::page::{Page, PageKind, PageState, UiPage};

pub(crate) static PAGE: Page = Page {
    id: "code-block",
    title: "Code block",
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
        card(ui, "Code block");
    }
}
