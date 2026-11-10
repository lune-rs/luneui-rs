//! The text rendering setting on the GPU: a runtime toggle never serves a glyph or a rotated-text
//! raster made under another setting, an app draw sees the effective setting, the rotated-text loop
//! agrees with `Buffer::draw`, the glyph atlas re-creates each glyph under its own setting when it
//! grows, a scene without text is the same at every setting, and rasters of settings no longer
//! drawn are dropped.
//!
//! GPU test: with no adapter it prints `skipped: no GPU adapter` and passes, unless
//! `LUNEUI_REQUIRE_GPU=1`, which makes a missing adapter a failure.

use lune_ui_core::image::{Fit, ImageAlign, ImageData, ImageId, ImageStore};
use lune_ui_core::paint::{Painter, Rect, StrokeStyle};
use lune_ui_core::scene::Scene;
use lune_ui_core::text::{Spec, StemDarkening, Style, TextRendering, TextSystem};
use lune_ui_core::theme::DARK;
use lune_ui_wgpu::custom::{CustomDraw, CustomFrame};
use lune_ui_wgpu::dump;
use lune_ui_wgpu::frame::{FrameParams, FrameRenderer};
use lune_ui_wgpu::gpu::{self, Gpu};
use lune_ui_wgpu::raster::GlyphRasterizer;
use lune_ui_wgpu::text_cache::TextCache;

const SCALE: f32 = 2.0;
/// Device px.
const SIZE: (u32, u32) = (400, 300);
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const WHITE: [u8; 4] = [255, 255, 255, 255];

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

/// One target: a renderer and its texture.
struct Target {
    renderer: FrameRenderer,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Target {
    fn new(gpu: &mut Gpu) -> Self {
        let renderer = FrameRenderer::new(gpu, FORMAT);
        let texture = dump::target(&gpu.device, FORMAT, SIZE);
        let view = texture.create_view(&Default::default());
        Self {
            renderer,
            texture,
            view,
        }
    }

    /// Draws `scene` at `rendering` and reads the target back. No `end_frame`.
    fn draw(
        &mut self,
        gpu: &mut Gpu,
        text: &mut TextCache,
        scene: &Scene,
        rendering: TextRendering,
    ) -> Vec<u8> {
        let params = FrameParams {
            size: SIZE,
            scale: SCALE,
            blur_sigma: 4.0,
            saturation: 1.0,
            text: rendering,
            ..FrameParams::default()
        };
        self.renderer
            .draw(gpu, text, scene, &self.view, params)
            .expect("draw");
        dump::read_rgba(&gpu.device, &gpu.queue, &self.texture).expect("readback")
    }
}

/// `scene` drawn once at `rendering` on a fresh device, text cache and renderer.
fn fresh(scene: impl FnOnce(&mut TextCache) -> Scene, rendering: TextRendering) -> Vec<u8> {
    let mut gpu = headless_gpu().expect("a device, as the caller's");
    let mut text = text_cache();
    let scene = scene(&mut text);
    Target::new(&mut gpu).draw(&mut gpu, &mut text, &scene, rendering)
}

fn small_text() -> Spec {
    Spec::plain("Hamburgefonstiv", Style::ui(13.0, WHITE))
}

fn rotated_spec() -> Spec {
    Spec::plain("Spinning words", Style::ui(16.0, WHITE))
}

/// 13 px text at scale 2 and a rotated text.
fn mixed(text: &mut TextCache) -> Scene {
    paint(text, |p| {
        p.text(&small_text(), 8.0, 8.0);
        p.text_rotated(&rotated_spec(), (100.0, 100.0), 0.5);
    })
}

/// 13 px text at scale 2 and nothing else (S_small).
fn s_small(text: &mut TextCache) -> Scene {
    paint(text, |p| {
        p.text(&small_text(), 8.0, 8.0);
    })
}

/// The 62 glyphs `A-Za-z0-9` at 96 px at scale 2, in rows of 8 (S_big).
fn s_big(text: &mut TextCache) -> Scene {
    let all: Vec<char> = ('A'..='Z').chain('a'..='z').chain('0'..='9').collect();
    paint(text, |p| {
        for (row, chunk) in all.chunks(8).enumerate() {
            let s: String = chunk.iter().collect();
            p.text(
                &Spec::plain(&s, Style::ui(96.0, WHITE)),
                0.0,
                row as f32 * 8.0,
            );
        }
    })
}

/// A setting that is valid and unlike `TextRendering::SMOOTH`.
fn darker(i: u32) -> TextRendering {
    TextRendering {
        stem_darkening: StemDarkening {
            factors: [0.010 + i as f32 * 0.0005, 0.0121 * 1.25],
            ..StemDarkening::MACOS
        },
        ..TextRendering::SMOOTH
    }
}

fn total_alpha(rgba: &[u8]) -> u64 {
    rgba.chunks_exact(4).map(|p| u64::from(p[3])).sum()
}

#[test]
fn a_toggle_never_serves_a_cached_glyph_or_raster_of_the_other_setting() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let mut target = Target::new(&mut gpu);
    let scene = mixed(&mut text);
    let mut reads = Vec::new();
    for rendering in [
        TextRendering::OFF,
        TextRendering::SMOOTH,
        TextRendering::OFF,
    ] {
        let got = target.draw(&mut gpu, &mut text, &scene, rendering);
        gpu.end_frame(&mut text);
        assert_eq!(
            gpu.text_image_count(),
            1,
            "one raster per text, after {rendering:?}"
        );
        let want = fresh(mixed, rendering);
        assert!(
            got == want,
            "{rendering:?} differs from a fresh device's draw"
        );
        reads.push(got);
    }
    assert!(
        reads[0] != reads[1],
        "instrument check: OFF and SMOOTH draw differently"
    );
    assert!(
        total_alpha(&reads[0]) > 0,
        "instrument check: text is drawn"
    );
}

/// Records the frame it is prepared with.
#[derive(Default)]
struct Recorder {
    seen: Vec<CustomFrame>,
}

impl CustomDraw for Recorder {
    fn prepare(&mut self, _: &wgpu::Device, _: &wgpu::Queue, frame: &CustomFrame) {
        self.seen.push(*frame);
    }

    fn render(&self, _: &mut wgpu::RenderPass<'_>, _: &CustomFrame) {}
}

#[test]
fn a_custom_draw_gets_the_effective_setting() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let mut target = Target::new(&mut gpu);
    let scene = paint(&mut text, |_| {});
    let mut draw = |rendering: TextRendering| {
        let mut rec = Recorder::default();
        let params = FrameParams {
            size: SIZE,
            scale: SCALE,
            text: rendering,
            ..FrameParams::default()
        };
        target
            .renderer
            .draw_with(
                &mut gpu,
                &mut text,
                &scene,
                &target.view,
                params,
                &mut [&mut rec],
            )
            .expect("draw");
        assert_eq!(rec.seen.len(), 1);
        rec.seen[0].text
    };
    assert_eq!(draw(TextRendering::SMOOTH), TextRendering::SMOOTH);
    assert_eq!(draw(TextRendering::OFF), TextRendering::OFF);
    let bad = TextRendering {
        stem_darkening: StemDarkening {
            factors: [f32::NAN, 0.02],
            ..StemDarkening::MACOS
        },
        ..TextRendering::SMOOTH
    };
    let seen = draw(bad);
    assert_eq!(seen, bad.effective());
    assert_eq!(
        seen.stem_darkening.factors,
        [0.0121, 0.02],
        "the NaN fell to the default"
    );
}

#[test]
fn the_rotated_loop_agrees_with_buffer_draw_and_smooth_text_is_heavier() {
    let mut text = text_cache();
    let (key, _) = text.prepare(&rotated_spec(), SCALE);
    let mut glyphs = GlyphRasterizer::new();
    // Every glyph gets `GlyphParams::OFF` through the per-glyph loop.
    let zero = TextRendering {
        smoothing: true,
        hinting: true,
        stem_darkening: StemDarkening {
            factors: [0.0, 0.0],
            max_px: [0.0, 0.0],
            max_ppem: 0.0,
        },
    };
    assert_ne!(
        zero.effective(),
        TextRendering::OFF,
        "instrument check: the loop runs"
    );
    let off = text
        .rasterize(key, &mut glyphs, TextRendering::OFF)
        .expect("rasterizes");
    let looped = text.rasterize(key, &mut glyphs, zero).expect("rasterizes");
    assert!(total_alpha(&off.rgba) > 0, "instrument check: ink drawn");
    assert!(
        looped == off,
        "the per-glyph loop is byte for byte `Buffer::draw`"
    );
    let smooth = text
        .rasterize(key, &mut glyphs, TextRendering::SMOOTH)
        .expect("rasterizes");
    assert_eq!((smooth.width, smooth.height), (off.width, off.height));
    assert!(smooth != off, "smooth differs from off");
    assert!(
        total_alpha(&smooth.rgba) > total_alpha(&off.rgba),
        "smooth {} vs off {}",
        total_alpha(&smooth.rgba),
        total_alpha(&off.rgba)
    );
}

#[test]
fn the_atlas_grows_under_one_setting_and_keeps_the_glyphs_of_another() {
    if headless_gpu().is_none() {
        return;
    }
    let off = TextRendering::OFF;
    let smooth = TextRendering::SMOOTH;
    // (a) control, (b) A at OFF with B at SMOOTH, (c) A at SMOOTH with B at OFF. Each case has a
    // `Gpu` of its own (an atlas never shrinks) and no `end_frame` between its draws, so the glyphs
    // of A are in use when B's draw grows the atlas.
    for (name, a, b) in [("a", off, off), ("b", off, smooth), ("c", smooth, off)] {
        let mut gpu = headless_gpu().expect("a device");
        let mut text = text_cache();
        let (mut ta, mut tb) = (Target::new(&mut gpu), Target::new(&mut gpu));
        let (small, big) = (s_small(&mut text), s_big(&mut text));
        let first = ta.draw(&mut gpu, &mut text, &small, a);
        let before = gpu.glyph_atlas_size();
        tb.draw(&mut gpu, &mut text, &big, b);
        let after = gpu.glyph_atlas_size();
        assert!(
            after > before,
            "case {name}: atlas did not grow; enlarge S_big ({before} -> {after})"
        );
        let again = ta.draw(&mut gpu, &mut text, &small, a);
        let want = fresh(s_small, a);
        assert!(
            first == want,
            "case {name}: A's first draw is a fresh device's"
        );
        assert!(
            again == want,
            "case {name}: A's draw after the atlas grew differs from a fresh device's"
        );
    }
}

#[test]
fn a_scene_without_text_reads_the_same_at_every_setting() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    gpu.register(
        ImageId(1),
        ImageData {
            width: 2,
            height: 2,
            rgba: vec![
                255, 0, 0, 255, 0, 255, 0, 255, //
                0, 0, 255, 255, 255, 255, 255, 255,
            ],
        },
    )
    .expect("registers");
    let scene = paint(&mut text, |p| {
        p.fill(Rect::new(0.0, 0.0, 200.0, 150.0), 0.0, [30, 40, 60, 255]);
        p.fill(Rect::new(10.0, 10.0, 60.0, 40.0), 6.0, [200, 90, 30, 255]);
        p.stroke(
            &[[20.0, 100.0], [80.0, 60.0], [140.0, 110.0]],
            false,
            StrokeStyle::solid([255, 255, 255, 255], 2.0),
        );
        p.frost(
            Rect::new(90.0, 10.0, 80.0, 50.0),
            8.0,
            [0.1, 0.1, 0.1, 0.6],
            true,
        );
        p.acrylic(
            Rect::new(10.0, 60.0, 60.0, 30.0),
            4.0,
            [0.1, 0.1, 0.2, 0.6],
            [0.0; 4],
            true,
        );
        p.image(
            Rect::new(150.0, 100.0, 16.0, 16.0),
            ImageId(1),
            Fit::Contain,
            ImageAlign::CENTER,
            1.0,
        );
    });
    let mut target = Target::new(&mut gpu);
    let off = target.draw(&mut gpu, &mut text, &scene, TextRendering::OFF);
    let smooth = target.draw(&mut gpu, &mut text, &scene, TextRendering::SMOOTH);
    assert!(total_alpha(&off) > 0, "instrument check: the scene draws");
    assert!(off == smooth, "no text: OFF and SMOOTH read the same");
}

#[test]
fn rasters_of_settings_no_longer_drawn_are_dropped() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let scene = paint(&mut text, |p| {
        p.text_rotated(&rotated_spec(), (100.0, 100.0), 0.5);
    });
    // (i) two targets at two settings in one app frame keep both; the next frame at one keeps one.
    let (mut ta, mut tb) = (Target::new(&mut gpu), Target::new(&mut gpu));
    ta.draw(&mut gpu, &mut text, &scene, TextRendering::OFF);
    tb.draw(&mut gpu, &mut text, &scene, TextRendering::SMOOTH);
    gpu.end_frame(&mut text);
    assert_eq!(
        gpu.text_image_count(),
        2,
        "both settings of the frame are kept"
    );
    tb.draw(&mut gpu, &mut text, &scene, TextRendering::SMOOTH);
    gpu.end_frame(&mut text);
    assert_eq!(
        gpu.text_image_count(),
        1,
        "only the setting still drawn is kept"
    );

    // (ii) a setting that changes every frame holds one raster.
    for i in 0..20 {
        ta.draw(&mut gpu, &mut text, &scene, darker(i));
        gpu.end_frame(&mut text);
        assert_eq!(gpu.text_image_count(), 1, "frame {i}");
    }
}
