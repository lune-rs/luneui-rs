//! The per-window `Ui` frame: stable widget ids and, as later steps land, the retained store,
//! layered hit testing, focus scopes, `Response` and the frame it returns. This module holds the
//! ids for now; the rest of the frame arrives with its own steps.

mod id;

pub use id::WidgetId;
