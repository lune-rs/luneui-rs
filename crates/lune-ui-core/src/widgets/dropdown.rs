//! Combo-box drop-down list: the geometry, a paint function, the Painter-API [`dropdown`] and
//! [`Ui::dropdown`], the list on the popup layer.

use std::hash::Hash;

use super::{label, popup_panel};
use crate::access::{Action, Node, Role, State};
use crate::icon;
use crate::input::NamedKey;
use crate::motion::{Easing, Kind, Motion, Reduce, Token};
use crate::paint::{LayerId, Painter, Rect};
use crate::ui::{OverlayOptions, Sense, Ui, WidgetId};

const ITEM_H: f32 = 32.0;

/// The list's panel for `n` options under (or, near the window's bottom edge, over) `anchor`.
pub fn panel_rect<H: Copy + PartialEq>(p: &Painter<'_, H>, anchor: Rect, n: usize) -> Rect {
    let h = n as f32 * ITEM_H + 8.0;
    let below = anchor.bottom() + 4.0;
    let y = if below + h > p.window.h - 8.0 {
        (anchor.y - 4.0 - h).max(8.0)
    } else {
        below
    };
    Rect::new(anchor.x, y, anchor.w, h)
}

/// Option `i`'s row inside `panel`.
pub fn row_rect(panel: Rect, i: usize) -> Rect {
    Rect::new(
        panel.x + 4.0,
        panel.y + 4.0 + i as f32 * ITEM_H,
        panel.w - 8.0,
        ITEM_H,
    )
}

/// Draws the panel and its rows, without hit regions. `hot[i]` is the hover (or keyboard
/// highlight) fill of option `i`; it applies to an enabled row that is not the selected one.
pub fn paint<H: Copy + PartialEq, S: AsRef<str>>(
    p: &mut Painter<'_, H>,
    panel: Rect,
    options: &[(S, bool)],
    selected: usize,
    hot: &[bool],
) {
    let t = p.theme;
    popup_panel(p, panel, t.dropdown_bg, t.border);
    for (i, (text, enabled)) in options.iter().enumerate() {
        let r = row_rect(panel, i);
        if i == selected {
            p.fill(r, 4.0, t.surface_selected);
        } else if *enabled && hot.get(i).copied().unwrap_or(false) {
            p.fill(r, 4.0, t.surface_hover);
        }
        let color = if *enabled {
            t.fg_primary
        } else {
            t.fg_tertiary
        };
        p.text_in(&label(&t, text.as_ref(), color), r, 10.0);
        if i == selected {
            let check = Rect::new(r.right() - 26.0, r.y, 20.0, r.h);
            p.text_centered(&icon::glyph(icon::CHECK, 12.0, t.fg_secondary), check);
        }
    }
}

/// A list of options under (or, near the window's bottom edge, over) `anchor`. `options` pairs
/// each text with whether it can be picked; `hit` names the hit region of option `i`.
pub fn dropdown<H: Copy + PartialEq, S: AsRef<str>>(
    p: &mut Painter<'_, H>,
    anchor: Rect,
    options: &[(S, bool)],
    selected: usize,
    hit: impl Fn(usize) -> H,
) {
    let panel = panel_rect(p, anchor, options.len());
    let hot: Vec<bool> = options
        .iter()
        .enumerate()
        .map(|(i, (_, enabled))| *enabled && p.hovered(hit(i)))
        .collect();
    paint(p, panel, options, selected, &hot);
    for (i, (_, enabled)) in options.iter().enumerate() {
        if *enabled {
            p.hit(row_rect(panel, i), hit(i));
        }
    }
}

/// Open and close are drawn at once (the list has no motion): a zero-duration row, so the overlay
/// never reports itself moving.
const SNAP: Motion = Motion {
    kind: Kind::Dismiss,
    token: Token::Zero,
    easing: Easing::Linear,
    reduced: Reduce::Snap,
    reduced_token: None,
};

/// What a dropdown frame reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DropdownResponse {
    /// The index of the option picked this frame (an enabled one).
    pub picked: Option<usize>,
    /// The Ui asks the app to close the list (Escape, an outside press, an accessibility dismiss).
    pub dismissed: bool,
}

/// The keyboard row and the row the pointer was last over.
#[derive(Debug, Default)]
struct Highlight {
    row: Option<usize>,
    pointer: Option<usize>,
}

/// The next enabled row from `from` in direction `down`, or `from` at the end.
fn step(options: &[bool], from: usize, down: bool) -> usize {
    let mut i = from;
    loop {
        let next = if down {
            i + 1
        } else {
            match i.checked_sub(1) {
                Some(n) => n,
                None => return from,
            }
        };
        if next >= options.len() {
            return from;
        }
        if options[next] {
            return next;
        }
        i = next;
    }
}

impl<'a> Ui<'a> {
    /// The options list of the combo `anchor_id` (at `anchor`), on the popup layer: non-modal,
    /// Escape and a press outside dismiss it (`dismissed`), focus returns to `anchor_id` when it
    /// closes. Up and Down move a keyboard highlight over the enabled rows (starting at
    /// `selected`), Enter or a click on an enabled row picks it (`picked`); a disabled row has no
    /// hit region. It opens and closes at once: call it every frame it is open.
    pub fn dropdown<S: AsRef<str>>(
        &mut self,
        key: impl Hash,
        anchor: Rect,
        anchor_id: WidgetId,
        options: &[(S, bool)],
        selected: usize,
    ) -> DropdownResponse {
        let id = self.id(key);
        let list_id = id.with("list");
        let panel = panel_rect(self.painter(), anchor, options.len());
        let name = self
            .previous_tree()
            .get(anchor_id)
            .map(|n| n.name.clone())
            .unwrap_or_default();
        let opts = OverlayOptions {
            layer: LayerId::Popup,
            modal: false,
            dismiss_on_escape: true,
            dismiss_on_outside_press: true,
            dismiss_on_focus_loss: false,
            anchor: Some(anchor_id),
            initial_focus: Some(list_id),
            open: SNAP,
            close: SNAP,
        };
        let (frame, picked) = self.overlay(
            id,
            &opts,
            |_, _, _| {},
            |ui, _| {
                // The list takes focus (and the keys); the rows are click targets above it.
                let list = ui.interact(list_id, panel, Sense::CLICK_FOCUS);
                let enabled: Vec<bool> = options.iter().map(|(_, e)| *e).collect();
                let mut rows = Vec::with_capacity(options.len());
                for (i, on) in enabled.iter().enumerate() {
                    rows.push(on.then(|| {
                        ui.interact(
                            list_id.with(i.to_string()),
                            row_rect(panel, i),
                            Sense::CLICK,
                        )
                    }));
                }
                let hovered = rows
                    .iter()
                    .position(|r| r.as_ref().is_some_and(|r| r.hovered));
                let clicked = rows
                    .iter()
                    .position(|r| r.as_ref().is_some_and(|r| r.clicked));
                let (down, up, enter) = if list.enabled {
                    (
                        ui.consume_key(list_id, NamedKey::ArrowDown),
                        ui.consume_key(list_id, NamedKey::ArrowUp),
                        ui.consume_key(list_id, NamedKey::Enter),
                    )
                } else {
                    (false, false, false)
                };
                let h = ui.transient::<Highlight>(list_id);
                let mut row = h.row.unwrap_or(selected);
                if h.pointer != hovered {
                    h.pointer = hovered;
                    if let Some(i) = hovered {
                        row = i;
                    }
                }
                if down {
                    row = step(&enabled, row, true);
                }
                if up {
                    row = step(&enabled, row, false);
                }
                h.row = Some(row);
                let hot: Vec<bool> = (0..options.len()).map(|i| i == row).collect();
                paint(ui.painter(), panel, options, selected, &hot);

                let mut list_node = Node::new(list_id, Role::List, &name, panel);
                list_node.states.insert(State::Focusable);
                ui.node_container(list_node, |ui| {
                    for (i, (text, on)) in options.iter().enumerate() {
                        let mut node = Node::new(
                            list_id.with(i.to_string()),
                            Role::ListItem,
                            text.as_ref(),
                            row_rect(panel, i),
                        );
                        if i == selected {
                            node.states.insert(State::Selected);
                        }
                        if *on {
                            node.actions.insert(Action::Default);
                        } else {
                            node.states.insert(State::Disabled);
                        }
                        ui.node(node);
                    }
                });
                clicked.or(enter
                    .then_some(row)
                    .filter(|r| enabled.get(*r) == Some(&true)))
            },
        );
        DropdownResponse {
            picked,
            dismissed: frame.dismissed.is_some(),
        }
    }
}
