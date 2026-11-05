//! The pie chart (`data.pie-chart`, D164, D166): a fan of sectors with value transitions.
//!
//! A 160 px disc drawn from the top-left of the rect the app gives. The first slice starts at the
//! top and the slices turn clockwise, each sweeping its share of the turn. A white highlight of
//! alpha 38 covers the top half, drawn after the slices. There is no legend, hover or slice node:
//! the app names the chart and describes its data.
//!
//! Each slice's share tweens with `Kind::Value`, and the shares drawn are divided by their sum
//! every frame, so the sweeps total a whole turn even in flight. `live` snaps. A slice the app
//! stops passing leaves at once and the rest take its place; from no slice to some, and a slice
//! new to a chart that has others, the shares appear at once.

use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::hash::Hash;

use crate::access::{Node, Role};
use crate::color::Rgba;
use crate::motion::{Kind, Motion};
use crate::paint::Rect;
use crate::ui::Ui;

/// The value change: the `value` kind (`fast`, quad-out; Reduced and Off snap).
pub const VALUE: Motion = Motion::of(Kind::Value);

/// The disc's diameter.
pub const DIAMETER: f32 = 160.0;
/// The glass highlight over the top half.
pub const HIGHLIGHT: Rgba = [255, 255, 255, 38];

/// One slice: the app's key (what the slice's tween is kept under), its size and its colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PieSlice {
    pub key: u64,
    /// The size. Not above 0, or not finite, is skipped.
    pub value: f64,
    pub color: Rgba,
}

/// What the chart shows.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PieOptions<'a> {
    pub slices: &'a [PieSlice],
    /// The accessible name; empty falls to `luneui.pie-chart.name`.
    pub label: &'a str,
    /// The accessible description (the slices as text).
    pub description: &'a str,
    /// Continuously updating data: a change snaps.
    pub live: bool,
}

/// The sweep of each of `values` in radians, so that the sweeps total a whole turn: its share of
/// the total times 2π. A value not above 0 or not finite sweeps 0 and adds nothing to the total. A
/// total not above 0 gives an empty list.
pub fn sweeps(values: &[f32]) -> Vec<f32> {
    let usable = |v: f32| v.is_finite() && v > 0.0;
    let total: f32 = values.iter().copied().filter(|&v| usable(v)).sum();
    if !(total.is_finite() && total > 0.0) {
        return Vec::new();
    }
    values
        .iter()
        .map(|&v| if usable(v) { v / total * TAU } else { 0.0 })
        .collect()
}

impl Ui<'_> {
    /// A disc under `key` at the top left of `rect` (its size is [`DIAMETER`]). The node is an
    /// `Image` named `opts.label` (or the toolkit's `luneui.pie-chart.name`) with the
    /// description.
    pub fn pie_chart(&mut self, key: impl Hash, rect: Rect, opts: &PieOptions<'_>) {
        let id = self.id(key);
        let name = if opts.label.is_empty() {
            self.tr("luneui.pie-chart.name")
        } else {
            opts.label
        };
        let mut node = Node::new(
            id,
            Role::Image,
            name,
            Rect::new(rect.x, rect.y, DIAMETER, DIAMETER),
        );
        if !opts.description.is_empty() {
            node.description = Some(opts.description.to_owned());
        }
        self.node(node);

        // Raw shares of the usable slices, as the totals of this frame's data.
        let usable: Vec<&PieSlice> = opts
            .slices
            .iter()
            .filter(|s| s.value.is_finite() && s.value > 0.0)
            .collect();
        let total: f64 = usable.iter().map(|s| s.value).sum();
        if usable.is_empty() || !(total.is_finite() && total > 0.0) {
            return;
        }
        let shares: Vec<f32> = usable
            .iter()
            .map(|s| {
                let target = (s.value / total) as f32;
                let aid = id.with(s.key);
                if opts.live {
                    self.snap_f32(aid, target)
                } else {
                    self.animate_f32(aid, target, &VALUE)
                }
            })
            .collect();
        let turn = sweeps(&shares);
        if turn.is_empty() {
            return;
        }
        let r = DIAMETER / 2.0;
        let centre = [rect.x + r, rect.y + r];
        let p = self.painter();
        let mut start = -FRAC_PI_2;
        for (slice, sweep) in usable.iter().zip(turn) {
            p.sector(centre, r, start, sweep, slice.color);
            start += sweep;
        }
        p.sector(centre, r, -PI, PI, HIGHLIGHT);
    }
}
