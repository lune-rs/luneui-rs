//! The `bar-chart` page: a stub the widget's step fills (live state, scenes and bench).

use lune_ui::ui::Ui;

use super::card;
use crate::page::{Page, PageKind, PageState, UiPage};

pub(crate) static PAGE: Page = Page {
    id: "bar-chart",
    title: "Bar chart",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: &[],
        bench: &[],
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Live)
}

/// The empty card until the widget lands.
struct Live;

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        card(ui, "Bar chart");
    }
}
