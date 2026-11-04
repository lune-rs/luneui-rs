//! The `menu` page (stub: its content lands with step 18 of M4).

use crate::page::{Page, PageKind, UiPage};

use super::Stub;

pub(crate) static PAGE: Page = Page {
    id: "menu",
    title: "Menu",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Stub("Menu")),
        scenes: &[],
        bench: &[],
    }),
};
