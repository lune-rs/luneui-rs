//! Gallery: the reference scenes of the toolkit.
//!
//! - no arguments: a glass window showing one scene (Left / Right cycle, Escape quits);
//! - `--dump <dir>`: renders every scene offscreen on a hidden window and writes
//!   `<id>@<scale>x.png` files, the frames the pixel gate compares;
//! - `--list`: prints the scene names.
//!
//! Dump mode reads no clock and no environment: time comes from the scene table, pixels from
//! the offscreen target's readback (no screen or window capture).

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
use lune_ui::text::TextCache;
use lune_ui::window;
use scenes::{BLUR_SATURATION, BLUR_SIGMA, H, SCENES, Scene, W};

const USAGE: &str = "usage: gallery [--dump <dir> | --list]";

enum Command {
    Window,
    Dump(PathBuf),
    List,
}

fn parse(args: &[String]) -> Result<Command, String> {
    match args {
        [] => Ok(Command::Window),
        [a] if a == "--list" => Ok(Command::List),
        [a, dir] if a == "--dump" => Ok(Command::Dump(PathBuf::from(dir))),
        _ => Err(USAGE.to_owned()),
    }
}

struct Renderer {
    gpu: Gpu,
    text: TextCache,
    frame: FrameRenderer,
    fill: custom::Fill,
}

struct App {
    /// `Some` in dump mode.
    dump_dir: Option<PathBuf>,
    renderer: Option<Renderer>,
    /// Window mode: the scene on screen.
    index: usize,
    /// Process exit status (non-zero when a dump failed).
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
    let dump_dir = match cmd {
        Command::List => {
            for scene in SCENES {
                println!("{}", scene.name());
            }
            return ExitCode::SUCCESS;
        }
        Command::Window => None,
        Command::Dump(dir) => Some(dir),
    };
    let event_loop = match EventLoop::new() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("event loop: {e}");
            return ExitCode::from(1);
        }
    };
    let mut app = App {
        dump_dir,
        renderer: None,
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

impl App {
    fn setup(&self, event_loop: &ActiveEventLoop) -> Result<(Arc<Window>, Renderer), String> {
        // Dump runs render offscreen only; keep their window out of sight.
        let attrs = window::glass_attributes(
            "luneui gallery",
            (f64::from(W), f64::from(H)),
            (560.0, 360.0),
            self.dump_dir.is_none(),
        );
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .map_err(|e| format!("window: {e}"))?,
        );
        if self.dump_dir.is_none()
            && let Err(e) = material::apply(&window, Material::Vibrancy)
        {
            eprintln!("glass: {e}");
        }
        let instance = gpu::instance(event_loop.owned_display_handle());
        let gpu = pollster::block_on(Gpu::new(window.clone(), &instance))?;
        let text = TextCache::new(
            window.scale_factor() as f32,
            &[
                include_bytes!("../assets/RobotoMonoNerdFontMono-Regular.ttf").as_slice(),
                include_bytes!("../assets/RznSans-Regular.ttf"),
                include_bytes!("../assets/RznSans-Medium.ttf"),
            ],
            UI_FONT,
            MONO_FONT,
        );
        let frame = FrameRenderer::new(&gpu);
        let fill = custom::Fill::new(&gpu.device, gpu.config.format);
        Ok((
            window,
            Renderer {
                gpu,
                text,
                frame,
                fill,
            },
        ))
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            return;
        }
        let (window, mut renderer) = match self.setup(event_loop) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{e}");
                self.exit_code = 1;
                event_loop.exit();
                return;
            }
        };
        if let Some(dir) = &self.dump_dir {
            // `window_event` returns early once exiting, so nothing reaches `render` after this.
            if let Err(e) = dump_scenes(&mut renderer, dir) {
                eprintln!("dump: {e}");
                self.exit_code = 1;
            }
            self.renderer = Some(renderer);
            event_loop.exit();
            return;
        }
        window.request_redraw();
        self.renderer = Some(renderer);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // After `exit()` winit still delivers queued events; ignore them so no frame is acquired.
        if event_loop.exiting() {
            return;
        }
        let Some(r) = &mut self.renderer else { return };
        let window = r.gpu.window.clone();
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                r.gpu.resize(size.width, size.height);
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
                if let Err(e) = render_window(r, &SCENES[self.index]) {
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
    r.text.set_scale(s);
    let mut p = Painter::new(s, win, &mut r.text, scene.hover, lunegit_dark::THEME);
    (scene.draw)(&mut p, scene.time);
    let Painter {
        layers, backdrop, ..
    } = p;
    let params = FrameParams {
        size,
        scale: s,
        blur_sigma: BLUR_SIGMA,
        saturation: BLUR_SATURATION,
    };
    let mut customs: [&mut dyn CustomDraw; 1] = [&mut r.fill];
    r.frame.draw_with(
        &r.gpu,
        &mut r.text,
        layers,
        backdrop,
        target,
        params,
        &mut customs,
    )
}

/// Draws one frame of `scene` into the window's surface.
fn render_window(r: &mut Renderer, scene: &Scene) -> Result<(), String> {
    let Some(frame) = r.gpu.acquire()? else {
        return Ok(());
    };
    let view = frame.texture.create_view(&Default::default());
    let size = (r.gpu.config.width, r.gpu.config.height);
    let s = r.gpu.window.scale_factor() as f32;
    draw_scene(r, scene, s, &view, size)?;
    r.gpu.present(frame);
    r.frame.end_frame(&mut r.text);
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
        let texture = dump::target(&r.gpu.device, r.gpu.config.format, size);
        let view = texture.create_view(&Default::default());
        for _ in 0..2 {
            draw_scene(r, scene, s, &view, size).map_err(|e| format!("scene {name}: {e}"))?;
            r.frame.end_frame(&mut r.text);
        }
        let rgba = dump::read_rgba(&r.gpu.device, &r.gpu.queue, &texture)
            .map_err(|e| format!("scene {name}: {e}"))?;
        dump::write_png(&dir.join(format!("{name}.png")), size, &rgba)
            .map_err(|e| format!("scene {name}: {e}"))?;
        eprintln!("dumped {name}.png");
    }
    Ok(())
}
