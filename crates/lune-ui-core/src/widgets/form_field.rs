//! A label, a control and an error line as one field (`widget.form-field`).
//!
//! Stacked: the label (13 on a 20 px line, `fg_secondary`), 4 px, the control, 4 px, the note (11
//! on a 16 px line, `fg_tertiary`). Inline: the label at the left of the control, 8 px away,
//! centred on it; the note under the control. An error rings the control with 1 px of `err` and
//! the note shows the error text in `err` instead of the hint. A press on the label focuses the
//! control. The control keeps its own size, states and Tab stop, and takes no error flag from the
//! field: the field draws the one ring.

use std::hash::Hash;

use crate::access::{Node, Role, State, Value};
use crate::paint::Rect;
use crate::text::{Spec, Style};
use crate::ui::{Sense, Ui, WidgetId};

/// The label's line and the note's line.
pub const LABEL_LINE: f32 = 20.0;
pub const NOTE_LINE: f32 = 16.0;
/// Between the label, the control and the note when stacked.
pub const GAP: f32 = 4.0;
/// Between an inline label and its control.
pub const INLINE_GAP: f32 = 8.0;
/// The error ring's width.
pub const RING: f32 = 1.0;

/// What a form field shows. Validation and its timing are the app's.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FormField<'a> {
    pub label: &'a str,
    pub hint: Option<&'a str>,
    /// An error: rings the control and replaces the hint.
    pub error: Option<&'a str>,
    /// The label at the left of the control instead of above it.
    pub inline: bool,
    /// The control's size; a width of 0 fills what the label leaves.
    pub control_size: (f32, f32),
}

impl Ui<'_> {
    /// A form field under `key` with its top left and width of `rect` (its height is ignored):
    /// `control` draws the control in the rect it is given and returns the control's id, which a
    /// press on the label focuses. Returns the height drawn. The node is a `Group` named by the
    /// label, described by the hint, `Invalid` with an error (whose text is a `StaticText` child);
    /// the control's node is its child.
    pub fn form_field(
        &mut self,
        key: impl Hash,
        rect: Rect,
        field: &FormField<'_>,
        control: impl FnOnce(&mut Ui<'_>, Rect) -> WidgetId,
    ) -> f32 {
        let t = *self.theme();
        let id = self.id(&key);
        let label_spec = Spec::plain(
            field.label,
            Style {
                line: LABEL_LINE,
                ..Style::ui(t.font_base, t.fg_secondary)
            },
        );
        let label_w = self.painter().measure(&label_spec).0;
        let (ctl_w, ctl_h) = field.control_size;

        let (ctl_x, avail) = if field.inline {
            let x = rect.x + label_w + INLINE_GAP;
            (x, (rect.right() - x).max(0.0))
        } else {
            (rect.x, rect.w)
        };
        let ctl_w = if ctl_w > 0.0 { ctl_w.min(avail) } else { avail };
        let (label_y, ctl_y) = if field.inline {
            let h = ctl_h.max(LABEL_LINE);
            (rect.y + (h - LABEL_LINE) / 2.0, rect.y + (h - ctl_h) / 2.0)
        } else {
            (rect.y, rect.y + LABEL_LINE + GAP)
        };
        let control_rect = Rect::new(ctl_x, ctl_y, ctl_w, ctl_h);

        let note = field
            .error
            .map(|e| (e, t.err))
            .or(field.hint.map(|h| (h, t.fg_tertiary)));
        let note_spec = note.map(|(text, color)| {
            Spec::plain(
                text,
                Style {
                    line: NOTE_LINE,
                    ..Style::ui(t.font_s, color)
                },
            )
            .wrapped(avail)
        });
        let note_h = note_spec
            .as_ref()
            .map_or(0.0, |s| self.painter().measure(s).1.max(NOTE_LINE));
        let body_bottom = if field.inline {
            (ctl_y + ctl_h).max(label_y + LABEL_LINE)
        } else {
            ctl_y + ctl_h
        };
        let height = body_bottom - rect.y + if note_h > 0.0 { GAP + note_h } else { 0.0 };
        let bounds = Rect::new(rect.x, rect.y, rect.w, height);

        self.scope(&key, |ui| {
            ui.painter().text(&label_spec, rect.x, label_y);
            let label = ui.id("label");
            let pressed = ui
                .interact(
                    label,
                    Rect::new(rect.x, label_y, label_w, LABEL_LINE),
                    Sense::CLICK,
                )
                .press_started;

            let mut node = Node::new(id, Role::Group, field.label, bounds);
            node.description = field.hint.map(str::to_owned);
            if field.error.is_some() {
                node.states.insert(State::Invalid);
            }
            ui.node_container(node, |ui| {
                let focus = control(ui, control_rect);
                if pressed {
                    ui.request_focus(focus);
                }
                if field.error.is_some() {
                    ui.painter()
                        .ring(control_rect, t.item_radius, RING, t.err, false);
                }
                if let Some(spec) = &note_spec {
                    ui.painter().text(spec, ctl_x, body_bottom + GAP);
                }
                if let Some(e) = field.error {
                    let mut n = Node::new(
                        id.with("error"),
                        Role::StaticText,
                        e,
                        Rect::new(ctl_x, body_bottom + GAP, avail, note_h),
                    );
                    n.value = Some(Value::Text(e.to_owned()));
                    ui.node(n);
                }
            });
        });
        height
    }
}
