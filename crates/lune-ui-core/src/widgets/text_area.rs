//! The multi-line text box: the [field](super::field)'s box and engine over a
//! [scroll area](super::scroll_area). Enter inserts a line break, and `wrap` passes the viewport's
//! width to the text.
//!
//! The focus events run before the area, so the caret moves into view in the frame that moved it,
//! at once: typing and caret motion never animate. The widget's region registers inside the content,
//! because the area's own region would block one under it; so the press and the drag run in the
//! content closure, and a caret they move comes into view on the next frame.

use std::hash::Hash;

use super::field::{
    CARET_ROOM, EDIT_SENSE, EditConfig, FieldResponse, NodeFacts, Shown, animated_look, begin_edit,
    caret_at, draw_shown, edit_node, edit_pointer, end_edit, paint_frame, screen_area, set_value,
};
use super::scroll_area::{Bars, Extent, Reveal, ScrollOptions, reveal_offset};
use crate::access::{AccessValue, Role};
use crate::paint::Rect;
use crate::text::Style;
use crate::ui::Ui;

/// From the viewport's left and top edges to the text.
const PAD_X: f32 = 9.0;
const PAD_Y: f32 = 6.0;
/// From the box to the viewport: the ring stays outside the clip.
const FRAME: f32 = 1.0;

/// What [`Ui::text_area`] is given besides its rect and text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AreaOptions<'a> {
    /// Shown while the text is empty; the accessible name when there is no `label`.
    pub placeholder: &'a str,
    pub error: bool,
    /// The caret and selection work, edits do not.
    pub read_only: bool,
    /// The most chars an edit may leave in the text.
    pub max_chars: Option<usize>,
    /// Wrap at the viewport's width, and scroll only vertically.
    pub wrap: bool,
    /// The accessible name.
    pub label: Option<&'a str>,
    /// The accessible description.
    pub description: Option<&'a str>,
}

impl Ui<'_> {
    /// A multi-line text box under `key` in `rect` editing `text`. It behaves like
    /// [`Self::text_field_with`], except that Enter inserts a line break, Up, Down, PageUp and
    /// PageDown move the caret, and the content scrolls.
    pub fn text_area(
        &mut self,
        key: impl Hash,
        rect: Rect,
        text: &mut String,
        opts: &AreaOptions<'_>,
    ) -> FieldResponse {
        let t = *self.theme();
        let id = self.id(&key);
        let viewport = rect.inset(FRAME, FRAME);
        let inner_w = (viewport.w - 2.0 * PAD_X - CARET_ROOM).max(1.0);
        let inner_h = (viewport.h - 2.0 * PAD_Y).max(0.0);
        let style = Style::ui(t.font_base, t.fg_primary);
        let cfg = EditConfig {
            style,
            password: false,
            read_only: opts.read_only,
            multiline: true,
            max_chars: opts.max_chars,
            wrap: opts.wrap.then_some(inner_w),
            page_rows: (inner_h / style.line).floor().max(1.0) as usize,
            rich: None,
        };
        let (mut st, frame) = begin_edit(self, id, &cfg, text);
        let enabled = self.is_enabled();
        let focused = enabled && self.focus_state().0 == Some(id);
        let hovered = enabled && self.painter().hovered(id);
        let look = animated_look(self, id, hovered, focused, opts.error);
        paint_frame(self, rect, &look);

        let shown = Shown::new(&cfg, &st.edit, text, focused);
        let (w, h) = self.painter().measure(&shown.spec);
        let extent = (
            f64::from(w + 2.0 * PAD_X + CARET_ROOM),
            f64::from(h.max(style.line) + 2.0 * PAD_Y),
        );
        if std::mem::take(&mut st.reveal) && focused {
            let c = caret_at(self, &shown.spec, shown.focus_at);
            // The padding comes along, so the first line and column show as they do at rest.
            let (x, y) = (f64::from(c.x), f64::from(c.y));
            let cw = f64::from(CARET_ROOM + 2.0 * PAD_X);
            let ch = f64::from(c.height.max(style.line) + 2.0 * PAD_Y);
            let (vw, vh) = (f64::from(viewport.w), f64::from(viewport.h));
            let (ox, oy) = st.offset;
            let by_x = reveal_offset(x, cw, vw, ox, Reveal::Nearest) - ox;
            let by_y = reveal_offset(y, ch, vh, oy, Reveal::Nearest) - oy;
            self.scope(&key, |ui| {
                ui.scroll_shift("area", by_y);
                if !opts.wrap {
                    ui.scroll_shift_x("area", by_x);
                }
            });
        }

        let scroll = ScrollOptions {
            bars: (
                if opts.wrap {
                    Bars::Disabled
                } else {
                    Bars::Auto.unless_fits(extent.0, f64::from(viewport.w))
                },
                Bars::Auto.unless_fits(extent.1, f64::from(viewport.h)),
            ),
            extent: Some(extent),
            ..ScrollOptions::default()
        };
        let mut r = None;
        let mut area = None;
        let area_out = self.scope(&key, |ui| {
            ui.scroll_area("area", viewport, &scroll, |ui, view| {
                // The region covers the viewport wherever the content has scrolled to.
                let hit = Rect::new(
                    viewport.x + view.offset.0 as f32,
                    viewport.y + view.offset.1 as f32,
                    viewport.w,
                    viewport.h,
                );
                let mut resp = ui.interact(id, hit, EDIT_SENSE);
                if frame.changed {
                    resp.mark_changed();
                }
                if let Some(AccessValue::Text(value)) = resp.set_value.clone()
                    && !opts.read_only
                {
                    set_value(ui, &cfg, &mut st, text, value);
                    resp.mark_changed();
                }
                let origin = (viewport.x + PAD_X, viewport.y + PAD_Y);
                edit_pointer(ui, &cfg, &mut st, &resp, text, origin);

                let shown = Shown::new(&cfg, &st.edit, text, focused);
                if shown.empty {
                    let hint = cfg.placeholder(opts.placeholder, t.fg_tertiary);
                    ui.painter().text(&hint, origin.0, origin.1);
                }
                let bar = draw_shown(ui, &shown, origin, look.caret);
                if focused {
                    area = Some(screen_area(ui, bar, hit));
                }
                let facts = NodeFacts {
                    role: Role::MultilineText,
                    name: opts.label.unwrap_or(opts.placeholder),
                    description: opts.description,
                    error: opts.error,
                    enabled,
                };
                edit_node(ui, id, hit, &cfg, &st.edit, text, facts);
                r = Some(resp);
                ((), Extent::default())
            })
        });
        st.offset = area_out.offset;
        if st.reveal {
            self.request_catch_up();
        }
        let ime_area = end_edit(self, id, &cfg, st, area);
        let response = r.unwrap_or(area_out.response);
        FieldResponse {
            response,
            submitted: frame.submitted,
            ime_area,
        }
    }
}
