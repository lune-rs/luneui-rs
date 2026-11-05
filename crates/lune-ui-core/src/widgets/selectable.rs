//! Read-only text that takes focus and selection, and copies (`data.kv-list`, D125).
//!
//! A value is drawn from a [`Spec`] (maybe elided by the caller) and copied from `full`. Where
//! the two are the same single-run text, the field's engine runs read-only over `full` (a
//! [`TextEdit`](crate::text_edit::TextEdit)): a press places the caret, a drag, a double press and
//! Shift extend it, Copy writes the selected bytes of `full`. Where they differ (an elided value,
//! a rich spec), no position of the drawn text maps to a position of `full`, so the value is
//! selected whole: a press, Select all or a double press selects all of it, the fill covers all of
//! what is drawn, and Copy writes all of `full`. Navigation keys do nothing there, so a partial
//! selection cannot exist.
//!
//! A value is no tab stop (`skip_tab`): a press or an assistive technology focuses it. The
//! selection fill draws only while it is focused, and a focus loss clears the selection.

use std::hash::Hash;

use super::field::{
    EditConfig, NodeFacts, Shown, begin_edit, edit_node, edit_pointer, end_edit, range_boxes,
};
use crate::access::{Action, Node, Role, State, TextValue, Value};
use crate::input::CursorIcon;
use crate::paint::Rect;
use crate::text::{Spec, joined_text};
use crate::text_edit::{EditCommand, EditFlags};
use crate::ui::{Response, Sense, Ui};

/// A value takes hover, clicks, drags and focus, but no Tab stop.
const SELECT_SENSE: Sense = Sense {
    hover: true,
    click: true,
    drag: true,
    focusable: true,
    scroll: false,
    skip_tab: true,
};

/// Whether a value drawn from a different text than it copies is selected (all of it).
#[derive(Debug, Default)]
struct WholeSelection(bool);

impl Ui<'_> {
    /// Read-only text under `key`: `spec` drawn with its top left at that of `rect`, `full` what
    /// Copy writes. The hit region is the drawn text's box (at most `rect`'s width), so a press
    /// beside the text does nothing. See the module docs for what selects when the two differ.
    /// The node is a `StaticText` named and valued by `full`.
    pub fn selectable_text(
        &mut self,
        key: impl Hash,
        rect: Rect,
        spec: &Spec,
        full: &str,
    ) -> Response {
        self.selectable_named(key, rect, spec, full, full)
    }

    /// [`Self::selectable_text`] with the node named `name`.
    pub(super) fn selectable_named(
        &mut self,
        key: impl Hash,
        rect: Rect,
        spec: &Spec,
        full: &str,
        name: &str,
    ) -> Response {
        let t = *self.theme();
        let id = self.id(key);
        let (w, h) = self.painter().measure(spec);
        let hit = Rect::new(
            rect.x,
            rect.y,
            w.min(rect.w),
            if h > 0.0 { h } else { rect.h },
        );
        let r = self.interact(id, hit, SELECT_SENSE);
        let focused = r.focused;
        // The key focus, kept in a disabled scope: the keys are taken there and nothing acts (D140).
        let held = self.focus_state().0 == Some(id);
        let single = spec.runs.len() == 1;
        let positioned = single && joined_text(spec) == full;
        let ink = t.text_selection;

        if positioned {
            let cfg = EditConfig {
                style: spec.runs[0].1,
                password: false,
                read_only: true,
                multiline: spec.wrap.is_some(),
                max_chars: None,
                wrap: spec.wrap,
                page_rows: 1,
            };
            let mut text = full.to_owned();
            let (mut st, _) = begin_edit(self, id, &cfg, &mut text);
            if !held {
                let c = st.edit.caret();
                st.edit.set_selection(c, c);
            }
            edit_pointer(self, &cfg, &mut st, &r, &mut text, (rect.x, rect.y));
            let shown = Shown::new(&cfg, &st.edit, &text, focused);
            let range = if focused { st.edit.selection() } else { 0..0 };
            for b in range_boxes(self, &shown.spec, &range) {
                let at = Rect::new(rect.x + b.x, rect.y + b.y, b.w, b.h);
                self.painter().fill(at, 0.0, ink);
            }
            self.painter().text(spec, rect.x, rect.y);
            let facts = NodeFacts {
                role: Role::StaticText,
                name,
                description: None,
                error: false,
                enabled: r.enabled,
            };
            edit_node(self, id, hit, &cfg, &st.edit, &text, facts);
            end_edit(self, id, &cfg, st, None);
        } else {
            let mut selected = self.persistent::<WholeSelection>(id).0 && held;
            if r.press_started {
                selected = true;
            }
            if held {
                let act = r.enabled;
                let keymap = self.keymap();
                let flags = EditFlags {
                    read_only: true,
                    ..EditFlags::default()
                };
                for (index, event) in self.focus_events(id) {
                    match EditCommand::from_event(&event, keymap, flags) {
                        Some(EditCommand::SelectAll) => {
                            selected |= act;
                            self.consume_focus_event(index);
                        }
                        // As the engine does: Copy with nothing selected is declined.
                        Some(EditCommand::Copy) if selected => {
                            if act {
                                self.clipboard_write(full.to_owned());
                            }
                            self.consume_focus_event(index);
                        }
                        _ => {}
                    }
                }
            }
            self.persistent::<WholeSelection>(id).0 = selected;
            let shown = selected && focused;
            if shown {
                self.painter()
                    .fill(Rect::new(rect.x, rect.y, w, h), 0.0, ink);
            }
            self.painter().text(spec, rect.x, rect.y);
            let mut node = Node::new(id, Role::StaticText, name, hit);
            node.value = Some(Value::Text(full.to_owned()));
            let n = full.chars().count();
            node.text = Some(TextValue {
                text: full.to_owned(),
                selection: if shown { (0, n) } else { (0, 0) },
            });
            node.states.insert(State::ReadOnly);
            if r.enabled {
                node.states.insert(State::Focusable);
                node.actions.insert(Action::Focus);
            } else {
                node.states.insert(State::Disabled);
            }
            self.node(node);
        }
        if r.hovered || r.pressed {
            self.set_cursor(CursorIcon::Text);
        }
        r
    }
}
