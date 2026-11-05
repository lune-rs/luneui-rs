//! The stacked bar chart (`data.bar-chart`, D164, D165): painter rectangles with value transitions.
//!
//! A grid of a 40 px gutter, an 8 px gap and the plot, over rows of the plot (160 high), 4 px, the
//! x labels (a 16 px line), 8 px and the legend. Three 1 px `border_subtle` lines cross the plot at
//! 0, 50 and 100%. The plot is cut into equal slots, one per column, each holding a 12 px bar
//! centred in it: the series stacked from the bottom, series 0 lowest, a 1 px gap between the
//! segments, flat ends. The app passes the scale (`max`, three y labels) because the scale is its
//! data.
//!
//! A segment's height tweens with `Kind::Value` on its share of the plot (`value / max`), so a
//! change of `max` animates the same way a change of a value does. `live` snaps. A column is keyed
//! by its index: a rolling window morphs in place, nothing slides. A new column appears at its
//! height at once.
//!
//! The top y label is centred on the top grid line, so it reaches 8 px above the rect; the bottom
//! one reaches 8 px below the plot.

use std::hash::Hash;

use crate::access::{Node, Role};
use crate::color::Rgba;
use crate::motion::{Kind, Motion};
use crate::paint::Rect;
use crate::text::{Spec, Style};
use crate::ui::{Sense, Ui};

/// The value change: the `value` kind (`fast`, quad-out; Reduced and Off snap).
pub const VALUE: Motion = Motion::of(Kind::Value);

/// The plot's height.
pub const PLOT: f32 = 160.0;
/// The y labels' column.
pub const GUTTER: f32 = 40.0;
/// Between the gutter and the plot.
pub const GUTTER_GAP: f32 = 8.0;
/// Between the plot and the x labels.
pub const ROW_GAP: f32 = 4.0;
/// A bar's width, narrower when the slot is.
pub const BAR: f32 = 12.0;
/// Between two segments of a bar (a segment under 1 px high brings a smaller gap, so the stack
/// stays continuous while a segment grows from nothing).
pub const SEG_GAP: f32 = 1.0;
/// A legend swatch's side.
pub const SWATCH: f32 = 8.0;
/// A legend swatch's corner radius.
pub const SWATCH_RADIUS: f32 = 3.0;
/// A label's and a legend entry's line.
pub const LABEL_LINE: f32 = 16.0;
/// Between the x labels and the legend.
pub const LEGEND_GAP: f32 = 8.0;
/// Between a swatch and its name.
pub const KEY_GAP: f32 = 8.0;
/// Between two legend entries.
pub const KEY_SPACING: f32 = 12.0;

/// One series: its legend name and the colour of its segments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarSeries<'a> {
    pub name: &'a str,
    pub color: Rgba,
}

/// One column: a value per series (series 0 first; a missing one reads 0) and the text of its
/// tooltip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarColumn<'a> {
    pub values: &'a [f64],
    /// The tooltip while the column is hovered; empty shows none.
    pub tip: &'a str,
}

/// What the chart shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarChartOptions<'a> {
    pub series: &'a [BarSeries<'a>],
    pub columns: &'a [BarColumn<'a>],
    /// The value at the plot's top. Not above 0, or not finite, is an empty chart.
    pub max: f64,
    /// The top, middle and bottom y labels, right-aligned in the gutter.
    pub y_labels: [&'a str; 3],
    /// X labels: the column they sit under and their text, centred in the slot and elided to the
    /// distance to the next label.
    pub x_labels: &'a [(usize, &'a str)],
    /// The accessible name; empty falls to `luneui.bar-chart.name`.
    pub label: &'a str,
    /// The accessible description (a summary of the data).
    pub description: &'a str,
    /// Continuously updating data: a change snaps.
    pub live: bool,
}

impl Default for BarChartOptions<'_> {
    fn default() -> Self {
        Self {
            series: &[],
            columns: &[],
            max: 0.0,
            y_labels: [""; 3],
            x_labels: &[],
            label: "",
            description: "",
            live: false,
        }
    }
}

/// What the app learns from a frame of the chart.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BarChartResponse {
    /// The column under the pointer.
    pub hovered: Option<usize>,
}

/// The height [`Ui::bar_chart`] occupies: the plot, the gap, the x labels, the gap and the legend.
pub fn height() -> f32 {
    PLOT + ROW_GAP + LABEL_LINE + LEGEND_GAP + LABEL_LINE
}

/// A value the chart can draw: finite and above 0, else 0.
fn clean(v: f64) -> f64 {
    if v.is_finite() && v > 0.0 { v } else { 0.0 }
}

/// Whether the chart has anything to scale: a usable `max` and a value above 0.
fn has_data(o: &BarChartOptions<'_>) -> bool {
    o.max.is_finite()
        && o.max > 0.0
        && o.columns.iter().any(|c| {
            c.values
                .iter()
                .take(o.series.len())
                .any(|&v| clean(v) > 0.0)
        })
}

impl Ui<'_> {
    /// A chart under `key` with the top left and width of `rect` (its height is [`height`]).
    /// Returns the column hovered. The node is an `Image` named `opts.label` (or the toolkit's
    /// `luneui.bar-chart.name`) with the description, over a `StaticText` per legend entry.
    pub fn bar_chart(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &BarChartOptions<'_>,
    ) -> BarChartResponse {
        let id = self.id(key);
        let t = *self.theme();
        let name = if opts.label.is_empty() {
            self.tr("luneui.bar-chart.name")
        } else {
            opts.label
        };
        let mut node = Node::new(
            id,
            Role::Image,
            name,
            Rect::new(rect.x, rect.y, rect.w, height()),
        );
        if !opts.description.is_empty() {
            node.description = Some(opts.description.to_owned());
        }
        let empty = !has_data(opts);
        let mut response = BarChartResponse::default();
        self.node_container(node, |ui| {
            let plot_x = rect.x + GUTTER + GUTTER_GAP;
            let plot_w = (rect.w - GUTTER - GUTTER_GAP).max(0.0);
            let plot_bottom = rect.y + PLOT;
            let n = opts.columns.len();
            let slot_w = if n == 0 { 0.0 } else { plot_w / n as f32 };

            // Grid: 0 and 100% are the plot's first and last pixel rows, 50% the middle.
            let tops = [0.0, PLOT / 2.0, PLOT - 1.0];
            for top in tops {
                ui.painter().fill(
                    Rect::new(plot_x, rect.y + top, plot_w, 1.0),
                    0.0,
                    t.border_subtle,
                );
            }
            // Y labels: centred on the lines, right-aligned against the plot's gutter.
            if !empty {
                let style = Style::mono(t.font_s, LABEL_LINE, t.fg_tertiary);
                for (label, centre) in opts.y_labels.iter().zip([0.0, PLOT / 2.0, PLOT]) {
                    if label.is_empty() {
                        continue;
                    }
                    let spec = ui.painter().elide(&Spec::plain(*label, style), GUTTER);
                    let (w, _) = ui.painter().measure(&spec);
                    ui.painter().text(
                        &spec,
                        rect.x + GUTTER - w,
                        rect.y + centre - LABEL_LINE / 2.0,
                    );
                }
            }

            // Bars. Every segment calls its animation, zero or not, so the id lives on and the
            // first data a column gets grows from 0.
            let bar_w = BAR.min(slot_w);
            for (c, column) in opts.columns.iter().enumerate() {
                let slot_x = plot_x + c as f32 * slot_w;
                let bar_x = slot_x + (slot_w - bar_w) / 2.0;
                let mut used = 0.0_f32;
                for (s, series) in opts.series.iter().enumerate() {
                    let v = if empty {
                        0.0
                    } else {
                        clean(column.values.get(s).copied().unwrap_or(0.0))
                    };
                    let target = if empty {
                        0.0
                    } else {
                        (v / opts.max).clamp(0.0, 1.0) as f32
                    };
                    let aid = id.with((c as u64, s as u64));
                    let share = if opts.live {
                        ui.snap_f32(aid, target)
                    } else {
                        ui.animate_f32(aid, target, &VALUE)
                    };
                    let h = share * PLOT;
                    if h <= 0.0 {
                        continue;
                    }
                    let bottom = plot_bottom - used;
                    let top = (bottom - h).max(rect.y);
                    if bottom > top {
                        ui.painter().fill(
                            Rect::new(bar_x, top, bar_w, bottom - top),
                            0.0,
                            series.color,
                        );
                    }
                    used += h + h.min(SEG_GAP);
                }
            }

            // X labels: centred in their slot, at most as wide as the distance to the next label.
            let style = Style::mono(t.font_s, LABEL_LINE, t.fg_tertiary);
            let label_y = plot_bottom + ROW_GAP;
            let mut labels: Vec<(usize, &str)> = opts
                .x_labels
                .iter()
                .copied()
                .filter(|&(i, text)| i < n && !text.is_empty())
                .collect();
            labels.sort_by_key(|&(i, _)| i);
            for (k, &(i, text)) in labels.iter().enumerate() {
                let next = labels.get(k + 1).map_or(n, |&(j, _)| j);
                let room = next.saturating_sub(i).max(1) as f32 * slot_w;
                let spec = ui.painter().elide(&Spec::plain(text, style), room);
                let (w, _) = ui.painter().measure(&spec);
                let centre = plot_x + (i as f32 + 0.5) * slot_w;
                ui.painter().text(&spec, centre - w / 2.0, label_y);
            }

            // Hover: one region per slot over the whole plot.
            for (c, column) in opts.columns.iter().enumerate() {
                let slot = Rect::new(plot_x + c as f32 * slot_w, rect.y, slot_w, PLOT);
                let r = ui.interact(id.with(("slot", c as u64)), slot, Sense::HOVER);
                if r.hovered {
                    response.hovered = Some(c);
                }
                if !column.tip.is_empty() {
                    ui.tooltip(&r, column.tip);
                }
            }

            // Legend.
            let legend_y = label_y + LABEL_LINE + LEGEND_GAP;
            let legend_style = Style {
                line: LABEL_LINE,
                ..Style::ui(t.font_s, t.fg_secondary)
            };
            let mut x = plot_x;
            let right = rect.x + rect.w;
            for (s, series) in opts.series.iter().enumerate() {
                if x >= right {
                    break;
                }
                let swatch = Rect::new(x, legend_y + (LABEL_LINE - SWATCH) / 2.0, SWATCH, SWATCH);
                ui.painter().fill(swatch, SWATCH_RADIUS, series.color);
                let text_x = x + SWATCH + KEY_GAP;
                let spec = ui.painter().elide(
                    &Spec::plain(series.name, legend_style),
                    (right - text_x).max(0.0),
                );
                let (w, h) = ui.painter().text(&spec, text_x, legend_y);
                ui.node(Node::new(
                    id.with(("legend", s as u64)),
                    Role::StaticText,
                    series.name,
                    Rect::new(x, legend_y, SWATCH + KEY_GAP + w, h.max(LABEL_LINE)),
                ));
                x = text_x + w + KEY_SPACING;
            }
        });
        response
    }
}
