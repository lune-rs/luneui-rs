//! One `Gpu`, many targets: two renderers on one device, sharing one text cache across two scales,
//! draw the same pixels as each alone, in either order, and a second colour format on the same
//! device matches a fresh device.
//!
//! GPU test: with no adapter it prints `skipped: no GPU adapter` and passes, unless
//! `LUNEUI_REQUIRE_GPU=1`, which makes a missing adapter a failure.

use lune_ui_core::color::premul;
use lune_ui_core::paint::{LayerId, Painter, Rect, StrokeStyle, Xform};
use lune_ui_core::scene::Scene;
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_wgpu::dump;
use lune_ui_wgpu::frame::{FrameParams, FrameRenderer};
use lune_ui_wgpu::gpu::{self, Gpu};
use lune_ui_wgpu::text_cache::TextCache;

const UI_FONT: &str = "RznSans";
const MONO_FONT: &str = "RobotoMono Nerd Font Mono";

fn fonts() -> [&'static [u8]; 3] {
    [
        include_bytes!("../../../apps/gallery/assets/RobotoMonoNerdFontMono-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Medium.ttf"),
    ]
}

/// A headless device, or `None` (after printing why) when there is no adapter and the run does
/// not require one.
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

/// A test scene: its scale and logical size, and whether its text is the large-glyph set.
#[derive(Clone, Copy)]
struct SceneSpec {
    scale: f32,
    w: f32,
    h: f32,
    many_glyphs: bool,
}

const A: SceneSpec = SceneSpec {
    scale: 2.0,
    w: 400.0,
    h: 240.0,
    many_glyphs: false,
};
const B: SceneSpec = SceneSpec {
    scale: 1.0,
    w: 320.0,
    h: 200.0,
    many_glyphs: true,
};

impl SceneSpec {
    fn size(&self) -> (u32, u32) {
        ((self.w * self.scale) as u32, (self.h * self.scale) as u32)
    }

    fn params(&self) -> FrameParams {
        FrameParams {
            size: self.size(),
            scale: self.scale,
            blur_sigma: 12.0,
            saturation: 1.8,
            ..FrameParams::default()
        }
    }
}

/// 62 distinct glyphs (letters and digits), in rows of 8, at 48 px.
fn glyph_rows() -> Vec<String> {
    let chars: Vec<char> = ('A'..='Z').chain('a'..='z').chain('0'..='9').collect();
    chars.chunks(8).map(|c| c.iter().collect()).collect()
}

/// Paints a Base fill, a Page acrylic with noise, a ring above text, a stroke, text on Base and
/// on Popup, a Popup backdrop request, and a popup under an `Xform` with alpha 0.6.
fn paint(spec: SceneSpec, text: &mut TextCache) -> Scene {
    let win = Rect::new(0.0, 0.0, spec.w, spec.h);
    let mut p: Painter<'_, u8> = Painter::new(spec.scale, win, text, None, DARK);
    p.on(LayerId::Base);
    p.fill(win, 0.0, DARK.dialog_footer);
    p.ring(
        Rect::new(8.0, 8.0, spec.w - 16.0, 40.0),
        6.0,
        1.0,
        DARK.border,
        true,
    );
    p.stroke(
        &[
            [12.0, spec.h - 20.0],
            [spec.w * 0.5, spec.h - 60.0],
            [spec.w - 12.0, spec.h - 24.0],
        ],
        false,
        StrokeStyle::solid(DARK.accent, 2.0),
    );
    if spec.many_glyphs {
        for (i, row) in glyph_rows().iter().enumerate() {
            let style = Style::ui(48.0, DARK.fg_primary);
            p.text(
                &Spec::plain(row.as_str(), style),
                4.0,
                4.0 + i as f32 * 18.0,
            );
        }
    } else {
        let style = Style::ui(14.0, DARK.fg_primary);
        p.text(&Spec::plain("Base layer text", style), 16.0, 16.0);
    }
    p.on(LayerId::Page);
    let card = Rect::new(24.0, 60.0, spec.w * 0.6, spec.h * 0.5);
    p.acrylic(card, 8.0, premul(DARK.menu_glass), [0.0; 4], true);

    p.on(LayerId::Popup);
    let menu = Rect::new(spec.w * 0.4, spec.h * 0.3, spec.w * 0.5, spec.h * 0.5);
    p.set_xform(Some(Xform {
        origin: (menu.x, menu.y),
        scale: 0.95,
        dx: 0.0,
        dy: 3.0,
        alpha: 0.6,
    }));
    p.backdrop(menu, 8.0);
    p.fill(menu, 8.0, DARK.menu_glass);
    p.ring(menu, 8.0, 1.0, DARK.menu_popup_border, false);
    let style = Style::ui(13.0, DARK.menu_text_primary);
    p.text(
        &Spec::plain("Popup item", style),
        menu.x + 12.0,
        menu.y + 10.0,
    );
    p.set_xform(None);
    p.scene
}

/// One target: a renderer and an offscreen texture.
struct Target {
    spec: SceneSpec,
    renderer: FrameRenderer,
    texture: wgpu::Texture,
}

impl Target {
    fn new(gpu: &mut Gpu, spec: SceneSpec, format: wgpu::TextureFormat) -> Self {
        Self {
            spec,
            renderer: FrameRenderer::new(gpu, format),
            texture: dump::target(&gpu.device, format, spec.size()),
        }
    }

    fn paint(&self, text: &mut TextCache) -> Scene {
        paint(self.spec, text)
    }

    fn draw(&mut self, gpu: &mut Gpu, text: &mut TextCache, scene: &Scene) {
        let view = self.texture.create_view(&Default::default());
        self.renderer
            .draw(gpu, text, scene, &view, self.spec.params())
            .expect("draw");
    }

    fn read(&self, gpu: &Gpu) -> Vec<u8> {
        dump::read_rgba(&gpu.device, &gpu.queue, &self.texture).expect("readback")
    }
}

fn text_cache() -> TextCache {
    TextCache::new(&fonts(), UI_FONT, MONO_FONT)
}

/// A fresh device, renderer and text cache drawing `spec` once: the reference pixels.
fn solo(spec: SceneSpec, format: wgpu::TextureFormat) -> Option<Vec<u8>> {
    let mut gpu = headless_gpu()?;
    let mut text = text_cache();
    let mut t = Target::new(&mut gpu, spec, format);
    let scene = t.paint(&mut text);
    t.draw(&mut gpu, &mut text, &scene);
    let px = t.read(&gpu);
    let covered = px.chunks_exact(4).filter(|p| p[3] > 0).count();
    assert!(
        covered > 1000,
        "reference has only {covered} covered pixels"
    );
    Some(px)
}

/// Compares without dumping whole buffers into the failure message.
fn assert_same(what: &str, got: &[u8], want: &[u8]) {
    assert_eq!(got.len(), want.len(), "{what}: length");
    let diff = got.iter().zip(want).filter(|(a, b)| a != b).count();
    assert_eq!(diff, 0, "{what}: {diff} bytes differ");
}

/// C2's contract test: one `Gpu` and ONE `TextCache` serve two targets at scales 2 and 1. Both
/// scenes are painted (shaped at their own scale) before either is drawn.
#[test]
fn two_targets_on_one_gpu_match_each_alone() {
    let format = wgpu::TextureFormat::Bgra8Unorm;
    let (Some(ref_a), Some(ref_b)) = (solo(A, format), solo(B, format)) else {
        return;
    };
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let mut ta = Target::new(&mut gpu, A, format);
    let mut tb = Target::new(&mut gpu, B, format);

    // Frame 1: paint A and B, draw A, then B; read both; end the frame.
    let (sa, sb) = (ta.paint(&mut text), tb.paint(&mut text));
    ta.draw(&mut gpu, &mut text, &sa);
    tb.draw(&mut gpu, &mut text, &sb);
    assert_same("frame 1, A", &ta.read(&gpu), &ref_a);
    assert_same("frame 1, B", &tb.read(&gpu), &ref_b);
    gpu.end_frame(&mut text);

    // Frame 2: paint both, draw B, then A.
    let (sa, sb) = (ta.paint(&mut text), tb.paint(&mut text));
    tb.draw(&mut gpu, &mut text, &sb);
    ta.draw(&mut gpu, &mut text, &sa);
    assert_same("frame 2, B", &tb.read(&gpu), &ref_b);
    assert_same("frame 2, A", &ta.read(&gpu), &ref_a);
}

#[test]
fn second_format_on_one_gpu_matches_a_fresh_gpu() {
    let Some(ref_c) = solo(A, wgpu::TextureFormat::Rgba8Unorm) else {
        return;
    };
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let mut ta = Target::new(&mut gpu, A, wgpu::TextureFormat::Bgra8Unorm);
    let scene = ta.paint(&mut text);
    ta.draw(&mut gpu, &mut text, &scene);
    let mut tc = Target::new(&mut gpu, A, wgpu::TextureFormat::Rgba8Unorm);
    let scene = tc.paint(&mut text);
    tc.draw(&mut gpu, &mut text, &scene);
    assert_same("Rgba8Unorm on a shared Gpu", &tc.read(&gpu), &ref_c);
}

/// The blur's offscreen textures follow the target's size: one `FrameRenderer` drawn at size A and
/// then at size B (and back) draws the same pixels as a fresh renderer at that size, so a size
/// change leaves no stale texture behind. Both scenes carry a backdrop request (the Popup menu).
#[test]
fn one_renderer_across_two_sizes_matches_a_fresh_renderer_at_each() {
    let format = wgpu::TextureFormat::Bgra8Unorm;
    let (Some(ref_a), Some(ref_b)) = (solo(A, format), solo(B, format)) else {
        return;
    };
    assert_ne!(A.size(), B.size(), "the two sizes must differ");
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let mut renderer = FrameRenderer::new(&mut gpu, format);
    let mut draw = |gpu: &mut Gpu, text: &mut TextCache, spec: SceneSpec| {
        let scene = paint(spec, text);
        let texture = dump::target(&gpu.device, format, spec.size());
        let view = texture.create_view(&Default::default());
        renderer
            .draw(gpu, text, &scene, &view, spec.params())
            .expect("draw");
        let px = dump::read_rgba(&gpu.device, &gpu.queue, &texture).expect("readback");
        gpu.end_frame(text);
        px
    };
    assert_same("A, first", &draw(&mut gpu, &mut text, A), &ref_a);
    assert_same("B after A", &draw(&mut gpu, &mut text, B), &ref_b);
    assert_same("A after B", &draw(&mut gpu, &mut text, A), &ref_a);
    assert_same("A again", &draw(&mut gpu, &mut text, A), &ref_a);
}
