//! The `toggle` page (stub: its content lands with step 12 of M4).

use crate::page::{Page, PageKind, UiPage};

use super::Stub;

pub(crate) static PAGE: Page = Page {
    id: "toggle",
    title: "Toggle",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Stub("Toggle")),
        scenes: &[],
        bench: &[],
    }),
};
