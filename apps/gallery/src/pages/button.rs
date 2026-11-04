//! The `button` page (stub: its content lands with step 12 of M4).

use crate::page::{Page, PageKind, UiPage};

use super::Stub;

pub(crate) static PAGE: Page = Page {
    id: "button",
    title: "Button",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Stub("Button")),
        scenes: &[],
        bench: &[],
    }),
};
