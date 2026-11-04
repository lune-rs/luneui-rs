//! The `a11y` page (stub: its content lands with step 20 of M4).

use crate::page::{Page, PageKind, UiPage};

use super::Stub;

pub(crate) static PAGE: Page = Page {
    id: "a11y",
    title: "Accessibility",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Stub("Accessibility")),
        scenes: &[],
        bench: &[],
    }),
};
