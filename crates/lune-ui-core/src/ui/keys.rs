//! Key routing: Escape and Tab are handled at begin (an Escape a live pointer gesture takes queues
//! for it instead, D175); every other key press, typed text, IME event and clipboard answer waits
//! in the frame's queue, in arrival order, for the focused widget of the active scope (or, for an
//! accessibility action's synthetic key, its target) to take it with
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
    /// The widget the pointer held the capture of when the key arrived, in the frame's event
    /// order: a release later in the same frame does not change it.
    pub(crate) held: Option<WidgetId>,
}

impl KeyPress {
    /// An event for whichever widget holds the key focus.
    pub(crate) fn real(event: FocusEvent) -> Self {
        Self {
            event,
            target: None,
            consumed: false,
            held: None,
        }
    }

    /// A key that arrived while the pointer held `held`.
    pub(crate) fn real_while(event: FocusEvent, held: Option<WidgetId>) -> Self {
        Self {
            held,
            ..Self::real(event)
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
            held: None,
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

    /// A Control, Alt or Command chord, which stays a shortcut.
    fn chord(&self) -> bool {
        matches!(&self.event, FocusEvent::Key { modifiers: m, .. }
            if m.control || m.alt || m.logo)
    }
}

impl Ui<'_> {
    /// Takes this frame's next press of `key` for `id`: true once per press, when `id` is the
    /// focused widget of the active scope (or the target of a synthetic key). A consumed real key
    /// turns the focus ring on.
    pub fn consume_key(&mut self, id: WidgetId, key: NamedKey) -> bool {
        self.take_key(id, key, true)
    }

    /// [`Self::consume_key`] that leaves a Control, Alt or Command chord for the shortcuts.
    pub(crate) fn consume_plain_key(&mut self, id: WidgetId, key: NamedKey) -> bool {
        self.take_key(id, key, false)
    }

    /// Takes this frame's next synthetic press of `key` aimed at `id` (an accessibility
    /// increment or decrement) and no real key: a real key on a focused widget is its own to
    /// read, and a chord or a disabled widget leaves it to the shortcuts.
    pub(crate) fn consume_synthetic_key(&mut self, id: WidgetId, key: NamedKey) -> bool {
        let Some(k) = self
            .keys
            .iter_mut()
            .find(|k| !k.consumed && k.named() == Some(key) && k.target == Some(id))
        else {
            return false;
        };
        k.consumed = true;
        self.input_acted = true;
        true
    }

    /// Whether `id` registered a region through `interact` since `mark` in a disabled scope: a
    /// focused widget a scroll area cannot see through its watch list.
    pub(crate) fn disabled_registered_since(&self, mark: usize, id: WidgetId) -> bool {
        self.registered
            .get(mark..)
            .unwrap_or_default()
            .iter()
            .any(|r| !r.enabled && self.painter.hits.get(r.index).is_some_and(|h| h.1 == id))
    }

    fn take_key(&mut self, id: WidgetId, key: NamedKey, chords: bool) -> bool {
        let focus = self.key_focus;
        let Some(k) = self.keys.iter_mut().find(|k| {
            !k.consumed
                && k.named() == Some(key)
                && (chords || !k.chord())
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
    /// focus (a drag that Escape cancels). A key counts when `id` holds the capture now, or held it
    /// where the key arrived in the frame's events: an Escape that came before the release of the
    /// same frame is the gesture's, though the release has ended the press by the time the
    /// widget asks.
    pub fn consume_key_while_pressed(&mut self, id: WidgetId, key: NamedKey) -> bool {
        let held_now = self.pointer.pressed == Some(id);
        let Some(k) = self.keys.iter_mut().find(|k| {
            !k.consumed
                && k.target.is_none()
                && k.named() == Some(key)
                && (held_now || k.held == Some(id))
        }) else {
            return false;
        };
        k.consumed = true;
        self.input_acted = true;
        true
    }

    /// Says that a pointer gesture which Escape cancels (a drag, a resize) is live at the end of this
    /// frame. The next frame's Escape is then queued for it, through
    /// [`Self::consume_key_while_pressed`], instead of dismissing the overlay on top; once the gesture
    /// ends nothing sets the flag and the next Escape dismisses (D175). An Escape in the frame the
    /// capture ends, after the release or the capture's loss, is the overlay's too (D176). Call it
    /// only while dragging, not while a press is merely armed.
    pub(crate) fn note_live_gesture(&mut self) {
        self.store.gesture_live = true;
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

    /// Whether the event at `index` of [`Self::focus_events`] stands for an accessibility action
    /// rather than a key the user pressed.
    pub(crate) fn focus_event_is_synthetic(&self, index: usize) -> bool {
        self.keys.get(index).is_some_and(|k| k.target.is_some())
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
