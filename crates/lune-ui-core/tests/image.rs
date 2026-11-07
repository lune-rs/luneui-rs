//! The image seam: fit geometry with known answers, the fake store and its contract, and what the
//! painter records for images and rotated text.

use lune_ui_core::image::{
    Anchor, Fit, ImageAlign, ImageData, ImageError, ImageId, ImageStore, image_rect,
};
use lune_ui_core::paint::{LayerId, Painter, Rect, Xform};
use lune_ui_core::scene::{FrameParams, ImageSource};
use lune_ui_core::testing::{
    DrawCall, FakeImages, FakeText, RecordingRenderer, image_store_contract,
};
use lune_ui_core::text::{Spec, Style, TextSystem};
use lune_ui_core::theme::DARK as THEME;

type P<'a> = Painter<'a, u8>;

fn painter(text: &mut FakeText, scale: f32) -> P<'_> {
    Painter::new(scale, Rect::new(0.0, 0.0, 400.0, 300.0), text, None, THEME)
}

// ── geometry ────────────────────────────────────────────────────────────────────────────────

#[test]
fn contain_known_answers() {
    let sq = Rect::new(0.0, 0.0, 100.0, 100.0);
    let c = ImageAlign::CENTER;
    assert_eq!(
        image_rect((200, 100), sq, Fit::Contain, c),
        Some(Rect::new(0.0, 25.0, 100.0, 50.0))
    );
    assert_eq!(
        image_rect((100, 200), sq, Fit::Contain, c),
        Some(Rect::new(25.0, 0.0, 50.0, 100.0))
    );
    assert_eq!(
        image_rect(
            (200, 100),
            Rect::new(0.0, 0.0, 300.0, 100.0),
            Fit::Contain,
            ImageAlign::LEFT
        ),
        Some(Rect::new(0.0, 0.0, 200.0, 100.0))
    );
}

#[test]
fn contain_scales_up_and_aligns_to_the_end() {
    let dst = Rect::new(10.0, 20.0, 400.0, 100.0);
    let end = ImageAlign {
        x: Anchor::End,
        y: Anchor::Start,
    };
    // 2 by 1 into 400 by 100: scale 100 vertically, 200 by 100, pushed against the right edge.
    assert_eq!(
        image_rect((2, 1), dst, Fit::Contain, end),
        Some(Rect::new(210.0, 20.0, 200.0, 100.0))
    );
}

#[test]
fn nothing_to_fit_is_none() {
    let dst = Rect::new(0.0, 0.0, 10.0, 10.0);
    let c = ImageAlign::CENTER;
    assert_eq!(image_rect((0, 5), dst, Fit::Contain, c), None);
    assert_eq!(image_rect((5, 0), dst, Fit::Contain, c), None);
    assert_eq!(
        image_rect((5, 5), Rect::new(0.0, 0.0, 0.0, 10.0), Fit::Contain, c),
        None
    );
    assert_eq!(
        image_rect((5, 5), Rect::new(0.0, 0.0, 10.0, -1.0), Fit::Contain, c),
        None
    );
    assert_eq!(
        image_rect(
            (5, 5),
            Rect::new(f32::NAN, 0.0, 10.0, 10.0),
            Fit::Contain,
            c
        ),
        None
    );
    assert_eq!(
        image_rect(
            (5, 5),
            Rect::new(0.0, 0.0, f32::INFINITY, 10.0),
            Fit::Contain,
            c
        ),
        None
    );
}

// ── the fake store ──────────────────────────────────────────────────────────────────────────

#[test]
fn the_fake_meets_the_contract_with_and_without_a_limit() {
    let mut s = FakeImages::new();
    assert_eq!(image_store_contract(&mut s, FakeImages::len, None), Ok(()));
    let mut s = FakeImages::with_limit(16);
    assert_eq!(
        image_store_contract(&mut s, FakeImages::len, Some(16)),
        Ok(())
    );
}

#[test]
fn a_refused_picture_leaves_the_old_one() {
    let mut s = FakeImages::new();
    let id = ImageId(7);
    let ok = ImageData {
        width: 2,
        height: 1,
        rgba: vec![0; 8],
    };
    s.register(id, ok).expect("registers");
    let bad = ImageData {
        width: 2,
        height: 2,
        rgba: vec![0; 8],
    };
    assert_eq!(s.register(id, bad), Err(ImageError::BadLength));
    assert_eq!(s.size(id), Some((2, 1)));
    assert_eq!(s.freed(), 0);
}

#[test]
fn a_length_that_overflows_is_refused() {
    let mut s = FakeImages::new();
    let d = ImageData {
        width: u32::MAX,
        height: u32::MAX,
        rgba: vec![0; 4],
    };
    assert_eq!(s.register(ImageId(1), d), Err(ImageError::BadLength));
}

// ── the painter ─────────────────────────────────────────────────────────────────────────────

#[test]
fn an_image_is_recorded_with_layer_clip_and_scale() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    p.on(LayerId::Popup);
    p.push_clip(Rect::new(10.0, 10.0, 50.0, 50.0));
    p.image(
        Rect::new(20.0, 30.0, 100.0, 60.0),
        ImageId(9),
        Fit::Contain,
        ImageAlign::LEFT,
        0.5,
    );
    assert!(p.scene.layers[0].images.is_empty());
    let it = p.scene.layers[2].images[0];
    assert_eq!(it.source, ImageSource::Id(ImageId(9)));
    assert_eq!(it.dst, Rect::new(40.0, 60.0, 200.0, 120.0));
    assert_eq!(it.clip, [20.0, 20.0, 100.0, 100.0]);
    assert_eq!((it.alpha, it.rotation), (0.5, 0.0));
    assert_eq!(it.align, ImageAlign::LEFT);
    assert_eq!(it.round_clip.radius, 0.0);
}

#[test]
fn an_image_follows_the_transform_stack() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    p.push_xform(Xform {
        origin: (0.0, 0.0),
        scale: 2.0,
        dx: 3.0,
        dy: 4.0,
        alpha: 0.5,
    });
    p.image(
        Rect::new(10.0, 10.0, 10.0, 5.0),
        ImageId(1),
        Fit::Contain,
        ImageAlign::CENTER,
        0.5,
    );
    let it = p.scene.layers[0].images[0];
    // (10, 10) -> (23, 24) logical -> x2 device; size 10x5 -> 20x10 -> x2.
    assert_eq!(it.dst, Rect::new(46.0, 48.0, 40.0, 20.0));
    assert_eq!(it.alpha, 0.25);
}

#[test]
fn the_rounded_clip_travels_on_the_image() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    p.push_clip_rounded(Rect::new(10.0, 10.0, 50.0, 50.0), 8.0);
    p.image(
        Rect::new(0.0, 0.0, 100.0, 100.0),
        ImageId(1),
        Fit::Contain,
        ImageAlign::CENTER,
        1.0,
    );
    let it = p.scene.layers[0].images[0];
    assert_eq!(it.round_clip.rect, [20.0, 20.0, 100.0, 100.0]);
    assert_eq!(it.round_clip.radius, 16.0);
    p.pop_clip();
    p.image(
        Rect::new(0.0, 0.0, 100.0, 100.0),
        ImageId(1),
        Fit::Contain,
        ImageAlign::CENTER,
        1.0,
    );
    assert_eq!(p.scene.layers[0].images[1].round_clip.radius, 0.0);
}

#[test]
fn image_rotated_keeps_the_angle() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 1.0);
    p.image_rotated(
        Rect::new(0.0, 0.0, 10.0, 10.0),
        ImageId(1),
        Fit::Contain,
        ImageAlign::CENTER,
        1.0,
        1.5,
    );
    assert_eq!(p.scene.layers[0].images[0].rotation, 1.5);
}

#[test]
fn bad_images_draw_nothing() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    let (f, a) = (Fit::Contain, ImageAlign::CENTER);
    let id = ImageId(1);
    let ok = Rect::new(0.0, 0.0, 10.0, 10.0);
    p.image(Rect::new(0.0, 0.0, 0.0, 10.0), id, f, a, 1.0);
    p.image(Rect::new(0.0, 0.0, 10.0, -1.0), id, f, a, 1.0);
    p.image(Rect::new(f32::NAN, 0.0, 10.0, 10.0), id, f, a, 1.0);
    p.image(Rect::new(0.0, 0.0, f32::INFINITY, 10.0), id, f, a, 1.0);
    p.image(ok, id, f, a, f32::NAN);
    p.image(ok, id, f, a, 0.0);
    p.image(ok, id, f, a, -1.0);
    p.image_rotated(ok, id, f, a, 1.0, f32::NAN);
    p.image_rotated(ok, id, f, a, 1.0, f32::INFINITY);
    p.text_rotated(
        &Spec::plain("x", Style::ui(13.0, THEME.fg_primary)),
        (f32::NAN, 0.0),
        1.0,
    );
    p.text_rotated(
        &Spec::plain("x", Style::ui(13.0, THEME.fg_primary)),
        (0.0, 0.0),
        f32::NAN,
    );
    assert!(p.scene.in_order().into_iter().all(|l| l.images.is_empty()));
    // An alpha above 1 is clamped, not refused.
    p.image(ok, id, f, a, 7.0);
    assert_eq!(p.scene.layers[0].images[0].alpha, 1.0);
}

#[test]
fn rotated_text_is_an_image_of_the_text_centred_on_the_point() {
    let mut t = FakeText;
    let spec = Spec::plain("hello", Style::ui(20.0, THEME.fg_primary));
    let (key, (w, h)) = FakeText.prepare(&spec, 2.0);
    let mut p = painter(&mut t, 2.0);
    let size = p.text_rotated(&spec, (50.0, 40.0), 1.0);
    assert_eq!(size, (w, h));
    let it = p.scene.layers[0].images[0];
    assert_eq!(it.source, ImageSource::Text(key, 1.0));
    assert_eq!(it.rotation, 1.0);
    let c = (it.dst.x + it.dst.w / 2.0, it.dst.y + it.dst.h / 2.0);
    assert_eq!(c, (100.0, 80.0));
    assert_eq!((it.dst.w, it.dst.h), (w * 2.0, h * 2.0));
    // A fade multiplies at draw time and does not change the text key.
    p.push_xform(Xform {
        origin: (0.0, 0.0),
        scale: 1.0,
        dx: 0.0,
        dy: 0.0,
        alpha: 0.5,
    });
    p.text_rotated(&spec, (50.0, 40.0), 1.0);
    let faded = p.scene.layers[0].images[1];
    assert_eq!(faded.source, it.source);
    assert_eq!(faded.alpha, 0.5);
}

#[test]
fn images_draw_after_customs_and_before_text() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 1.0);
    let spec = Spec::plain("hi", Style::ui(13.0, THEME.fg_primary));
    let r = Rect::new(0.0, 0.0, 20.0, 20.0);
    p.text(&spec, 1.0, 1.0);
    p.image(r, ImageId(1), Fit::Contain, ImageAlign::CENTER, 1.0);
    p.custom(0, r);
    let params = FrameParams {
        size: (400, 300),
        scale: 1.0,
        blur_sigma: 4.0,
        saturation: 1.0,
        ..FrameParams::default()
    };
    let mut rec = RecordingRenderer::new();
    rec.render(&p.scene, &params);
    let kinds: Vec<&str> = rec
        .last()
        .expect("frame")
        .calls
        .iter()
        .map(|c| match c {
            DrawCall::Custom { .. } => "custom",
            DrawCall::Image { .. } => "image",
            DrawCall::Text { .. } => "text",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["custom", "image", "text"]);
}

#[test]
fn images_replay_to_the_same_scene() {
    let mut t = FakeText;
    let mut a = painter(&mut t, 2.0);
    a.start_recording();
    a.push_clip_rounded(Rect::new(0.0, 0.0, 80.0, 80.0), 4.0);
    a.image(
        Rect::new(0.0, 0.0, 50.0, 30.0),
        ImageId(3),
        Fit::Contain,
        ImageAlign::LEFT,
        0.8,
    );
    a.image_rotated(
        Rect::new(0.0, 0.0, 50.0, 30.0),
        ImageId(4),
        Fit::Contain,
        ImageAlign::CENTER,
        1.0,
        0.5,
    );
    a.text_rotated(
        &Spec::plain("go", Style::ui(13.0, THEME.fg_primary)),
        (30.0, 30.0),
        0.3,
    );
    a.pop_clip();
    let ops = a.take_recording();
    assert_eq!(ops.len(), 5);
    let mut t2 = FakeText;
    let mut b = painter(&mut t2, 2.0);
    b.replay(&ops);
    assert_eq!(a.scene, b.scene);
    assert_eq!(a.scene.layers[0].images.len(), 3);
}
