//! Pixel probes of the vector primitives at scale 2: dash, sector, path-fill and the rounded
//! clip, drawn through the real renderer into an offscreen target and read back.
//!
//! GPU test: with no adapter it prints `skipped: no GPU adapter` and passes, unless
//! `LUNEUI_REQUIRE_GPU=1`, which makes a missing adapter a failure.

use lune_ui_core::paint::{Painter, Rect, StrokeStyle};
use lune_ui_core::scene::Scene;
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_wgpu::dump;
use lune_ui_wgpu::frame::{FrameParams, FrameRenderer};
use lune_ui_wgpu::gpu::{self, Gpu};
use lune_ui_wgpu::text_cache::TextCache;

const SCALE: f32 = 2.0;
/// Device px.
const SIZE: (u32, u32) = (240, 160);
const WHITE: [u8; 4] = [255, 255, 255, 255];

fn fonts() -> [&'static [u8]; 3] {
    [
        include_bytes!("../../../apps/gallery/assets/RobotoMonoNerdFontMono-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Medium.ttf"),
    ]
}

fn text_cache() -> TextCache {
    TextCache::new(&fonts(), "RznSans", "RobotoMono Nerd Font Mono")
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

/// Paints with `f` into a fresh painter over the window (logical px) and returns the scene.
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
    dump::read_rgba(&gpu.device, &gpu.queue, &texture).expect("readback")
}

fn px(img: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * SIZE.0 as usize + x) * 4;
    [img[i], img[i + 1], img[i + 2], img[i + 3]]
}

#[test]
fn a_dashed_line_has_its_runs_and_gaps() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    // Pattern 1 on, 2 off logical px at scale 2: 2 device px on, 4 off, 2 px thick (1 logical).
    let scene = paint(&mut text, |p| {
        p.dash(&[[0.0, 10.0], [50.0, 10.0]], false, 1.0, 1.0, 2.0, WHITE);
    });
    let img = render(&mut gpu, &mut text, &scene);
    // Instrument check: a pixel far from the line is empty, one on a run is white.
    assert_eq!(px(&img, 100, 100), [0, 0, 0, 0]);
    for row in [19, 20] {
        for k in 0..8 {
            let start = 6 * k;
            for dx in 0..2 {
                assert_eq!(
                    px(&img, start + dx, row),
                    WHITE,
                    "run {k} px {dx} row {row}"
                );
            }
            for dx in 2..6 {
                assert_eq!(px(&img, start + dx, row)[3], 0, "gap {k} px {dx} row {row}");
            }
        }
    }
    // Two device px thick: the rows beside the line are empty.
    assert_eq!(px(&img, 1, 17)[3], 0);
    assert_eq!(px(&img, 1, 22)[3], 0);
}

#[test]
fn a_sector_has_an_antialiased_edge_and_a_solid_inside() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    // Centre (60, 60) device, radius 40, from the right turning down a quarter.
    let scene = paint(&mut text, |p| {
        p.sector([30.0, 30.0], 20.0, 0.0, std::f32::consts::FRAC_PI_2, WHITE);
    });
    let img = render(&mut gpu, &mut text, &scene);
    // Instrument check: outside the sector is empty.
    assert_eq!(px(&img, 30, 30), [0, 0, 0, 0]);
    assert_eq!(px(&img, 150, 100), [0, 0, 0, 0]);
    // The radial edge along y = 60: the row above is the boundary pixel.
    let edge = px(&img, 80, 59);
    assert!(edge[3] > 0 && edge[3] < 255, "radial edge alpha {edge:?}");
    // The arc, at 45 degrees: pixel (88, 88) straddles the circle.
    let arc = px(&img, 88, 88);
    assert!(arc[3] > 0 && arc[3] < 255, "arc edge alpha {arc:?}");
    // Two px inside every edge it is full.
    for (x, y) in [(80, 62), (62, 80), (75, 75), (63, 63)] {
        assert_eq!(px(&img, x, y), WHITE, "inside ({x}, {y})");
    }
    // No seam at the centre: the pixels around the apex are filled where the sector is.
    for (x, y) in [(62, 62), (61, 64), (64, 61)] {
        assert_eq!(px(&img, x, y)[3], 255, "apex ({x}, {y})");
    }
}

#[test]
fn a_filled_rect_through_fill_path_draws_under_later_text() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let square = [[10.0, 10.0], [100.0, 10.0], [100.0, 60.0], [10.0, 60.0]];
    let dark = Spec::plain("Hello", Style::ui(24.0, [0, 0, 0, 255]));
    // The text call comes first and the fill second: the fill still goes below.
    let with_text = paint(&mut text, |p| {
        p.text(&dark, 20.0, 20.0);
        p.fill_path(&square, WHITE);
    });
    let only_fill = paint(&mut text, |p| p.fill_path(&square, WHITE));
    let a = render(&mut gpu, &mut text, &only_fill);
    let b = render(&mut gpu, &mut text, &with_text);
    // Instrument check: the fill is white inside and empty outside.
    assert_eq!(px(&a, 100, 60), WHITE);
    assert_eq!(px(&a, 5, 5)[3], 0);
    assert_eq!(px(&a, 215, 140)[3], 0);
    // An edge pixel is partial (the fringe), two px outside is empty.
    assert_eq!(px(&a, 18, 40)[3], 0, "outside left");
    assert_eq!(
        px(&a, 19, 40)[3],
        64,
        "boundary pixel left: {:?}",
        px(&a, 19, 40)
    );
    assert_eq!(px(&a, 20, 40), WHITE);
    // Inside the rect only the text darkens pixels.
    let darkest = |img: &[u8]| {
        let mut m = 255;
        for y in 24..120 {
            for x in 24..200 {
                m = m.min(px(img, x, y)[0]);
            }
        }
        m
    };
    assert_eq!(darkest(&a), 255);
    assert!(
        darkest(&b) < 60,
        "text pixels over the fill: {}",
        darkest(&b)
    );
}

#[test]
fn a_rounded_clip_cuts_the_corner_of_shapes_fills_and_strokes() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    // Clip rect (10, 10, 60, 40) logical = (20, 20, 120, 80) device, radius 8 = 16 device.
    let clip = Rect::new(10.0, 10.0, 60.0, 40.0);
    let all = Rect::new(0.0, 0.0, 120.0, 80.0);
    let scenes: [(&str, Scene); 3] = [
        (
            "shape",
            paint(&mut text, |p| {
                p.push_clip_rounded(clip, 8.0);
                p.fill(all, 0.0, WHITE);
            }),
        ),
        (
            "fill_path",
            paint(&mut text, |p| {
                p.push_clip_rounded(clip, 8.0);
                p.fill_path(
                    &[[0.0, 0.0], [120.0, 0.0], [120.0, 80.0], [0.0, 80.0]],
                    WHITE,
                );
            }),
        ),
        (
            "stroke",
            paint(&mut text, |p| {
                p.push_clip_rounded(clip, 8.0);
                // A thick horizontal bar over the whole clip.
                for y in 0..80 {
                    p.stroke(
                        &[[0.0, y as f32 + 0.5], [120.0, y as f32 + 0.5]],
                        false,
                        StrokeStyle::solid(WHITE, 1.0),
                    );
                }
            }),
        ),
    ];
    for (what, scene) in &scenes {
        let img = render(&mut gpu, &mut text, scene);
        // Instrument check: the middle of the clip is filled, outside it is empty.
        assert_eq!(px(&img, 80, 60), WHITE, "{what}: middle");
        assert_eq!(px(&img, 10, 60)[3], 0, "{what}: outside the clip rect");
        // The corner pixel is cut away; the pixel 16 device px in is filled.
        assert_eq!(px(&img, 20, 20)[3], 0, "{what}: corner");
        assert_eq!(px(&img, 139, 99)[3], 0, "{what}: opposite corner");
        assert_eq!(px(&img, 36, 36), WHITE, "{what}: 16 px in");
        // A pixel on the corner arc is partial: (24, 24) is at distance ~ 17.5 - ... from the
        // arc centre (36, 36): 16.97 vs radius 16.
        let arc = px(&img, 24, 24);
        assert!(arc[3] < 255, "{what}: the arc cuts (24, 24): {arc:?}");
        // Along the straight edge the clip does not round anything.
        assert_eq!(px(&img, 80, 21), WHITE, "{what}: straight edge");
    }
}

#[test]
fn a_radius_of_zero_takes_the_old_path() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let clip = Rect::new(10.0, 10.0, 60.0, 40.0);
    let plain = paint(&mut text, |p| {
        p.push_clip(clip);
        p.fill(Rect::new(0.0, 0.0, 120.0, 80.0), 0.0, WHITE);
    });
    let zero = paint(&mut text, |p| {
        p.push_clip_rounded(clip, 0.0);
        p.fill(Rect::new(0.0, 0.0, 120.0, 80.0), 0.0, WHITE);
    });
    assert_eq!(plain, zero);
    let img = render(&mut gpu, &mut text, &zero);
    assert_eq!(px(&img, 20, 20), WHITE);
}
