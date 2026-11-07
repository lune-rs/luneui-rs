//! A layer per overlay through the real renderer: two overlays on one fixed layer do not
//! interleave by kind, so a newer overlay's fill hides an older overlay's text, whichever was
//! called first in the frame.
//!
//! GPU test: with no adapter it prints `skipped: no GPU adapter` and passes, unless
//! `LUNEUI_REQUIRE_GPU=1`, which makes a missing adapter a failure.

use std::time::Duration;

use lune_ui_core::input::Keymap;
use lune_ui_core::motion::{Kind, Motion, MotionTier};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::shortcuts::Shortcuts;
use lune_ui_core::strings::{DisplayFormat, StringTable};
use lune_ui_core::testing::ScriptedInput;
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{FrameConfig, OverlayOptions, Ui, UiStore};
use lune_ui_wgpu::dump;
use lune_ui_wgpu::frame::{FrameParams, FrameRenderer};
use lune_ui_wgpu::gpu::{self, Gpu};
use lune_ui_wgpu::text_cache::TextCache;

const SIZE: (u32, u32) = (240, 80);
const BASE: [u8; 4] = [20, 30, 40, 255];
const FILL: [u8; 4] = [200, 60, 40, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const AT: (f32, f32) = (20.0, 20.0);

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

fn raised() -> OverlayOptions {
    OverlayOptions {
        layer: LayerId::Raised,
        modal: false,
        dismiss_on_escape: false,
        dismiss_on_outside_press: false,
        dismiss_on_focus_loss: false,
        anchor: None,
        initial_focus: None,
        open: Motion::of(Kind::Appear),
        close: Motion::of(Kind::Dismiss),
    }
}

fn spec() -> Spec {
    Spec::plain("MMMMMMMMMM", Style::ui(24.0, WHITE))
}

/// Overlay A: the white text. Returns its box.
fn text_overlay(ui: &mut Ui<'_>) -> Rect {
    ui.overlay(
        "a",
        &raised(),
        |_, _, _| {},
        |ui, _| {
            let (w, h) = ui.painter().measure(&spec());
            ui.painter().text(&spec(), AT.0, AT.1);
            Rect::new(AT.0, AT.1, w, h)
        },
    )
    .1
}

/// Overlay B: an opaque fill over the whole window.
fn fill_overlay(ui: &mut Ui<'_>) {
    let win = ui.painter().window;
    ui.overlay(
        "b",
        &raised(),
        |_, _, _| {},
        |ui, _| ui.painter().fill(win, 0.0, FILL),
    );
}

/// One window's retained state, run a frame at a time.
struct Window {
    store: UiStore,
    input: ScriptedInput,
    strings: StringTable,
    shortcuts: Shortcuts,
    text: TextCache,
    ms: u64,
}

impl Window {
    fn new() -> Self {
        let mut input = ScriptedInput::new((SIZE.0 as f32, SIZE.1 as f32), 1.0);
        input.focus(true);
        Self {
            store: UiStore::default(),
            input,
            strings: StringTable::default(),
            shortcuts: Shortcuts::default(),
            text: TextCache::new(&fonts(), "RznSans", "RobotoMono Nerd Font Mono"),
            ms: 0,
        }
    }

    fn frame<R>(&mut self, draw: impl FnOnce(&mut Ui<'_>) -> R) -> (Scene, R) {
        self.ms += 1000;
        self.input.set_time(Duration::from_millis(self.ms));
        let raw = self.input.end_frame();
        let config = FrameConfig {
            theme: DARK,
            tier: MotionTier::Full,
            strings: &self.strings,
            arg_format: &DisplayFormat,
            keymap: Keymap::MAC,
            shortcuts: &self.shortcuts,
        };
        let mut ui = Ui::begin(&mut self.store, &raw, config, &mut self.text);
        let win = ui.painter().window;
        ui.painter().fill(win, 0.0, BASE);
        let r = draw(&mut ui);
        (ui.finish().0, r)
    }

    fn render(&mut self, gpu: &mut Gpu, scene: &Scene) -> Vec<u8> {
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let mut renderer = FrameRenderer::new(gpu, format);
        let texture = dump::target(&gpu.device, format, SIZE);
        let view = texture.create_view(&Default::default());
        let params = FrameParams {
            size: SIZE,
            scale: 1.0,
            blur_sigma: 4.0,
            saturation: 1.0,
        };
        renderer
            .draw(gpu, &mut self.text, scene, &view, params)
            .expect("draw");
        dump::read_rgba(&gpu.device, &gpu.queue, &texture).expect("readback")
    }
}

/// Every pixel of `img` inside `r`.
fn pixels(img: &[u8], r: Rect) -> Vec<[u8; 4]> {
    let mut out = Vec::new();
    for y in r.y as usize..r.bottom().ceil() as usize {
        for x in r.x as usize..r.right().ceil() as usize {
            let i = (y * SIZE.0 as usize + x) * 4;
            out.push([img[i], img[i + 1], img[i + 2], img[i + 3]]);
        }
    }
    out
}

#[test]
fn a_newer_overlays_fill_hides_an_older_overlays_text() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };

    // Instrument check: alone, A's text does reach the box.
    let mut alone = Window::new();
    let (scene, text_box) = alone.frame(text_overlay);
    assert!(text_box.w > 50.0 && text_box.h > 10.0, "{text_box:?}");
    assert!(text_box.right() < SIZE.0 as f32 && text_box.bottom() < SIZE.1 as f32);
    let img = alone.render(&mut gpu, &scene);
    let inked = pixels(&img, text_box);
    assert!(inked.iter().any(|p| p[0] > 200), "the text draws");
    assert!(inked.contains(&BASE), "and the base shows between glyphs");

    // A opened first. Called in that order, then with A's text drawn after B.
    let mut w = Window::new();
    w.frame(|ui| {
        text_overlay(ui);
        fill_overlay(ui);
    });
    let (in_open_order, _) = w.frame(|ui| {
        text_overlay(ui);
        fill_overlay(ui);
    });
    let (b_called_first, _) = w.frame(|ui| {
        fill_overlay(ui);
        text_overlay(ui);
    });
    assert_eq!(b_called_first.overlays.len(), 2);
    assert!(
        b_called_first.overlays[0].tag > b_called_first.overlays[1].tag,
        "B drew first this frame"
    );
    for (what, scene) in [
        ("A called first", in_open_order),
        ("B called first", b_called_first),
    ] {
        let img = w.render(&mut gpu, &scene);
        for p in pixels(&img, text_box) {
            assert_eq!(p, FILL, "{what}");
        }
    }
}
