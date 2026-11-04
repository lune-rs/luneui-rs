//! The `motion` page (stub: its content lands with step 19 of M4).

use crate::page::{Page, PageKind, UiPage};

use super::Stub;

pub(crate) static PAGE: Page = Page {
    id: "motion",
    title: "Motion",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Stub("Motion")),
        scenes: &[],
        bench: &[],
    }),
};
