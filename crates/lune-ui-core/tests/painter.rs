//! The painter's transform stack, op recorder and faded acrylic records.

use lune_ui_core::paint::{LayerId, Painter, Rect, StrokeStyle, Xform};
use lune_ui_core::scene::{Acrylic, FrameParams, Scene};
use lune_ui_core::testing::{DrawCall, FakeText, RecordingRenderer};
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK as THEME;

type P<'a> = Painter<'a, u8>;

fn painter(text: &mut FakeText) -> P<'_> {
    Painter::new(2.0, Rect::new(0.0, 0.0, 400.0, 300.0), text, None, THEME)
}

fn xf(origin: (f32, f32), scale: f32, dy: f32, alpha: f32) -> Xform {
    Xform {
        origin,
        scale,
        dx: 0.0,
        dy,
        alpha,
    }
}

const WHITE: [u8; 4] = [255, 255, 255, 255];

#[test]
fn nested_known_answer() {
    let mut t = FakeText;
    let mut p = painter(&mut t);
    p.push_xform(xf((0.0, 0.0), 2.0, 10.0, 0.5));
    p.push_xform(xf((10.0, 10.0), 0.5, 0.0, 0.5));
    // Innermost first: (20, 20) -> (15, 15) -> (30, 40).
    let moved = p.transformed(Rect::new(20.0, 20.0, 10.0, 10.0));
    assert_eq!(moved, Rect::new(30.0, 40.0, 10.0, 10.0));
    p.fill(Rect::new(20.0, 20.0, 10.0, 10.0), 0.0, WHITE);
    let spec = Spec::plain("hi", Style::ui(13.0, THEME.fg_primary));
    p.text(&spec, 20.0, 20.0);
    let layer = &p.scene.layers[0];
    let sh = layer.geometry.under[0];
    assert_eq!(sh.rect, [60.0, 80.0, 20.0, 20.0]);
    assert_eq!(sh.color, [0.25; 4]);
    let item = layer.texts[0];
    assert_eq!((item.x, item.y, item.scale), (30.0, 40.0, 1.0));
}

#[test]
fn single_entry_uses_the_one_transform_formulas() {
    let x = xf((5.0, 7.0), 0.9, 3.0, 0.6);
    let r = Rect::new(10.0, 20.0, 30.0, 40.0);
    for use_push in [false, true] {
        let mut t = FakeText;
        let mut p = painter(&mut t);
        if use_push {
            p.push_xform(x);
        } else {
            p.set_xform(Some(x));
        }
        p.fill(r, 4.0, WHITE);
        let sh = p.scene.layers[0].geometry.under[0];
        let (px, py) = (5.0 + (10.0 - 5.0) * 0.9, 7.0 + (20.0 - 7.0) * 0.9 + 3.0);
        assert_eq!(
            sh.rect,
            [px * 2.0, py * 2.0, (30.0 * 0.9) * 2.0, (40.0 * 0.9) * 2.0]
        );
        assert_eq!(sh.radii, [4.0 * 2.0 * 0.9; 4]);
        assert_eq!(sh.color, [1.0 * 0.6; 4]);
        assert_eq!(p.transformed(r), Rect::new(px, py, 30.0 * 0.9, 40.0 * 0.9));
    }
}

#[test]
fn pop_empty_is_a_no_op_and_set_replaces_the_stack() {
    let mut t = FakeText;
    let mut p = painter(&mut t);
    let r = Rect::new(1.0, 2.0, 3.0, 4.0);
    p.pop_xform();
    assert_eq!(p.transformed(r), r);
    p.push_xform(xf((0.0, 0.0), 2.0, 0.0, 1.0));
    p.push_xform(xf((0.0, 0.0), 2.0, 0.0, 1.0));
    assert_eq!(p.transformed(r), Rect::new(4.0, 8.0, 12.0, 16.0));
    p.set_xform(Some(xf((0.0, 0.0), 3.0, 0.0, 1.0)));
    assert_eq!(p.transformed(r), Rect::new(3.0, 6.0, 9.0, 12.0));
    p.pop_xform();
    assert_eq!(p.transformed(r), r);
    p.push_xform(xf((0.0, 0.0), 2.0, 0.0, 1.0));
    p.set_xform(None);
    assert_eq!(p.transformed(r), r);
}

/// Every recorded op kind, once or twice, on two layers, with a clip.
fn script(p: &mut P<'_>) {
    let r = Rect::new(10.0, 12.0, 80.0, 30.0);
    let spec = Spec::plain("label", Style::ui(13.0, THEME.fg_primary));
    p.fill(r, 4.0, THEME.border);
    p.fill_radii(r, [1.0, 2.0, 3.0, 4.0], THEME.accent);
    p.fill_over(r, 3.0, [0.1, 0.2, 0.3, 0.4]);
    p.ring(r, 4.0, 1.0, THEME.border, false);
    p.ring(r, 4.0, 1.0, THEME.border, true);
    p.acrylic(r, 6.0, [0.1, 0.1, 0.1, 0.6], [0.0; 4], true);
    p.set_clip(Some(Rect::new(0.0, 0.0, 60.0, 100.0)));
    p.text(&spec, 12.0, 14.0);
    p.text_in(&spec, r, 4.0);
    p.text_centered(&spec, r);
    p.stroke(
        &[[0.0, 0.0], [20.0, 20.0], [40.0, 5.0]],
        false,
        StrokeStyle::solid(THEME.fg_primary, 1.5),
    );
    p.custom(2, r);
    p.set_clip(None);
    p.on(LayerId::Popup);
    p.backdrop(r, 8.0);
    p.fill(r, 8.0, THEME.menu_glass);
    p.text(&spec, 20.0, 20.0);
    p.hit(r, 1);
    let _ = p.measure(&spec);
}

fn with_transform(p: &mut P<'_>) {
    p.set_xform(Some(xf((10.0, 12.0), 0.9, 3.0, 0.6)));
    p.push_xform(xf((0.0, 0.0), 1.1, -2.0, 0.8));
    script(p);
    p.pop_xform();
    script(p);
    p.set_xform(None);
}

fn scene_of(f: impl Fn(&mut P<'_>)) -> Scene {
    let mut t = FakeText;
    let mut p = painter(&mut t);
    f(&mut p);
    p.scene
}

#[test]
fn replay_equals_the_recorded_calls() {
    for transformed in [false, true] {
        let run = |p: &mut P<'_>| {
            if transformed {
                with_transform(p);
            } else {
                script(p);
            }
        };
        let direct = scene_of(run);

        let mut t = FakeText;
        let mut rec = painter(&mut t);
        rec.start_recording();
        run(&mut rec);
        let ops = rec.take_recording();
        assert_eq!(rec.scene, direct, "recording must not change the scene");
        assert!(!ops.is_empty());

        let mut t2 = FakeText;
        let mut fresh = painter(&mut t2);
        fresh.replay(&ops);
        assert_eq!(fresh.scene, direct, "replay (transformed: {transformed})");
        assert!(fresh.hits.is_empty(), "hits are not replayed");
    }
}

#[test]
fn recording_state() {
    let mut t = FakeText;
    let mut p = painter(&mut t);
    assert!(p.take_recording().is_empty());
    p.fill(Rect::new(0.0, 0.0, 5.0, 5.0), 0.0, WHITE);
    assert!(p.take_recording().is_empty(), "not recording yet");
    p.start_recording();
    p.fill(Rect::new(0.0, 0.0, 5.0, 5.0), 0.0, WHITE);
    p.hit(Rect::new(0.0, 0.0, 5.0, 5.0), 1);
    let spec = Spec::plain("x", Style::ui(13.0, THEME.fg_primary));
    let _ = p.measure(&spec);
    assert_eq!(p.take_recording().len(), 1, "fill only, not hit or measure");
    assert!(p.take_recording().is_empty(), "taking stops the recording");
}

#[test]
fn acrylic_opacity_comes_from_the_stack() {
    let col = [0.1, 0.2, 0.3, 0.6];
    let r = Rect::new(10.0, 10.0, 50.0, 40.0);
    let acrylics = |f: &dyn Fn(&mut P<'_>)| -> Vec<Acrylic> {
        scene_of(|p| {
            f(p);
            p.acrylic(r, 4.0, col, [0.0; 4], false);
        })
        .layers[0]
            .geometry
            .acrylic
            .clone()
    };
    let plain = acrylics(&|_| {});
    assert_eq!(plain.len(), 1);
    assert_eq!(plain[0].opacity, 1.0);
    assert_eq!(plain[0].shape.color, col);
    assert_eq!(plain[0].shape.params, [0.0, 0.0, 1.0, 0.0]);

    let faded = acrylics(&|p| p.set_xform(Some(xf((0.0, 0.0), 1.0, 0.0, 0.5))));
    assert_eq!(faded[0].opacity, 0.5);
    assert_eq!(faded[0].shape.color, col, "colour stays unfaded");

    let nested = acrylics(&|p| {
        p.push_xform(xf((0.0, 0.0), 1.0, 0.0, 0.5));
        p.push_xform(xf((0.0, 0.0), 1.0, 0.0, 0.5));
    });
    assert_eq!(nested[0].opacity, 0.25);

    assert!(acrylics(&|p| p.set_xform(Some(xf((0.0, 0.0), 1.0, 0.0, 0.0)))).is_empty());
    assert!(acrylics(&|p| p.set_xform(Some(xf((0.0, 0.0), 1.0, 0.0, -1.0)))).is_empty());
}

#[test]
fn recording_renderer_carries_opacity() {
    let scene = scene_of(|p| {
        p.set_xform(Some(xf((0.0, 0.0), 1.0, 0.0, 0.5)));
        p.acrylic(
            Rect::new(0.0, 0.0, 10.0, 10.0),
            2.0,
            [0.1; 4],
            [0.0; 4],
            true,
        );
    });
    let mut rr = RecordingRenderer::new();
    rr.render(
        &scene,
        &FrameParams {
            size: (800, 600),
            scale: 2.0,
            blur_sigma: 4.0,
            saturation: 1.0,
        },
    );
    let calls = &rr.last().expect("a frame").calls;
    assert!(matches!(
        calls.as_slice(),
        [DrawCall::Acrylic { layer: 0, opacity, .. }] if *opacity == 0.5
    ));
}
