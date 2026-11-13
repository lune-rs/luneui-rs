//! Window mode: a glass window showing the showcase page, which hosts every other page. The winit
//! `Translator` turns window events into `RawInput`, the page draws through the `Ui` each frame,
//! and `OutputState` applies what the frame asks of the window. Legacy pages (the fixed
//! Painter-API scenes) are not shown here; the dump and the pixel gate still draw them.
//!
//! App code: it reads the clock (time since start is the frame's time) and runs the event loop.
//! No gate opens a window, so this module is compile- and review-checked only; the showcase's
//! tests send the same `Translator` calls through the shell.

use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState};
use winit::window::{Window, WindowId};

use lune_ui::access::{AccessBridge, RootInfo};
use lune_ui::clipboard::Clipboard;
use lune_ui::gpu::{self, Gpu};
use lune_ui::input::{ClipboardRequest, OutputState, Repaint, Translator, control_flow, keymap};
use lune_ui::lunegit_dark::THEME;
use lune_ui::material::{self, Material};
use lune_ui::motion::{Cadence, MotionTier};
use lune_ui::shortcuts::Shortcuts;
use lune_ui::strings::{DisplayFormat, StringTable};
use lune_ui::surface::{Acquired, WindowSurface};
use lune_ui::tray::TrayState;
use lune_ui::ui::{FrameConfig, Ui, UiStore};
use lune_ui::window::glass_attributes;

use crate::page::{FrameTweaks, Page, PageKind, PageState, Stage, register_images};
use crate::panels::PanelsHost;
use crate::popover::{AppEvent, TrayHost, event_loop, route_tray_events};
use crate::showcase;
use crate::{Renderer, custom, draw_with, text_cache};
use lune_ui::image::ImageId;

/// Window mode: the window, its surface and the renderer drawing into it.
pub(crate) struct Shown {
    /// First, so it drops before the window it serves.
    pub(crate) bridge: AccessBridge,
    pub(crate) window: Arc<Window>,
    pub(crate) surface: WindowSurface,
    pub(crate) r: Renderer,
    pub(crate) translator: Translator,
    pub(crate) output: OutputState,
}

/// What a keyboard shortcut of the gallery does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shortcut {
    NextPage,
    PrevPage,
    Quit,
}

/// The shortcut `key` is under `mods`, if any. Every other key, Escape included, is the page's.
fn shortcut(key: &Key, mods: ModifiersState) -> Option<Shortcut> {
    let command = mods.super_key() || mods.control_key();
    match key {
        Key::Character(c) if command && c.as_str() == "]" => Some(Shortcut::NextPage),
        Key::Character(c) if command && c.as_str() == "[" => Some(Shortcut::PrevPage),
        Key::Character(c) if command && c.eq_ignore_ascii_case("q") => Some(Shortcut::Quit),
        _ => None,
    }
}

/// The window's first size, in logical pixels: the 1,020 x 616 canvas of a page, the sidebar and
/// the margins around them.
const WINDOW: (f64, f64) = (1600.0, 900.0);

/// The page's clock: the real time that has passed, scaled by the speed the page asks for. It
/// adds up scaled slices, so changing the speed changes how fast time runs from now on and never
/// moves it.
#[derive(Debug, Default, Clone, Copy)]
struct Clock {
    /// The real time of the last frame, since the app's start.
    real: Duration,
    /// The page's time at that frame.
    scaled: Duration,
}

impl Clock {
    /// Moves the clock to the real time `now` at `speed` and returns the page's time.
    fn advance(&mut self, now: Duration, speed: f32) -> Duration {
        let slice = now.saturating_sub(self.real);
        self.real = now.max(self.real);
        self.scaled += slice.mul_f64(f64::from(speed.max(0.0)));
        self.scaled
    }

    /// A repaint asked for at a page time, as the real time it comes due at.
    fn in_real_time(&self, repaint: Repaint, speed: f32) -> Repaint {
        match repaint {
            Repaint::At(t) => {
                let ahead = t.saturating_sub(self.scaled);
                Repaint::At(self.real + ahead.div_f64(f64::from(speed.max(0.01))))
            }
            other => other,
        }
    }
}

struct App {
    shown: Option<Shown>,
    /// Lets the accessibility handlers, on their own thread, reach the event loop.
    proxy: EventLoopProxy<AppEvent>,
    /// The showcase page's retained store and state.
    store: UiStore,
    state: Option<Box<dyn PageState>>,
    /// The pictures the page's state registered with the renderer.
    images: Vec<ImageId>,
    /// The hosted page's string table, and the `PageState::epoch` it was read at.
    strings: StringTable,
    strings_epoch: u64,
    /// The shortcut table; the gallery's own keys are handled before the frame.
    shortcuts: Shortcuts,
    mods: ModifiersState,
    /// The OS clipboard; opened on the first copy or paste.
    clipboard: Clipboard,
    /// The live tray, while the `tray` page is on screen.
    tray: Option<TrayHost>,
    /// The state the page's tray control said last: the surface follows it.
    tray_state: TrayState,
    /// The live docked panels, while the `docked-panels` page's switch is on.
    panels: Option<PanelsHost>,
    /// The control flow the main window's last frame asked for, the tray surface's and the
    /// panels'.
    main_flow: ControlFlow,
    pop_flow: ControlFlow,
    panels_flow: ControlFlow,
    /// The app's time origin: the real time the clock runs from.
    start: Instant,
    /// The page's clock: real time scaled by the page's speed.
    clock: Clock,
    /// The OS material last put on the window, and the zoom the translator reads at.
    material: Material,
    /// Process exit status (non-zero when setup or a frame failed).
    exit_code: u8,
}

/// Opens the window and runs until it closes.
pub(crate) fn run() -> ExitCode {
    let event_loop = match event_loop(false) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    route_tray_events(event_loop.create_proxy());
    let mut app = App {
        shown: None,
        proxy: event_loop.create_proxy(),
        store: UiStore::default(),
        state: None,
        images: Vec::new(),
        strings: StringTable::default(),
        strings_epoch: 0,
        shortcuts: Shortcuts::default(),
        mods: ModifiersState::empty(),
        clipboard: Clipboard::new(),
        tray: None,
        tray_state: TrayState::Serving,
        panels: None,
        main_flow: ControlFlow::Wait,
        pop_flow: ControlFlow::Wait,
        panels_flow: ControlFlow::Wait,
        start: Instant::now(),
        clock: Clock::default(),
        material: Material::Vibrancy,
        exit_code: 0,
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("event loop: {e}");
        return ExitCode::from(1);
    }
    ExitCode::from(app.exit_code)
}

/// The window, its surface, the GPU and the accessibility bridge. The window is created hidden (the
/// bridge wants one never shown) and shown at the end when `show` is set; the smoke leaves it hidden.
pub(crate) fn setup(
    event_loop: &ActiveEventLoop,
    proxy: EventLoopProxy<AppEvent>,
    show: bool,
) -> Result<Shown, String> {
    let attrs =
        glass_attributes("luneui gallery", WINDOW, (900.0, 600.0), true).with_visible(false);
    let window = Arc::new(
        event_loop
            .create_window(attrs)
            .map_err(|e| format!("window: {e}"))?,
    );
    // The bridge wants a window never shown yet. A wake only posts: the event loop redraws.
    let bridge = AccessBridge::new(event_loop, &window, move || {
        let _ = proxy.send_event(AppEvent::Wake);
    })
    .map_err(|e| format!("accessibility: {e}"))?;
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
    if show {
        window.set_visible(true);
    }
    Ok(Shown {
        bridge,
        window,
        surface,
        r: Renderer {
            gpu,
            text,
            frame,
            fill,
            text_rendering: None,
        },
        translator,
        output: OutputState::new(),
    })
}

/// The window's title on `page`.
pub(crate) fn window_title(page: &Page) -> String {
    format!("luneui gallery - {}", page.title)
}

/// Puts the live tray in while the hosted page has a tray control (the `tray` page) and takes it
/// out otherwise. An item that cannot be made (no tray here, or the OS refused) is reported once
/// and the page draws without it. Returns whether the tray surface's flow must be reset.
fn sync_tray(tray: &mut Option<TrayHost>, wanted: bool, state: TrayState, scale: f64) -> bool {
    match (wanted, tray.is_some()) {
        (true, false) => {
            match TrayHost::new(state, scale) {
                Ok(host) => *tray = Some(host),
                Err(e) => eprintln!("tray: {e}"),
            }
            true
        }
        (false, true) => {
            *tray = None;
            true
        }
        _ => false,
    }
}

/// Puts the live panels up while the hosted page's switch is on and takes them down otherwise.
/// Panels that cannot be made are reported once and the page's switch goes off, so they are not
/// tried again every frame. Returns whether the panels' flow must be reset.
fn sync_panels(
    panels: &mut Option<PanelsHost>,
    state: &mut dyn PageState,
    event_loop: &ActiveEventLoop,
    r: &mut Renderer,
    anchor: &Window,
    start: Instant,
) -> bool {
    match (state.panels(), panels.is_some()) {
        (true, false) => {
            match PanelsHost::new(event_loop, r, anchor, start) {
                Ok(host) => *panels = Some(host),
                Err(e) => {
                    eprintln!("panels: {e}");
                    state.panels_closed();
                }
            }
            true
        }
        (false, true) => {
            *panels = None;
            true
        }
        _ => false,
    }
}

impl App {
    /// Hands the event loop the earliest of what the main window, the tray surface and the live
    /// panels ask for.
    fn set_flow(&self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(merge_flow(
            merge_flow(self.main_flow, self.pop_flow),
            self.panels_flow,
        ));
    }

    /// Draws the current page into the window and sets the loop's control flow from the frame's
    /// `Repaint`.
    fn render(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let page: &'static Page = &showcase::PAGE;
        let Some(shown) = &mut self.shown else {
            return Ok(());
        };
        let Shown {
            bridge,
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
        bridge.drain(translator);
        // The page's zoom multiplies the display's scale; the translator divides the pointer by
        // the same number, so the hits stay exact.
        let (zoom, speed) = self
            .state
            .as_ref()
            .map_or((1.0, 1.0), |st| (st.zoom(), st.time_scale()));
        let scale = window.scale_factor() * f64::from(zoom);
        translator.scale_factor(scale);
        let s = scale as f32;
        let now = self.clock.advance(self.start.elapsed(), speed);
        let mut input = translator.take(now);
        // The status item learns what bar it is on a few turns after it is made: ask again soon.
        let mut poll_tray = false;
        let root = RootInfo {
            title: window_title(page),
            scale,
        };
        let PageKind::Ui(ui_page) = page.kind else {
            return Ok(());
        };
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
        state.inject(&self.store, &mut input);
        if state.epoch() != self.strings_epoch {
            self.strings_epoch = state.epoch();
            self.strings = state.strings();
        }
        let config = FrameConfig {
            theme: state.theme().unwrap_or(THEME),
            tier: state.tier().unwrap_or(MotionTier::Full),
            // Ticks are app time: under slow motion a 30 a second grid would draw a few frames a
            // real second, so any other speed keeps every frame (D8).
            busy_cadence: if speed == 1.0 {
                Cadence::BUSY
            } else {
                Cadence::EveryFrame
            },
            strings: &self.strings,
            arg_format: &DisplayFormat,
            keymap: keymap(),
            shortcuts: &self.shortcuts,
        };
        let mut ui = Ui::begin(&mut self.store, &input, config, &mut r.text);
        ui.scope(page.id, |ui| state.draw(ui, &Stage::window()));
        let (display, mut out) = ui.finish();
        // A hosted page's window commands (the window page's caption buttons) are demonstrations:
        // its Close must not close the gallery. The gallery quits on its shortcut.
        out.commands.clear();
        if output.apply(window, &out, s) {
            event_loop.exit();
        }
        let tray_state = state.tray_state();
        if let Some(ts) = tray_state {
            self.tray_state = ts;
        }
        if sync_tray(
            &mut self.tray,
            tray_state.is_some(),
            self.tray_state,
            window.scale_factor(),
        ) {
            self.pop_flow = ControlFlow::Wait;
        }
        if let (Some(ts), Some(host)) = (tray_state, &mut self.tray) {
            host.sync(event_loop, ts);
            poll_tray = host.wants_poll();
        }
        if sync_panels(
            &mut self.panels,
            &mut **state,
            event_loop,
            r,
            window,
            self.start,
        ) {
            self.panels_flow = match &self.panels {
                Some(host) => host.flow(),
                None => ControlFlow::Wait,
            };
        }
        // Writes go to the OS; a read is answered with the next frame's input, which needs a frame.
        self.clipboard.service(&out, translator);
        if out
            .clipboard
            .iter()
            .any(|r| matches!(r, ClipboardRequest::Read))
        {
            window.request_redraw();
        }
        bridge.update(&out, &root);
        let repaint = out.repaint;
        let tweaks = state.frame_params().unwrap_or(FrameTweaks::DEFAULT);
        draw_with(r, &display, &view, tweaks.params(size, s))?;
        // The material is the page's to ask for; whether it took is the page's to hear.
        if let Some(wanted) = state.material().filter(|m| *m != self.material) {
            self.material = wanted;
            let result = material::apply(window, wanted);
            if let Err(e) = &result {
                eprintln!("material: {e}");
            }
            state.material_applied(result);
            window.request_redraw();
        }
        let speed = state.time_scale();
        window.pre_present_notify();
        surface.present(&r.gpu, frame);
        r.gpu.end_frame(&mut r.text);
        let (mut flow, redraw) = control_flow(self.clock.in_real_time(repaint, speed), self.start);
        if poll_tray && flow == ControlFlow::Wait {
            flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(50));
        }
        if redraw {
            window.request_redraw();
        }
        self.main_flow = flow;
        self.set_flow(event_loop);
        Ok(())
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.shown.is_some() {
            return;
        }
        let shown = match setup(event_loop, self.proxy.clone(), true) {
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

    /// A wake from the accessibility handlers (an action or a tree request waits for a frame), or
    /// a click on the tray item.
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::Wake => {
                if let Some(shown) = &self.shown {
                    shown.window.request_redraw();
                }
            }
            AppEvent::Tray(click) => {
                let now = self.start.elapsed();
                if let (Some(host), Some(shown)) = (&mut self.tray, &mut self.shown)
                    && let Err(e) =
                        host.click(event_loop, &mut shown.r, click, self.tray_state, now)
                {
                    eprintln!("tray: {e}");
                }
            }
        }
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        // A `Repaint::At` request: its time has come.
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            if let Some(shown) = &self.shown {
                shown.window.request_redraw();
            }
            if let Some(p) = self.tray.as_ref().and_then(TrayHost::popover) {
                p.window().request_redraw();
            }
            if let Some(host) = &mut self.panels {
                host.tick();
                self.panels_flow = host.flow();
                self.set_flow(event_loop);
            }
        }
    }

    /// The event loop is done: the tray item and its window go before it returns, the window
    /// through the guard that restores it.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.panels = None;
        self.tray = None;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        // After `exit()` winit still delivers queued events; ignore them so no frame is acquired.
        if event_loop.exiting() {
            return;
        }
        if self.tray.as_ref().and_then(TrayHost::window_id) == Some(id) {
            self.tray_window_event(event_loop, &event);
            return;
        }
        if self.panels.as_ref().is_some_and(|p| p.owns(id)) {
            self.panels_window_event(event_loop, id, &event);
            return;
        }
        // A window the host already dropped (a later `Destroyed` or `Focused(false)`) is not the
        // showcase window: its events never reach the main window's input or frame.
        let Some(shown) = &mut self.shown else { return };
        if shown.window.id() != id {
            return;
        }
        shown.bridge.window_event(&shown.window, &event);
        let action = match &event {
            WindowEvent::KeyboardInput { event: key, .. } if key.state == ElementState::Pressed => {
                shortcut(&key.logical_key, self.mods)
            }
            _ => None,
        };
        if let Some(action) = action {
            match action {
                Shortcut::Quit => event_loop.exit(),
                Shortcut::NextPage | Shortcut::PrevPage => {
                    let cue = if action == Shortcut::NextPage {
                        "next-page"
                    } else {
                        "prev-page"
                    };
                    if let Some(state) = &mut self.state {
                        state.cue(cue);
                    }
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

impl App {
    /// An event of a panel's window or the menu's: a frame, or input for it. A close takes the
    /// panels down and turns the page's switch off.
    fn panels_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        event: &WindowEvent,
    ) {
        let now = self.start.elapsed();
        let (Some(host), Some(shown)) = (&mut self.panels, &mut self.shown) else {
            return;
        };
        let outcome = if matches!(event, WindowEvent::RedrawRequested) {
            match host.render(event_loop, &mut shown.r, id, now) {
                Ok(outcome) => outcome,
                Err(e) => {
                    eprintln!("panels: {e}");
                    self.exit_code = 1;
                    event_loop.exit();
                    return;
                }
            }
        } else {
            host.window_event(id, event)
        };
        if outcome.close {
            self.panels = None;
            self.panels_flow = ControlFlow::Wait;
            if let Some(state) = &mut self.state {
                state.panels_closed();
            }
            shown.window.request_redraw();
        } else {
            self.panels_flow = host.flow();
        }
        self.set_flow(event_loop);
    }

    /// An event of the tray surface's window: a frame, or input for it.
    fn tray_window_event(&mut self, event_loop: &ActiveEventLoop, event: &WindowEvent) {
        let now = self.start.elapsed();
        let (Some(host), Some(shown)) = (&mut self.tray, &mut self.shown) else {
            return;
        };
        if !matches!(event, WindowEvent::RedrawRequested) {
            host.window_event(event, now);
            return;
        }
        match host.render(&mut shown.r, self.tray_state, now) {
            Ok(Some(drawn)) => {
                let (flow, redraw) = TrayHost::flow(&drawn, self.start);
                self.pop_flow = if drawn.hidden {
                    ControlFlow::Wait
                } else {
                    flow
                };
                if redraw && let Some(p) = host.popover() {
                    p.window().request_redraw();
                }
            }
            Ok(None) => self.pop_flow = ControlFlow::Wait,
            Err(e) => {
                eprintln!("tray surface: {e}");
                self.exit_code = 1;
                event_loop.exit();
            }
        }
        self.set_flow(event_loop);
    }
}

/// The earlier of two control flows: polling beats a deadline, a deadline beats waiting.
pub(crate) fn merge_flow(a: ControlFlow, b: ControlFlow) -> ControlFlow {
    match (a, b) {
        (ControlFlow::Poll, _) | (_, ControlFlow::Poll) => ControlFlow::Poll,
        (ControlFlow::WaitUntil(x), ControlFlow::WaitUntil(y)) => ControlFlow::WaitUntil(x.min(y)),
        (ControlFlow::WaitUntil(t), ControlFlow::Wait)
        | (ControlFlow::Wait, ControlFlow::WaitUntil(t)) => ControlFlow::WaitUntil(t),
        (ControlFlow::Wait, ControlFlow::Wait) => ControlFlow::Wait,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::NamedKey;

    fn ch(s: &str) -> Key {
        Key::Character(s.into())
    }

    #[test]
    fn bracket_keys_switch_pages_under_command_or_control() {
        for mods in [ModifiersState::SUPER, ModifiersState::CONTROL] {
            assert_eq!(shortcut(&ch("]"), mods), Some(Shortcut::NextPage));
            assert_eq!(shortcut(&ch("["), mods), Some(Shortcut::PrevPage));
            assert_eq!(shortcut(&ch("q"), mods), Some(Shortcut::Quit));
        }
        assert_eq!(shortcut(&ch("]"), ModifiersState::empty()), None);
    }

    #[test]
    fn the_arrow_keys_are_the_pages() {
        let none = ModifiersState::empty();
        assert_eq!(shortcut(&Key::Named(NamedKey::ArrowDown), none), None);
        assert_eq!(shortcut(&Key::Named(NamedKey::ArrowUp), none), None);
    }

    #[test]
    fn half_speed_halves_the_time_that_passes_and_a_change_never_jumps_it() {
        let ms = Duration::from_millis;
        let mut c = Clock::default();
        assert_eq!(c.advance(ms(100), 1.0), ms(100));
        assert_eq!(c.advance(ms(300), 0.5), ms(200));
        // Back to full speed: from where it was, not from the real time.
        assert_eq!(c.advance(ms(400), 1.0), ms(300));
        // Twice: 100 ms of real time is 200 ms of the page's.
        assert_eq!(c.advance(ms(500), 2.0), ms(500));
        // A frame at the same real time adds nothing, and the clock never runs back.
        assert_eq!(c.advance(ms(500), 0.1), ms(500));
        assert_eq!(c.advance(ms(400), 1.0), ms(500));
    }

    #[test]
    fn a_repaint_at_a_page_time_comes_due_at_the_real_time_it_maps_to() {
        let ms = Duration::from_millis;
        let mut c = Clock::default();
        c.advance(ms(1000), 0.5);
        // The page is at 500 ms; 100 ms later on its clock is 200 ms later on the real one.
        assert_eq!(
            c.in_real_time(Repaint::At(ms(600)), 0.5),
            Repaint::At(ms(1200))
        );
        assert_eq!(c.in_real_time(Repaint::Now, 0.5), Repaint::Now);
        assert_eq!(c.in_real_time(Repaint::Idle, 0.5), Repaint::Idle);
        // Already due: due now.
        assert_eq!(
            c.in_real_time(Repaint::At(ms(400)), 0.5),
            Repaint::At(ms(1000))
        );
    }

    /// Zoom is the translator's scale times the zoom: a point of the pointer in physical pixels
    /// lands on the logical pixel the page drew at, exactly, and the window is that much smaller
    /// in logical pixels.
    #[test]
    fn zoom_keeps_the_pointer_on_the_pixel_it_is_over() {
        use lune_ui::input::Event;
        use winit::dpi::PhysicalSize;
        let mut t = Translator::new(PhysicalSize::new(3200, 1800), 2.0);
        for zoom in [1.0_f64, 1.5, 2.0] {
            let scale = 2.0 * zoom;
            t.scale_factor(scale);
            // The physical pixel under which a logical point (100.5, 40) is drawn.
            t.cursor_moved(100.5 * scale, 40.0 * scale);
            let input = t.take(Duration::ZERO);
            assert_eq!(input.size, (3200.0 / scale as f32, 1800.0 / scale as f32));
            assert!((input.scale - scale as f32).abs() < f32::EPSILON);
            let Some(Event::PointerMoved { pos }) = input.events.first() else {
                panic!("a pointer move");
            };
            assert!(
                (pos.0 - 100.5).abs() < 1e-3 && (pos.1 - 40.0).abs() < 1e-3,
                "zoom {zoom}: {pos:?}"
            );
        }
    }

    #[test]
    fn the_earlier_flow_wins() {
        let (soon, later) = (Instant::now(), Instant::now() + Duration::from_secs(1));
        let until = ControlFlow::WaitUntil;
        assert_eq!(
            merge_flow(ControlFlow::Wait, ControlFlow::Wait),
            ControlFlow::Wait
        );
        assert_eq!(merge_flow(until(later), ControlFlow::Wait), until(later));
        assert_eq!(merge_flow(ControlFlow::Wait, until(later)), until(later));
        assert_eq!(merge_flow(until(later), until(soon)), until(soon));
        assert_eq!(
            merge_flow(until(soon), ControlFlow::Poll),
            ControlFlow::Poll
        );
    }

    #[test]
    fn escape_reaches_the_page() {
        let esc = Key::Named(NamedKey::Escape);
        assert_eq!(shortcut(&esc, ModifiersState::empty()), None);
    }
}
