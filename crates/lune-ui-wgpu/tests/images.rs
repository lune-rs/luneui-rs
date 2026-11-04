//! The image seam on the GPU: the store contract, exact pixel probes of drawn pictures, rotated
//! text through `Buffer::draw`, and the rounded clip on an image.
//!
//! GPU test: with no adapter it prints `skipped: no GPU adapter` and passes, unless
//! `LUNEUI_REQUIRE_GPU=1`, which makes a missing adapter a failure.

use lune_ui_core::image::{Fit, ImageAlign, ImageData, ImageId, ImageStore};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::image_store_contract;
use lune_ui_core::text::{Spec, Style, TextSystem};
use lune_ui_core::theme::DARK;
use lune_ui_wgpu::dump;
use lune_ui_wgpu::frame::{FrameParams, FrameRenderer};
use lune_ui_wgpu::gpu::{self, Gpu};
use lune_ui_wgpu::text_cache::TextCache;

const SCALE: f32 = 2.0;
/// Device px.
const SIZE: (u32, u32) = (240, 160);
const CENTER: ImageAlign = ImageAlign::CENTER;

fn text_cache() -> TextCache {
    let fonts: [&[u8]; 3] = [
        include_bytes!("../../../apps/gallery/assets/RobotoMonoNerdFontMono-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Medium.ttf"),
    ];
    TextCache::new(&fonts, "RznSans", "RobotoMono Nerd Font Mono")
}

fn headless_gpu() -> Option<Gpu> {
    match pollster::block_on(Gpu::new(gpu::instance(None), None)) {
        Ok(g) => Some(g),
        Err(e) => {
            let required = std::env::var("LUNEUI_REQUIRE_GPU").is_ok_and(|v| v == "1");
            assert!(!required, "no GPU adapter: {e}");
            println!("skipped: no GPU adapter");
            None
        }
    }
}

fn paint(text: &mut TextCache, f: impl FnOnce(&mut Painter<'_, u8>)) -> Scene {
    let win = Rect::new(0.0, 0.0, SIZE.0 as f32 / SCALE, SIZE.1 as f32 / SCALE);
    let mut p: Painter<'_, u8> = Painter::new(SCALE, win, text, None, DARK);
    f(&mut p);
    p.scene
}

fn render(gpu: &mut Gpu, text: &mut TextCache, scene: &Scene) -> Vec<u8> {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = FrameRenderer::new(gpu, format);
    let texture = dump::target(&gpu.device, format, SIZE);
    let view = texture.create_view(&Default::default());
    let params = FrameParams {
        size: SIZE,
        scale: SCALE,
        blur_sigma: 4.0,
        saturation: 1.0,
    };
    renderer
        .draw(gpu, text, scene, &view, params)
        .expect("draw");
    gpu.end_frame(text);
    dump::read_rgba(&gpu.device, &gpu.queue, &texture).expect("readback")
}

fn px(img: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * SIZE.0 as usize + x) * 4;
    [img[i], img[i + 1], img[i + 2], img[i + 3]]
}

/// A 2 by 2 picture: red, green / blue, white, all opaque.
fn quad() -> ImageData {
    ImageData {
        width: 2,
        height: 2,
        rgba: vec![
            255, 0, 0, 255, 0, 255, 0, 255, //
            0, 0, 255, 255, 255, 255, 255, 255,
        ],
    }
}

fn solid(w: u32, h: u32, c: [u8; 4]) -> ImageData {
    ImageData {
        width: w,
        height: h,
        rgba: c
            .iter()
            .copied()
            .cycle()
            .take(4 * (w * h) as usize)
            .collect(),
    }
}

#[test]
fn the_gpu_store_meets_the_contract() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let limit = gpu.device.limits().max_texture_dimension_2d;
    assert_eq!(
        image_store_contract(&mut gpu, Gpu::image_count, Some(limit)),
        Ok(())
    );
    assert_eq!(gpu.image_count(), 0);
}

#[test]
fn an_oversize_or_short_picture_is_refused_before_the_gpu() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let limit = gpu.device.limits().max_texture_dimension_2d;
    let err = gpu.register(ImageId(1), solid(limit + 1, 1, [0; 4]));
    assert_eq!(
        err,
        Err(lune_ui_core::image::ImageError::TooLarge { limit })
    );
    let short = ImageData {
        width: 4,
        height: 4,
        rgba: vec![0; 63],
    };
    assert_eq!(
        gpu.register(ImageId(1), short),
        Err(lune_ui_core::image::ImageError::BadLength)
    );
    assert_eq!(gpu.image_count(), 0);
}

#[test]
fn four_exact_quadrants_at_an_integer_ratio() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    gpu.register(ImageId(1), quad()).expect("registers");
    // 8 by 8 logical = 16 by 16 device at (10, 10): 8 device px per source pixel.
    let scene = paint(&mut text, |p| {
        p.image(
            Rect::new(5.0, 5.0, 8.0, 8.0),
            ImageId(1),
            Fit::Contain,
            CENTER,
            1.0,
        );
    });
    let img = render(&mut gpu, &mut text, &scene);
    // Instrument check: outside the picture is empty.
    assert_eq!(px(&img, 5, 5), [0, 0, 0, 0]);
    assert_eq!(px(&img, 27, 27), [0, 0, 0, 0]);
    // 16 device px: each source pixel covers 8; the picture starts at device (10, 10).
    for (x, y, want) in [
        (10, 10, [255, 0, 0, 255]),
        (17, 17, [255, 0, 0, 255]),
        (18, 10, [0, 255, 0, 255]),
        (25, 17, [0, 255, 0, 255]),
        (10, 18, [0, 0, 255, 255]),
        (17, 25, [0, 0, 255, 255]),
        (18, 18, [255, 255, 255, 255]),
        (25, 25, [255, 255, 255, 255]),
    ] {
        assert_eq!(px(&img, x, y), want, "({x}, {y})");
    }
}

#[test]
fn contain_fits_a_wide_picture_centred() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    gpu.register(ImageId(2), solid(20, 10, [255, 255, 255, 255]))
        .expect("registers");
    // 20 by 10 into 20 by 20 logical at (10, 10) = device (20, 20, 40, 40): drawn 40 by 20 at
    // (20, 30).
    let scene = paint(&mut text, |p| {
        p.image(
            Rect::new(10.0, 10.0, 20.0, 20.0),
            ImageId(2),
            Fit::Contain,
            CENTER,
            1.0,
        );
    });
    let img = render(&mut gpu, &mut text, &scene);
    assert_eq!(px(&img, 30, 40), [255; 4]);
    assert_eq!(px(&img, 21, 30), [255; 4]);
    assert_eq!(px(&img, 38, 49), [255; 4]);
    assert_eq!(px(&img, 30, 28)[3], 0);
    assert_eq!(px(&img, 30, 51)[3], 0);
}

#[test]
fn alpha_blends_over_a_fill() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    gpu.register(ImageId(3), solid(2, 2, [0, 0, 255, 255]))
        .expect("registers");
    let bg = [200, 100, 50, 255];
    let scene = paint(&mut text, |p| {
        p.fill(Rect::new(0.0, 0.0, 120.0, 80.0), 0.0, bg);
        p.image(
            Rect::new(10.0, 10.0, 20.0, 20.0),
            ImageId(3),
            Fit::Contain,
            CENTER,
            0.5,
        );
    });
    let img = render(&mut gpu, &mut text, &scene);
    assert_eq!(px(&img, 5, 5), bg, "instrument check: the fill");
    let got = px(&img, 30, 30);
    let want = [100.0, 50.0, 152.5, 255.0];
    for c in 0..4 {
        assert!(
            (f32::from(got[c]) - want[c]).abs() <= 1.5,
            "{got:?} vs {want:?}"
        );
    }
}

#[test]
fn straight_alpha_is_premultiplied_on_upload() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    // Straight red at alpha 128 over transparent: premultiplied (128, 0, 0, 128).
    gpu.register(ImageId(4), solid(1, 1, [255, 0, 0, 128]))
        .expect("registers");
    let scene = paint(&mut text, |p| {
        p.image(
            Rect::new(10.0, 10.0, 10.0, 10.0),
            ImageId(4),
            Fit::Contain,
            CENTER,
            1.0,
        );
    });
    let img = render(&mut gpu, &mut text, &scene);
    let got = px(&img, 30, 30);
    assert!(
        got[0].abs_diff(128) <= 1 && got[1] == 0 && got[3].abs_diff(128) <= 1,
        "{got:?}"
    );
}

#[test]
fn unknown_released_and_refused_ids_draw_nothing() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let limit = gpu.device.limits().max_texture_dimension_2d;
    let r = Rect::new(10.0, 10.0, 20.0, 20.0);
    gpu.register(ImageId(5), solid(2, 2, [255; 4]))
        .expect("registers");
    gpu.register(ImageId(6), solid(2, 2, [255; 4]))
        .expect("registers");
    gpu.release(ImageId(6));
    assert!(
        gpu.register(ImageId(7), solid(limit + 1, 1, [255; 4]))
            .is_err()
    );
    let scene = paint(&mut text, |p| {
        p.image(r, ImageId(5), Fit::Contain, CENTER, 1.0);
        for id in [6, 7, 8] {
            p.image(
                Rect::new(70.0, 10.0, 20.0, 20.0),
                ImageId(id),
                Fit::Contain,
                CENTER,
                1.0,
            );
        }
    });
    let img = render(&mut gpu, &mut text, &scene);
    assert_eq!(
        px(&img, 30, 30),
        [255; 4],
        "the registered id draws (instrument check)"
    );
    for x in 130..200 {
        assert_eq!(px(&img, x, 40)[3], 0, "x {x}");
    }
}

#[test]
fn replacing_an_id_draws_the_new_picture() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    gpu.register(ImageId(1), solid(1, 1, [255, 0, 0, 255]))
        .expect("registers");
    gpu.register(ImageId(1), solid(1, 1, [0, 255, 0, 255]))
        .expect("replaces");
    let scene = paint(&mut text, |p| {
        p.image(
            Rect::new(10.0, 10.0, 10.0, 10.0),
            ImageId(1),
            Fit::Contain,
            CENTER,
            1.0,
        );
    });
    let img = render(&mut gpu, &mut text, &scene);
    assert_eq!(px(&img, 30, 30), [0, 255, 0, 255]);
    assert_eq!(gpu.image_count(), 1);
}

#[test]
fn an_image_under_clip_rounded_loses_its_corner() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    gpu.register(ImageId(1), solid(4, 4, [255; 4]))
        .expect("registers");
    // Clip (10, 10, 60, 40) logical = device (20, 20, 120, 80), radius 8 = 16 device.
    let scene = paint(&mut text, |p| {
        p.push_clip_rounded(Rect::new(10.0, 10.0, 60.0, 40.0), 8.0);
        p.image(
            Rect::new(0.0, 0.0, 120.0, 80.0),
            ImageId(1),
            Fit::Contain,
            CENTER,
            1.0,
        );
    });
    let img = render(&mut gpu, &mut text, &scene);
    // The picture is 80 by 80 logical centred in 120 by 80: device x 40..200, y 0..160.
    assert_eq!(px(&img, 80, 60), [255; 4], "instrument check: the middle");
    // The clip's left edge at x = 20 is left of the picture; use its top-right corner instead.
    assert_eq!(
        px(&img, 139, 20)[3],
        0,
        "top-right corner of the clip is cut"
    );
    assert_eq!(px(&img, 139, 99)[3], 0, "bottom-right corner is cut");
    assert_eq!(px(&img, 123, 36), [255; 4], "16 px in from the corner");
}

fn ink_box(img: &[u8]) -> Option<(f32, f32, f32, f32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    for y in 0..SIZE.1 as usize {
        for x in 0..SIZE.0 as usize {
            if px(img, x, y)[3] >= 128 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    (x1 > 0).then_some((x0 as f32, y0 as f32, x1 as f32, y1 as f32))
}

#[test]
fn rotated_text_is_drawn_through_buffer_draw() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let spec = Spec::plain("Hello", Style::ui(20.0, [255, 255, 255, 255]));
    let (_, (w, h)) = text.prepare(&spec, SCALE);
    assert!(
        w > 20.0 && h > 10.0,
        "fonts are loaded (instrument check): {w} x {h}"
    );
    let (cx, cy) = (60.0, 40.0);

    // A: the normal text path, placed with the same centre.
    let a_scene = paint(&mut text, |p| {
        p.text(&spec, cx - w / 2.0, cy - h / 2.0);
    });
    let a = render(&mut gpu, &mut text, &a_scene);
    let a_box = ink_box(&a).expect("ink drawn by the text path");
    assert!(a_box.2 - a_box.0 > 40.0, "{a_box:?}");

    // B: rotated by 0 through the raster: the same ink within a pixel.
    let b_scene = paint(&mut text, |p| {
        p.text_rotated(&spec, (cx, cy), 0.0);
    });
    let b = render(&mut gpu, &mut text, &b_scene);
    let b_box = ink_box(&b).expect("ink drawn by the raster path");
    for (g, w) in [b_box.0, b_box.1, b_box.2, b_box.3]
        .into_iter()
        .zip([a_box.0, a_box.1, a_box.2, a_box.3])
    {
        assert!(
            (g - w).abs() <= 1.0,
            "unrotated raster {b_box:?} vs text path {a_box:?}"
        );
    }

    // C: a quarter turn clockwise about the centre: width and height swap, and the ink centre
    // turns with it about the text's centre.
    let c_scene = paint(&mut text, |p| {
        p.text_rotated(&spec, (cx, cy), std::f32::consts::FRAC_PI_2);
    });
    let c = render(&mut gpu, &mut text, &c_scene);
    let c_box = ink_box(&c).expect("ink drawn rotated");
    let (aw, ah) = (a_box.2 - a_box.0, a_box.3 - a_box.1);
    let (cw, ch) = (c_box.2 - c_box.0, c_box.3 - c_box.1);
    assert!(
        (cw - ah).abs() <= 1.0 && (ch - aw).abs() <= 1.0,
        "swapped: {aw}x{ah} -> {cw}x{ch}"
    );
    let (ox, oy) = (cx * SCALE, cy * SCALE);
    let a_c = (
        (a_box.0 + a_box.2) / 2.0 - ox,
        (a_box.1 + a_box.3) / 2.0 - oy,
    );
    let want = (ox - a_c.1, oy + a_c.0);
    let got = ((c_box.0 + c_box.2) / 2.0, (c_box.1 + c_box.3) / 2.0);
    assert!(
        (got.0 - want.0).abs() <= 1.0 && (got.1 - want.1).abs() <= 1.0,
        "ink centre {got:?}, expected {want:?}"
    );
}

#[test]
fn a_text_raster_lives_while_its_text_is_cached_and_a_fade_does_not_redo_it() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let spec = Spec::plain("spin", Style::ui(16.0, [255, 255, 255, 255]));
    let scene = paint(&mut text, |p| {
        p.text_rotated(&spec, (40.0, 40.0), 0.7);
    });
    let one = render(&mut gpu, &mut text, &scene);
    assert_eq!(gpu.text_image_count(), 1);
    // Same key at half alpha: still one raster; the pixels are half as strong.
    let faded = paint(&mut text, |p| {
        p.push_xform(lune_ui_core::paint::Xform {
            origin: (0.0, 0.0),
            scale: 1.0,
            dx: 0.0,
            dy: 0.0,
            alpha: 0.5,
        });
        p.text_rotated(&spec, (40.0, 40.0), 0.7);
    });
    let half = render(&mut gpu, &mut text, &faded);
    assert_eq!(gpu.text_image_count(), 1);
    let max = |img: &[u8]| {
        (0..img.len() / 4)
            .map(|i| img[4 * i + 3])
            .max()
            .unwrap_or(0)
    };
    assert!(max(&one) >= 250, "{}", max(&one));
    assert!(
        max(&half).abs_diff(max(&one) / 2) <= 2,
        "{} vs {}",
        max(&half),
        max(&one)
    );
    // The text is no longer used: the cache drops it after 180 frames, and the raster with it.
    for _ in 0..200 {
        gpu.end_frame(&mut text);
    }
    assert_eq!(gpu.text_image_count(), 0);
}
