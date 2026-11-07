//! Faded acrylic, on the window's `Base` layer where acrylic lives: a card over an opaque acrylic
//! background blends in as `opacity·card + (1 − opacity)·base` (opacity 1 is the plain replace, 0
//! leaves the base). Above `Base` an acrylic call is a frost, which blends over what is below.
//!
//! GPU test: with no adapter it prints `skipped: no GPU adapter` and passes, unless
//! `LUNEUI_REQUIRE_GPU=1`, which makes a missing adapter a failure.

use lune_ui_core::color::premul;
use lune_ui_core::paint::{LayerId, Painter, Rect, Xform};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::FakeText;
use lune_ui_core::theme::DARK;
use lune_ui_wgpu::dump;
use lune_ui_wgpu::frame::{FrameParams, FrameRenderer};
use lune_ui_wgpu::gpu::{self, Gpu};
use lune_ui_wgpu::text_cache::TextCache;

const SIZE: (u32, u32) = (200, 120);
const BASE: [u8; 4] = [200, 60, 40, 255];

fn fonts() -> [&'static [u8]; 3] {
    [
        include_bytes!("../../../apps/gallery/assets/RobotoMonoNerdFontMono-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Medium.ttf"),
    ]
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

/// The card rects, left to right, 50 px wide with a gap.
fn card(i: usize) -> Rect {
    Rect::new(10.0 + i as f32 * 62.0, 20.0, 50.0, 80.0)
}

fn centre(i: usize) -> (usize, usize) {
    let c = card(i);
    ((c.x + c.w / 2.0) as usize, (c.y + c.h / 2.0) as usize)
}

/// A Base acrylic background and one acrylic card per entry of `alphas` (each under a transform
/// with that alpha, or none for `None`).
fn scene(alphas: &[Option<f32>]) -> Scene {
    let mut text = FakeText;
    let win = Rect::new(0.0, 0.0, SIZE.0 as f32, SIZE.1 as f32);
    let mut p: Painter<'_, u8> = Painter::new(1.0, win, &mut text, None, DARK);
    p.acrylic(win, 0.0, premul(BASE), [0.0; 4], false);
    for (i, a) in alphas.iter().enumerate() {
        p.set_xform(a.map(|alpha| Xform {
            origin: (0.0, 0.0),
            scale: 1.0,
            dx: 0.0,
            dy: 0.0,
            alpha,
        }));
        p.acrylic(card(i), 4.0, premul(DARK.menu_glass), [0.0; 4], false);
    }
    p.scene
}

fn render(gpu: &mut Gpu, scene: &Scene) -> Vec<u8> {
    render_with(gpu, scene, FrameParams::default())
}

/// `scene` drawn with `over`'s frame values; the size, scale and blur are the test's own.
fn render_with(gpu: &mut Gpu, scene: &Scene, over: FrameParams) -> Vec<u8> {
    let mut text = TextCache::new(&fonts(), "RznSans", "RobotoMono Nerd Font Mono");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = FrameRenderer::new(gpu, format);
    let texture = dump::target(&gpu.device, format, SIZE);
    let view = texture.create_view(&Default::default());
    let params = FrameParams {
        size: SIZE,
        scale: 1.0,
        blur_sigma: 4.0,
        saturation: 1.0,
        noise_opacity: over.noise_opacity,
        blur: over.blur,
    };
    renderer
        .draw(gpu, &mut text, scene, &view, params)
        .expect("draw");
    dump::read_rgba(&gpu.device, &gpu.queue, &texture).expect("readback")
}

fn px(img: &[u8], (x, y): (usize, usize)) -> [u8; 4] {
    let i = (y * SIZE.0 as usize + x) * 4;
    [img[i], img[i + 1], img[i + 2], img[i + 3]]
}

fn near(what: &str, got: [u8; 4], want: [f32; 4]) {
    for c in 0..4 {
        assert!(
            (f32::from(got[c]) - want[c]).abs() <= 1.0,
            "{what}: channel {c}: got {got:?}, want about {want:?}"
        );
    }
}

#[test]
fn fade_blends_between_the_base_and_the_card() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    // Opacity 1 goes through the unmodified replace path (no transform at all).
    let one = render(&mut gpu, &scene(&[None]));
    let card_px = px(&one, centre(0));
    let base_px = px(&one, (2, 2));
    assert_eq!(base_px, BASE, "base pixel (instrument check)");
    let premul_card = premul(DARK.menu_glass).map(|v| v * 255.0);
    near("opacity 1 is the replace result", card_px, premul_card);
    assert_ne!(
        card_px, base_px,
        "card must differ from base (instrument check)"
    );

    // Opacity 0: a hand-built record through the fade pipeline leaves the base.
    let mut zero = scene(&[None]);
    zero.layers[LayerId::Base as usize].geometry.acrylic[1].opacity = 0.0;
    let zero_img = render(&mut gpu, &zero);
    assert_eq!(px(&zero_img, centre(0)), base_px, "opacity 0 = base");

    // Opacity 0.5 from the painter's transform alpha, and the same record built by hand.
    let half = render(&mut gpu, &scene(&[Some(0.5)]));
    let want = [0, 1, 2, 3].map(|c| (f32::from(card_px[c]) + f32::from(base_px[c])) / 2.0);
    near("opacity 0.5 painter", px(&half, centre(0)), want);
    let mut by_hand = scene(&[None]);
    by_hand.layers[LayerId::Base as usize].geometry.acrylic[1].opacity = 0.5;
    assert_eq!(
        render(&mut gpu, &by_hand),
        half,
        "hand-built record = painter record"
    );
}

#[test]
fn draw_order_survives_mixed_opacities() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let img = render(&mut gpu, &scene(&[None, Some(0.5), None]));
    let solo = render(&mut gpu, &scene(&[None]));
    let card_px = px(&solo, centre(0));
    let base_px = px(&solo, (2, 2));
    assert_eq!(px(&img, centre(0)), card_px);
    assert_eq!(px(&img, centre(2)), card_px);
    let want = [0, 1, 2, 3].map(|c| (f32::from(card_px[c]) + f32::from(base_px[c])) / 2.0);
    near("middle card", px(&img, centre(1)), want);
}

#[test]
fn an_acrylic_call_above_base_blends_over_it_and_leaves_the_base_visible() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = FakeText;
    let win = Rect::new(0.0, 0.0, SIZE.0 as f32, SIZE.1 as f32);
    let mut p: Painter<'_, u8> = Painter::new(1.0, win, &mut text, None, DARK);
    p.fill(win, 0.0, BASE);
    p.on(LayerId::Modal);
    let glass = premul(DARK.menu_glass);
    p.acrylic(card(0), 4.0, glass, [0.0; 4], false);
    let img = render(&mut gpu, &p.scene);
    let over = [0, 1, 2, 3].map(|c| glass[c] * 255.0 + f32::from(BASE[c]) * (1.0 - glass[3]));
    near("the card over the base", px(&img, centre(0)), over);
    assert_eq!(px(&img, centre(0))[3], 255, "no hole");
    assert_eq!(px(&img, (2, 2)), BASE);
}

/// A Base acrylic fill of one colour, with the noise flag as given.
fn flat(noise: bool) -> Scene {
    let mut text = FakeText;
    let win = Rect::new(0.0, 0.0, SIZE.0 as f32, SIZE.1 as f32);
    let mut p: Painter<'_, u8> = Painter::new(1.0, win, &mut text, None, DARK);
    p.acrylic(win, 0.0, premul(BASE), [0.0; 4], noise);
    p.scene
}

fn with_noise(noise_opacity: f32) -> FrameParams {
    FrameParams {
        noise_opacity,
        ..FrameParams::default()
    }
}

/// The mean distance of the colour channels from the flat colour, over the whole target.
fn deviation(img: &[u8]) -> f32 {
    let sum: f32 = img
        .chunks_exact(4)
        .map(|p| {
            (0..3)
                .map(|c| (f32::from(p[c]) - f32::from(BASE[c])).abs())
                .sum::<f32>()
        })
        .sum();
    sum / (img.len() / 4) as f32 / 3.0
}

#[test]
fn noise_zero_equals_the_flag_off_and_doubled_noise_doubles_the_deviation() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let off = render(&mut gpu, &flat(false));
    assert_eq!(deviation(&off), 0.0, "no noise leaves the flat colour");
    let on = render(&mut gpu, &flat(true));
    let d1 = deviation(&on);
    assert!(d1 > 0.1, "the default noise shows (instrument check): {d1}");
    let zero = render_with(&mut gpu, &flat(true), with_noise(0.0));
    assert_eq!(zero, off, "noise 0 is the flag off");
    // Doubling the strength doubles what the noise adds, to within the 8-bit rounding.
    let default = FrameParams::default().noise_opacity;
    let doubled = render_with(&mut gpu, &flat(true), with_noise(default * 2.0));
    let d2 = deviation(&doubled);
    assert!(
        (d2 / d1 - 2.0).abs() < 0.2,
        "deviation {d1} then {d2} when doubled"
    );
}

#[test]
fn a_noise_that_is_not_a_strength_draws_as_the_default() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let default = render(&mut gpu, &flat(true));
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.5] {
        let img = render_with(&mut gpu, &flat(true), with_noise(bad));
        assert_eq!(img, default, "noise {bad}");
    }
}
