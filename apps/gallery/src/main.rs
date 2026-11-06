//! Gallery: the pages of the toolkit.
//!
//! Every page is a [`page::Page`] in the registry (`pages/mod.rs`): legacy pages hold the fixed
//! Painter-API scenes the pixel gate compares, Ui pages draw through the `Ui` with scripted scenes.
//!
//! - no arguments: a glass window showing one page (Cmd/Ctrl+] and Cmd/Ctrl+[ switch pages, Up /
//!   Down cycle a legacy page's scenes, Cmd/Ctrl+Q quits; every other key reaches the page);
//! - `--dump <dir>`: renders every scene offscreen on a headless GPU device (no event loop, no
//!   window) and writes `<id>@<scale>x.png` files, the frames the pixel gate compares;
//! - `--list`: prints the scene names, legacy pages first;
//! - `--bench [--runs N] [--frames N] [--size WxH]`: times frames offscreen (see `bench`);
//! - `--smoke-platform`: puts an item in the tray, opens and closes its surface and tears down,
//!   in about two seconds (see `smoke`). It needs a logged-in GUI session: exit 0 when it ran
//!   and held, 1 when a check failed, 2 when there is no session to run in.
//!
//! Dump mode reads no clock and no environment: time comes from the scene tables and scripts,
//! pixels from the offscreen target's readback (no screen or window capture).

mod bench;
mod custom;
mod page;
mod pages;
mod popover;
mod scenes;
mod smoke;
mod window;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lune_ui::dump;
use lune_ui::frame::{FrameParams, FrameRenderer};
use lune_ui::gpu::{self, Gpu};
use lune_ui::scene::Scene;
use lune_ui::text::TextCache;
use page::{PageKind, meets_expectation, register_images, release_images, run_scene};
use pages::PAGES;
use scenes::{BLUR_SATURATION, BLUR_SIGMA, H, W};

const USAGE: &str = "usage: gallery [--dump <dir> | --list | --bench [--runs N] [--frames N] [--size WxH] | --smoke-platform]\n  --smoke-platform needs a logged-in GUI session (it puts an item in the tray for about 2 s); exit 2 without one";

enum Command {
    Window,
    Dump(PathBuf),
    List,
    Bench(bench::Options),
    SmokePlatform,
}

fn parse(args: &[String]) -> Result<Command, String> {
    match args {
        [] => Ok(Command::Window),
        [a] if a == "--list" => Ok(Command::List),
        [a] if a == "--smoke-platform" => Ok(Command::SmokePlatform),
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
            for name in scene_names() {
                println!("{name}");
            }
            ExitCode::SUCCESS
        }
        Command::Dump(dir) => match run_dump(&dir) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("dump: {e}");
                ExitCode::from(1)
            }
        },
        Command::Bench(opts) => bench::run(&opts),
        Command::SmokePlatform => smoke::run(),
        Command::Window => window::run(),
    }
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

/// Every scene name, legacy pages first (the registry keeps them first).
fn scene_names() -> Vec<String> {
    let mut names = Vec::new();
    for page in PAGES {
        match page.kind {
            PageKind::Legacy(scenes) => names.extend(scenes.iter().map(|s| s.name())),
            PageKind::Ui(ui) => names.extend(ui.scenes.iter().map(|s| s.name())),
        }
    }
    names
}

/// Draws a finished display list at scale `s` into `target` (`size` physical pixels).
fn draw_scene(
    r: &mut Renderer,
    scene: &Scene,
    s: f32,
    target: &wgpu::TextureView,
    size: (u32, u32),
) -> Result<(), String> {
    let params = FrameParams {
        size,
        scale: s,
        blur_sigma: BLUR_SIGMA,
        saturation: BLUR_SATURATION,
    };
    let mut customs: [&mut dyn lune_ui::custom::CustomDraw; 1] = [&mut r.fill];
    r.frame
        .draw_with(&mut r.gpu, &mut r.text, scene, target, params, &mut customs)
}

/// Renders every scene offscreen and writes `<id>@<scale>x.png` into `dir`. Each scene's frame
/// is drawn twice (a warm-up, then the captured one). A legacy scene is painted again for each
/// draw, as it always was; a Ui scene runs its script once and draws its capture frame's scene
/// twice.
fn dump_scenes(r: &mut Renderer, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    for page in PAGES {
        match page.kind {
            PageKind::Legacy(scenes) => {
                for scene in scenes {
                    let s = scene.scale as f32;
                    let size = (W * scene.scale, H * scene.scale);
                    dump_one(r, dir, &scene.name(), size, |r, view| {
                        let display = scene.paint(&mut r.text, s, size);
                        draw_scene(r, &display, s, view, size)
                    })?;
                }
            }
            PageKind::Ui(ui) => {
                for scene in ui.scenes {
                    let s = scene.scale as f32;
                    let size = (W * scene.scale, H * scene.scale);
                    let images = register_images(&mut r.gpu, (scene.state)().images())
                        .map_err(|e| format!("scene {}: {e}", scene.name()))?;
                    let (display, out) = run_scene(page.id, scene, &mut r.text)
                        .map_err(|e| format!("scene {}: {e}", scene.name()))?;
                    if !meets_expectation(scene.expect, out.repaint) {
                        return Err(format!(
                            "scene {}: capture frame reports {:?}, expected {:?}",
                            scene.name(),
                            out.repaint,
                            scene.expect
                        ));
                    }
                    let dumped = dump_one(r, dir, &scene.name(), size, |r, view| {
                        draw_scene(r, &display, s, view, size)
                    });
                    release_images(&mut r.gpu, &images);
                    dumped?;
                }
            }
        }
    }
    Ok(())
}

/// Draws a scene twice with `draw` into a fresh target and writes it as `<name>.png`.
fn dump_one(
    r: &mut Renderer,
    dir: &Path,
    name: &str,
    size: (u32, u32),
    mut draw: impl FnMut(&mut Renderer, &wgpu::TextureView) -> Result<(), String>,
) -> Result<(), String> {
    let texture = dump::target(&r.gpu.device, r.frame.format(), size);
    let view = texture.create_view(&Default::default());
    for _ in 0..2 {
        draw(r, &view).map_err(|e| format!("scene {name}: {e}"))?;
        r.gpu.end_frame(&mut r.text);
    }
    let rgba = dump::read_rgba(&r.gpu.device, &r.gpu.queue, &texture)
        .map_err(|e| format!("scene {name}: {e}"))?;
    dump::write_png(&dir.join(format!("{name}.png")), size, &rgba)
        .map_err(|e| format!("scene {name}: {e}"))?;
    eprintln!("dumped {name}.png");
    Ok(())
}
