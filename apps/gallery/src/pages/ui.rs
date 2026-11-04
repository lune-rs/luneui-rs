//! The `ui` page (stub: its content lands with step 20 of M4).

use crate::page::{Page, PageKind, UiPage};

use super::Stub;

pub(crate) static PAGE: Page = Page {
    id: "ui",
    title: "Ui",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Stub("Ui")),
        scenes: &[],
        bench: &[],
    }),
};
