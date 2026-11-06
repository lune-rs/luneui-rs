//! The focus rings' geometry and colours (`core.focus`), at scale 2: one 1 px outline per kind,
//! faded by the ring opacity and absent at 0.

use lune_ui_core::color::premul;
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::Shape;
use lune_ui_core::testing::FakeText;
use lune_ui_core::theme::DARK;
use lune_ui_core::widgets::focus_ring::{RingKind, paint};

const BOX: Rect = Rect::new(10.0, 10.0, 100.0, 30.0);

fn rings(kind: RingKind, opacity: f32) -> Vec<Shape> {
    let mut t = FakeText;
    let mut p: Painter<'_, u8> =
        Painter::new(2.0, Rect::new(0.0, 0.0, 200.0, 100.0), &mut t, None, DARK);
    paint(&mut p, BOX, kind, opacity);
    let g = &p.scene.layers[0].geometry;
    g.under.iter().chain(&g.over).copied().collect()
}

fn one(kind: RingKind) -> Shape {
    let v = rings(kind, 1.0);
    assert_eq!(v.len(), 1, "{kind:?}");
    // A ring shape: the ring flag set, 1 logical px = 2 device px wide.
    assert_eq!((v[0].params[0], v[0].params[1]), (1.0, 2.0), "{kind:?}");
    v[0]
}

#[test]
fn a_button_ring_sits_on_the_rects_own_edge_in_fg_secondary() {
    let s = one(RingKind::Button);
    assert_eq!(s.rect, [20.0, 20.0, 200.0, 60.0]);
    assert_eq!(s.radii, [DARK.item_radius * 2.0; 4]);
    assert_eq!(s.color, premul(DARK.fg_secondary));
    assert_eq!(
        one(RingKind::PrimaryButton).color,
        premul(DARK.on_accent_text)
    );
}

#[test]
fn an_offset_ring_clears_the_control_by_2_px_and_draws_1_px_outside_that() {
    // 2 px gap and the 1 px line: the shape is the control grown by 3 on each side, so its inner
    // edge is the control grown by 2 (a 40 x 20 track gives the spec's 44 x 24 box).
    let s = one(RingKind::Offset { radius: 10.0 });
    assert_eq!(s.rect, [14.0, 14.0, 212.0, 72.0]);
    assert_eq!(s.radii, [26.0; 4], "(10 + 3) * 2");
    assert_eq!(s.color, premul(DARK.fg_secondary));
}

#[test]
fn a_border_ring_is_the_accent_and_an_edge_ring_keeps_its_own_radius() {
    let b = one(RingKind::Border);
    assert_eq!(
        (b.rect, b.color),
        ([20.0, 20.0, 200.0, 60.0], premul(DARK.accent))
    );
    let e = one(RingKind::Edge {
        radius: 4.0,
        on_accent: false,
    });
    assert_eq!((e.radii, e.color), ([8.0; 4], premul(DARK.fg_secondary)));
    let e = one(RingKind::Edge {
        radius: 4.0,
        on_accent: true,
    });
    assert_eq!(e.color, premul(DARK.on_accent_text));
}

#[test]
fn a_ring_fades_with_its_opacity_and_is_absent_at_zero() {
    for kind in [
        RingKind::Button,
        RingKind::Border,
        RingKind::Offset { radius: 4.0 },
    ] {
        assert!(rings(kind, 0.0).is_empty(), "{kind:?}");
        let full = rings(kind, 1.0)[0].color[3];
        let half = rings(kind, 0.5)[0].color[3];
        assert!(half > 0.0 && half < full, "{kind:?}: {half} of {full}");
    }
}
