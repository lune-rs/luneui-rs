//! Key routing: Escape and Tab are handled at begin; every other key press waits in the frame's
//! queue for the focused widget of the active scope (or, for an accessibility action's synthetic
//! key, its target) to take it with [`Ui::consume_key`].

use crate::input::NamedKey;

use super::{Ui, WidgetId};

/// One key press of the frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct KeyPress {
    pub(crate) key: NamedKey,
    /// `Some` for a synthetic key from an accessibility action: only that id may take it.
    pub(crate) target: Option<WidgetId>,
    pub(crate) consumed: bool,
}

impl Ui<'_> {
    /// Takes this frame's next press of `key` for `id`: true once per press, when `id` is the
    /// focused widget of the active scope (or the target of a synthetic key). A consumed real key
    /// turns the focus ring on.
    pub fn consume_key(&mut self, id: WidgetId, key: NamedKey) -> bool {
        let focus = self.key_focus;
        let Some(k) = self.keys.iter_mut().find(|k| {
            !k.consumed
                && k.key == key
                && match k.target {
                    Some(t) => t == id,
                    None => focus == Some(id),
                }
        }) else {
            return false;
        };
        k.consumed = true;
        if k.target.is_none() {
            self.store.focus.visible = true;
        }
        true
    }
}
