//! The `checkbox` page (stub: its content lands with step 13 of M4).

use crate::page::{Page, PageKind, UiPage};

use super::Stub;

pub(crate) static PAGE: Page = Page {
    id: "checkbox",
    title: "Checkbox",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Stub("Checkbox")),
        scenes: &[],
        bench: &[],
    }),
};
