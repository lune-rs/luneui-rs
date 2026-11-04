//! The `combo` page (stub: its content lands with step 15 of M4).

use crate::page::{Page, PageKind, UiPage};

use super::Stub;

pub(crate) static PAGE: Page = Page {
    id: "combo",
    title: "Combo",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Stub("Combo")),
        scenes: &[],
        bench: &[],
    }),
};
