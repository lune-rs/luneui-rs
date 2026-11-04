//! The `text-field` page (stub: its content lands with step 16 of M4).

use crate::page::{Page, PageKind, UiPage};

use super::Stub;

pub(crate) static PAGE: Page = Page {
    id: "text-field",
    title: "Text field",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Stub("Text field")),
        scenes: &[],
        bench: &[],
    }),
};
