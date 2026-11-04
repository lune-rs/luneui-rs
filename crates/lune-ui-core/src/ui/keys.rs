//! Key routing: Escape and Tab are handled at begin; every other key press, typed text, IME event
//! and clipboard answer waits in the frame's queue, in arrival order, for the focused widget of the
//! active scope (or, for an accessibility action's synthetic key, its target) to take it with
//! [`Ui::consume_key`] or [`Ui::consume_focus_event`].

use crate::input::{FocusEvent, Key, Keymap, NamedKey};
use crate::shortcuts::{Binding, CommandId, Scope, Shortcuts};

use super::{Ui, WidgetId};

/// One queued input of the frame.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KeyPress {
    pub(crate) event: FocusEvent,
    /// `Some` for a synthetic key from an accessibility action: only that id may take it.
    pub(crate) target: Option<WidgetId>,
    pub(crate) consumed: bool,
}

impl KeyPress {
    /// An event for whichever widget holds the key focus.
    pub(crate) fn real(event: FocusEvent) -> Self {
        Self {
            event,
            target: None,
            consumed: false,
        }
    }

    /// A named key only `target` may take.
    pub(crate) fn synthetic(key: NamedKey, target: WidgetId) -> Self {
        Self {
            event: FocusEvent::Key {
                key: Key::Named(key),
                modifiers: Default::default(),
                repeat: false,
            },
            target: Some(target),
            consumed: false,
        }
    }

    fn named(&self) -> Option<NamedKey> {
        match &self.event {
            FocusEvent::Key {
                key: Key::Named(k), ..
            } => Some(*k),
            _ => None,
        }
    }
}

impl Ui<'_> {
    /// Takes this frame's next press of `key` for `id`: true once per press, when `id` is the
    /// focused widget of the active scope (or the target of a synthetic key). A consumed real key
    /// turns the focus ring on.
    pub fn consume_key(&mut self, id: WidgetId, key: NamedKey) -> bool {
        let focus = self.key_focus;
        let Some(k) = self.keys.iter_mut().find(|k| {
            !k.consumed
                && k.named() == Some(key)
                && match k.target {
                    Some(t) => t == id,
                    None => focus == Some(id),
                }
        }) else {
            return false;
        };
        k.consumed = true;
        self.input_acted = true;
        if k.target.is_none() {
            self.store.focus.visible = true;
        }
        true
    }

    /// Like [`Self::consume_key`], but for the widget that holds the pointer capture, whatever the
    /// focus (a drag that Escape cancels).
    pub fn consume_key_while_pressed(&mut self, id: WidgetId, key: NamedKey) -> bool {
        if self.pointer.pressed != Some(id) {
            return false;
        }
        let Some(k) = self
            .keys
            .iter_mut()
            .find(|k| !k.consumed && k.target.is_none() && k.named() == Some(key))
        else {
            return false;
        };
        k.consumed = true;
        self.input_acted = true;
        true
    }

    /// The frame's unconsumed input for `id`, in arrival order, each with the index
    /// [`Self::consume_focus_event`] takes it by: keys (named and character, with modifiers and
    /// the repeat flag), typed text, IME events and clipboard answers. Empty unless `id` holds the
    /// key focus of the active scope (or is the target of a synthetic key).
    pub fn focus_events(&self, id: WidgetId) -> Vec<(usize, FocusEvent)> {
        let focus = self.key_focus;
        self.keys
            .iter()
            .enumerate()
            .filter(|(_, k)| {
                !k.consumed
                    && match k.target {
                        Some(t) => t == id,
                        None => focus == Some(id),
                    }
            })
            .map(|(i, k)| (i, k.event.clone()))
            .collect()
    }

    /// Takes the event at `index` of [`Self::focus_events`]; the frame then asks for a catch-up
    /// frame. An index out of range or already taken does nothing.
    pub fn consume_focus_event(&mut self, index: usize) {
        let Some(k) = self.keys.get_mut(index).filter(|k| !k.consumed) else {
            return;
        };
        k.consumed = true;
        self.input_acted = true;
        if k.target.is_none() && matches!(k.event, FocusEvent::Key { .. }) {
            self.store.focus.visible = true;
        }
    }
}

/// What the bubble phase reads besides the queue.
pub(crate) struct BubbleScope<'s> {
    pub(crate) shortcuts: &'s Shortcuts,
    pub(crate) keymap: &'s Keymap,
    /// The active overlays as `(id, tag, modal)`, in open order.
    pub(crate) overlays: &'s [(WidgetId, u32, bool)],
    /// The focused widget and its highest hit tag, if it registered this frame.
    pub(crate) focused: Option<(WidgetId, u32)>,
    /// The id scopes around the focused widget, outermost first.
    pub(crate) path: &'s [WidgetId],
    /// The overlay whose `preempt` bindings the begin phase already offered.
    pub(crate) preempt_overlay: Option<WidgetId>,
}

/// Offers every unconsumed key press to the registry, innermost scope first: `Within` scopes of
/// the focused widget, then overlays from the top down (stopping at a modal), then global (unless
/// a modal is open). The first enabled binding that matches consumes the press and fires.
/// Bindings the begin phase already offered (`preempt` ones in the global scope and the top
/// overlay's) are not offered again.
pub(crate) fn bubble(keys: &mut [KeyPress], at: &BubbleScope<'_>, fired: &mut Vec<CommandId>) {
    let modal_tag = at.overlays.iter().rev().find(|o| o.2).map(|o| o.1);
    let mut levels: Vec<Scope> = Vec::new();
    if let Some((f, tag)) = at.focused
        && modal_tag.is_none_or(|t| tag >= t)
    {
        levels.push(Scope::Within(f));
        levels.extend(at.path.iter().rev().map(|s| Scope::Within(*s)));
    }
    for (id, _, modal) in at.overlays.iter().rev() {
        levels.push(Scope::Overlay(*id));
        if *modal {
            break;
        }
    }
    if modal_tag.is_none() {
        levels.push(Scope::Global);
    }
    let offered = |b: &Binding| {
        b.preempt
            && match b.scope {
                Scope::Global => true,
                Scope::Overlay(id) => at.preempt_overlay == Some(id),
                Scope::Within(_) => false,
            }
    };
    for k in keys
        .iter_mut()
        .filter(|k| !k.consumed && k.target.is_none())
    {
        let FocusEvent::Key {
            key,
            modifiers,
            repeat,
        } = &k.event
        else {
            continue;
        };
        for level in &levels {
            if let Some(b) = at.shortcuts.find(key, *modifiers, *repeat, at.keymap, |b| {
                b.scope == *level && !offered(b)
            }) {
                fired.push(b.command.clone());
                k.consumed = true;
                break;
            }
        }
    }
}
