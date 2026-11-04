//! The `label` page (stub: its content lands with step 13 of M4).

use crate::page::{Page, PageKind, UiPage};

use super::Stub;

pub(crate) static PAGE: Page = Page {
    id: "label",
    title: "Label",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Stub("Label")),
        scenes: &[],
        bench: &[],
    }),
};
