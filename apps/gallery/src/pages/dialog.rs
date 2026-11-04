//! The `dialog` page (stub: its content lands with step 17 of M4).

use crate::page::{Page, PageKind, UiPage};

use super::Stub;

pub(crate) static PAGE: Page = Page {
    id: "dialog",
    title: "Dialog",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Stub("Dialog")),
        scenes: &[],
        bench: &[],
    }),
};
