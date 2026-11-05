//! A search box with a clear button over the bare field chrome (`widget.filter-bar`): an inset
//! surface with a 14 px search icon at the left, a [`FieldChrome::Bare`] field, and an 18 px clear
//! [`icon_button`](Ui::icon_button) at the right while the text is non-empty. There is no ring: the
//! caret is the only focus cue.
//!
//! The field's id is the bar's, so [`FieldResponse::response`] names the field; the clear button is
//! the child `"clear"` of it. The button registers after the field to come after it in the Tab
//! order, so a clear lands after the field drew: the frame reports `changed`, and the frame the
//! input asks for after it (D81) shows the empty field and no button.

use std::hash::Hash;

use super::button::IconButton;
use super::field::{FieldChrome, FieldOptions, FieldResponse};
use crate::icon;
use crate::paint::Rect;
use crate::ui::{Sense, Ui, WidgetId};

/// The surface's corner radius and padding.
pub const RADIUS: f32 = 4.0;
pub const PADDING: (f32, f32) = (8.0, 6.0);
/// The search icon and the space between it and the text.
pub const ICON: f32 = 14.0;
pub const ICON_GAP: f32 = 8.0;
/// The clear button's side and its icon.
pub const CLEAR: f32 = 18.0;
pub const CLEAR_ICON: f32 = 12.0;

/// How far a plain field's text sits inside its rect: the bar pulls the field left by this, so the
/// text starts where the icon's margin ends.
const FIELD_INSET: f32 = 10.0;

/// What [`Ui::filter_bar`] is given besides its rect and text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FilterOptions<'a> {
    /// Shown while the text is empty; the accessible name when there is no `label`.
    pub placeholder: &'a str,
    /// The accessible name.
    pub label: Option<&'a str>,
    /// The clear button takes no focus and is no tab stop, so focus stays in the field: for a
    /// composite whose keys go through the field. The pointer and accessibility still reach it.
    pub clear_unfocusable: bool,
}

/// The clear button of the bar `id`.
pub(super) fn clear_id(id: WidgetId) -> WidgetId {
    id.with("clear")
}

impl Ui<'_> {
    /// A filter bar under `key` in `rect` editing `text`. The field edits as
    /// [`Self::text_field_with`] does; a press on the surface around it focuses it; the clear
    /// button empties `text`, marks the response `changed` and leaves focus on the field. Disabled
    /// (see [`Ui::with_enabled`]) it takes no text and clears nothing.
    pub fn filter_bar(
        &mut self,
        key: impl Hash,
        rect: Rect,
        text: &mut String,
        opts: &FilterOptions<'_>,
    ) -> FieldResponse {
        let t = *self.theme();
        let id = self.id(&key);
        // Under everything else of the bar, so only its padding and the icon reach it.
        let surface = self.interact(id.with("surface"), rect, Sense::CLICK);
        self.painter().fill(rect, RADIUS, t.surface_input);
        let at = Rect::new(rect.x + PADDING.0, rect.y, ICON, rect.h);
        let glyph = icon::glyph(icon::SEARCH, ICON, t.fg_dim);
        self.painter().text_centered(&glyph, at);

        let clear = Rect::new(
            rect.right() - PADDING.0 - CLEAR,
            rect.y + (rect.h - CLEAR) / 2.0,
            CLEAR,
            CLEAR,
        );
        let left = rect.x + PADDING.0 + ICON + ICON_GAP - FIELD_INSET;
        let right = if text.is_empty() {
            rect.right() - PADDING.0
        } else {
            clear.x
        };
        let field = Rect::new(left, rect.y, (right - left).max(0.0), rect.h);
        let field_opts = FieldOptions {
            placeholder: opts.placeholder,
            label: opts.label,
            chrome: FieldChrome::Bare,
            ..FieldOptions::default()
        };
        let mut out = self.text_field_with(&key, field, text, &field_opts);

        // After the field: what it typed this frame counts.
        if !text.is_empty() {
            let button = IconButton {
                glyph: icon::CHROME_CLOSE,
                glyph_size: CLEAR_ICON,
                color: Some(t.fg_dim),
                radius: RADIUS,
                name: self.tr("luneui.text-field.clear"),
            };
            let sense = if opts.clear_unfocusable {
                Sense::CLICK
            } else {
                Sense::CLICK_FOCUS
            };
            let c = self.scope(&key, |ui| {
                ui.icon_button_sensed("clear", clear, &button, sense)
            });
            // While composing the engine refuses a clear, as it does any press.
            if c.clicked && !self.composing() {
                text.clear();
                out.response.mark_changed();
                self.request_focus(id);
            }
        }
        if surface.press_started {
            self.request_focus(id);
        }
        out
    }
}
