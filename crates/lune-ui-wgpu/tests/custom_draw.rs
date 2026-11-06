//! The `CustomDraw` hook through `FrameRenderer::draw_with`: every supplied implementation is
//! prepared once per frame, whether or not an item refers to it; `render` runs once per item of
//! its slot; both get the frame's size, scale and format.
//!
//! GPU test: with no adapter it prints `skipped: no GPU adapter` and passes, unless
//! `LUNEUI_REQUIRE_GPU=1`, which makes a missing adapter a failure.

use std::cell::RefCell;

use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::testing::FakeText;
use lune_ui_core::theme::DARK;
use lune_ui_wgpu::custom::{CustomDraw, CustomFrame};
use lune_ui_wgpu::dump;
use lune_ui_wgpu::frame::{FrameParams, FrameRenderer};
use lune_ui_wgpu::gpu::{self, Gpu};
use lune_ui_wgpu::text_cache::TextCache;

const SIZE: (u32, u32) = (200, 120);
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Keeps the frames it was prepared and rendered with; draws nothing.
#[derive(Default)]
struct Counter {
    prepared: Vec<CustomFrame>,
    rendered: RefCell<Vec<CustomFrame>>,
}

impl CustomDraw for Counter {
    fn prepare(&mut self, _: &wgpu::Device, _: &wgpu::Queue, frame: &CustomFrame) {
        self.prepared.push(*frame);
    }

    fn render(&self, _: &mut wgpu::RenderPass<'_>, frame: &CustomFrame) {
        self.rendered.borrow_mut().push(*frame);
    }
}

#[test]
fn every_slot_is_prepared_once_and_rendered_once_per_item() {
    let mut gpu = match pollster::block_on(Gpu::new(gpu::instance(None), None)) {
        Ok(g) => g,
        Err(e) => {
            assert!(
                !std::env::var("LUNEUI_REQUIRE_GPU").is_ok_and(|v| v == "1"),
                "no GPU adapter: {e}"
            );
            println!("skipped: no GPU adapter");
            return;
        }
    };
    let mut text = TextCache::new(&[], "RznSans", "RobotoMono Nerd Font Mono");
    let win = Rect::new(0.0, 0.0, 100.0, 60.0);
    let mut fake = FakeText;
    let mut p: Painter<'_, u8> = Painter::new(2.0, win, &mut fake, None, DARK);
    // Two items for slot 0; slot 1 is supplied and unused.
    p.custom(0, Rect::new(5.0, 5.0, 20.0, 20.0));
    p.custom(0, Rect::new(40.0, 5.0, 20.0, 20.0));
    let scene = p.scene;
    let texture = dump::target(&gpu.device, FORMAT, SIZE);
    let view = texture.create_view(&Default::default());
    let params = FrameParams {
        size: SIZE,
        scale: 2.0,
        blur_sigma: 4.0,
        saturation: 1.0,
    };
    let (mut used, mut unused) = (Counter::default(), Counter::default());
    FrameRenderer::new(&mut gpu, FORMAT)
        .draw_with(
            &mut gpu,
            &mut text,
            &scene,
            &view,
            params,
            &mut [&mut used, &mut unused],
        )
        .expect("draw");
    let frame = CustomFrame {
        size: SIZE,
        scale: 2.0,
        format: FORMAT,
    };
    assert_eq!(used.prepared, [frame], "prepared once");
    assert_eq!(
        unused.prepared,
        [frame],
        "prepared though no item refers to it"
    );
    assert_eq!(*used.rendered.borrow(), [frame, frame], "once per item");
    assert!(unused.rendered.borrow().is_empty());
}
