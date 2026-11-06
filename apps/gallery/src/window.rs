//! Window mode: a glass window showing one page. The winit `Translator` turns window events into
//! `RawInput`, a Ui page draws through the `Ui` each frame, and `OutputState` applies what the
//! frame asks of the window. Legacy pages paint their fixed scenes through the Painter API.
//!
//! App code: it reads the clock (time since start is the frame's time) and runs the event loop.
//! No gate opens a window, so this module is compile- and review-checked only.

use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowId};

use lune_ui::clipboard::Clipboard;
use lune_ui::gpu::{self, Gpu};
use lune_ui::input::{ClipboardRequest, OutputState, Repaint, Translator, control_flow, keymap};
use lune_ui::lunegit_dark::THEME;
use lune_ui::material::{self, Material};
use lune_ui::motion::MotionTier;
use lune_ui::shortcuts::Shortcuts;
use lune_ui::strings::{DisplayFormat, StringTable};
use lune_ui::surface::{Acquired, WindowSurface};
use lune_ui::ui::{FrameConfig, Ui, UiStore};
use lune_ui::window::glass_attributes;

use crate::page::{Page, PageKind, PageState, register_images, release_images};
use crate::pages::PAGES;
use crate::scenes::{H, W};
use crate::{Renderer, custom, draw_scene, text_cache};
use lune_ui::image::ImageId;

/// Window mode: the window, its surface and the renderer drawing into it.
struct Shown {
    window: Arc<Window>,
    surface: WindowSurface,
    r: Renderer,
    translator: Translator,
    output: OutputState,
}

/// What a keyboard shortcut of the gallery does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shortcut {
    NextPage,
    PrevPage,
    NextScene,
    PrevScene,
    Quit,
}

/// The shortcut `key` is under `mods`, if any. Up and Down cycle scenes only on a legacy page
/// (`legacy`); every other key, Escape included, is the page's.
fn shortcut(key: &Key, mods: ModifiersState, legacy: bool) -> Option<Shortcut> {
    let command = mods.super_key() || mods.control_key();
    match key {
        Key::Character(c) if command && c.as_str() == "]" => Some(Shortcut::NextPage),
        Key::Character(c) if command && c.as_str() == "[" => Some(Shortcut::PrevPage),
        Key::Character(c) if command && c.eq_ignore_ascii_case("q") => Some(Shortcut::Quit),
        Key::Named(NamedKey::ArrowDown) if legacy => Some(Shortcut::NextScene),
        Key::Named(NamedKey::ArrowUp) if legacy => Some(Shortcut::PrevScene),
        _ => None,
    }
}

struct App {
    shown: Option<Shown>,
    /// Index of the page on screen.
    page: usize,
    /// The legacy scene on screen.
    scene: usize,
    /// The Ui page's retained state; reset when the page changes.
    store: UiStore,
    state: Option<Box<dyn PageState>>,
    /// The pictures the page's state registered with the renderer.
    images: Vec<ImageId>,
    strings: StringTable,
    /// The shortcut table; the gallery's own keys are handled before the frame.
    shortcuts: Shortcuts,
    mods: ModifiersState,
    /// The OS clipboard; opened on the first copy or paste.
    clipboard: Clipboard,
    /// The app's time origin: the frame time is the time since it.
    start: Instant,
    /// Process exit status (non-zero when setup or a frame failed).
    exit_code: u8,
}

/// Opens the window and runs until it closes.
pub(crate) fn run() -> ExitCode {
    let event_loop = match EventLoop::new() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("event loop: {e}");
            return ExitCode::from(1);
        }
    };
    let mut app = App {
        shown: None,
        page: 0,
        scene: 0,
        store: UiStore::default(),
        state: None,
        images: Vec::new(),
        strings: StringTable::default(),
        shortcuts: Shortcuts::default(),
        mods: ModifiersState::empty(),
        clipboard: Clipboard::new(),
        start: Instant::now(),
        exit_code: 0,
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("event loop: {e}");
        return ExitCode::from(1);
    }
    ExitCode::from(app.exit_code)
}

fn setup(event_loop: &ActiveEventLoop) -> Result<Shown, String> {
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
    let frame = lune_ui::frame::FrameRenderer::new(&mut gpu, surface.format());
    let fill = custom::Fill::new(&gpu.device, surface.format());
    let translator = Translator::new(size, window.scale_factor());
    Ok(Shown {
        window,
        surface,
        r: Renderer {
            gpu,
            text,
            frame,
            fill,
        },
        translator,
        output: OutputState::new(),
    })
}

impl App {
    fn current(&self) -> &'static Page {
        PAGES[self.page]
    }

    fn legacy_len(&self) -> Option<usize> {
        match self.current().kind {
            PageKind::Legacy(scenes) => Some(scenes.len()),
            PageKind::Ui(_) => None,
        }
    }

    /// Shows page `index`, with a fresh Ui state.
    fn go_to(&mut self, index: usize) {
        self.page = index;
        self.scene = 0;
        self.store = UiStore::default();
        self.state = None;
        if let Some(shown) = &mut self.shown {
            release_images(&mut shown.r.gpu, &self.images);
            self.images.clear();
        }
        if let Some(shown) = &self.shown {
            shown
                .window
                .set_title(&format!("luneui gallery - {}", PAGES[index].title));
        }
    }

    /// Draws the current page into the window and sets the loop's control flow from the frame's
    /// `Repaint`.
    fn render(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let page = self.current();
        let Some(shown) = &mut self.shown else {
            return Ok(());
        };
        let Shown {
            window,
            surface,
            r,
            translator,
            output,
        } = shown;
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
        let input = translator.take(self.start.elapsed());
        let mut repaint = Repaint::Idle;
        let display = match page.kind {
            PageKind::Legacy(scenes) => {
                scenes[self.scene % scenes.len()].paint(&mut r.text, s, size)
            }
            PageKind::Ui(ui_page) => {
                if self.state.is_none() {
                    let fresh = (ui_page.state)();
                    match register_images(&mut r.gpu, fresh.images()) {
                        Ok(ids) => self.images = ids,
                        Err(e) => eprintln!("images: {e}"),
                    }
                    self.state = Some(fresh);
                }
                let Some(state) = self.state.as_mut() else {
                    return Ok(());
                };
                let config = FrameConfig {
                    theme: THEME,
                    tier: MotionTier::Full,
                    strings: &self.strings,
                    arg_format: &DisplayFormat,
                    keymap: keymap(),
                    shortcuts: &self.shortcuts,
                };
                let mut ui = Ui::begin(&mut self.store, &input, config, &mut r.text);
                ui.scope(page.id, |ui| state.draw(ui));
                let (display, out) = ui.finish();
                if output.apply(window, &out) {
                    event_loop.exit();
                }
                // Writes go to the OS; a read is answered with the next frame's input, which needs
                // a frame.
                self.clipboard.service(&out, translator);
                if out
                    .clipboard
                    .iter()
                    .any(|r| matches!(r, ClipboardRequest::Read))
                {
                    window.request_redraw();
                }
                repaint = out.repaint;
                display
            }
        };
        draw_scene(r, &display, s, &view, size)?;
        window.pre_present_notify();
        surface.present(&r.gpu, frame);
        r.gpu.end_frame(&mut r.text);
        let (flow, redraw) = control_flow(repaint, self.start);
        event_loop.set_control_flow(flow);
        if redraw {
            window.request_redraw();
        }
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.shown.is_some() {
            return;
        }
        let shown = match setup(event_loop) {
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

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        // A `Repaint::At` request: its time has come.
        if matches!(cause, StartCause::ResumeTimeReached { .. })
            && let Some(shown) = &self.shown
        {
            shown.window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // After `exit()` winit still delivers queued events; ignore them so no frame is acquired.
        if event_loop.exiting() {
            return;
        }
        if self.shown.is_none() {
            return;
        }
        let action = match &event {
            WindowEvent::KeyboardInput { event: key, .. } if key.state == ElementState::Pressed => {
                shortcut(&key.logical_key, self.mods, self.legacy_len().is_some())
            }
            _ => None,
        };
        if let Some(action) = action {
            match action {
                Shortcut::Quit => event_loop.exit(),
                Shortcut::NextPage => self.go_to((self.page + 1) % PAGES.len()),
                Shortcut::PrevPage => self.go_to((self.page + PAGES.len() - 1) % PAGES.len()),
                Shortcut::NextScene | Shortcut::PrevScene => {
                    let n = self.legacy_len().unwrap_or(1);
                    self.scene = if action == Shortcut::NextScene {
                        (self.scene + 1) % n
                    } else {
                        (self.scene + n - 1) % n
                    };
                }
            }
            if let Some(shown) = &self.shown {
                shown.window.request_redraw();
            }
            return;
        }
        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.render(event_loop) {
                    eprintln!("render: {e}");
                    self.exit_code = 1;
                    event_loop.exit();
                }
                return;
            }
            _ => {}
        }
        let Some(shown) = &mut self.shown else { return };
        if let WindowEvent::Resized(size) = &event {
            shown.surface.resize(&shown.r.gpu, size.width, size.height);
        }
        shown.translator.window_event(&event);
        shown.window.request_redraw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(s: &str) -> Key {
        Key::Character(s.into())
    }

    #[test]
    fn bracket_keys_switch_pages_under_command_or_control() {
        for mods in [ModifiersState::SUPER, ModifiersState::CONTROL] {
            assert_eq!(shortcut(&ch("]"), mods, false), Some(Shortcut::NextPage));
            assert_eq!(shortcut(&ch("["), mods, false), Some(Shortcut::PrevPage));
            assert_eq!(shortcut(&ch("q"), mods, false), Some(Shortcut::Quit));
        }
        assert_eq!(shortcut(&ch("]"), ModifiersState::empty(), false), None);
    }

    #[test]
    fn up_and_down_cycle_scenes_on_legacy_pages_only() {
        let (up, down) = (
            Key::Named(NamedKey::ArrowUp),
            Key::Named(NamedKey::ArrowDown),
        );
        let none = ModifiersState::empty();
        assert_eq!(shortcut(&down, none, true), Some(Shortcut::NextScene));
        assert_eq!(shortcut(&up, none, true), Some(Shortcut::PrevScene));
        assert_eq!(shortcut(&down, none, false), None);
        assert_eq!(shortcut(&up, none, false), None);
    }

    #[test]
    fn escape_reaches_the_page() {
        let esc = Key::Named(NamedKey::Escape);
        assert_eq!(shortcut(&esc, ModifiersState::empty(), true), None);
        assert_eq!(shortcut(&esc, ModifiersState::empty(), false), None);
    }
}
