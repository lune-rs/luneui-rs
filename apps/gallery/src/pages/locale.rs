//! The `locale` page (stub: its content lands with step 20 of M4).

use crate::page::{Page, PageKind, UiPage};

use super::Stub;

pub(crate) static PAGE: Page = Page {
    id: "locale",
    title: "Locale",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Stub("Locale")),
        scenes: &[],
        bench: &[],
    }),
};
