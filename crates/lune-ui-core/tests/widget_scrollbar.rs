//! The overlay scrollbar (`widget.scrollbar` acceptance): the thumb geometry, when it shows, the
//! drag and its capture, presses that reach no content, the reveal and hover tweens, and the scale
//! 2 pixel probes on the display list.

use lune_ui_core::color::premul;
use lune_ui_core::input::{FrameOutput, PointerButton};
use lune_ui_core::layout::Axis;
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::testing::UiHarness;
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Sense, WidgetId};
use lune_ui_core::widgets::scroll_area::{Bars, Extent, ScrollOptions};
use lune_ui_core::widgets::scrollbar::{
    BAR, HOVER, MIN_LENGTH, RADIUS, REVEAL, THUMB, THUMB_INSET, thumb_geometry,
};

const WIN: (f32, f32) = (400.0, 600.0);
const AREA: Rect = Rect::new(20.0, 20.0, 300.0, 400.0);
const FRAME_MS: f64 = 1000.0 / 60.0;

struct Frame {
    scene: Scene,
    out: FrameOutput,
    offset: (f64, f64),
    clicked: bool,
}

/// One frame of the area with a button under the whole viewport, to see whether a press reaches it.
fn run(h: &mut UiHarness, o: &ScrollOptions, ext: (f64, f64)) -> Frame {
    let mut got = None;
    let (scene, out) = h.frame(|ui| {
        got = Some(ui.scroll_area("a", AREA, o, |ui, view| {
            let r = ui.interact(WidgetId::ROOT.with("under"), view.viewport, Sense::CLICK);
            (r.clicked, Extent::new(ext.0, ext.1))
        }));
    });
    let r = got.expect("frame ran");
    Frame {
        scene,
        out,
        offset: r.offset,
        clicked: r.value,
    }
}

fn at(h: &mut UiHarness, ms: f64) {
    h.at_ms(ms.round() as u64);
}

fn tall() -> (f64, f64) {
    (300.0, 1200.0)
}

fn opts() -> ScrollOptions {
    ScrollOptions::default()
}

/// The thumb fills in a scene: `over` shapes of the base layer.
fn thumbs(scene: &Scene) -> Vec<Shape> {
    scene.layers[0].geometry.over.clone()
}

fn settled(h: &mut UiHarness, o: &ScrollOptions, ext: (f64, f64), from: f64) -> Frame {
    at(h, from + 1000.0);
    run(h, o, ext);
    at(h, from + 2000.0);
    run(h, o, ext)
}

// Geometry.

#[test]
fn the_vertical_thumb_sits_one_px_from_the_right_and_is_proportional() {
    let g = thumb_geometry(Axis::Vertical, AREA, 0.0, 1200.0, false).expect("overflows");
    assert_eq!(g.track, Rect::new(AREA.right() - BAR, AREA.y, BAR, 400.0));
    assert_eq!(g.thumb.x, AREA.right() - THUMB_INSET - THUMB);
    assert_eq!(g.thumb.w, THUMB);
    assert_eq!(g.thumb.y, AREA.y);
    // viewport / extent * track
    assert!((g.thumb.h - 400.0 / 1200.0 * 400.0).abs() < 1e-3);
    assert!((g.travel - (400.0 - 400.0 / 3.0)).abs() < 1e-9);
    assert_eq!(g.range, 800.0);
    let end = thumb_geometry(Axis::Vertical, AREA, 800.0, 1200.0, false).expect("overflows");
    assert!((end.thumb.bottom() - AREA.bottom()).abs() < 1e-3);
    let mid = thumb_geometry(Axis::Vertical, AREA, 400.0, 1200.0, false).expect("overflows");
    assert!((mid.thumb.y - (AREA.y + g.travel as f32 / 2.0)).abs() < 1e-3);
}

#[test]
fn the_horizontal_thumb_sits_one_px_from_the_bottom() {
    let g = thumb_geometry(Axis::Horizontal, AREA, 0.0, 900.0, false).expect("overflows");
    assert_eq!(g.track, Rect::new(AREA.x, AREA.bottom() - BAR, 300.0, BAR));
    assert_eq!(g.thumb.y, AREA.bottom() - THUMB_INSET - THUMB);
    assert_eq!(g.thumb.h, THUMB);
    assert!((g.thumb.w - 100.0).abs() < 1e-3);
}

#[test]
fn the_thumb_has_a_minimum_length_and_the_corner_is_left_clear() {
    let g = thumb_geometry(Axis::Vertical, AREA, 0.0, 1_000_000.0, false).expect("overflows");
    assert_eq!(g.thumb.h, MIN_LENGTH);
    let c = thumb_geometry(Axis::Vertical, AREA, 0.0, 1200.0, true).expect("overflows");
    assert_eq!(c.track.h, 400.0 - BAR);
}

#[test]
fn content_that_fits_or_bad_numbers_give_no_thumb() {
    for ext in [300.0, 400.0, 0.0, f64::NAN, f64::INFINITY, -5.0] {
        assert!(
            thumb_geometry(Axis::Vertical, AREA, 0.0, ext, false).is_none(),
            "{ext}"
        );
    }
    assert!(
        thumb_geometry(
            Axis::Vertical,
            Rect::new(0.0, 0.0, 10.0, 0.0),
            0.0,
            100.0,
            false
        )
        .is_none()
    );
    // a non-finite offset is the top
    let g = thumb_geometry(Axis::Vertical, AREA, f64::NAN, 1200.0, false).expect("overflows");
    assert_eq!(g.thumb.y, AREA.y);
    assert_eq!(g.offset_delta(g.travel / 2.0), 400.0);
}

// When it shows.

#[test]
fn pointer_away_no_thumb_pointer_over_the_area_a_thumb_and_content_that_fits_none() {
    let o = opts();
    let mut h = UiHarness::new(WIN);
    run(&mut h, &o, tall());
    assert!(
        thumbs(&settled(&mut h, &o, tall(), 0.0).scene).is_empty(),
        "away"
    );
    h.input.pointer_to(100.0, 100.0);
    at(&mut h, 3000.0);
    run(&mut h, &o, tall());
    let shown = thumbs(&settled(&mut h, &o, tall(), 3000.0).scene);
    assert_eq!(shown.len(), 1, "over");
    // fits: none even with the pointer over
    let fits = (300.0, 300.0);
    let mut h = UiHarness::new(WIN);
    run(&mut h, &o, fits);
    h.input.pointer_to(100.0, 100.0);
    at(&mut h, 10.0);
    assert!(
        thumbs(&settled(&mut h, &o, fits, 10.0).scene).is_empty(),
        "fits"
    );
}

#[test]
fn hidden_and_disabled_bars_show_no_thumb() {
    for bars in [
        (Bars::Hidden, Bars::Hidden),
        (Bars::Disabled, Bars::Disabled),
    ] {
        let o = ScrollOptions { bars, ..opts() };
        let mut h = UiHarness::new(WIN);
        run(&mut h, &o, tall());
        h.input.pointer_to(100.0, 100.0);
        at(&mut h, 10.0);
        assert!(
            thumbs(&settled(&mut h, &o, tall(), 10.0).scene).is_empty(),
            "{bars:?}"
        );
    }
}

#[test]
fn both_bars_show_when_both_axes_overflow() {
    let o = opts();
    let ext = (900.0, 1200.0);
    let mut h = UiHarness::new(WIN);
    run(&mut h, &o, ext);
    h.input.pointer_to(100.0, 100.0);
    at(&mut h, 10.0);
    assert_eq!(thumbs(&settled(&mut h, &o, ext, 10.0).scene).len(), 2);
}

// Probes at scale 2.

#[test]
fn at_scale_2_the_bar_is_18_device_px_and_the_thumb_12_wide_with_radius_6() {
    let o = opts();
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    run(&mut h, &o, tall());
    h.input.pointer_to(100.0, 100.0);
    at(&mut h, 10.0);
    let f = settled(&mut h, &o, tall(), 10.0);
    let t = thumbs(&f.scene);
    assert_eq!(t.len(), 1);
    let s = t[0];
    assert_eq!(BAR * 2.0, 18.0);
    assert_eq!(s.rect[2], 12.0);
    assert_eq!(s.radii, [6.0; 4]);
    assert_eq!(RADIUS * 2.0, 6.0);
    // 1 px (2 device px) from the viewport's right edge
    assert_eq!(s.rect[0] + s.rect[2], (AREA.right() - 1.0) * 2.0);
    // the fill is `accent` at alpha 0x8C, premultiplied
    assert_eq!(s.color, premul([0x76, 0x7D, 0x85, 0x8C]));
    assert_eq!(
        DARK.scroll_thumb,
        [DARK.accent[0], DARK.accent[1], DARK.accent[2], 0x8C]
    );
    assert_eq!(DARK.scroll_thumb_hover[3], 0xE6);
}

#[test]
fn at_scale_2_the_minimum_thumb_length_is_56_device_px() {
    let o = opts();
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let ext = (300.0, 1_000_000.0);
    run(&mut h, &o, ext);
    h.input.pointer_to(100.0, 100.0);
    at(&mut h, 10.0);
    let t = thumbs(&settled(&mut h, &o, ext, 10.0).scene);
    assert_eq!(t[0].rect[3], 56.0);
}

#[test]
fn the_hovered_thumb_is_accent_at_0xe6() {
    let o = opts();
    let mut h = UiHarness::new(WIN);
    run(&mut h, &o, tall());
    let g = thumb_geometry(Axis::Vertical, AREA, 0.0, 1200.0, false).expect("overflows");
    h.input.pointer_to(g.thumb.x + 3.0, g.thumb.y + 10.0);
    at(&mut h, 10.0);
    run(&mut h, &o, tall());
    let t = thumbs(&settled(&mut h, &o, tall(), 10.0).scene);
    assert_eq!(t[0].color, premul([0x76, 0x7D, 0x85, 0xE6]));
}

// The drag.

/// Frames until the thumb is hovered, then a press on it; the pointer is at the thumb's centre.
fn grab(
    h: &mut UiHarness,
    o: &ScrollOptions,
    g: &lune_ui_core::widgets::scrollbar::ThumbGeometry,
) -> (f32, f32) {
    let p = (g.thumb.x + 3.0, g.thumb.y + g.thumb.h / 2.0);
    h.input.pointer_to(p.0, p.1);
    at(h, 100.0);
    run(h, o, tall());
    h.input.press(PointerButton::Primary);
    at(h, 120.0);
    run(h, o, tall());
    p
}

#[test]
fn dragging_by_half_the_travel_moves_the_offset_by_half_the_range() {
    let o = opts();
    let mut h = UiHarness::new(WIN);
    run(&mut h, &o, tall());
    let g = thumb_geometry(Axis::Vertical, AREA, 0.0, 1200.0, false).expect("overflows");
    let p = grab(&mut h, &o, &g);
    h.input.pointer_to(p.0, p.1 + g.travel as f32 / 2.0);
    at(&mut h, 140.0);
    let f = run(&mut h, &o, tall());
    assert!((f.offset.1 - 400.0).abs() < 0.01, "{:?}", f.offset);
    // the content follows on the catch-up frame; the offset holds while still
    at(&mut h, 160.0);
    let f = run(&mut h, &o, tall());
    assert!((f.offset.1 - 400.0).abs() < 0.01);
    // dragging past the end clamps
    h.input.pointer_to(p.0, p.1 + 1000.0);
    at(&mut h, 180.0);
    assert_eq!(run(&mut h, &o, tall()).offset.1, 800.0);
}

#[test]
fn the_thumb_stays_visible_and_keeps_the_grab_after_the_pointer_leaves_the_area() {
    let o = opts();
    let mut h = UiHarness::new(WIN);
    run(&mut h, &o, tall());
    let g = thumb_geometry(Axis::Vertical, AREA, 0.0, 1200.0, false).expect("overflows");
    let p = grab(&mut h, &o, &g);
    // far outside the area
    h.input.pointer_to(380.0, p.1 + 60.0);
    at(&mut h, 140.0);
    let f = run(&mut h, &o, tall());
    assert!(f.offset.1 > 0.0);
    at(&mut h, 1500.0);
    let f = run(&mut h, &o, tall());
    assert_eq!(thumbs(&f.scene).len(), 1, "still shown while held");
    let before = f.offset.1;
    h.input.pointer_to(385.0, p.1 + 90.0);
    at(&mut h, 1520.0);
    assert!(run(&mut h, &o, tall()).offset.1 > before, "still dragging");
    // release ends the drag; away from the area, the thumb fades out
    h.input.release(PointerButton::Primary);
    at(&mut h, 1540.0);
    let held = run(&mut h, &o, tall()).offset.1;
    h.input.pointer_to(385.0, p.1 + 200.0);
    at(&mut h, 1560.0);
    assert_eq!(run(&mut h, &o, tall()).offset.1, held, "released");
    at(&mut h, 3000.0);
    run(&mut h, &o, tall());
    at(&mut h, 4000.0);
    assert!(thumbs(&run(&mut h, &o, tall()).scene).is_empty());
}

#[test]
fn a_press_on_the_thumb_or_the_empty_track_reaches_no_content() {
    let o = opts();
    let mut h = UiHarness::new(WIN);
    run(&mut h, &o, tall());
    let g = thumb_geometry(Axis::Vertical, AREA, 0.0, 1200.0, false).expect("overflows");
    // thumb: a click without moving
    let on_thumb = (g.thumb.x + 3.0, g.thumb.y + 20.0);
    h.input.pointer_to(on_thumb.0, on_thumb.1);
    at(&mut h, 10.0);
    run(&mut h, &o, tall());
    h.input.press(PointerButton::Primary);
    at(&mut h, 20.0);
    run(&mut h, &o, tall());
    h.input.release(PointerButton::Primary);
    at(&mut h, 30.0);
    let f = run(&mut h, &o, tall());
    assert!(!f.clicked, "thumb press selected the content");
    assert_eq!(f.offset.1, 0.0);
    // empty track below the thumb: nothing happens (D9)
    let on_track = (g.track.x + 4.0, g.thumb.bottom() + 40.0);
    h.input.pointer_to(on_track.0, on_track.1);
    at(&mut h, 40.0);
    run(&mut h, &o, tall());
    h.input.press(PointerButton::Primary);
    at(&mut h, 50.0);
    run(&mut h, &o, tall());
    h.input.release(PointerButton::Primary);
    at(&mut h, 60.0);
    let f = run(&mut h, &o, tall());
    assert!(!f.clicked, "track press reached the content");
    assert_eq!(f.offset.1, 0.0);
    // the content itself still takes a click outside the bar
    h.input.pointer_to(100.0, 100.0);
    at(&mut h, 70.0);
    run(&mut h, &o, tall());
    h.input.click(PointerButton::Primary);
    at(&mut h, 80.0);
    assert!(run(&mut h, &o, tall()).clicked);
}

// Motion.

#[test]
fn reveal_and_hover_colour_conform_in_every_tier() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        conformance(&REVEAL, &DARK.motion, tier, 0.0_f32, 1.0)
            .unwrap_or_else(|e| panic!("reveal {tier:?}: {e}"));
        conformance(
            &HOVER,
            &DARK.motion,
            tier,
            [118.0, 125.0, 133.0, 140.0],
            [118.0, 125.0, 133.0, 230.0],
        )
        .unwrap_or_else(|e| panic!("hover {tier:?}: {e}"));
    }
    // Reduced runs the fade at 120 ms; Off snaps
    let reduced = DARK
        .motion
        .resolve(&REVEAL, MotionTier::Reduced)
        .expect("kept");
    assert_eq!(reduced.duration.as_millis(), 120);
    assert!(DARK.motion.resolve(&REVEAL, MotionTier::Off).is_none());
}

fn thumb_alpha(scene: &Scene) -> f32 {
    thumbs(scene).first().map_or(0.0, |s| s.color[3])
}

#[test]
fn the_thumb_fades_in_monotonically_and_settles_within_180_ms_and_a_frame() {
    for tier in [MotionTier::Full, MotionTier::Reduced] {
        let o = opts();
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        run(&mut h, &o, tall());
        at(&mut h, 500.0);
        run(&mut h, &o, tall());
        h.input.pointer_to(100.0, 100.0);
        let first = run(&mut h, &o, tall());
        let mut xs = vec![(0.0, thumb_alpha(&first.scene))];
        for k in 1..=14 {
            let dt = f64::from(k) * FRAME_MS;
            at(&mut h, 500.0 + dt);
            xs.push((dt, thumb_alpha(&run(&mut h, &o, tall()).scene)));
        }
        assert!(
            xs.windows(2).all(|w| w[1].1 >= w[0].1),
            "{tier:?} monotonic: {xs:?}"
        );
        let full = premul(DARK.scroll_thumb)[3];
        assert!(
            xs.iter().any(|&(_, a)| a > 0.0 && a < full),
            "{tier:?} tween: {xs:?}"
        );
        let limit = if tier == MotionTier::Full {
            180.0
        } else {
            120.0
        } + FRAME_MS;
        for &(dt, a) in &xs {
            if dt >= limit {
                assert_eq!(a, full, "{tier:?} settled at {dt}");
            }
        }
    }
}

#[test]
fn a_settled_scroll_area_reports_idle() {
    let o = opts();
    let mut h = UiHarness::new(WIN);
    run(&mut h, &o, tall());
    h.input.pointer_to(100.0, 100.0);
    at(&mut h, 10.0);
    let f = settled(&mut h, &o, tall(), 10.0);
    assert_eq!(f.out.repaint, lune_ui_core::input::Repaint::Idle);
}
