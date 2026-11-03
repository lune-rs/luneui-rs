//! Gallery: the reference scenes of the toolkit.
//!
//! - no arguments: a glass window showing one scene (Left / Right cycle, Escape quits);
//! - `--dump <dir>`: renders every scene offscreen on a headless GPU device (no event loop, no
//!   window) and writes `<id>@<scale>x.png` files, the frames the pixel gate compares;
//! - `--list`: prints the scene names;
//! - `--bench [--runs N] [--frames N] [--size WxH]`: times frames offscreen (see `bench`).
//!
//! Dump mode reads no clock and no environment: time comes from the scene table, pixels from
//! the offscreen target's readback (no screen or window capture).

mod bench;
mod custom;
mod scenes;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use lune_ui::custom::CustomDraw;
use lune_ui::dump;
use lune_ui::frame::{FrameParams, FrameRenderer};
use lune_ui::gpu::{self, Gpu};
use lune_ui::lunegit_dark;
use lune_ui::material::{self, Material};
use lune_ui::paint::{Painter, Rect};
use lune_ui::surface::{Acquired, WindowSurface};
use lune_ui::text::TextCache;
use lune_ui::window::glass_attributes;
use scenes::{BLUR_SATURATION, BLUR_SIGMA, H, SCENES, Scene, W};

const USAGE: &str =
    "usage: gallery [--dump <dir> | --list | --bench [--runs N] [--frames N] [--size WxH]]";

enum Command {
    Window,
    Dump(PathBuf),
    List,
    Bench(bench::Options),
}

fn parse(args: &[String]) -> Result<Command, String> {
    match args {
        [] => Ok(Command::Window),
        [a] if a == "--list" => Ok(Command::List),
        [a, dir] if a == "--dump" => Ok(Command::Dump(PathBuf::from(dir))),
        [a, rest @ ..] if a == "--bench" => bench::Options::parse(rest).map(Command::Bench),
        _ => Err(USAGE.to_owned()),
    }
}

/// What draws a scene into a target: the GPU, text, one renderer and the custom draw.
struct Renderer {
    gpu: Gpu,
    text: TextCache,
    frame: FrameRenderer,
    fill: custom::Fill,
}

/// Window mode: the window, its surface and the renderer drawing into it.
struct Shown {
    window: Arc<Window>,
    surface: WindowSurface,
    r: Renderer,
}

struct App {
    shown: Option<Shown>,
    /// The scene on screen.
    index: usize,
    /// Process exit status (non-zero when setup or a frame failed).
    exit_code: u8,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = match parse(&args) {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::from(2);
        }
    };
    match cmd {
        Command::List => {
            for scene in SCENES {
                println!("{}", scene.name());
            }
            return ExitCode::SUCCESS;
        }
        Command::Dump(dir) => {
            return match run_dump(&dir) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("dump: {e}");
                    ExitCode::from(1)
                }
            };
        }
        Command::Bench(opts) => return bench::run(&opts),
        Command::Window => {}
    }
    let event_loop = match EventLoop::new() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("event loop: {e}");
            return ExitCode::from(1);
        }
    };
    let mut app = App {
        shown: None,
        index: 0,
        exit_code: 0,
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("event loop: {e}");
        return ExitCode::from(1);
    }
    ExitCode::from(app.exit_code)
}

const UI_FONT: &str = "RznSans";
const MONO_FONT: &str = "RobotoMono Nerd Font Mono";

/// The gallery's fonts, loaded in a fixed order.
fn text_cache() -> TextCache {
    TextCache::new(
        &[
            include_bytes!("../assets/RobotoMonoNerdFontMono-Regular.ttf").as_slice(),
            include_bytes!("../assets/RznSans-Regular.ttf"),
            include_bytes!("../assets/RznSans-Medium.ttf"),
        ],
        UI_FONT,
        MONO_FONT,
    )
}

/// A renderer over `gpu` for `Bgra8Unorm` offscreen targets.
fn offscreen_renderer(mut gpu: Gpu) -> Renderer {
    let format = wgpu::TextureFormat::Bgra8Unorm;
    let frame = FrameRenderer::new(&mut gpu, format);
    let fill = custom::Fill::new(&gpu.device, format);
    Renderer {
        gpu,
        text: text_cache(),
        frame,
        fill,
    }
}

/// Dump mode: a headless device, `Bgra8Unorm` targets, every scene written to `dir`.
fn run_dump(dir: &Path) -> Result<(), String> {
    let gpu = pollster::block_on(Gpu::new(gpu::instance(None), None))?;
    let mut r = offscreen_renderer(gpu);
    dump_scenes(&mut r, dir)
}

impl App {
    fn setup(&self, event_loop: &ActiveEventLoop) -> Result<Shown, String> {
        let attrs = glass_attributes(
            "luneui gallery",
            (f64::from(W), f64::from(H)),
            (560.0, 360.0),
            true,
        );
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .map_err(|e| format!("window: {e}"))?,
        );
        if let Err(e) = material::apply(&window, Material::Vibrancy) {
            eprintln!("glass: {e}");
        }
        let instance = gpu::instance(Some(Box::new(event_loop.owned_display_handle())));
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| format!("surface: {e}"))?;
        let mut gpu = pollster::block_on(Gpu::new(instance, Some(&surface)))?;
        let size = window.inner_size();
        let surface = WindowSurface::new(&gpu, surface, (size.width, size.height))?;
        let text = text_cache();
        let frame = FrameRenderer::new(&mut gpu, surface.format());
        let fill = custom::Fill::new(&gpu.device, surface.format());
        Ok(Shown {
            window,
            surface,
            r: Renderer {
                gpu,
                text,
                frame,
                fill,
            },
        })
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.shown.is_some() {
            return;
        }
        let shown = match self.setup(event_loop) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{e}");
                self.exit_code = 1;
                event_loop.exit();
                return;
            }
        };
        shown.window.request_redraw();
        self.shown = Some(shown);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // After `exit()` winit still delivers queued events; ignore them so no frame is acquired.
        if event_loop.exiting() {
            return;
        }
        let Some(shown) = &mut self.shown else { return };
        let window = shown.window.clone();
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                shown.surface.resize(&shown.r.gpu, size.width, size.height);
                window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => window.request_redraw(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let n = SCENES.len();
                match event.logical_key {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Named(NamedKey::ArrowRight) => {
                        self.index = (self.index + 1) % n;
                        window.request_redraw();
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        self.index = (self.index + n - 1) % n;
                        window.request_redraw();
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = render_window(shown, &SCENES[self.index]) {
                    eprintln!("render: {e}");
                    self.exit_code = 1;
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}

/// Paints `scene` at scale `s` and draws it into `target` (`size` physical pixels).
fn draw_scene(
    r: &mut Renderer,
    scene: &Scene,
    s: f32,
    target: &wgpu::TextureView,
    size: (u32, u32),
) -> Result<(), String> {
    let win = Rect::new(0.0, 0.0, size.0 as f32 / s, size.1 as f32 / s);
    let mut p = Painter::new(s, win, &mut r.text, scene.hover, lunegit_dark::THEME);
    (scene.draw)(&mut p, scene.time);
    let Painter { scene, .. } = p;
    let params = FrameParams {
        size,
        scale: s,
        blur_sigma: BLUR_SIGMA,
        saturation: BLUR_SATURATION,
    };
    let mut customs: [&mut dyn CustomDraw; 1] = [&mut r.fill];
    r.frame.draw_with(
        &mut r.gpu,
        &mut r.text,
        &scene,
        target,
        params,
        &mut customs,
    )
}

/// Draws one frame of `scene` into the window's surface.
fn render_window(shown: &mut Shown, scene: &Scene) -> Result<(), String> {
    let Shown { window, surface, r } = shown;
    let frame = match surface.acquire(&r.gpu)? {
        Acquired::Frame(frame) => frame,
        Acquired::Skip => return Ok(()),
        Acquired::Redraw => {
            window.request_redraw();
            return Ok(());
        }
    };
    let view = frame.texture.create_view(&Default::default());
    let size = surface.size();
    let s = window.scale_factor() as f32;
    draw_scene(r, scene, s, &view, size)?;
    window.pre_present_notify();
    surface.present(&r.gpu, frame);
    r.gpu.end_frame(&mut r.text);
    Ok(())
}

/// Renders every scene offscreen (a warm-up frame, then the captured one) and writes
/// `<id>@<scale>x.png` into `dir`.
fn dump_scenes(r: &mut Renderer, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    for scene in SCENES {
        let name = scene.name();
        let s = scene.scale as f32;
        let size = (W * scene.scale, H * scene.scale);
        let texture = dump::target(&r.gpu.device, r.frame.format(), size);
        let view = texture.create_view(&Default::default());
        for _ in 0..2 {
            draw_scene(r, scene, s, &view, size).map_err(|e| format!("scene {name}: {e}"))?;
            r.gpu.end_frame(&mut r.text);
        }
        let rgba = dump::read_rgba(&r.gpu.device, &r.gpu.queue, &texture)
            .map_err(|e| format!("scene {name}: {e}"))?;
        dump::write_png(&dir.join(format!("{name}.png")), size, &rgba)
            .map_err(|e| format!("scene {name}: {e}"))?;
        eprintln!("dumped {name}.png");
    }
    Ok(())
}
