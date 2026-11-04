//! The `stepper` page (stub: its content lands with step 14 of M4).

use crate::page::{Page, PageKind, UiPage};

use super::Stub;

pub(crate) static PAGE: Page = Page {
    id: "stepper",
    title: "Stepper",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Stub("Stepper")),
        scenes: &[],
        bench: &[],
    }),
};
