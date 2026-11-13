//! Desktop units, monitors, popup placement and window shapes: known answers.

use lune_ui_core::desktop::{
    DesktopPoint, DesktopRect, Monitor, ShapeError, WindowShape, monitor_at, place_popup, to_units,
};
use lune_ui_core::paint::Rect;
use lune_ui_core::widgets::popover::{PlaceAlign, Side};

fn monitor(bounds: DesktopRect, work: DesktopRect, upl: f32) -> Monitor {
    Monitor {
        bounds,
        work,
        units_per_logical: upl,
        primary: true,
    }
}

#[test]
fn rect_edges_and_containment() {
    let r = DesktopRect::new(10, 20, 100, 50);
    assert_eq!((r.right(), r.bottom()), (110, 70));
    assert_eq!(r.origin(), DesktopPoint::new(10, 20));
    assert!(r.contains(DesktopPoint::new(10, 20)));
    assert!(r.contains(DesktopPoint::new(109, 69)));
    assert!(!r.contains(DesktopPoint::new(110, 20)));
    assert!(!r.contains(DesktopPoint::new(10, 70)));
    assert!(!DesktopRect::new(0, 0, 0, 10).contains(DesktopPoint::new(0, 0)));
    assert!(DesktopRect::new(0, 0, 0, 10).is_empty());
    assert!(DesktopRect::new(0, 0, 10, -1).is_empty());
    assert_eq!(
        r.moved_to(DesktopPoint::new(1, 2)),
        DesktopRect::new(1, 2, 100, 50)
    );
    assert_eq!(r.translated(-10, 5), DesktopRect::new(0, 25, 100, 50));
}

#[test]
fn intersects_needs_area_and_saturation_holds() {
    let a = DesktopRect::new(0, 0, 10, 10);
    assert!(a.intersects(DesktopRect::new(9, 9, 10, 10)));
    assert!(
        !a.intersects(DesktopRect::new(10, 0, 10, 10)),
        "a shared edge is not overlap"
    );
    assert!(!a.intersects(DesktopRect::new(5, 5, 0, 5)));
    let edge = DesktopRect::new(i32::MAX - 10, i32::MIN + 5, 100, 100);
    assert_eq!(edge.right(), i32::MAX);
    assert_eq!(edge.translated(1000, -1000).x, i32::MAX);
    assert_eq!(edge.translated(1000, -1000).y, i32::MIN);
    assert_eq!(edge.bottom(), i32::MIN + 105);
}

#[test]
fn to_units_rounds_and_refuses_non_finite() {
    assert_eq!(to_units(10.0, 1.5), 15);
    assert_eq!(to_units(275.0, 1.5), 413);
    assert_eq!(to_units(190.0, 1.5), 285);
    assert_eq!(to_units(f32::NAN, 1.5), 0);
    assert_eq!(to_units(10.0, f32::INFINITY), 0);
    assert_eq!(to_units(f32::INFINITY, 1.0), 0);
    assert_eq!(to_units(1.0e30, 2.0), i32::MAX);
}

#[test]
fn monitor_at_picks_the_containing_or_nearest() {
    let ms = [
        monitor(
            DesktopRect::new(0, 0, 1920, 1080),
            DesktopRect::new(0, 0, 1920, 1040),
            1.0,
        ),
        monitor(
            DesktopRect::new(1920, 0, 2560, 1440),
            DesktopRect::new(1920, 0, 2560, 1440),
            1.0,
        ),
    ];
    assert_eq!(monitor_at(&ms, DesktopPoint::new(2000, 10)), Some(1));
    assert_eq!(monitor_at(&ms, DesktopPoint::new(-50, 500)), Some(0));
    assert_eq!(monitor_at(&ms, DesktopPoint::new(5000, 2000)), Some(1));
    assert_eq!(monitor_at(&ms, DesktopPoint::new(1919, 1079)), Some(0));
    assert_eq!(monitor_at(&[], DesktopPoint::new(0, 0)), None);
}

#[test]
fn monitor_at_ties_go_to_the_lower_index_and_extremes_do_not_panic() {
    let ms = [
        monitor(
            DesktopRect::new(0, 0, 100, 100),
            DesktopRect::new(0, 0, 100, 100),
            1.0,
        ),
        monitor(
            DesktopRect::new(200, 0, 100, 100),
            DesktopRect::new(200, 0, 100, 100),
            1.0,
        ),
    ];
    // 50 from the right edge of the first and 50 from the left edge of the second.
    assert_eq!(monitor_at(&ms, DesktopPoint::new(150, 50)), Some(0));
    assert_eq!(
        monitor_at(&ms, DesktopPoint::new(i32::MIN, i32::MAX)),
        Some(0)
    );
}

fn popup_monitor() -> Monitor {
    monitor(
        DesktopRect::new(0, 0, 3840, 2160),
        DesktopRect::new(0, 0, 3840, 2088),
        1.5,
    )
}

#[test]
fn a_popup_flips_and_shifts_inside_the_work_area() {
    let m = popup_monitor();
    let at = |x, y| DesktopRect::new(x, y, 0, 0);
    let size = (190.0, 200.0);
    assert_eq!(
        place_popup(
            at(3800, 2050),
            size,
            Side::Bottom,
            PlaceAlign::Start,
            0.0,
            &m
        ),
        Some(DesktopRect::new(3555, 1750, 285, 300))
    );
    assert_eq!(
        place_popup(at(100, 100), size, Side::Bottom, PlaceAlign::Start, 0.0, &m),
        Some(DesktopRect::new(100, 100, 285, 300))
    );
}

#[test]
fn a_popup_gap_is_scaled() {
    let m = popup_monitor();
    let anchor = DesktopRect::new(100, 100, 50, 20);
    // A gap of 4 logical px is 6 units: the popup starts 6 units under the anchor.
    let r = place_popup(
        anchor,
        (10.0, 10.0),
        Side::Bottom,
        PlaceAlign::Start,
        4.0,
        &m,
    )
    .expect("placed");
    assert_eq!((r.x, r.y, r.w, r.h), (100, 126, 15, 15));
}

#[test]
fn a_popup_refuses_bad_numbers() {
    let a = DesktopRect::new(0, 0, 10, 10);
    let go = |size: (f32, f32), gap: f32, upl: f32| {
        let mut m = popup_monitor();
        m.units_per_logical = upl;
        place_popup(a, size, Side::Bottom, PlaceAlign::Start, gap, &m)
    };
    assert!(go((10.0, 10.0), 0.0, 1.0).is_some());
    assert!(go((10.0, 10.0), 0.0, 0.0).is_none());
    assert!(go((10.0, 10.0), 0.0, -1.0).is_none());
    assert!(go((10.0, 10.0), 0.0, f32::NAN).is_none());
    assert!(go((f32::NAN, 10.0), 0.0, 1.0).is_none());
    assert!(go((10.0, f32::INFINITY), 0.0, 1.0).is_none());
    assert!(go((-1.0, 10.0), 0.0, 1.0).is_none());
    assert!(go((10.0, 10.0), f32::NAN, 1.0).is_none());
}

#[test]
fn a_polygon_needs_three_finite_points() {
    assert_eq!(
        WindowShape::new().with_polygon(&[(0.0, 0.0), (1.0, 1.0)]),
        Err(ShapeError::TooFewPoints)
    );
    assert_eq!(
        WindowShape::new().with_polygon(&[(0.0, 0.0), (1.0, 1.0), (f32::NAN, 2.0)]),
        Err(ShapeError::NotFinite)
    );
    assert_eq!(
        WindowShape::new().with_rect(Rect::new(0.0, f32::INFINITY, 1.0, 1.0)),
        Err(ShapeError::NotFinite)
    );
    assert!(!ShapeError::TooFewPoints.to_string().is_empty());
    assert!(!ShapeError::NotFinite.to_string().is_empty());
}

#[test]
fn a_triangle_contains_its_inside() {
    let s = WindowShape::new()
        .with_polygon(&[(0.0, 0.0), (400.0, 0.0), (0.0, 300.0)])
        .expect("triangle");
    assert!(s.contains((100.0, 50.0)));
    assert!(s.contains((100.0, 150.0)));
    assert!(!s.contains((100.0, 250.0)));
    assert!(!s.contains((300.0, 150.0)));
    assert!(!s.contains((200.0, 150.0)));
}

#[test]
fn separate_rects_leave_the_gap_out() {
    let s = WindowShape::new()
        .with_rect(Rect::new(0.0, 0.0, 150.0, 300.0))
        .and_then(|s| s.with_rect(Rect::new(250.0, 0.0, 150.0, 300.0)))
        .expect("rects");
    assert!(s.contains((100.0, 50.0)));
    assert!(s.contains((300.0, 150.0)));
    assert!(!s.contains((200.0, 150.0)));
    assert_eq!(s.polygons().len(), 2);
}

#[test]
fn overlapping_polygons_are_a_union_not_even_odd() {
    let s = WindowShape::new()
        .with_rect(Rect::new(0.0, 0.0, 100.0, 100.0))
        .and_then(|s| s.with_rect(Rect::new(50.0, 0.0, 100.0, 100.0)))
        .expect("rects");
    assert!(s.contains((75.0, 50.0)));
}

#[test]
fn an_empty_shape_has_no_bounds_and_contains_nothing() {
    let s = WindowShape::new();
    assert!(s.is_empty());
    assert_eq!(s.bounds(), None);
    assert!(!s.contains((0.0, 0.0)));
    let s = s
        .with_polygon(&[(10.0, 20.0), (30.0, 5.0), (25.0, 60.0)])
        .expect("polygon");
    assert!(!s.is_empty());
    assert_eq!(s.bounds(), Some(Rect::new(10.0, 5.0, 20.0, 55.0)));
}
