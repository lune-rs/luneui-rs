//! Device loss: `Gpu::is_lost` reports it, and a draw on a lost device never reports success (wgpu
//! panics inside it).
//!
//! GPU test: with no adapter it prints `skipped: no GPU adapter` and passes, unless
//! `LUNEUI_REQUIRE_GPU=1`, which makes a missing adapter a failure.

use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_wgpu::dump;
use lune_ui_wgpu::frame::{FrameParams, FrameRenderer};
use lune_ui_wgpu::gpu::{self, Gpu};
use lune_ui_wgpu::text_cache::TextCache;

const SIZE: (u32, u32) = (120, 80);

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

fn scene(text: &mut TextCache) -> Scene {
    let win = Rect::new(0.0, 0.0, SIZE.0 as f32, SIZE.1 as f32);
    let mut p: Painter<'_, u8> = Painter::new(1.0, win, text, None, DARK);
    p.fill(Rect::new(10.0, 10.0, 60.0, 30.0), 4.0, [255, 255, 255, 255]);
    p.text(&Spec::plain("lost", Style::ui(13.0, [255; 4])), 12.0, 50.0);
    p.scene
}

#[test]
fn a_new_device_is_not_lost() {
    let Some(gpu) = headless_gpu() else {
        return;
    };
    assert!(!gpu.is_lost());
    // A poll with nothing wrong changes nothing.
    let _ = gpu.device.poll(wgpu::PollType::Poll);
    assert!(!gpu.is_lost());
}

#[test]
fn a_destroyed_device_is_lost_a_draw_on_it_never_succeeds_and_a_rebuild_draws() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut text = TextCache::new(&[], "sans-serif", "monospace");
    let mut renderer = FrameRenderer::new(&mut gpu, format);
    let texture = dump::target(&gpu.device, format, SIZE);
    let view = texture.create_view(&Default::default());
    let params = FrameParams {
        size: SIZE,
        scale: 1.0,
        blur_sigma: 4.0,
        saturation: 1.0,
        ..FrameParams::default()
    };
    // Instrument check: before the loss the same draw works and the device reports it healthy.
    let s = scene(&mut text);
    renderer
        .draw(&mut gpu, &mut text, &s, &view, params)
        .expect("draw before the loss");
    assert!(!gpu.is_lost());

    // wgpu reports a destroyed device from a poll that finds its queue drained, so wait for the
    // frame above to finish first.
    gpu.device.destroy();
    let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
    assert!(gpu.is_lost(), "destroy then poll must report the loss");

    // The documented order: ask `is_lost` first. A draw on the lost device is not safe to make
    // (wgpu panics creating a buffer on it); pin that it never comes back as a good frame.
    let s = scene(&mut text);
    let drew = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        renderer.draw(&mut gpu, &mut text, &s, &view, params)
    }));
    assert!(
        matches!(drew, Err(_) | Ok(Err(_))),
        "a draw on a lost device must not report success"
    );
    assert!(gpu.is_lost(), "a lost device stays lost");

    // The recovery the doc describes: everything is rebuilt, and the new device draws the frame.
    drop((renderer, view, texture));
    let Some(mut gpu2) = headless_gpu() else {
        return;
    };
    assert!(!gpu2.is_lost());
    let mut text2 = TextCache::new(&[], "sans-serif", "monospace");
    let mut renderer2 = FrameRenderer::new(&mut gpu2, format);
    let texture2 = dump::target(&gpu2.device, format, SIZE);
    let view2 = texture2.create_view(&Default::default());
    let s2 = scene(&mut text2);
    renderer2
        .draw(&mut gpu2, &mut text2, &s2, &view2, params)
        .expect("draw on the rebuilt device");
    let img = dump::read_rgba(&gpu2.device, &gpu2.queue, &texture2).expect("readback");
    // Inside the filled rectangle (10, 10, 60, 30): white.
    let i = (20 * SIZE.0 as usize + 20) * 4;
    assert_eq!(&img[i..i + 4], &[255, 255, 255, 255]);
}
