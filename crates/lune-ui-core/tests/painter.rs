//! The painter's transform stack, op recorder and faded acrylic records.

use lune_ui_core::paint::{LayerId, Painter, Rect, StrokeStyle, Xform};
use lune_ui_core::scene::{Acrylic, BackdropRequest, FrameParams, Scene};
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
            ..FrameParams::default()
        },
    );
    let calls = &rr.last().expect("a frame").calls;
    assert!(matches!(
        calls.as_slice(),
        [DrawCall::Acrylic { layer: 0, opacity, .. }] if *opacity == 0.5
    ));
}

#[test]
fn push_translate_moves_the_content_and_keeps_the_clip_on_screen() {
    let mut t = FakeText;
    let mut p = painter(&mut t);
    let clip = Rect::new(10.0, 10.0, 100.0, 100.0);
    p.push_clip(clip);
    p.push_translate(0.0, -40.0);
    // 10 px x 10 px at (10, 90) in the shifted space is at (10, 50) on screen
    p.fill(Rect::new(10.0, 90.0, 10.0, 10.0), 0.0, WHITE);
    let sh = p.scene.layers[0].geometry.under[0];
    assert_eq!(sh.rect, [20.0, 100.0, 20.0, 20.0]);
    // the clip is where it was on screen, in device px
    assert_eq!(sh.clip, [20.0, 20.0, 200.0, 200.0]);
    // a hit clips against the clip in the shifted space and lands on screen
    p.hit(Rect::new(0.0, 0.0, 400.0, 400.0), 1);
    assert_eq!(p.hits[0].0, clip);
    p.pop_translate();
    p.fill(Rect::new(10.0, 90.0, 10.0, 10.0), 0.0, WHITE);
    let sh = p.scene.layers[0].geometry.under[1];
    assert_eq!(sh.rect, [20.0, 180.0, 20.0, 20.0]);
    assert_eq!(sh.clip, [20.0, 20.0, 200.0, 200.0]);
    p.pop_clip();
    p.fill(Rect::new(10.0, 90.0, 10.0, 10.0), 0.0, WHITE);
    assert_eq!(p.scene.layers[0].geometry.under[2].clip[2], 0.0);
}

#[test]
fn push_translate_replays_to_the_same_scene() {
    let draw = |p: &mut P<'_>| {
        p.push_clip(Rect::new(10.0, 10.0, 100.0, 100.0));
        p.push_translate(5.0, -40.0);
        p.fill(Rect::new(10.0, 90.0, 10.0, 10.0), 0.0, WHITE);
        p.pop_translate();
        p.pop_clip();
    };
    let mut t = FakeText;
    let mut direct = painter(&mut t);
    draw(&mut direct);
    let mut t2 = FakeText;
    let mut rec = painter(&mut t2);
    rec.start_recording();
    draw(&mut rec);
    let ops = rec.take_recording();
    let mut t3 = FakeText;
    let mut replayed = painter(&mut t3);
    replayed.replay(&ops);
    assert_eq!(direct.scene, replayed.scene);
    assert_eq!(direct.scene, rec.scene);
}

#[test]
fn text_queries_forward_to_the_text_system() {
    use lune_ui_core::text::{CaretPos, Unit};
    let mut t = FakeText;
    let mut p = painter(&mut t);
    let spec = Spec::plain("ab\ncd", Style::ui(10.0, THEME.fg_primary));
    // The fake is in logical px, whatever the painter's scale (2).
    assert_eq!(
        p.caret(&spec, 4),
        CaretPos {
            x: 5.0,
            y: 13.0,
            height: 13.0,
            line: 1
        }
    );
    assert_eq!(p.text_hit(&spec, (8.0, 14.0)), 5);
    let lines = p.text_lines(&spec);
    assert_eq!(lines.len(), 2);
    assert_eq!((lines[1].start, lines[1].end), (3, 5));
    assert_eq!(p.segments("a\u{301}b", Unit::Grapheme), [0, 3, 4]);
}

fn backdrop_of(f: impl Fn(&mut P<'_>)) -> Option<BackdropRequest> {
    scene_of(f).backdrops().next().copied()
}

const LOW: Rect = Rect::new(10.0, 10.0, 40.0, 30.0);
const HIGH: Rect = Rect::new(60.0, 10.0, 40.0, 30.0);

#[test]
fn backdrops_on_two_layers_both_stand_in_draw_order_whichever_came_first() {
    let low_first = scene_of(|p| {
        p.on(LayerId::Popup);
        p.backdrop(LOW, 4.0);
        p.on(LayerId::Raised);
        p.backdrop(HIGH, 4.0);
    });
    let high_first = scene_of(|p| {
        p.on(LayerId::Raised);
        p.backdrop(HIGH, 4.0);
        p.on(LayerId::Popup);
        p.backdrop(LOW, 4.0);
    });
    for scene in [low_first, high_first] {
        let rects: Vec<_> = scene.backdrops().map(|b| b.rect).collect();
        assert_eq!(rects, [LOW.phys(2.0), HIGH.phys(2.0)]);
        assert_eq!(scene.layers[LayerId::Popup as usize].backdrops.len(), 1);
        assert_eq!(scene.layers[LayerId::Raised as usize].backdrops.len(), 1);
    }
}

#[test]
fn backdrops_on_one_layer_both_stand_in_record_order() {
    let scene = scene_of(|p| {
        p.on(LayerId::Popup);
        p.backdrop(LOW, 4.0);
        p.backdrop(HIGH, 4.0);
    });
    let rects: Vec<_> = scene.backdrops().map(|b| b.rect).collect();
    assert_eq!(rects, [LOW.phys(2.0), HIGH.phys(2.0)]);
}

#[test]
fn a_backdrop_that_shows_nothing_records_nothing_but_still_replays() {
    let invisible = xf((0.0, 0.0), 1.0, 0.0, 0.0);
    let scene = scene_of(|p| {
        p.backdrop(Rect::new(10.0, 10.0, 0.0, 30.0), 4.0);
        p.backdrop(Rect::new(10.0, 10.0, 30.0, 0.0), 4.0);
        p.push_xform(invisible);
        p.backdrop(LOW, 4.0);
        p.pop_xform();
    });
    assert_eq!(scene.backdrops().count(), 0);
    // The op stays in a recording: it shows when replayed where it is visible.
    let mut t1 = FakeText;
    let mut rec = painter(&mut t1);
    rec.push_xform(invisible);
    rec.start_recording();
    rec.backdrop(LOW, 4.0);
    let ops = rec.take_recording();
    assert_eq!(rec.scene.backdrops().count(), 0);
    let mut t2 = FakeText;
    let mut replayed = painter(&mut t2);
    replayed.replay(&ops);
    assert_eq!(replayed.scene.backdrops().count(), 1);
}

#[test]
fn a_backdrop_carries_the_current_layer_and_its_own_numbers() {
    for layer in [
        LayerId::Base,
        LayerId::Popup,
        LayerId::Raised,
        LayerId::Toast,
    ] {
        let scene = scene_of(|p| {
            p.on(layer);
            p.backdrop(LOW, 4.0);
        });
        let plain = scene.layers[layer as usize].backdrops[0];
        assert_eq!((plain.sigma, plain.saturation), (0.0, 0.0), "the frame's");
    }
    let own = backdrop_of(|p| p.backdrop_with(LOW, 4.0, 8.0, 1.5)).expect("asked");
    assert_eq!((own.sigma, own.saturation), (8.0, 1.5));
}

#[test]
fn a_replayed_backdrop_is_the_same_request() {
    let draw = |p: &mut P<'_>| {
        p.on(LayerId::Raised);
        p.push_xform(xf((10.0, 10.0), 0.5, 3.0, 0.5));
        p.backdrop_with(LOW, 4.0, 8.0, 1.5);
        p.pop_xform();
    };
    let mut t1 = FakeText;
    let mut rec = painter(&mut t1);
    rec.start_recording();
    draw(&mut rec);
    let ops = rec.take_recording();
    let mut t2 = FakeText;
    let mut replayed = painter(&mut t2);
    replayed.replay(&ops);
    assert_eq!(rec.scene.backdrops().count(), 1);
    assert!(rec.scene.backdrops().eq(replayed.scene.backdrops()));
}

/// Where the backdrop falls in the recorded calls: the layers drawn before it and after it.
fn around_backdrop(scene: &Scene) -> (Vec<usize>, Vec<usize>) {
    let mut rec = RecordingRenderer::new();
    rec.render(scene, &FrameParams::default());
    let calls = &rec.last().expect("one frame").calls;
    let at = calls
        .iter()
        .position(|c| matches!(c, DrawCall::Backdrop(_)))
        .unwrap_or(calls.len());
    let layers = |c: &[DrawCall]| {
        let mut l: Vec<usize> = c.iter().filter_map(DrawCall::layer).collect();
        l.dedup();
        l
    };
    (layers(&calls[..at]), layers(&calls[at..]))
}

fn every_layer(p: &mut P<'_>) {
    let layers = [
        LayerId::Base,
        LayerId::Page,
        LayerId::Popup,
        LayerId::Modal,
        LayerId::Raised,
        LayerId::Outside,
        LayerId::Toast,
    ];
    for l in layers {
        p.on(l);
        p.fill(LOW, 0.0, THEME.fg_primary);
    }
}

#[test]
fn the_recording_renderer_puts_a_backdrop_before_its_layer_and_no_lower_than_the_floor() {
    let at = |layer: LayerId| {
        around_backdrop(&scene_of(|p| {
            every_layer(p);
            p.on(layer);
            p.backdrop(LOW, 4.0);
        }))
    };
    assert_eq!(at(LayerId::Base), (vec![0, 1], vec![2, 3, 4, 5, 6]));
    assert_eq!(at(LayerId::Popup), (vec![0, 1], vec![2, 3, 4, 5, 6]));
    assert_eq!(at(LayerId::Modal), (vec![0, 1, 2], vec![3, 4, 5, 6]));
    assert_eq!(at(LayerId::Raised), (vec![0, 1, 2, 3], vec![4, 5, 6]));
    assert_eq!(at(LayerId::Outside), (vec![0, 1, 2, 3, 4], vec![5, 6]));
}

#[test]
fn acrylic_radii_keeps_each_corner_and_acrylic_is_the_even_case() {
    let radii = [8.0, 0.0, 2.0, 0.0];
    let colour = [0.1, 0.2, 0.3, 1.0];
    let scene = scene_of(|p| {
        p.acrylic_radii(LOW, radii, colour, [0.0; 4], true);
        p.acrylic(LOW, 5.0, colour, [0.0; 4], true);
    });
    let a = &scene.layers[0].geometry.acrylic;
    assert_eq!(a[0].shape.radii, radii.map(|r| r * 2.0), "scale 2");
    assert_eq!(a[1].shape.radii, [10.0; 4]);

    let mut t1 = FakeText;
    let mut rec = painter(&mut t1);
    rec.start_recording();
    rec.acrylic_radii(LOW, radii, colour, [0.0; 4], true);
    let ops = rec.take_recording();
    let mut t2 = FakeText;
    let mut replayed = painter(&mut t2);
    replayed.replay(&ops);
    assert_eq!(rec.scene, replayed.scene);
}

// ── `image` and `text_rotated`: the right item, on the right layer, under the clip and Xform ────

/// The same state for both: layer `Popup` (index 2), a logical clip (10, 10, 50, 50) and an Xform
/// of scale 2 moved by (3, 4) at alpha 0.5, all at device scale 2. The clip travels through the
/// Xform: (10, 10, 50, 50) -> (23, 24, 100, 100) logical -> (46, 48, 200, 200) device.
fn popup_under_clip_and_xform(p: &mut P<'_>) {
    p.on(LayerId::Popup);
    p.push_clip(Rect::new(10.0, 10.0, 50.0, 50.0));
    p.push_xform(Xform {
        origin: (0.0, 0.0),
        scale: 2.0,
        dx: 3.0,
        dy: 4.0,
        alpha: 0.5,
    });
}

const POPUP: usize = 2;

#[test]
fn image_lands_on_the_current_layer_under_the_clip_and_xform() {
    use lune_ui_core::image::{Fit, ImageAlign, ImageId};
    use lune_ui_core::scene::ImageSource;
    let scene = scene_of(|p| {
        popup_under_clip_and_xform(p);
        p.image(
            Rect::new(10.0, 10.0, 10.0, 5.0),
            ImageId(7),
            Fit::Contain,
            ImageAlign::CENTER,
            0.5,
        );
    });
    // Only the Popup layer holds it.
    for (i, l) in scene.in_order().into_iter().enumerate() {
        assert_eq!(l.images.len(), usize::from(i == POPUP), "layer {i}");
    }
    let it = scene.layers[POPUP].images[0];
    assert_eq!(it.source, ImageSource::Id(ImageId(7)));
    // (10, 10, 10, 5) -> (23, 24, 20, 10) logical -> device.
    assert_eq!(it.dst, Rect::new(46.0, 48.0, 40.0, 20.0));
    assert_eq!(it.clip, [46.0, 48.0, 200.0, 200.0]);
    assert_eq!(it.alpha, 0.25, "0.5 own times the Xform's 0.5");
    // The renderer fake sees the same item as an Image call on layer 2.
    let mut rec = RecordingRenderer::new();
    rec.render(&scene, &FrameParams::default());
    let calls = &rec.last().expect("one frame").calls;
    let images: Vec<_> = calls
        .iter()
        .filter(|c| matches!(c, DrawCall::Image { .. }))
        .collect();
    assert_eq!(images.len(), 1);
    assert!(matches!(images[0], DrawCall::Image { layer: POPUP, item } if *item == it));
}

#[test]
fn text_rotated_lands_on_the_current_layer_under_the_clip_and_xform() {
    use lune_ui_core::scene::ImageSource;
    use lune_ui_core::text::TextSystem;
    let spec = Spec::plain("turn", Style::ui(13.0, THEME.fg_primary));
    let (key, (w, h)) = FakeText.prepare(&spec, 2.0);
    let scene = scene_of(|p| {
        popup_under_clip_and_xform(p);
        p.text_rotated(&spec, (50.0, 40.0), 0.5);
    });
    for (i, l) in scene.in_order().into_iter().enumerate() {
        assert_eq!(l.images.len(), usize::from(i == POPUP), "layer {i}");
        assert!(
            l.texts.is_empty(),
            "rotated text is an image, not a text item"
        );
    }
    let it = scene.layers[POPUP].images[0];
    // The text's raster is drawn at the Xform's scale (2): the key is the unscaled shaped text.
    assert_eq!(it.source, ImageSource::Text(key, 2.0));
    assert_eq!(it.rotation, 0.5);
    // Centre (50, 40) -> (103, 84) logical -> (206, 168) device; size is the shaped size times
    // the Xform's scale 2 times the device scale 2.
    assert_eq!((it.dst.w, it.dst.h), (w * 4.0, h * 4.0));
    let c = (it.dst.x + it.dst.w / 2.0, it.dst.y + it.dst.h / 2.0);
    assert_eq!(c, (206.0, 168.0));
    assert_eq!(it.clip, [46.0, 48.0, 200.0, 200.0]);
    // The Xform's alpha is applied when the picture is drawn (a fade must not change the key).
    assert_eq!(it.alpha, 0.5);
}

// ── the existing set under an Xform: the right layer, the clip, the scale and the alpha ────────

/// `r` through the Xform of `popup_under_clip_and_xform` (scale 2, moved (3, 4)) at device scale 2.
const MOVED: Rect = Rect::new(20.0, 30.0, 10.0, 8.0); // -> (43, 64, 20, 16) -> device (86, 128, 40, 32)
const CLIP_DEVICE: [f32; 4] = [46.0, 48.0, 200.0, 200.0];

#[test]
fn fills_rings_and_backdrops_take_the_layer_clip_scale_and_alpha_of_the_xform() {
    let scene = scene_of(|p| {
        popup_under_clip_and_xform(p);
        p.fill_radii(MOVED, [1.0, 2.0, 3.0, 4.0], WHITE);
        p.fill_over(MOVED, 1.0, [1.0; 4]);
        p.ring(MOVED, 1.0, 1.0, WHITE, false);
        p.ring(MOVED, 1.0, 1.0, WHITE, true);
        p.backdrop(MOVED, 1.0);
    });
    for (i, l) in scene.in_order().into_iter().enumerate() {
        let n = usize::from(i == POPUP) * 2;
        assert_eq!(
            (l.geometry.under.len(), l.geometry.over.len()),
            (n, n),
            "layer {i}"
        );
    }
    let g = &scene.layers[POPUP].geometry;
    for sh in g.under.iter().chain(&g.over) {
        assert_eq!(sh.rect, [86.0, 128.0, 40.0, 32.0]);
        assert_eq!(sh.clip, CLIP_DEVICE);
        assert_eq!(sh.color, [0.5; 4], "premultiplied white at the Xform's 0.5");
    }
    // A radius is logical: times the device scale 2 times the Xform's scale 2.
    assert_eq!(g.under[0].radii, [4.0, 8.0, 12.0, 16.0]);
    assert_eq!(g.over[0].radii, [4.0; 4]);
    assert_eq!((g.under[1].radii, g.over[1].radii), ([4.0; 4], [4.0; 4]));
    let b = scene.layers[POPUP].backdrops[0];
    assert_eq!(
        (b.rect, b.radius, b.alpha),
        ([86.0, 128.0, 40.0, 32.0], 4.0, 0.5)
    );
}

#[test]
fn strokes_fans_and_polygons_take_the_layer_clip_scale_and_alpha_of_the_xform() {
    let scene = scene_of(|p| {
        popup_under_clip_and_xform(p);
        p.stroke(
            &[[20.0, 30.0], [40.0, 30.0]],
            false,
            StrokeStyle::solid(WHITE, 1.0),
        );
        p.fill_path(&[[20.0, 30.0], [30.0, 30.0], [30.0, 38.0]], WHITE);
        p.sector([20.0, 30.0], 5.0, 0.0, std::f32::consts::FRAC_PI_2, WHITE);
    });
    for (i, l) in scene.in_order().into_iter().enumerate() {
        let on = i == POPUP;
        assert_eq!(!l.geometry.strokes.is_empty(), on, "strokes, layer {i}");
        assert_eq!(!l.geometry.fills.is_empty(), on, "fills, layer {i}");
    }
    let g = &scene.layers[POPUP].geometry;
    for v in &g.strokes {
        assert_eq!((v.clip, v.color), (CLIP_DEVICE, [0.5; 4]));
        // Half the width: 1 logical times the Xform's 2 times the device 2, halved.
        assert_eq!(v.style[1], 2.0);
    }
    // The line (20, 30)-(40, 30) is (86, 128)-(166, 128) on the device.
    let xs: Vec<f32> = g.strokes.iter().map(|v| v.pos[0]).collect();
    assert_eq!(
        (
            xs.iter().cloned().fold(f32::MAX, f32::min),
            xs.iter().cloned().fold(f32::MIN, f32::max)
        ),
        (86.0, 166.0)
    );
    for v in &g.fills {
        assert_eq!((v.clip, v.color), (CLIP_DEVICE, [0.5; 4]));
    }
    // The polygon's first corner and the fan's centre are (20, 30) moved; the fan's first arc point
    // is 5 logical px to the right: 5 * 2 * 2 = 20 device px.
    assert!(g.fills.iter().any(|v| v.pos == [86.0, 128.0]));
    assert!(g.fills.iter().any(|v| v.pos == [106.0, 128.0]));
}

#[test]
fn text_takes_the_layer_clip_scale_and_alpha_of_the_xform() {
    use lune_ui_core::text::TextSystem;
    let spec = Spec::plain("fade", Style::ui(13.0, THEME.fg_primary));
    let mut faded = spec.clone();
    for (_, style) in &mut faded.runs {
        style.color[3] = (f32::from(style.color[3]) * 0.5).round() as u8;
    }
    let scene = scene_of(|p| {
        popup_under_clip_and_xform(p);
        p.text(&spec, 20.0, 30.0);
    });
    for (i, l) in scene.in_order().into_iter().enumerate() {
        assert_eq!(l.texts.len(), usize::from(i == POPUP), "layer {i}");
    }
    let it = scene.layers[POPUP].texts[0];
    assert_eq!((it.x, it.y, it.scale), (43.0, 64.0, 2.0));
    assert_eq!(
        it.clip,
        Rect::new(23.0, 24.0, 100.0, 100.0),
        "logical, through the Xform"
    );
    // The alpha is part of the text's colour: the key is the faded spec's.
    assert_eq!(it.key, FakeText.prepare(&faded, 2.0).0);
    assert_ne!(
        it.key,
        FakeText.prepare(&spec, 2.0).0,
        "a fade must change the key"
    );
}

#[test]
fn an_arc_radius_takes_the_xform_scale() {
    let scene = scene_of(|p| {
        popup_under_clip_and_xform(p);
        p.arc([20.0, 30.0], 5.0, 0.0, 1.0, StrokeStyle::solid(WHITE, 1.0));
    });
    let v = &scene.layers[POPUP].geometry.strokes;
    assert!(!v.is_empty());
    // The centre (20, 30) is (86, 128) on the device; the radius 5 is 5 * 2 * 2 = 20 device px.
    // A vertex sits the stroke's half width plus one px of fringe (2 + 1) to either side.
    for q in v {
        let d = ((q.pos[0] - 86.0).powi(2) + (q.pos[1] - 128.0).powi(2)).sqrt();
        assert!((17.0 - 0.3..=23.0 + 0.3).contains(&d), "{d}");
    }
}

const ABOVE_BASE: [LayerId; 5] = [
    LayerId::Page,
    LayerId::Popup,
    LayerId::Modal,
    LayerId::Raised,
    LayerId::Toast,
];

#[test]
fn acrylic_above_base_is_a_frost_of_the_same_colour_and_under_is_ignored() {
    let colour = [0.1, 0.2, 0.3, 0.6];
    for layer in ABOVE_BASE {
        let scene = scene_of(|p| {
            p.on(layer);
            p.acrylic(LOW, 4.0, colour, [0.9; 4], true);
        });
        let g = &scene.layers[layer as usize].geometry;
        assert!(g.acrylic.is_empty(), "{layer:?}");
        assert_eq!(g.under.len(), 1, "{layer:?}");
        assert!(g.under[0].is_frost());
        assert_eq!(g.under[0].color, colour);
        assert_eq!(g.under[0].under, [0.0; 4]);
    }
    let on_base = scene_of(|p| p.acrylic(LOW, 4.0, colour, [0.9; 4], true));
    assert_eq!(on_base.layers[0].geometry.acrylic.len(), 1);
    assert!(on_base.layers[0].geometry.under.is_empty());
}

#[test]
fn acrylic_inside_an_overlay_on_any_layer_is_a_frost() {
    use lune_ui_core::testing::UiHarness;
    use lune_ui_core::ui::OverlayOptions;
    let mut h = UiHarness::new((200.0, 100.0));
    let motion = lune_ui_core::motion::Motion::of(lune_ui_core::motion::Kind::Appear);
    for layer in [LayerId::Base, LayerId::Popup, LayerId::Raised] {
        let options = OverlayOptions {
            layer,
            modal: false,
            dismiss_on_escape: false,
            dismiss_on_outside_press: false,
            dismiss_on_focus_loss: false,
            anchor: None,
            initial_focus: None,
            open: motion,
            close: motion,
        };
        let (scene, _) = h.frame(|ui| {
            ui.overlay(
                ("o", layer as usize),
                &options,
                |_, _, _| {},
                |ui, _| ui.painter().acrylic(LOW, 4.0, [0.1; 4], [0.0; 4], true),
            );
        });
        let drawn = scene.merged(layer);
        assert!(drawn.geometry.acrylic.is_empty(), "{layer:?}");
        assert!(drawn.geometry.under.iter().any(|s| s.is_frost()));
    }
}

/// How many frosts and acrylics `ops` draw when replayed on `layer`.
fn replayed_on(layer: LayerId, ops: &lune_ui_core::paint::Ops) -> (usize, usize) {
    let mut text = FakeText;
    let mut p = painter(&mut text);
    p.on(layer);
    p.replay(ops);
    let g = &p.scene.layers[layer as usize].geometry;
    let frosts = g.under.iter().filter(|s| s.is_frost()).count();
    (frosts, g.acrylic.len())
}

#[test]
fn one_acrylic_call_replays_as_one_draw() {
    let recorded_on = |layer: LayerId| {
        let mut text = FakeText;
        let mut p = painter(&mut text);
        p.on(layer);
        p.start_recording();
        p.acrylic(LOW, 4.0, [0.1; 4], [0.0; 4], true);
        p.take_recording()
    };
    let above = recorded_on(LayerId::Modal);
    assert_eq!(replayed_on(LayerId::Modal, &above), (1, 0));
    assert_eq!(replayed_on(LayerId::Base, &above), (1, 0), "an op of frost");
    let on_base = recorded_on(LayerId::Base);
    assert_eq!(replayed_on(LayerId::Modal, &on_base), (1, 0));
    assert_eq!(replayed_on(LayerId::Base, &on_base), (0, 1));
}

#[test]
fn a_frost_at_alpha_k_carries_k_and_keeps_its_colour() {
    let colour = [0.1, 0.2, 0.3, 0.6];
    let scene = scene_of(|p| {
        p.push_xform(xf((0.0, 0.0), 1.0, 0.0, 0.25));
        p.frost(LOW, 4.0, colour, true);
        p.pop_xform();
        p.push_xform(xf((0.0, 0.0), 1.0, 0.0, 0.0));
        p.frost(LOW, 4.0, colour, true);
        p.pop_xform();
    });
    let under = &scene.layers[0].geometry.under;
    assert_eq!(under.len(), 1, "alpha 0 draws nothing");
    assert_eq!(under[0].color, colour);
    assert_eq!(under[0].params, [0.0, 0.25, 2.0, 1.0]);
}

/// A ring's width is a width like a stroke's: a transform's scale scales it (the radius and the
/// rect already follow it).
#[test]
fn a_ring_under_a_scale_transform_is_scaled_like_everything_else() {
    for (device, scale) in [(1.0_f32, 0.9_f32), (2.0, 0.5), (1.5, 2.0)] {
        let mut t = FakeText;
        let mut p: P<'_> = Painter::new(
            device,
            Rect::new(0.0, 0.0, 400.0, 300.0),
            &mut t,
            None,
            THEME,
        );
        p.ring(Rect::new(10.0, 10.0, 50.0, 30.0), 4.0, 1.0, WHITE, false);
        p.push_xform(xf((30.0, 25.0), scale, 0.0, 1.0));
        p.ring(Rect::new(10.0, 10.0, 50.0, 30.0), 4.0, 1.0, WHITE, false);
        p.pop_xform();
        let under = &p.scene.layers[0].geometry.under;
        assert_eq!(under.len(), 2);
        assert_eq!(under[0].params[1], device, "the plain ring is 1 logical px");
        assert!(
            (under[1].params[1] - device * scale).abs() < 1e-5,
            "scale {scale} at {device}: {}",
            under[1].params[1]
        );
    }
}
