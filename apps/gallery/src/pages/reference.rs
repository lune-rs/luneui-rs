//! The `reference` page: widgets, menus, dropdowns and dialogs, drawn through the Painter API.

use crate::page::{Page, PageKind};
use crate::scenes::REFERENCE;

pub(crate) static PAGE: Page = Page {
    id: "reference",
    title: "Reference",
    kind: PageKind::Legacy(REFERENCE),
};
