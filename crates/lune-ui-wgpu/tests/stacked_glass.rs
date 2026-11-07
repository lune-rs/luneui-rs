//! Stacked glass through the real renderer: each backdrop request blurs what is drawn before its
//! layer, with its own numbers, however many a frame has.
//!
//! GPU test: with no adapter it prints `skipped: no GPU adapter` and passes, unless
//! `LUNEUI_REQUIRE_GPU=1`, which makes a missing adapter a failure.

use lune_ui_core::color::premul;
use lune_ui_core::motion::{Kind, Motion};
use lune_ui_core::paint::{LayerId, Painter, Rect, Xform};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::OverlayOptions;
use lune_ui_wgpu::dump;
use lune_ui_wgpu::frame::{FrameParams, FrameRenderer};
use lune_ui_wgpu::gpu::{self, Gpu};
use lune_ui_wgpu::text_cache::TextCache;

const SIZE: (u32, u32) = (480, 160);
const WIN: Rect = Rect::new(0.0, 0.0, SIZE.0 as f32, SIZE.1 as f32);
const DARKEST: [u8; 4] = [20, 20, 20, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const GREEN: [u8; 4] = [40, 220, 60, 255];
/// A row through the middle of the regions.
const ROW: usize = 80;
/// A fill light enough to show what is under it.
const VEIL: [u8; 4] = [30, 30, 30, 40];

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

fn params() -> FrameParams {
    FrameParams {
        size: SIZE,
        scale: 1.0,
        blur_sigma: 8.0,
        saturation: 1.0,
    }
}

fn text_cache() -> TextCache {
    TextCache::new(&[], "sans-serif", "monospace")
}

fn draw(gpu: &mut Gpu, renderer: &mut FrameRenderer, scene: &Scene) -> Vec<u8> {
    let texture = dump::target(&gpu.device, wgpu::TextureFormat::Rgba8Unorm, SIZE);
    let view = texture.create_view(&Default::default());
    renderer
        .draw(gpu, &mut text_cache(), scene, &view, params())
        .expect("draw");
    dump::read_rgba(&gpu.device, &gpu.queue, &texture).expect("readback")
}

fn render(gpu: &mut Gpu, scene: &Scene) -> Vec<u8> {
    let mut renderer = FrameRenderer::new(gpu, wgpu::TextureFormat::Rgba8Unorm);
    draw(gpu, &mut renderer, scene)
}

fn scene_of(f: impl FnOnce(&mut Painter<'_, u8>)) -> Scene {
    let mut text = FakeText;
    let mut p: Painter<'_, u8> = Painter::new(1.0, WIN, &mut text, None, DARK);
    p.fill(WIN, 0.0, DARKEST);
    f(&mut p);
    p.scene
}

/// White bars 40 px wide every 80 px across the window, on `layer`.
fn bars(p: &mut Painter<'_, u8>, layer: LayerId) {
    p.on(layer);
    for i in 0..6 {
        p.fill(
            Rect::new(i as f32 * 80.0 + 20.0, 0.0, 40.0, SIZE.1 as f32),
            0.0,
            WHITE,
        );
    }
}

/// The green channel along `ROW` from `x` to `to`, as a luminance stand-in on grey bars.
fn row(img: &[u8], x: usize, to: usize) -> Vec<f32> {
    (x..to)
        .map(|x| f32::from(img[(ROW * SIZE.0 as usize + x) * 4 + 1]))
        .collect()
}

fn range(v: &[f32]) -> f32 {
    let max = v.iter().copied().fold(f32::MIN, f32::max);
    let min = v.iter().copied().fold(f32::MAX, f32::min);
    max - min
}

/// The steepest step between neighbours: a softer edge has a smaller one.
fn steepest(v: &[f32]) -> f32 {
    let steps = v.windows(2).map(|w| (w[1] - w[0]).abs());
    steps.fold(0.0, f32::max)
}

fn px(img: &[u8], x: usize) -> [u8; 4] {
    let i = (ROW * SIZE.0 as usize + x) * 4;
    [img[i], img[i + 1], img[i + 2], img[i + 3]]
}

#[test]
fn bars_under_a_request_do_not_read_through_its_fill() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let panel = Rect::new(100.0, 40.0, 280.0, 80.0);
    // Sigma wide against the 80 px period, as a glass over text is against its strokes.
    let sharp = render(&mut gpu, &scene_of(|p| bars(p, LayerId::Modal)));
    let sharp_range = range(&row(&sharp, 110, 370));
    assert!(sharp_range > 200.0, "the bars show (instrument check)");
    let glass = render(
        &mut gpu,
        &scene_of(|p| {
            bars(p, LayerId::Modal);
            p.on(LayerId::Raised);
            p.backdrop_with(panel, 0.0, 32.0, 1.0);
            p.fill(panel, 0.0, VEIL);
        }),
    );
    let inside = range(&row(&glass, 110, 370));
    assert!(
        inside < sharp_range / 8.0,
        "range {inside} of sharp {sharp_range}"
    );
    assert_eq!(row(&glass, 0, 100), row(&sharp, 0, 100), "only the region");
}

const FIRST: Rect = Rect::new(40.0, 20.0, 400.0, 120.0);
const SECOND: Rect = Rect::new(180.0, 50.0, 120.0, 60.0);
/// A sharp dark stripe on the first surface, inside the second region.
const STRIPE: Rect = Rect::new(232.0, 20.0, 16.0, 120.0);

/// The first surface: its request, its green fill and the stripe.
fn first_surface(p: &mut Painter<'_, impl Copy + PartialEq>) {
    p.backdrop(FIRST, 0.0);
    p.fill(FIRST, 0.0, GREEN);
    p.fill(STRIPE, 0.0, DARKEST);
}

/// In the second region the first surface shows blurred: its green, the stripe a soft dip, not
/// the page's dark and not the stripe's sharp edge. Around it the first surface and the page are
/// as drawn.
fn assert_stacked(img: &[u8]) {
    let inside = row(img, 190, 290);
    let (low, high) = (
        inside.iter().copied().fold(f32::MAX, f32::min),
        inside.iter().copied().fold(f32::MIN, f32::max),
    );
    assert!(
        high > 180.0,
        "the first surface's green, not the page: {high}"
    );
    assert!(low > 80.0, "the stripe is blurred away: {low}");
    assert!(
        steepest(&inside) < 40.0,
        "no sharp edge: {}",
        steepest(&inside)
    );
    assert_eq!(px(img, 100), GREEN, "the first surface around it");
    assert_eq!(px(img, 10), DARKEST, "the page around that");
}

#[test]
fn a_second_request_shows_the_first_surface_blurred_not_the_page() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    // Instrument check: with no second request the stripe is sharp.
    let alone = scene_of(|p| {
        p.on(LayerId::Modal);
        first_surface(p);
    });
    let alone = render(&mut gpu, &alone);
    assert_eq!(px(&alone, 240), DARKEST, "the stripe");
    assert!(steepest(&row(&alone, 190, 290)) > 150.0);
    let scene = scene_of(|p| {
        p.on(LayerId::Modal);
        first_surface(p);
        p.on(LayerId::Raised);
        p.backdrop(SECOND, 0.0);
    });
    assert_stacked(&render(&mut gpu, &scene));
}

#[test]
fn glass_stacks_through_overlays_on_one_layer_too() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let options = OverlayOptions {
        layer: LayerId::Raised,
        modal: false,
        dismiss_on_escape: false,
        dismiss_on_outside_press: false,
        dismiss_on_focus_loss: false,
        anchor: None,
        initial_focus: None,
        open: Motion::of(Kind::Appear),
        close: Motion::of(Kind::Dismiss),
    };
    let mut h = UiHarness::new((SIZE.0 as f32, SIZE.1 as f32));
    let (scene, _) = h.frame(|ui| {
        ui.painter().fill(WIN, 0.0, DARKEST);
        ui.overlay(
            "a",
            &options,
            |_, _, _| {},
            |ui, _| first_surface(ui.painter()),
        );
        ui.overlay(
            "b",
            &options,
            |_, _, _| {},
            |ui, _| ui.painter().backdrop(SECOND, 0.0),
        );
    });
    assert_eq!(scene.backdrops().count(), 2);
    assert_stacked(&render(&mut gpu, &scene));
}

#[test]
fn two_requests_on_two_layers_each_have_their_own_sigma() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let narrow = Rect::new(0.0, 40.0, 240.0, 80.0);
    let wide = Rect::new(240.0, 40.0, 240.0, 80.0);
    let scene = scene_of(|p| {
        bars(p, LayerId::Page);
        p.on(LayerId::Modal);
        p.backdrop_with(narrow, 0.0, 6.0, 1.0);
        p.on(LayerId::Raised);
        p.backdrop_with(wide, 0.0, 24.0, 1.0);
    });
    let img = render(&mut gpu, &scene);
    // The same two bars' worth of the pattern on each side, 240 being three periods.
    let (soft, softer) = (row(&img, 20, 180), row(&img, 260, 420));
    assert!(
        steepest(&softer) * 1.5 < steepest(&soft),
        "the wider sigma is softer: {} vs {}",
        steepest(&softer),
        steepest(&soft)
    );
}

#[test]
fn two_rects_on_one_layer_are_both_frosted_and_share_nothing_but_the_picture() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let left = Rect::new(20.0, 40.0, 200.0, 80.0);
    let right = Rect::new(260.0, 40.0, 200.0, 80.0);
    let scene = |right_sigma: f32| {
        scene_of(|p| {
            bars(p, LayerId::Modal);
            p.on(LayerId::Raised);
            p.backdrop_with(left, 0.0, 32.0, 1.0);
            p.backdrop_with(right, 0.0, right_sigma, 1.0);
        })
    };
    let img = render(&mut gpu, &scene(32.0));
    let sharp = render(&mut gpu, &scene_of(|p| bars(p, LayerId::Modal)));
    let bound = range(&row(&sharp, 30, 210)) / 8.0;
    assert!(range(&row(&img, 30, 210)) < bound, "left frosted");
    assert!(range(&row(&img, 270, 450)) < bound, "right frosted");
    // Two sigmas on one layer: the left keeps its own, and the right is sharper.
    let mixed = render(&mut gpu, &scene(6.0));
    assert_eq!(
        row(&mixed, 30, 210),
        row(&img, 30, 210),
        "the left is unmoved"
    );
    assert!(steepest(&row(&mixed, 270, 450)) > steepest(&row(&img, 270, 450)));
}

#[test]
fn one_renderer_grows_from_one_stop_to_three_and_back() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let rect = |i: f32| Rect::new(20.0 + i * 150.0, 30.0, 130.0, 100.0);
    let one = scene_of(|p| {
        bars(p, LayerId::Page);
        p.on(LayerId::Modal);
        p.backdrop(rect(0.0), 8.0);
    });
    let three = scene_of(|p| {
        bars(p, LayerId::Page);
        for (i, layer) in [LayerId::Popup, LayerId::Modal, LayerId::Raised]
            .into_iter()
            .enumerate()
        {
            p.on(layer);
            p.backdrop_with(rect(i as f32), 8.0, 6.0 + 8.0 * i as f32, 1.0);
        }
    });
    let want_one = render(&mut gpu, &one);
    let want_three = render(&mut gpu, &three);
    let mut renderer = FrameRenderer::new(&mut gpu, wgpu::TextureFormat::Rgba8Unorm);
    assert!(draw(&mut gpu, &mut renderer, &one) == want_one);
    assert!(draw(&mut gpu, &mut renderer, &three) == want_three);
    assert!(draw(&mut gpu, &mut renderer, &one) == want_one);
}

/// The lowest alpha of any pixel of `img`.
fn lowest_alpha(img: &[u8]) -> u8 {
    img.chunks_exact(4).map(|p| p[3]).min().unwrap_or(0)
}

const ABOVE_BASE: [LayerId; 5] = [
    LayerId::Page,
    LayerId::Popup,
    LayerId::Modal,
    LayerId::Raised,
    LayerId::Toast,
];

#[test]
fn no_draw_above_the_background_lowers_the_window_alpha() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let panel = Rect::new(100.0, 40.0, 280.0, 80.0);
    let glass = premul(DARK.menu_glass);
    let rows: Vec<(String, Scene)> = ABOVE_BASE
        .into_iter()
        .map(|layer| {
            let scene = scene_of(|p| {
                p.on(layer);
                p.acrylic(panel, 8.0, glass, [0.0; 4], true);
            });
            (format!("acrylic on {layer:?}"), scene)
        })
        .chain([
            (
                "frost".to_owned(),
                scene_of(|p| {
                    p.on(LayerId::Modal);
                    p.frost(panel, 8.0, glass, true);
                }),
            ),
            (
                "a backdrop at alpha 1".to_owned(),
                scene_of(|p| {
                    bars(p, LayerId::Page);
                    p.on(LayerId::Modal);
                    p.backdrop(panel, 8.0);
                }),
            ),
            (
                "a backdrop at alpha 0.5".to_owned(),
                scene_of(|p| {
                    bars(p, LayerId::Page);
                    p.on(LayerId::Modal);
                    p.push_xform(half());
                    p.backdrop(panel, 8.0);
                    p.pop_xform();
                }),
            ),
        ])
        .collect();
    for (name, scene) in rows {
        let low = lowest_alpha(&render(&mut gpu, &scene));
        assert!(low >= 254, "{name}: lowest alpha {low}");
    }
}

/// A transform at alpha 0.5.
fn half() -> Xform {
    Xform {
        origin: (0.0, 0.0),
        scale: 1.0,
        dx: 0.0,
        dy: 0.0,
        alpha: 0.5,
    }
}

/// A half-transparent window with a bright bar in it, and a request over the whole of it.
fn translucent(alpha: Option<Xform>) -> Scene {
    let mut text = FakeText;
    let mut p: Painter<'_, u8> = Painter::new(1.0, WIN, &mut text, None, DARK);
    p.fill(WIN, 0.0, [20, 20, 20, 128]);
    p.fill(Rect::new(220.0, 0.0, 40.0, SIZE.1 as f32), 0.0, WHITE);
    if let Some(x) = alpha {
        p.on(LayerId::Modal);
        p.set_xform(Some(x));
        p.backdrop(WIN, 0.0);
        p.set_xform(None);
    }
    p.scene
}

/// The alpha channel along `ROW`.
fn alphas(img: &[u8]) -> Vec<f32> {
    let row = img[ROW * SIZE.0 as usize * 4..]
        .chunks_exact(4)
        .take(SIZE.0 as usize);
    row.map(|p| f32::from(p[3])).collect()
}

#[test]
fn a_full_request_in_a_translucent_window_leaves_no_sharp_residue() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let full = Xform {
        alpha: 1.0,
        ..half()
    };
    let sharp = render(&mut gpu, &translucent(None));
    let img = render(&mut gpu, &translucent(Some(full)));
    assert!(steepest(&row(&sharp, 200, 280)) > 150.0, "the sharp edge");
    assert!(steepest(&row(&img, 200, 280)) < 20.0, "no residue of it");
    assert!(steepest(&alphas(&img)) < 20.0, "in the alpha either");
    // Far from the bar the region is the window as it was, not the blur over the window.
    let far = px(&img, 20);
    assert!(
        (i32::from(far[3]) - 128).abs() <= 2,
        "alpha {} stays the window's",
        far[3]
    );
}

#[test]
fn a_request_at_half_alpha_is_the_midpoint_of_sharp_and_blurred() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let full = Xform {
        alpha: 1.0,
        ..half()
    };
    let sharp = render(&mut gpu, &translucent(None));
    let blurred = render(&mut gpu, &translucent(Some(full)));
    let mid = render(&mut gpu, &translucent(Some(half())));
    for (i, ((s, b), m)) in sharp.iter().zip(&blurred).zip(&mid).enumerate() {
        let want = (f32::from(*s) + f32::from(*b)) / 2.0;
        assert!(
            (f32::from(*m) - want).abs() <= 2.0,
            "byte {i}: {m} vs ({s} + {b}) / 2"
        );
    }
}

#[test]
fn a_frost_at_half_alpha_is_half_the_frost_noise_included() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let frost = |alpha: f32| {
        let mut text = FakeText;
        let mut p: Painter<'_, u8> = Painter::new(1.0, WIN, &mut text, None, DARK);
        p.set_xform(Some(Xform { alpha, ..half() }));
        p.frost(WIN, 0.0, premul(DARK.menu_glass), true);
        p.scene
    };
    let whole = render(&mut gpu, &frost(1.0));
    let halved = render(&mut gpu, &frost(0.5));
    assert!(
        range(&row(&whole, 0, 480)) > 0.0,
        "the noise shows (instrument check)"
    );
    for (i, (w, h)) in whole.iter().zip(&halved).enumerate() {
        let want = f32::from(*w) / 2.0;
        assert!((f32::from(*h) - want).abs() <= 1.0, "byte {i}: {h} of {w}");
    }
}
