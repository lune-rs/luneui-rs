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
//!   and held, 1 when a check failed, 2 when there is no session to run in;
//! - `--smoke-panels`: opens the docked panels and the menu window of the `docked-panels` page and
//!   checks them on the real desktop with synthetic pointers (see `smoke_panels`). It needs a
//!   logged-in GUI session: exit 0 when every applicable check held, 1 when one failed, 2 when there
//!   is no session to run in;
//! - `--pace-probe`: times the frame pacing on the real renderer in four phases (see `pace`). It
//!   needs a logged-in GUI session; a window opens for about 15 s, leave it visible and untouched;
//!   exit 0 held, 1 did not hold, 2 could not measure.
//!
//! Dump mode reads no clock: time comes from the scene tables and scripts, pixels from the
//! offscreen target's readback (no screen or window capture). It reads one environment variable,
//! `LUNEUI_GALLERY_TEXT` (`on` or `off`): the text rendering every dumped scene is drawn with, in
//! place of the scene's own (`off` is the look before font smoothing, what the pixel gate proves).

mod bench;
#[cfg(test)]
mod catalogue;
mod custom;
mod own_chrome;
mod pace;
mod page;
mod pages;
mod panels;
mod popover;
mod scenes;
mod showcase;
mod smoke;
mod smoke_panels;
mod window;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lune_ui::dump;
use lune_ui::frame::{FrameParams, FrameRenderer};
use lune_ui::gpu::{self, Gpu};
use lune_ui::scene::Scene;
use lune_ui::text::{TextCache, TextRendering};
use page::{FrameTweaks, PageKind, meets_expectation, register_images, release_images, run_scene};
use pages::PAGES;
use scenes::{H, W};

const USAGE: &str = "usage: gallery [--dump <dir> | --list | --bench [--runs N] [--frames N] [--size WxH] | --smoke-platform | --smoke-panels | --pace-probe]\n  --smoke-platform needs a logged-in GUI session (it puts an item in the tray for about 2 s); exit 2 without one\n  --smoke-panels needs a logged-in GUI session (three panel windows and a menu open for about 10 s; it never moves the cursor); exit 0 held, 1 failed, 2 no session\n  --pace-probe needs a logged-in GUI session; a window opens for about 15 s, leave it visible and untouched; exit 0 held, 1 did not hold, 2 could not measure";

enum Command {
    Window,
    Dump(PathBuf),
    List,
    Bench(bench::Options),
    SmokePlatform,
    SmokePanels,
    PaceProbe,
}

fn parse(args: &[String]) -> Result<Command, String> {
    match args {
        [] => Ok(Command::Window),
        [a] if a == "--list" => Ok(Command::List),
        [a] if a == "--smoke-platform" => Ok(Command::SmokePlatform),
        [a] if a == "--smoke-panels" => Ok(Command::SmokePanels),
        [a] if a == "--pace-probe" => Ok(Command::PaceProbe),
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
    /// A text rendering that replaces every frame's own (dump mode, `LUNEUI_GALLERY_TEXT`).
    text_rendering: Option<TextRendering>,
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
        Command::Dump(dir) => {
            let var = std::env::var_os(TEXT_VAR);
            let text = match text_override(var.as_deref()) {
                Ok(text) => text,
                Err(msg) => {
                    eprintln!("{msg}");
                    return ExitCode::from(2);
                }
            };
            match run_dump(&dir, text) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("dump: {e}");
                    ExitCode::from(1)
                }
            }
        }
        Command::Bench(opts) => bench::run(&opts),
        Command::SmokePlatform => smoke::run(),
        Command::SmokePanels => smoke_panels::run(),
        Command::PaceProbe => pace::run(),
        Command::Window => window::run(),
    }
}

/// The environment variable that sets the text rendering of a dump.
const TEXT_VAR: &str = "LUNEUI_GALLERY_TEXT";

/// The text rendering `LUNEUI_GALLERY_TEXT` asks for: unset is `None` (each scene's own), `on` is
/// the smooth look and `off` is the look without smoothing. Any other value, or one that is not
/// Unicode, is an error naming the variable.
fn text_override(value: Option<&std::ffi::OsStr>) -> Result<Option<TextRendering>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    match value.to_str() {
        Some("on") => Ok(Some(TextRendering::SMOOTH)),
        Some("off") => Ok(Some(TextRendering::OFF)),
        _ => Err(format!(
            "{TEXT_VAR} must be `on` or `off` (got {})",
            value.to_string_lossy()
        )),
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
        text_rendering: None,
    }
}

/// Dump mode: a headless device, `Bgra8Unorm` targets, every scene written to `dir`.
/// `text` replaces every scene's own text rendering when given.
fn run_dump(dir: &Path, text: Option<TextRendering>) -> Result<(), String> {
    let gpu = pollster::block_on(Gpu::new(gpu::instance(None), None))?;
    let mut r = offscreen_renderer(gpu);
    r.text_rendering = text;
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

/// Draws a finished display list at scale `s` into `target` (`size` physical pixels) with the
/// gallery's own blur.
fn draw_scene(
    r: &mut Renderer,
    scene: &Scene,
    s: f32,
    target: &wgpu::TextureView,
    size: (u32, u32),
) -> Result<(), String> {
    draw_with(r, scene, target, FrameTweaks::DEFAULT.params(size, s))
}

/// Draws a finished display list into `target` with `params`.
fn draw_with(
    r: &mut Renderer,
    scene: &Scene,
    target: &wgpu::TextureView,
    mut params: FrameParams,
) -> Result<(), String> {
    if let Some(text) = r.text_rendering {
        params.text = text;
    }
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
                    let s = scene.scale;
                    let size = scene.size();
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

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn the_text_variable_picks_the_dump_text_rendering() {
        assert_eq!(text_override(None), Ok(None));
        assert_eq!(
            text_override(Some(OsStr::new("on"))),
            Ok(Some(TextRendering::SMOOTH))
        );
        assert_eq!(
            text_override(Some(OsStr::new("off"))),
            Ok(Some(TextRendering::OFF))
        );
        for bad in ["", "ON", "Off", "true", "1", "bogus"] {
            let err = text_override(Some(OsStr::new(bad))).expect_err(bad);
            assert!(err.contains("LUNEUI_GALLERY_TEXT"), "{err}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_value_that_is_not_unicode_is_refused() {
        use std::os::unix::ffi::OsStrExt;
        let err = text_override(Some(OsStr::from_bytes(&[0x6f, 0xff]))).expect_err("not Unicode");
        assert!(err.contains("LUNEUI_GALLERY_TEXT"), "{err}");
    }
}
