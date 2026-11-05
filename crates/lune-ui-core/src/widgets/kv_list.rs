//! A list of key and value rows whose values can be selected (`data.kv-list`).
//!
//! Default: a key column as wide as the longest key (measured), a 16 px gap, rows of 24 with the
//! key (13, tertiary) and the value (13, primary, elided) on a 20 px line 2 px down. Wrap: the
//! same, but a value wraps at the column and its row is the text plus 2 px above and below.
//! Stacked: two columns of cells, the key (11 on a 16 px line, tertiary) over the value (13 mono on
//! a 20 px line), 12 px between rows. Values are [selectable](super::selectable): the keys and
//! the rest of a row take no input.

use std::hash::Hash;

use crate::access::{Node, Role, Value};
use crate::paint::Rect;
use crate::text::{Spec, Style};
use crate::ui::Ui;

/// A row's height in the default mode.
pub const ROW: f32 = 24.0;
/// Between the key column and the value column, and between the two columns of the stacked mode.
pub const GAP: f32 = 16.0;
/// A text line of the default and wrap modes.
pub const LINE: f32 = 20.0;
/// A wrapped value's padding above and below.
pub const WRAP_PAD: f32 = 2.0;
/// The stacked key's line and the gap between its rows.
pub const STACKED_KEY_LINE: f32 = 16.0;
pub const STACKED_ROW_GAP: f32 = 12.0;

/// How the pairs are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KvMode {
    /// A key column and an elided value, rows of 24.
    #[default]
    Default,
    /// Two columns of cells, the key over a mono value.
    Stacked,
    /// A value wraps instead of eliding.
    Wrap,
}

/// What [`Ui::kv_list`] is given besides its rect and pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KvOptions {
    pub mode: KvMode,
    /// Values in the mono face (ids, paths, figures). The stacked mode is always mono.
    pub mono: bool,
}

impl Ui<'_> {
    /// Key and value `pairs` under `key` with their top left at that of `rect`, wide as `rect`
    /// (its height is ignored). Returns the height drawn. Each pair is a `Group` named by its key
    /// over a `StaticText` key and a selectable `StaticText` value named by the key.
    pub fn kv_list(
        &mut self,
        key: impl Hash,
        rect: Rect,
        pairs: &[(&str, &str)],
        opts: &KvOptions,
    ) -> f32 {
        let t = *self.theme();
        let value_font = |mono: bool| Style {
            line: LINE,
            ..if mono {
                Style::mono(t.font_base, LINE, t.fg_primary)
            } else {
                Style::ui(t.font_base, t.fg_primary)
            }
        };
        let key_style = Style {
            line: LINE,
            ..Style::ui(t.font_base, t.fg_tertiary)
        };
        let stacked = opts.mode == KvMode::Stacked;
        let key_w = if stacked {
            0.0
        } else {
            let mut w = 0.0_f32;
            for (k, _) in pairs {
                w = w.max(self.painter().measure(&Spec::plain(*k, key_style)).0);
            }
            w
        };
        let col_w = ((rect.w - GAP) / 2.0).max(0.0);
        let value_x = rect.x + key_w + GAP;
        let value_w = (rect.w - key_w - GAP).max(0.0);

        self.scope(key, |ui| {
            let mut y = rect.y;
            let mut bottom = rect.y;
            for (i, (k, v)) in pairs.iter().enumerate() {
                ui.scope(i.to_string(), |ui| {
                    let gid = ui.id("pair");
                    if stacked {
                        let (col, row) = (i % 2, i / 2);
                        let x = rect.x + col as f32 * (col_w + GAP);
                        let top = rect.y + row as f32 * (STACKED_KEY_LINE + LINE + STACKED_ROW_GAP);
                        let ks = Style {
                            line: STACKED_KEY_LINE,
                            ..Style::ui(t.font_s, t.fg_tertiary)
                        };
                        let key_spec = ui.painter().elide(&Spec::plain(*k, ks), col_w);
                        let vs = value_font(true);
                        let val_spec = ui.painter().elide(&Spec::plain(*v, vs), col_w);
                        let cell = Rect::new(x, top, col_w, STACKED_KEY_LINE + LINE);
                        ui.node_container(Node::new(gid, Role::Group, k, cell), |ui| {
                            let kr = ui.painter().text(&key_spec, x, top);
                            key_node(ui, gid, k, Rect::new(x, top, kr.0, kr.1));
                            let vr = Rect::new(x, top + STACKED_KEY_LINE, col_w, LINE);
                            ui.selectable_named("value", vr, &val_spec, v, k);
                        });
                        bottom = bottom.max(top + STACKED_KEY_LINE + LINE);
                    } else {
                        let wrap = opts.mode == KvMode::Wrap;
                        let vs = value_font(opts.mono);
                        let (val_spec, text_h) = if wrap {
                            let s = Spec::plain(*v, vs).wrapped(value_w);
                            let h = ui.painter().measure(&s).1.max(LINE);
                            (s, h)
                        } else {
                            (ui.painter().elide(&Spec::plain(*v, vs), value_w), LINE)
                        };
                        let row_h = if wrap {
                            (text_h + 2.0 * WRAP_PAD).max(ROW)
                        } else {
                            ROW
                        };
                        let top = y + WRAP_PAD;
                        let cell = Rect::new(rect.x, y, rect.w, row_h);
                        ui.node_container(Node::new(gid, Role::Group, k, cell), |ui| {
                            let kr = ui.painter().text(&Spec::plain(*k, key_style), rect.x, top);
                            key_node(ui, gid, k, Rect::new(rect.x, top, kr.0, kr.1));
                            let vr = Rect::new(value_x, top, value_w, text_h);
                            ui.selectable_named("value", vr, &val_spec, v, k);
                        });
                        y += row_h;
                        bottom = y;
                    }
                });
            }
            bottom - rect.y
        })
    }
}

/// The key's `StaticText` node.
fn key_node(ui: &mut Ui<'_>, group: crate::ui::WidgetId, key: &str, at: Rect) {
    let mut node = Node::new(group.with("key"), Role::StaticText, key, at);
    node.value = Some(Value::Text(key.to_owned()));
    ui.node(node);
}
