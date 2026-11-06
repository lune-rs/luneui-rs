//! The shortcut registry: scoped key chords mapped to command ids. The toolkit does not run
//! commands; a frame reports the ones that fired in `FrameOutput::fired` and the app acts on them.
//!
//! A key press meets the registry twice. At frame begin, before Escape and Tab, bindings marked
//! `preempt` in the top overlay's scope and in the global scope see it before any widget. At
//! frame finish, a press no widget consumed bubbles through the rest: `Within` scopes from the
//! innermost out, then overlays from the top down (stopping at a modal), then global. A modal
//! overlay hides page and global bindings that are not `preempt`. A press fires at most one
//! binding, in table order within a scope, and nothing fires while the focused widget composes.
//!
//! "Primary" is the platform's main modifier, resolved from the frame's [`Keymap`]: the toolkit
//! never reads the OS.

use std::borrow::Cow;

use crate::input::{Key, Keymap, Modifiers, NamedKey, Primary};
use crate::ui::WidgetId;

/// An app-chosen command name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandId(pub Cow<'static, str>);

impl CommandId {
    /// A command id from a static name; usable in a `const`.
    pub const fn new(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }

    /// The command's name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for CommandId {
    fn from(name: &'static str) -> Self {
        Self::new(name)
    }
}

impl From<String> for CommandId {
    fn from(name: String) -> Self {
        Self(Cow::Owned(name))
    }
}

/// The modifiers a chord requires. Matching is exact: a held modifier the chord does not name
/// stops it matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mods {
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
    pub logo: bool,
    /// The platform's primary modifier (Command on macOS, Control elsewhere).
    pub primary: bool,
    /// Control or Logo, either one (not both).
    pub ctrl_or_logo: bool,
}

/// A key with the modifiers that must be held with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord {
    pub mods: Mods,
    pub key: Key,
}

impl Chord {
    /// `key` with no modifiers.
    pub fn key(key: Key) -> Self {
        Self {
            mods: Mods::default(),
            key,
        }
    }

    /// Requires the platform's primary modifier.
    pub fn primary(mut self) -> Self {
        self.mods.primary = true;
        self
    }

    /// Requires Shift.
    pub fn shift(mut self) -> Self {
        self.mods.shift = true;
        self
    }

    /// Requires Alt.
    pub fn alt(mut self) -> Self {
        self.mods.alt = true;
        self
    }

    /// Requires Control.
    pub fn ctrl(mut self) -> Self {
        self.mods.ctrl = true;
        self
    }

    /// Requires Logo (Command on macOS, the Windows key elsewhere).
    pub fn logo(mut self) -> Self {
        self.mods.logo = true;
        self
    }

    /// Requires Control or Logo, either one.
    pub fn ctrl_or_logo(mut self) -> Self {
        self.mods.ctrl_or_logo = true;
        self
    }

    /// Whether a press of `key` with `held` modifiers is this chord under `keymap`.
    /// `Key::Character` compares case-insensitively.
    pub fn matches(&self, key: &Key, held: Modifiers, keymap: &Keymap) -> bool {
        let keys_match = match (&self.key, key) {
            (Key::Named(a), Key::Named(b)) => a == b,
            (Key::Character(a), Key::Character(b)) => a.to_lowercase() == b.to_lowercase(),
            _ => false,
        };
        if !keys_match || self.mods.shift != held.shift || self.mods.alt != held.alt {
            return false;
        }
        let (mut ctrl, mut logo) = (self.mods.ctrl, self.mods.logo);
        if self.mods.primary {
            match keymap.primary {
                Primary::Logo => logo = true,
                Primary::Control => ctrl = true,
            }
        }
        let exact = |ctrl: bool, logo: bool| ctrl == held.control && logo == held.logo;
        if self.mods.ctrl_or_logo {
            exact(true, logo) || exact(ctrl, true)
        } else {
            exact(ctrl, logo)
        }
    }

    /// The chord as menu text in the fixed order Cmd, Ctrl, Alt, Shift, key: "Cmd+N" or
    /// "Ctrl+N" by the keymap's primary modifier, "Cmd+,". A `ctrl_or_logo` chord shows the
    /// platform's primary one.
    pub fn hint(&self, keymap: &Keymap) -> String {
        let m = self.mods;
        let platform_logo = keymap.primary == Primary::Logo;
        let cmd = m.logo || ((m.primary || m.ctrl_or_logo) && platform_logo);
        let ctrl = m.ctrl || ((m.primary || m.ctrl_or_logo) && !platform_logo);
        let mut out = String::new();
        for (on, name) in [
            (cmd, "Cmd"),
            (ctrl, "Ctrl"),
            (m.alt, "Alt"),
            (m.shift, "Shift"),
        ] {
            if on {
                out.push_str(name);
                out.push('+');
            }
        }
        out.push_str(&key_name(&self.key));
        out
    }
}

fn key_name(key: &Key) -> String {
    match key {
        Key::Character(c) => c.to_uppercase(),
        Key::Named(n) => match n {
            NamedKey::Enter => "Enter".into(),
            NamedKey::Tab => "Tab".into(),
            NamedKey::Space => "Space".into(),
            NamedKey::Backspace => "Backspace".into(),
            NamedKey::Delete => "Delete".into(),
            NamedKey::Insert => "Insert".into(),
            NamedKey::Escape => "Esc".into(),
            NamedKey::Home => "Home".into(),
            NamedKey::End => "End".into(),
            NamedKey::PageUp => "PageUp".into(),
            NamedKey::PageDown => "PageDown".into(),
            NamedKey::ArrowLeft => "Left".into(),
            NamedKey::ArrowRight => "Right".into(),
            NamedKey::ArrowUp => "Up".into(),
            NamedKey::ArrowDown => "Down".into(),
            NamedKey::F1 => "F1".into(),
            NamedKey::F2 => "F2".into(),
            NamedKey::F3 => "F3".into(),
            NamedKey::F4 => "F4".into(),
            NamedKey::F5 => "F5".into(),
            NamedKey::F6 => "F6".into(),
            NamedKey::F7 => "F7".into(),
            NamedKey::F8 => "F8".into(),
            NamedKey::F9 => "F9".into(),
            NamedKey::F10 => "F10".into(),
            NamedKey::F11 => "F11".into(),
            NamedKey::F12 => "F12".into(),
        },
        Key::Unidentified => "?".into(),
    }
}

/// Where a binding is live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The whole window.
    Global,
    /// While the focused widget is `id` or sits under the id scope `id` (`Ui::scope_id`), counting
    /// scopes opened inside the focused widget's own overlay only.
    Within(WidgetId),
    /// While the overlay `id` (the id `Ui::overlay` derives from its key) is open.
    Overlay(WidgetId),
}

/// One chord bound to a command in a scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub scope: Scope,
    pub chord: Chord,
    pub command: CommandId,
    /// Sees the press before the focused widget (only in `Global` and the top overlay's scope).
    pub preempt: bool,
    /// A disabled binding never fires and consumes nothing.
    pub enabled: bool,
    /// Fires on key repeat as well as on the first press.
    pub repeat: bool,
}

impl Binding {
    /// Enabled, not preempting, not repeating.
    pub fn new(scope: Scope, chord: Chord, command: impl Into<CommandId>) -> Self {
        Self {
            scope,
            chord,
            command: command.into(),
            preempt: false,
            enabled: true,
            repeat: false,
        }
    }

    /// Sees the press before the focused widget (see the module doc).
    pub fn preempt(mut self) -> Self {
        self.preempt = true;
        self
    }

    /// Fires on key repeat as well as on the first press.
    pub fn repeating(mut self) -> Self {
        self.repeat = true;
        self
    }

    /// Turns the binding on or off; a disabled one never fires and consumes nothing.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// The binding table. Bindings run in table order inside a scope.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Shortcuts {
    bindings: Vec<Binding>,
}

impl Shortcuts {
    /// Appends `binding`; table order decides which of two matching bindings in one scope fires.
    pub fn add(&mut self, binding: Binding) -> &mut Self {
        self.bindings.push(binding);
        self
    }

    /// Enables or disables every binding of `command`.
    pub fn set_enabled(&mut self, command: &CommandId, enabled: bool) {
        for b in self.bindings.iter_mut().filter(|b| &b.command == command) {
            b.enabled = enabled;
        }
    }

    /// Removes every binding.
    pub fn clear(&mut self) {
        self.bindings.clear();
    }

    /// The bindings, in table order.
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// The first enabled binding for a press of `key` with `held` modifiers that `live` admits.
    pub(crate) fn find(
        &self,
        key: &Key,
        held: Modifiers,
        repeat: bool,
        keymap: &Keymap,
        live: impl Fn(&Binding) -> bool,
    ) -> Option<&Binding> {
        self.bindings.iter().find(|b| {
            b.enabled && (b.repeat || !repeat) && live(b) && b.chord.matches(key, held, keymap)
        })
    }
}
