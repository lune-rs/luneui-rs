//! The stacked bar chart (`data.bar-chart` acceptance, D164, D165): the entry's probes at scale 2,
//! the stack order and the segment gap, the labels, the empty chart, values that are not numbers,
//! the column tooltip, the value transitions (a value, `max`, empty to data and back, `live`, a
//! column count) on every tier, and the semantic nodes.

use lune_ui_core::access::{Role, SemanticTree};
use lune_ui_core::color::{Rgba, premul};
use lune_ui_core::input::Repaint;
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::scene::{Scene, Shape, TextItem};
use lune_ui_core::testing::{ScriptedInput, UiHarness, disabled_violations};
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::bar_chart::{
    BarChartOptions, BarChartResponse, BarColumn, BarSeries, VALUE, height,
};

const WIN: (f32, f32) = (600.0, 400.0);
/// The plot starts at x 58 and, at 100 wide, holds two slots of 50.
const RECT: Rect = Rect::new(10.0, 20.0, 148.0, 0.0);
const LOW: Rgba = [10, 200, 30, 255];
const HIGH: Rgba = [200, 10, 30, 255];
const SERIES: [BarSeries<'static>; 2] = [
    BarSeries {
        name: "Cache read",
        color: LOW,
    },
    BarSeries {
        name: "Uncached",
        color: HIGH,
    },
];

fn column(values: &'static [f64], tip: &'static str) -> BarColumn<'static> {
    BarColumn { values, tip }
}

fn base() -> BarChartOptions<'static> {
    static COLUMNS: [BarColumn<'static>; 2] = [
        BarColumn {
            values: &[40.0, 40.0],
            tip: "00:00: 80",
        },
        BarColumn {
            values: &[80.0, 0.0],
            tip: "01:00: 80",
        },
    ];
    BarChartOptions {
        series: &SERIES,
        columns: &COLUMNS,
        max: 100.0,
        y_labels: ["1.4M", "700k", "0"],
        x_labels: &[(0, "00"), (1, "03")],
        ..BarChartOptions::default()
    }
}

struct Drawn {
    scene: Scene,
    tree: SemanticTree,
    repaint: Repaint,
    response: BarChartResponse,
}

fn draw_in(h: &mut UiHarness, o: &BarChartOptions<'_>) -> Drawn {
    let mut response = BarChartResponse::default();
    let (scene, out) = h.frame(|ui| response = ui.bar_chart("c", RECT, o));
    Drawn {
        scene,
        tree: out.tree,
        repaint: out.repaint,
        response,
    }
}

fn draw(o: &BarChartOptions<'_>) -> Drawn {
    draw_in(&mut UiHarness::new(WIN), o)
}

fn under(scene: &Scene) -> Vec<Shape> {
    scene.layers[0].geometry.under.clone()
}

fn of_color(scene: &Scene, c: Rgba) -> Vec<Shape> {
    under(scene)
        .into_iter()
        .filter(|s| s.color == premul(c))
        .collect()
}

fn texts(scene: &Scene) -> Vec<TextItem> {
    scene.layers[0].texts.clone()
}

fn label_spec(text: &str, scale: f32) -> u64 {
    Spec::plain(text, Style::mono(11.0, 16.0, DARK.fg_tertiary)).cache_key(scale)
}

fn at_scale_2() -> UiHarness {
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    h
}

#[test]
fn the_entry_probes_at_scale_2_plot_320_bar_24_grid_2_gutter_80() {
    let d = draw_in(&mut at_scale_2(), &base());
    let grid = of_color(&d.scene, DARK.border_subtle);
    assert_eq!(grid.len(), 3);
    assert!(grid.iter().all(|s| s.rect[3] == 2.0), "a 2 px line");
    assert_eq!(
        grid[2].rect[1] + grid[2].rect[3] - grid[0].rect[1],
        320.0,
        "the plot is 320 px between the first line's top and the last one's bottom"
    );
    assert_eq!(grid[0].rect[0], 2.0 * 58.0);
    assert_eq!(grid[0].rect[2], 2.0 * 100.0);
    let bars = of_color(&d.scene, LOW);
    assert_eq!(bars[0].rect[2], 24.0, "a bar is 12 logical px");
    // Centred in its 50 px slot.
    assert_eq!(bars[0].rect[0], 2.0 * (58.0 + 19.0));
    // The gutter: the right edge of a y label is 40 px from the rect's left.
    let top = texts(&d.scene)
        .into_iter()
        .find(|t| t.key.0 == label_spec("1.4M", 2.0))
        .expect("the top y label");
    let width = 4.0 * 11.0 * 0.5;
    assert_eq!((top.x + width - RECT.x) * 2.0, 80.0);
}

#[test]
fn series_0_is_at_the_bottom_and_segments_are_1_px_apart() {
    let d = draw(&base());
    let low = &of_color(&d.scene, LOW)[0];
    let high = &of_color(&d.scene, HIGH)[0];
    let plot_bottom = RECT.y + 160.0;
    assert_eq!(
        low.rect[1] + low.rect[3],
        plot_bottom,
        "series 0 on the floor"
    );
    assert_eq!(low.rect[3], 0.4 * 160.0);
    assert_eq!(high.rect[3], 0.4 * 160.0);
    assert_eq!(
        high.rect[1] + high.rect[3],
        low.rect[1] - 1.0,
        "a 1 px gap between the segments"
    );
    let d2 = draw_in(&mut at_scale_2(), &base());
    let low = &of_color(&d2.scene, LOW)[0];
    let high = &of_color(&d2.scene, HIGH)[0];
    assert_eq!(
        high.rect[1] + high.rect[3],
        low.rect[1] - 2.0,
        "2 px at scale 2"
    );
}

#[test]
fn a_zero_segment_draws_nothing_and_leaves_no_gap() {
    let d = draw(&base());
    // Column 1 is [80, 0]: one segment, flush with the plot's floor, none for the second series.
    let bars = |c| {
        of_color(&d.scene, c)
            .into_iter()
            .filter(|s| s.rect[2] == 12.0)
            .collect::<Vec<_>>()
    };
    let low = bars(LOW);
    assert_eq!(low.len(), 2);
    assert_eq!(low[1].rect[1] + low[1].rect[3], RECT.y + 160.0);
    assert_eq!(bars(HIGH).len(), 1);
}

#[test]
fn y_labels_are_centred_on_their_lines_and_right_aligned() {
    let d = draw(&base());
    let t = texts(&d.scene);
    let find = |s: &str| {
        t.iter()
            .find(|t| t.key.0 == label_spec(s, 1.0))
            .unwrap_or_else(|| panic!("label {s}"))
    };
    let grid = of_color(&d.scene, DARK.border_subtle);
    for (text, line) in [("1.4M", &grid[0]), ("700k", &grid[1]), ("0", &grid[2])] {
        let l = find(text);
        let centre = l.y + 8.0;
        let line_centre = line.rect[1] + 0.5;
        assert!(
            (centre - line_centre).abs() <= 1.0,
            "{text}: {centre} vs {line_centre}"
        );
        let width = text.chars().count() as f32 * 5.5;
        assert_eq!(
            l.x + width,
            RECT.x + 40.0,
            "{text} ends at the gutter's edge"
        );
    }
}

#[test]
fn x_labels_are_centred_in_their_slot_below_the_plot() {
    let d = draw(&base());
    let t = texts(&d.scene);
    let l = t
        .iter()
        .find(|t| t.key.0 == label_spec("03", 1.0))
        .expect("x label");
    assert_eq!(l.y, RECT.y + 160.0 + 4.0);
    let slot_centre = 58.0 + 75.0;
    assert_eq!(l.x + 5.5, slot_centre, "two chars of 5.5, centred");
}

#[test]
fn an_x_label_is_elided_to_the_distance_to_the_next_label() {
    // Twelve chars of 5.5 px are 66: more than the 50 px slot, less than the plot's 100.
    let long = "000000000000";
    let o = BarChartOptions {
        x_labels: &[(0, long), (1, "03")],
        ..base()
    };
    let d = draw(&o);
    assert!(
        texts(&d.scene)
            .iter()
            .all(|t| t.key.0 != label_spec(long, 1.0)),
        "it does not fit the one slot to the next label"
    );
    // The last label has the rest of the plot.
    let o = BarChartOptions {
        x_labels: &[(0, long)],
        ..base()
    };
    let d = draw(&o);
    assert!(
        texts(&d.scene)
            .iter()
            .any(|t| t.key.0 == label_spec(long, 1.0)),
        "with no label after it, it has the plot to the end"
    );
}

// Six columns of 16.67 px: a twelve-char label (66 px) needs four slots.
const SIX: [BarColumn<'static>; 6] = [
    BarColumn {
        values: &[10.0, 10.0],
        tip: "",
    },
    BarColumn {
        values: &[10.0, 10.0],
        tip: "",
    },
    BarColumn {
        values: &[10.0, 10.0],
        tip: "",
    },
    BarColumn {
        values: &[10.0, 10.0],
        tip: "",
    },
    BarColumn {
        values: &[10.0, 10.0],
        tip: "",
    },
    BarColumn {
        values: &[10.0, 10.0],
        tip: "",
    },
];

#[test]
fn x_labels_are_ordered_by_column_whatever_the_order_they_are_given_in() {
    let long = "000000000000";
    // Given last-first: column 0's label still has only the three slots to the label at 3.
    let o = BarChartOptions {
        columns: &SIX,
        x_labels: &[(3, "03"), (0, long)],
        ..base()
    };
    let d = draw(&o);
    assert!(
        texts(&d.scene)
            .iter()
            .all(|t| t.key.0 != label_spec(long, 1.0)),
        "elided to the distance to the next label by column"
    );
    // Given in order: the same.
    let o = BarChartOptions {
        columns: &SIX,
        x_labels: &[(0, long), (3, "03")],
        ..base()
    };
    let d = draw(&o);
    assert!(
        texts(&d.scene)
            .iter()
            .all(|t| t.key.0 != label_spec(long, 1.0))
    );
    // And the label at 3 has the rest of the plot, in either order: eight chars (44 px) fit
    // its three slots (50 px).
    let eight = "00000000";
    for labels in [[(3, eight), (0, "03")], [(0, "03"), (3, eight)]] {
        let o = BarChartOptions {
            columns: &SIX,
            x_labels: &labels,
            ..base()
        };
        let d = draw(&o);
        assert!(
            texts(&d.scene)
                .iter()
                .any(|t| t.key.0 == label_spec(eight, 1.0)),
            "{labels:?}"
        );
    }
}

#[test]
fn a_label_for_a_column_that_is_not_there_is_not_drawn() {
    let o = BarChartOptions {
        x_labels: &[(0, "00"), (2, "zz"), (9, "yy")],
        ..base()
    };
    let d = draw(&o);
    for gone in ["zz", "yy"] {
        assert!(
            texts(&d.scene)
                .iter()
                .all(|t| t.key.0 != label_spec(gone, 1.0)),
            "{gone}: two columns only"
        );
    }
    assert!(
        texts(&d.scene)
            .iter()
            .any(|t| t.key.0 == label_spec("00", 1.0))
    );
}

#[test]
fn values_past_the_series_count_do_not_make_an_empty_chart_a_chart() {
    // Two series, three values: the third is nobody's.
    static COLUMNS: [BarColumn<'static>; 2] = [
        BarColumn {
            values: &[0.0, 0.0, 50.0],
            tip: "",
        },
        BarColumn {
            values: &[0.0, 0.0, 70.0],
            tip: "",
        },
    ];
    let o = BarChartOptions {
        columns: &COLUMNS,
        ..base()
    };
    let d = draw(&o);
    assert!(
        of_color(&d.scene, LOW).iter().all(|s| s.rect[2] == 8.0),
        "no bar of series 0"
    );
    assert!(
        texts(&d.scene)
            .iter()
            .all(|t| t.key.0 != label_spec("1.4M", 1.0)),
        "still empty: no y labels"
    );
    assert!(
        texts(&d.scene)
            .iter()
            .any(|t| t.key.0 == label_spec("00", 1.0)),
        "x labels stay"
    );
}

#[test]
fn a_segment_under_a_pixel_takes_a_smaller_gap_than_a_pixel() {
    // 0.5 px of series 0 (value 0.3125 of 100 over 160 px), then series 1.
    static COLUMNS: [BarColumn<'static>; 1] = [BarColumn {
        values: &[0.3125, 40.0],
        tip: "",
    }];
    let o = BarChartOptions {
        columns: &COLUMNS,
        ..base()
    };
    let mut h = UiHarness::new(WIN);
    draw_in(&mut h, &o);
    h.at_ms(5000);
    let d = draw_in(&mut h, &o);
    let bottom = RECT.y + 160.0;
    let low = of_color(&d.scene, LOW)
        .into_iter()
        .find(|s| s.rect[2] == 12.0)
        .expect("series 0");
    assert_eq!(low.rect[3], 0.5);
    let high = of_color(&d.scene, HIGH)
        .into_iter()
        .find(|s| s.rect[2] == 12.0)
        .expect("series 1");
    // The gap is the segment's own half pixel, not a whole one: 0.5 + 0.5.
    assert_eq!(high.rect[1] + high.rect[3], bottom - 1.0);
}

#[test]
fn the_legend_names_the_series_with_swatches_of_8_px_radius_3() {
    let d = draw(&base());
    let swatch = of_color(&d.scene, LOW)
        .into_iter()
        .find(|s| s.rect[2] == 8.0)
        .expect("a swatch");
    assert_eq!(swatch.rect[3], 8.0);
    assert_eq!(swatch.radii, [3.0; 4]);
    let legend: Vec<_> = d
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::StaticText)
        .map(|n| n.name.as_str())
        .collect();
    assert_eq!(legend, ["Cache read", "Uncached"]);
    assert!(height() > 160.0 + 4.0 + 16.0 + 16.0);
}

#[test]
fn an_empty_chart_keeps_the_grid_and_the_x_labels() {
    for o in [
        BarChartOptions { max: 0.0, ..base() },
        BarChartOptions {
            max: f64::NAN,
            ..base()
        },
        BarChartOptions {
            max: f64::INFINITY,
            ..base()
        },
        BarChartOptions {
            max: -3.0,
            ..base()
        },
        BarChartOptions {
            columns: &[BarColumn {
                values: &[0.0, 0.0],
                tip: "",
            }],
            ..base()
        },
        BarChartOptions {
            columns: &[BarColumn {
                values: &[f64::NAN, -2.0],
                tip: "",
            }],
            ..base()
        },
    ] {
        let d = draw(&o);
        assert_eq!(of_color(&d.scene, DARK.border_subtle).len(), 3);
        assert!(
            of_color(&d.scene, LOW).iter().all(|s| s.rect[2] == 8.0),
            "no bars"
        );
        assert!(of_color(&d.scene, HIGH).iter().all(|s| s.rect[2] == 8.0));
        let t = texts(&d.scene);
        assert!(
            t.iter().all(|t| t.key.0 != label_spec("1.4M", 1.0)),
            "no y labels"
        );
        assert!(t.iter().all(|t| t.key.0 != label_spec("0", 1.0)));
        assert!(
            t.iter().any(|t| t.key.0 == label_spec("00", 1.0)),
            "the x labels stay"
        );
    }
}

#[test]
fn values_that_are_not_numbers_never_panic_and_count_as_zero() {
    let weird: [&[f64]; 6] = [
        &[f64::NAN, f64::INFINITY],
        &[f64::NEG_INFINITY, -1.0],
        &[1e308, 1e308],
        &[],
        &[5.0],
        &[5.0, 5.0, 5.0, 5.0],
    ];
    let columns: Vec<BarColumn<'_>> = weird.iter().map(|v| column(v, "")).collect();
    for max in [100.0, 1e-300, 1e300, f64::MIN_POSITIVE] {
        let o = BarChartOptions {
            columns: &columns,
            max,
            x_labels: &[(99, "late"), (0, ""), (3, "x"), (1, "y")],
            ..base()
        };
        let d = draw(&o);
        for s in under(&d.scene) {
            assert!(s.rect.iter().all(|v| v.is_finite()), "max {max}: {s:?}");
        }
    }
    let none = BarChartOptions {
        series: &[],
        ..base()
    };
    draw(&none);
    let no_columns = BarChartOptions {
        columns: &[],
        ..base()
    };
    let d = draw(&no_columns);
    assert_eq!(d.response.hovered, None);
    draw(&BarChartOptions {
        series: &[],
        columns: &[],
        ..base()
    });
    // Narrower than its own gutter.
    let mut h = UiHarness::new(WIN);
    h.frame(|ui| {
        ui.bar_chart("narrow", Rect::new(0.0, 0.0, 20.0, 0.0), &base());
    });
}

#[test]
fn a_stack_above_max_is_cut_at_the_top_of_the_plot() {
    let o = BarChartOptions {
        columns: &[BarColumn {
            values: &[90.0, 90.0],
            tip: "",
        }],
        ..base()
    };
    let d = draw(&o);
    let high = &of_color(&d.scene, HIGH)[0];
    assert_eq!(high.rect[1], RECT.y, "cut at the plot's top");
    assert!(high.rect[3] > 0.0 && high.rect[3] < 0.9 * 160.0);
}

fn hover_slot(h: &mut UiHarness, slot: usize) {
    h.input
        .pointer_to(58.0 + 50.0 * slot as f32 + 25.0, RECT.y + 80.0);
}

#[test]
fn hovered_reports_the_slot_under_the_pointer() {
    let mut h = UiHarness::new(WIN);
    draw_in(&mut h, &base());
    assert_eq!(draw_in(&mut h, &base()).response.hovered, None);
    hover_slot(&mut h, 1);
    draw_in(&mut h, &base());
    assert_eq!(draw_in(&mut h, &base()).response.hovered, Some(1));
    hover_slot(&mut h, 0);
    draw_in(&mut h, &base());
    assert_eq!(draw_in(&mut h, &base()).response.hovered, Some(0));
    h.input.pointer_to(5.0, 5.0);
    draw_in(&mut h, &base());
    assert_eq!(draw_in(&mut h, &base()).response.hovered, None);
}

fn popup_texts(scene: &Scene) -> Vec<TextItem> {
    scene.merged(LayerId::Popup).texts.clone()
}

#[test]
fn a_column_shows_its_tip_after_the_delay_and_an_empty_tip_shows_none() {
    let tip_key = Spec::plain("00:00: 80", Style::ui(11.0, DARK.fg_primary)).cache_key(1.0);
    let mut h = UiHarness::new(WIN);
    hover_slot(&mut h, 0);
    h.at_ms(0);
    draw_in(&mut h, &base());
    h.at_ms(10);
    draw_in(&mut h, &base());
    h.at_ms(799);
    let d = draw_in(&mut h, &base());
    assert!(popup_texts(&d.scene).is_empty(), "not yet");
    h.at_ms(800);
    let d = draw_in(&mut h, &base());
    let shown = popup_texts(&d.scene);
    assert_eq!(shown.len(), 1, "appearing");
    h.at_ms(1200);
    let d = draw_in(&mut h, &base());
    let shown = popup_texts(&d.scene);
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].key.0, tip_key, "the exact tip text, once settled");
    assert!(
        d.tree.nodes.iter().any(|n| n.role == Role::Tooltip),
        "the tooltip is in the tree"
    );
    // The slot's hit region has no node: the tooltip's patch of its anchor finds nothing.
    assert!(
        d.tree
            .nodes
            .iter()
            .all(|n| n.description.as_deref() != Some("00:00: 80")),
        "no node took the tip as its description"
    );

    // A column with an empty tip shows nothing, however long the pointer rests.
    let quiet = BarChartOptions {
        columns: &[column(&[40.0, 40.0], ""), column(&[80.0, 0.0], "")],
        ..base()
    };
    let mut h = UiHarness::new(WIN);
    hover_slot(&mut h, 0);
    for ms in [0, 10, 900, 2000] {
        h.at_ms(ms);
        let d = draw_in(&mut h, &quiet);
        assert!(popup_texts(&d.scene).is_empty(), "{ms}");
        assert!(d.tree.nodes.iter().all(|n| n.role != Role::Tooltip));
    }
}

#[test]
fn the_tip_does_not_rename_the_chart_or_leave_a_node_for_a_slot() {
    let d = draw(&base());
    let images: Vec<_> = d
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::Image)
        .collect();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].name, "Bar chart");
}

/// The first bar's height in px (logical, scale 1).
fn low_height(scene: &Scene, column: usize) -> f32 {
    of_color(scene, LOW)
        .into_iter()
        .filter(|s| s.rect[2] == 12.0)
        .nth(column)
        .map_or(0.0, |s| s.rect[3])
}

fn with_first(v: &'static [f64]) -> [BarColumn<'static>; 1] {
    [BarColumn { values: v, tip: "" }]
}

#[test]
fn the_value_row_conforms_under_every_tier() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        conformance(&VALUE, &DARK.motion, tier, 0.0_f32, 1.0)
            .unwrap_or_else(|e| panic!("value {tier:?}: {e}"));
    }
}

#[test]
fn a_change_of_value_animates_once_and_ends_exactly_at_its_height() {
    let first = with_first(&[25.0, 0.0]);
    let second = with_first(&[75.0, 0.0]);
    let o1 = BarChartOptions {
        columns: &first,
        ..base()
    };
    let o2 = BarChartOptions {
        columns: &second,
        ..base()
    };
    let mut h = UiHarness::new(WIN);
    h.at_ms(1000);
    let rest = draw_in(&mut h, &o1);
    assert_eq!(rest.repaint, Repaint::Idle);
    assert_eq!(low_height(&rest.scene, 0), 40.0);
    let start = draw_in(&mut h, &o2);
    assert_eq!(start.repaint, Repaint::Now);
    assert_eq!(low_height(&start.scene, 0), 40.0, "starts where it was");
    h.at_ms(1090);
    let mid = draw_in(&mut h, &o2);
    assert_eq!(mid.repaint, Repaint::Now);
    let m = low_height(&mid.scene, 0);
    assert!(m > 40.0 && m < 120.0, "{m}");
    h.at_ms(1180);
    let end = draw_in(&mut h, &o2);
    assert_eq!(low_height(&end.scene, 0), 120.0);
    assert_eq!(end.repaint, Repaint::Idle);
    h.at_ms(2000);
    let after = draw_in(&mut h, &o2);
    assert_eq!(after.repaint, Repaint::Idle);
    assert_eq!(after.scene, end.scene);
}

#[test]
fn a_new_value_mid_flight_retargets_from_where_it_is() {
    let a = with_first(&[25.0, 0.0]);
    let b = with_first(&[75.0, 0.0]);
    let (oa, ob) = (
        BarChartOptions {
            columns: &a,
            ..base()
        },
        BarChartOptions {
            columns: &b,
            ..base()
        },
    );
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    draw_in(&mut h, &oa);
    draw_in(&mut h, &ob);
    h.at_ms(90);
    let mid = low_height(&draw_in(&mut h, &ob).scene, 0);
    let back = low_height(&draw_in(&mut h, &oa).scene, 0);
    assert!(
        (back - mid).abs() < 1.0,
        "no jump on retarget: {mid} then {back}"
    );
}

#[test]
fn a_change_of_max_animates_the_same_way() {
    let c = with_first(&[50.0, 0.0]);
    let o = |max| BarChartOptions {
        columns: &c,
        max,
        ..base()
    };
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    assert_eq!(low_height(&draw_in(&mut h, &o(100.0)).scene, 0), 80.0);
    let start = draw_in(&mut h, &o(200.0));
    assert_eq!(start.repaint, Repaint::Now);
    h.at_ms(90);
    let mid = low_height(&draw_in(&mut h, &o(200.0)).scene, 0);
    assert!(mid < 80.0 && mid > 40.0, "{mid}");
    h.at_ms(500);
    let end = draw_in(&mut h, &o(200.0));
    assert_eq!(low_height(&end.scene, 0), 40.0);
    assert_eq!(end.repaint, Repaint::Idle);
}

#[test]
fn empty_to_data_grows_from_nothing_and_data_to_empty_shrinks_to_nothing() {
    let data = base();
    let empty = BarChartOptions { max: 0.0, ..base() };
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    let rest = draw_in(&mut h, &empty);
    assert_eq!(rest.repaint, Repaint::Idle);
    assert_eq!(low_height(&rest.scene, 0), 0.0);
    let start = draw_in(&mut h, &data);
    assert_eq!(low_height(&start.scene, 0), 0.0, "grows from 0");
    assert_eq!(start.repaint, Repaint::Now);
    h.at_ms(90);
    let mid = low_height(&draw_in(&mut h, &data).scene, 0);
    assert!(mid > 0.0 && mid < 64.0, "{mid}");
    h.at_ms(300);
    let full = draw_in(&mut h, &data);
    assert_eq!(low_height(&full.scene, 0), 64.0);
    assert_eq!(full.repaint, Repaint::Idle);

    let back = draw_in(&mut h, &empty);
    assert_eq!(back.repaint, Repaint::Now);
    assert_eq!(low_height(&back.scene, 0), 64.0, "starts from where it was");
    assert!(
        texts(&back.scene)
            .iter()
            .all(|t| t.key.0 != label_spec("1.4M", 1.0)),
        "the y labels go at once"
    );
    h.at_ms(600);
    let gone = draw_in(&mut h, &empty);
    assert_eq!(low_height(&gone.scene, 0), 0.0);
    assert_eq!(gone.repaint, Repaint::Idle);
}

#[test]
fn live_snaps_and_a_live_turned_on_mid_flight_settles_at_the_target() {
    let a = with_first(&[25.0, 0.0]);
    let b = with_first(&[75.0, 0.0]);
    let make = |c, live| BarChartOptions {
        columns: c,
        live,
        ..base()
    };
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    draw_in(&mut h, &make(&a, true));
    h.at_ms(10);
    let d = draw_in(&mut h, &make(&b, true));
    assert_eq!(d.repaint, Repaint::Idle);
    assert_eq!(low_height(&d.scene, 0), 120.0);

    // Mid-flight, then live: the target at once, and the next frame is idle.
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    draw_in(&mut h, &make(&a, false));
    draw_in(&mut h, &make(&b, false));
    h.at_ms(60);
    assert_eq!(draw_in(&mut h, &make(&b, false)).repaint, Repaint::Now);
    h.at_ms(70);
    let d = draw_in(&mut h, &make(&b, true));
    assert_eq!(low_height(&d.scene, 0), 120.0);
    assert_eq!(d.repaint, Repaint::Idle);
    h.at_ms(80);
    let d = draw_in(&mut h, &make(&a, false));
    assert_eq!(low_height(&d.scene, 0), 120.0, "animates again from there");
    assert_eq!(d.repaint, Repaint::Now);
}

#[test]
fn a_column_count_change_keeps_the_rest_and_shows_a_new_column_at_once() {
    let two = base();
    let three_cols = [
        column(&[40.0, 40.0], ""),
        column(&[80.0, 0.0], ""),
        column(&[20.0, 0.0], ""),
    ];
    let three = BarChartOptions {
        columns: &three_cols,
        ..base()
    };
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    draw_in(&mut h, &two);
    let d = draw_in(&mut h, &three);
    assert_eq!(d.repaint, Repaint::Idle, "the new column does not tween");
    assert_eq!(low_height(&d.scene, 2), 32.0);
    // Back to two: no leftover motion.
    h.at_ms(10);
    assert_eq!(draw_in(&mut h, &two).repaint, Repaint::Idle);
    // The column index holds its state across the window rolling.
    h.at_ms(20);
    let d = draw_in(&mut h, &three);
    assert_eq!(d.repaint, Repaint::Idle);
}

#[test]
fn off_and_reduced_snap() {
    let a = with_first(&[25.0, 0.0]);
    let b = with_first(&[75.0, 0.0]);
    for tier in [MotionTier::Off, MotionTier::Reduced] {
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        h.at_ms(0);
        draw_in(
            &mut h,
            &BarChartOptions {
                columns: &a,
                ..base()
            },
        );
        h.at_ms(10);
        let d = draw_in(
            &mut h,
            &BarChartOptions {
                columns: &b,
                ..base()
            },
        );
        assert_eq!(d.repaint, Repaint::Idle, "{tier:?}");
        assert_eq!(low_height(&d.scene, 0), 120.0, "{tier:?}");
    }
}

#[test]
fn the_chart_is_an_image_named_by_the_app_over_its_legend() {
    let o = BarChartOptions {
        label: "Tokens per hour",
        description: "Peaks at 11:00",
        ..base()
    };
    let d = draw(&o);
    let image = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Image)
        .expect("image");
    assert_eq!(image.name, "Tokens per hour");
    assert_eq!(image.description.as_deref(), Some("Peaks at 11:00"));
    assert_eq!(image.children.len(), 2);
    assert_eq!(image.bounds.h, height());
    for id in &image.children {
        let n = d.tree.get(*id).expect("legend entry");
        assert_eq!(n.role, Role::StaticText);
    }
}

#[test]
fn an_empty_label_falls_to_the_toolkit_string_through_the_table() {
    let d = draw(&base());
    let image = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Image)
        .expect("image");
    assert_eq!(image.name, "Bar chart");
    assert_eq!(image.description, None);
    let mut h = UiHarness::new(WIN);
    h.strings
        .insert("luneui.bar-chart.name", "Carta bar")
        .expect("a toolkit key");
    let d = draw_in(&mut h, &base());
    let image = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Image)
        .expect("image");
    assert_eq!(image.name, "Carta bar");
}

#[test]
fn a_disabled_scope_marks_every_node() {
    let mut h = UiHarness::new(WIN);
    let (_, out) = h.frame(|ui| {
        ui.with_enabled(false, |ui| {
            ui.bar_chart("c", RECT, &base());
        });
    });
    let root = WidgetId::ROOT.with("c");
    assert!(out.tree.get(root).is_some());
    assert_eq!(disabled_violations(&out.tree, root), vec![]);
}

#[test]
fn idle_at_rest() {
    let mut h = UiHarness::new(WIN);
    for ms in [0, 16, 5000] {
        h.at_ms(ms);
        assert_eq!(draw_in(&mut h, &base()).repaint, Repaint::Idle, "{ms}");
    }
}
