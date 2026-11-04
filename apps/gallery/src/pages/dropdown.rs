//! The `dropdown` page (stub: its content lands with step 18 of M4).

use crate::page::{Page, PageKind, UiPage};

use super::Stub;

pub(crate) static PAGE: Page = Page {
    id: "dropdown",
    title: "Dropdown",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Stub("Dropdown")),
        scenes: &[],
        bench: &[],
    }),
};
