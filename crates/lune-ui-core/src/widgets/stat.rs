//! A label and a value that tweens when the number changes (`data.stat`, D126).
//!
//! A cell is padding 12 16, the label (11 on a 16 px line, tertiary), 4 px, the value (mono Medium
//! 20 on a 28 px line, bright, elided), the sub-line (11 on 16) and an optional meter 8 px below.
//! Compact (the flyout): padding 6 8, no gap above the value, value 14 on 20. A strip is cells that
//! share the width with a 1 px hairline between them.
//!
//! The value animates only when the app gives it a `number`: the tween runs on the number, the
//! drawn text in flight is the app's formatter of the tweened number, and at rest the drawn text is
//! exactly `value`. A text-only value, or one marked `live`, never animates.

use std::fmt;
use std::hash::Hash;

use super::progress;
use super::tone::Tone;
use crate::access::{Node, Role, Value};
use crate::motion::{Easing, Kind, Motion, Reduce};
use crate::paint::Rect;
use crate::text::{Spec, Style};
use crate::ui::Ui;

/// The value's change: the `value` kind at `fast`, linear; it keeps its tween under Reduced.
pub const VALUE: Motion = Motion {
    easing: Easing::Linear,
    reduced: Reduce::Keep,
    ..Motion::of(Kind::Value)
};

/// The cell's padding: vertical and horizontal.
pub const PAD: (f32, f32) = (12.0, 16.0);
/// The compact cell's padding.
pub const COMPACT_PAD: (f32, f32) = (6.0, 8.0);
/// The label's line, and the sub-line's.
pub const SMALL_LINE: f32 = 16.0;
/// Between the label and the value (not compact).
pub const VALUE_GAP: f32 = 4.0;
/// Between the cell's text and its meter.
pub const METER_GAP: f32 = 8.0;
/// The placeholder value of a cell with no data.
pub const PLACEHOLDER: &str = "\u{2014}";

/// What one cell shows.
#[derive(Clone, Copy, Default)]
pub struct StatOptions<'a> {
    pub label: &'a str,
    /// The value's text at rest, exactly as drawn.
    pub value: &'a str,
    /// The value as a number and the app's formatter for it: a change of the number tweens, and
    /// the text in flight is `format(tweened)`. `None` is a text-only value: it never animates.
    pub number: Option<(f64, &'a dyn Fn(f64) -> String)>,
    /// A continuously updating value: a change snaps whatever `number` says.
    pub live: bool,
    /// The sub-line, if any.
    pub sub: Option<&'a str>,
    /// The sub-line's tone: `Ok` and `Warn` colour it, any other is tertiary text.
    pub sub_tone: Tone,
    /// The value in `gold`: the one figure that is net savings. Never on a cell with no data.
    pub gold: bool,
    /// The flyout's smaller cell.
    pub compact: bool,
    /// The value is a word: sans 14 on a 24 px line.
    pub text: bool,
    /// A meter closing the cell: the fraction 0 to 1.
    pub meter: Option<f32>,
    /// No data: the value is "\u{2014}" and the sub-line the toolkit's `luneui.stat.no-data`;
    /// `value`, `number`, `sub` and `gold` are ignored.
    pub no_data: bool,
}

impl fmt::Debug for StatOptions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StatOptions")
            .field("label", &self.label)
            .field("value", &self.value)
            .field("number", &self.number.map(|(n, _)| n))
            .field("live", &self.live)
            .field("sub", &self.sub)
            .field("sub_tone", &self.sub_tone)
            .field("gold", &self.gold)
            .field("compact", &self.compact)
            .field("text", &self.text)
            .field("meter", &self.meter)
            .field("no_data", &self.no_data)
            .finish()
    }
}

impl Ui<'_> {
    /// A cell under `key` with its top left and width of `rect` (its height is ignored). Returns
    /// the height drawn. The node is a `Group` named by the label over `StaticText` nodes for the
    /// value (valued by `value`, not by a tweened frame) and the sub-line, and a `Progress` for a
    /// meter.
    pub fn stat(&mut self, key: impl Hash, rect: Rect, opts: &StatOptions<'_>) -> f32 {
        let id = self.id(key);
        let t = *self.theme();
        let (pad_y, pad_x) = if opts.compact { COMPACT_PAD } else { PAD };
        let value_style = if opts.text {
            Style {
                line: 24.0,
                ..Style::ui(t.font_m, t.fg_bright).medium()
            }
        } else {
            let (size, line) = if opts.compact {
                (t.font_m, 20.0)
            } else {
                (20.0, 28.0)
            };
            Style::mono(size, line, t.fg_bright).medium()
        };
        let value_style = Style {
            color: if opts.gold && !opts.no_data {
                t.gold
            } else {
                value_style.color
            },
            ..value_style
        };
        let rest = if opts.no_data {
            PLACEHOLDER
        } else {
            opts.value
        };
        let drawn = match opts.number.filter(|(n, _)| n.is_finite() && !opts.no_data) {
            Some((n, format)) => {
                let target = n as f32;
                let nid = id.with("number");
                if opts.live {
                    self.snap_f32(nid, target);
                    rest.to_owned()
                } else {
                    let v = self.animate_f32(nid, target, &VALUE);
                    if v == target {
                        rest.to_owned()
                    } else {
                        format(f64::from(v))
                    }
                }
            }
            None => rest.to_owned(),
        };

        let inner_w = (rect.w - 2.0 * pad_x).max(0.0);
        let x = rect.x + pad_x;
        let y = rect.y + pad_y;
        let label_style = Style {
            line: SMALL_LINE,
            ..Style::ui(t.font_s, t.fg_tertiary)
        };
        let sub_text = if opts.no_data {
            Some(self.tr("luneui.stat.no-data"))
        } else {
            opts.sub
        };

        // Measure first: the group's bounds hold everything below it.
        let label_spec = self
            .painter()
            .elide(&Spec::plain(opts.label, label_style), inner_w);
        let value_spec = self
            .painter()
            .elide(&Spec::plain(drawn, value_style), inner_w);
        let value_top = y + SMALL_LINE + if opts.compact { 0.0 } else { VALUE_GAP };
        let mut bottom = value_top + value_style.line;
        let sub_top = bottom;
        if sub_text.is_some() {
            bottom += SMALL_LINE;
        }
        let meter_top = bottom + METER_GAP;
        if opts.meter.is_some() {
            bottom = meter_top + progress::HEIGHT;
        }
        let height = bottom + pad_y - rect.y;

        let group = Node::new(
            id,
            Role::Group,
            opts.label,
            Rect::new(rect.x, rect.y, rect.w, height),
        );
        self.node_container(group, |ui| {
            let p = ui.painter();
            p.text(&label_spec, x, y);
            let vs = p.text(&value_spec, x, value_top);
            let mut value_node = Node::new(
                id.with("value"),
                Role::StaticText,
                rest,
                Rect::new(x, value_top, vs.0, vs.1),
            );
            value_node.value = Some(Value::Text(rest.to_owned()));
            ui.node(value_node);
            if let Some(sub) = sub_text {
                // A placeholder carries no delta, so no tone.
                let color = match opts.sub_tone {
                    Tone::Ok if !opts.no_data => t.ok,
                    Tone::Warn if !opts.no_data => t.warn,
                    _ => t.fg_tertiary,
                };
                let spec = Spec::plain(
                    sub,
                    Style {
                        line: SMALL_LINE,
                        ..Style::ui(t.font_s, color)
                    },
                );
                let spec = ui.painter().elide(&spec, inner_w);
                let ss = ui.painter().text(&spec, x, sub_top);
                let mut node = Node::new(
                    id.with("sub"),
                    Role::StaticText,
                    sub,
                    Rect::new(x, sub_top, ss.0, ss.1),
                );
                node.value = Some(Value::Text(sub.to_owned()));
                ui.node(node);
            }
            if let Some(fraction) = opts.meter {
                let track = Rect::new(x, meter_top, inner_w, progress::HEIGHT);
                progress::paint(ui.painter(), track, fraction, Tone::Neutral);
                let mut node = Node::new(id.with("meter"), Role::Progress, opts.label, track);
                node.value = Some(Value::Number {
                    value: f64::from(progress::clamp(fraction) * 100.0),
                    min: 0.0,
                    max: 100.0,
                    step: 0.0,
                });
                ui.node(node);
            }
        });
        height
    }

    /// `cells` side by side under `key`, sharing the width of `rect` with a 1 px `border_subtle`
    /// hairline between neighbours. Returns the height of the tallest cell, which the hairlines
    /// span.
    pub fn stat_strip(&mut self, key: impl Hash, rect: Rect, cells: &[StatOptions<'_>]) -> f32 {
        let hairline = self.theme().border_subtle;
        let n = cells.len();
        if n == 0 {
            return 0.0;
        }
        let w = ((rect.w - (n - 1) as f32) / n as f32).max(0.0);
        self.scope(key, |ui| {
            let mut tallest = 0.0_f32;
            for (i, cell) in cells.iter().enumerate() {
                let x = rect.x + i as f32 * (w + 1.0);
                let h = ui.scope(i.to_string(), |ui| {
                    ui.stat("cell", Rect::new(x, rect.y, w, 0.0), cell)
                });
                tallest = tallest.max(h);
            }
            for i in 1..n {
                let x = rect.x + i as f32 * (w + 1.0) - 1.0;
                ui.painter()
                    .fill(Rect::new(x, rect.y, 1.0, tallest), 0.0, hairline);
            }
            tallest
        })
    }
}
