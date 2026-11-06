//! The live tray (`platform.tray-icon`, M10): a tray item, and the window its surfaces fill.
//!
//! [`TrayHost`] puts an item in the tray, turns its clicks into [`TrayAction`]s with the toolkit's
//! click rules and places the surface from the item's rect. The surface is [`PopoverHost`]: a second
//! window on the app's one `Gpu` and `TextCache`, with a surface, a frame renderer, a `UiStore`, a
//! `Translator` and an `OutputState` of its own, created on the first click and reused after.
//! The window is wrapped in a [`PopoverWindow`], whose guard restores it before it is freed, so
//! everything here keeps it alive until its own `Drop` and lets it drop on the main thread.
//!
//! App code: it reads the clock through the times it is given and talks to the OS. No gate opens
//! a window or a tray item; the smoke (`--smoke-platform`) and the `tray` page do.

use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use lune_ui::frame::{FrameParams, FrameRenderer};
use lune_ui::input::{OutputState, Repaint, Translator, control_flow, keymap};
use lune_ui::lunegit_dark::THEME;
use lune_ui::material::{self, Material};
use lune_ui::paint::{Painter, Rect};
use lune_ui::popover::PopoverWindow;
use lune_ui::shortcuts::Shortcuts;
use lune_ui::strings::{DisplayFormat, StringTable};
use lune_ui::surface::{Acquired, WindowSurface};
use lune_ui::tray::{
    GlyphTone, Tray, TrayAction, TrayButton, TrayError, TrayEvent, TrayImage, TrayPlatform,
    TrayRect, TrayState, TraySurface, TrayToggle, bar_from_tray, icon_bitmap, place_surface,
    set_event_handler, tray_surface,
};
use lune_ui::ui::{FrameConfig, Ui, UiStore};
use lune_ui::window::popover_attributes;

use crate::Renderer;
use crate::pages::{SurfaceFrame, draw_surface, surface_size};
use crate::scenes::{BLUR_SATURATION, BLUR_SIGMA};

/// What the app's event loop is woken with.
#[derive(Debug, Clone, Copy)]
pub(crate) enum AppEvent {
    /// A handler on another thread (accessibility) has something waiting for a frame.
    Wake,
    /// The tray item was clicked.
    Tray(TrayEvent),
}

/// The tray item's click handler: it only passes the event on to the event loop, as the toolkit's
/// docs ask. Call it before the first [`Tray::new`]: only the first call takes effect.
pub(crate) fn route_tray_events(proxy: winit::event_loop::EventLoopProxy<AppEvent>) {
    set_event_handler(move |event| {
        let _ = proxy.send_event(AppEvent::Tray(event));
    });
}

/// An event loop for the gallery. `quiet` keeps the app from taking the front when its first window
/// shows (macOS): the platform smoke checks that a popover took nothing from the user's app.
pub(crate) fn event_loop(quiet: bool) -> Result<EventLoop<AppEvent>, String> {
    let mut builder = EventLoop::<AppEvent>::with_user_event();
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        builder.with_activate_ignoring_other_apps(!quiet);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = quiet;
    builder.build().map_err(|e| format!("event loop: {e}"))
}

/// The OS whose click rules apply here.
const fn platform() -> TrayPlatform {
    if cfg!(target_os = "macos") {
        TrayPlatform::MacOs
    } else {
        TrayPlatform::Windows
    }
}

/// How many times a frame polls for the status item to be placed before the tone is left as
/// guessed: about 5 seconds.
const TONE_POLLS: u32 = 300;

/// How long the item's rect must hold still before it is taken as where the item is: the status
/// item slides into the bar for a moment after it is made.
const SETTLE: Duration = Duration::from_millis(300);

/// The corner radius of the OS glass behind a surface.
const GLASS_RADIUS: f64 = 9.0;

/// What a window event meant for the tray.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Signal {
    None,
    /// The window lost focus while a surface was open.
    FocusLost,
}

/// What one popover frame did.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Drawn {
    /// What the surface reported this frame.
    pub(crate) frame: SurfaceFrame,
    /// The frame's `Repaint`.
    pub(crate) repaint: Repaint,
    /// The window was hidden after this frame: the surface had closed and gone still.
    pub(crate) hidden: bool,
}

/// The window a tray surface fills.
pub(crate) struct PopoverHost {
    // Fields drop in this order: the surface (it holds a handle to the window) first, then the
    // guard, which restores the window, then the rest.
    surface: WindowSurface,
    guard: PopoverWindow,
    frame: FrameRenderer,
    store: UiStore,
    translator: Translator,
    output: OutputState,
    strings: StringTable,
    shortcuts: Shortcuts,
    /// What the window shows now.
    kind: TraySurface,
    /// The app calls the surface's widget while this is set; once it is cleared the widget's exit
    /// plays and the window hides when the frame is idle.
    open: bool,
    visible: bool,
    /// The window gained focus since the surface opened: only then can a loss of it be real (a
    /// late loss from a hide before the open is not).
    had_focus: bool,
    frames: u64,
}

impl std::fmt::Debug for PopoverHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PopoverHost")
            .field("kind", &self.kind)
            .field("open", &self.open)
            .field("visible", &self.visible)
            .field("frames", &self.frames)
            .finish_non_exhaustive()
    }
}

impl PopoverHost {
    /// A hidden popover window with a surface on `gpu`.
    pub(crate) fn new(
        event_loop: &ActiveEventLoop,
        gpu: &mut lune_ui::gpu::Gpu,
    ) -> Result<Self, String> {
        let window = Arc::new(
            event_loop
                .create_window(popover_attributes((340.0, 400.0)))
                .map_err(|e| format!("popover window: {e}"))?,
        );
        let guard = PopoverWindow::wrap(window.clone())?;
        if let Err(e) = material::apply_rounded(&window, Material::Vibrancy, GLASS_RADIUS) {
            eprintln!("popover glass: {e}");
        }
        let target = gpu
            .instance
            .create_surface(window.clone())
            .map_err(|e| format!("popover surface: {e}"))?;
        let size = window.inner_size();
        let surface = WindowSurface::new(gpu, target, (size.width, size.height))?;
        let frame = FrameRenderer::new(gpu, surface.format());
        let translator = Translator::new(size, window.scale_factor());
        Ok(Self {
            surface,
            guard,
            frame,
            store: UiStore::default(),
            translator,
            output: OutputState::new(),
            strings: StringTable::default(),
            shortcuts: Shortcuts::default(),
            kind: TraySurface::MenuUnderBar,
            open: false,
            visible: false,
            had_focus: false,
            frames: 0,
        })
    }

    pub(crate) fn window(&self) -> &Arc<Window> {
        self.guard.window()
    }

    pub(crate) fn id(&self) -> WindowId {
        self.window().id()
    }

    pub(crate) fn is_visible(&self) -> bool {
        self.visible
    }

    /// Frames drawn since the window was created.
    pub(crate) fn frames(&self) -> u64 {
        self.frames
    }

    /// The monitor the tray rect is on: the one whose area holds its centre, else the primary.
    fn screen_of(&self, tray: TrayRect) -> (Rect, f64) {
        let window = self.window();
        let centre = (tray.x + tray.w / 2.0, tray.y + tray.h / 2.0);
        let rect_of = |m: &winit::monitor::MonitorHandle| {
            let (p, s) = (m.position(), m.size());
            (
                Rect::new(p.x as f32, p.y as f32, s.width as f32, s.height as f32),
                m.scale_factor(),
            )
        };
        let found = window
            .available_monitors()
            .map(|m| rect_of(&m))
            .find(|(r, _)| r.contains((centre.0 as f32, centre.1 as f32)));
        found
            .or_else(|| window.primary_monitor().map(|m| rect_of(&m)))
            .unwrap_or((Rect::new(0.0, 0.0, 1920.0, 1080.0), window.scale_factor()))
    }

    /// Opens `kind` from the tray item at `tray`: sizes the window to the surface, places it
    /// ([`place_surface`]), and shows it. Reopening what is already shown moves and resizes it.
    pub(crate) fn open(
        &mut self,
        r: &mut Renderer,
        kind: TraySurface,
        tray: TrayRect,
        state: TrayState,
    ) {
        let (screen, scale) = self.screen_of(tray);
        let logical = {
            let win = Rect::new(0.0, 0.0, 4096.0, 4096.0);
            let mut p: Painter<'_, ()> = Painter::new(scale as f32, win, &mut r.text, None, THEME);
            surface_size(&mut p, kind, state)
        };
        let size = (
            (f64::from(logical.0) * scale).ceil() as f32,
            (f64::from(logical.1) * scale).ceil() as f32,
        );
        let tray_rect = Rect::new(tray.x as f32, tray.y as f32, tray.w as f32, tray.h as f32);
        let bar = bar_from_tray(screen, tray_rect);
        let placed = place_surface(kind, screen, bar, size);
        let window = self.window().clone();
        // Size first, then move: macOS keeps a resized window's bottom edge where it was, so a move
        // made before the resize lands a different top edge than the one asked for.
        let _ =
            window.request_inner_size(LogicalSize::new(f64::from(logical.0), f64::from(logical.1)));
        window.set_outer_position(PhysicalPosition::new(
            f64::from(placed.x).round(),
            f64::from(placed.y).round(),
        ));
        self.kind = kind;
        self.open = true;
        self.had_focus = false;
        if !self.visible {
            self.guard.show();
            self.visible = true;
        }
        window.request_redraw();
    }

    /// Asks the surface to close: its exit plays, then the window hides.
    pub(crate) fn close(&mut self) {
        self.open = false;
        self.window().request_redraw();
    }

    /// Feeds a window event of the popover. A focus loss while a surface is open is reported so the
    /// click rules can note it.
    pub(crate) fn window_event(&mut self, event: &WindowEvent) -> Signal {
        if matches!(event, WindowEvent::Focused(true)) && self.open {
            self.had_focus = true;
        }
        let lost = matches!(event, WindowEvent::Focused(false))
            && self.open
            && self.visible
            && self.had_focus;
        // The window has no close button, so a close request is ignored. A resize reaches the
        // surface in `render`, which compares it with the window's size.
        if !matches!(
            event,
            WindowEvent::CloseRequested | WindowEvent::RedrawRequested
        ) {
            self.translator.window_event(event);
            self.window().request_redraw();
        }
        if lost {
            Signal::FocusLost
        } else {
            Signal::None
        }
    }

    /// Draws one frame, if the window is shown. `now` is the app's time.
    pub(crate) fn render(
        &mut self,
        r: &mut Renderer,
        state: TrayState,
        now: Duration,
    ) -> Result<Option<Drawn>, String> {
        if !self.visible {
            return Ok(None);
        }
        let window = self.window().clone();
        // A size asked for a moment ago has not always arrived as an event yet.
        let inner = window.inner_size();
        if (inner.width, inner.height) != self.surface.size() {
            self.surface.resize(&r.gpu, inner.width, inner.height);
            self.translator.resized(inner);
        }
        let frame = match self.surface.acquire(&r.gpu)? {
            Acquired::Frame(frame) => frame,
            Acquired::Skip => return Ok(None),
            Acquired::Redraw => {
                window.request_redraw();
                return Ok(None);
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let size = self.surface.size();
        let scale = window.scale_factor() as f32;
        let input = self.translator.take(now);
        let config = FrameConfig {
            theme: THEME,
            tier: lune_ui::motion::MotionTier::Full,
            strings: &self.strings,
            arg_format: &DisplayFormat,
            keymap: keymap(),
            shortcuts: &self.shortcuts,
        };
        let mut ui = Ui::begin(&mut self.store, &input, config, &mut r.text);
        let mut drawn = SurfaceFrame::default();
        if self.open {
            let kind = self.kind;
            ui.scope("tray", |ui| drawn = draw_surface(ui, kind, state));
        }
        let (display, out) = ui.finish();
        self.output.apply(&window, &out);
        let params = FrameParams {
            size,
            scale,
            blur_sigma: BLUR_SIGMA,
            saturation: BLUR_SATURATION,
        };
        self.frame
            .draw_with(&mut r.gpu, &mut r.text, &display, &view, params, &mut [])?;
        window.pre_present_notify();
        self.surface.present(&r.gpu, frame);
        r.gpu.end_frame(&mut r.text);
        self.frames += 1;
        if drawn.dismissed || drawn.picked.is_some() {
            self.open = false;
        }
        // Closed and still: the exit has played out, so the window can go.
        let hidden = !self.open && out.repaint == Repaint::Idle;
        if hidden {
            self.guard.hide();
            self.visible = false;
        }
        Ok(Some(Drawn {
            frame: drawn,
            repaint: out.repaint,
            hidden,
        }))
    }
}

/// The item in the tray, its click rules and, once it is first opened, the window of its
/// surfaces.
#[derive(Debug)]
pub(crate) struct TrayHost {
    // The window goes before the item.
    popover: Option<PopoverHost>,
    tray: Tray,
    toggle: TrayToggle,
    /// The state and tone of the picture on the item.
    shown: (TrayState, GlyphTone),
    tone_known: bool,
    polls: u32,
    /// The on-screen rect last seen and since when, and the one that held for [`SETTLE`].
    watch: Option<(TrayRect, Instant)>,
    settled: Option<TrayRect>,
    /// 1 or 2: which bitmap the item gets.
    scale: u32,
}

/// The tooltip for a state: its name in words, the second channel for colour.
fn tooltip(state: TrayState) -> &'static str {
    match state {
        TrayState::Serving => "Wane - serving on :4646",
        TrayState::Degraded => "Wane - anthropic degraded",
        TrayState::Paused => "Wane - paused",
    }
}

fn image(state: TrayState, tone: GlyphTone, scale: u32) -> TrayImage {
    let bitmap = icon_bitmap(state, tone, scale, &THEME);
    TrayImage {
        width: bitmap.width,
        height: bitmap.height,
        rgba: bitmap.rgba,
    }
}

impl TrayHost {
    /// Puts the item in the tray. Call [`route_tray_events`] first.
    pub(crate) fn new(state: TrayState, display_scale: f64) -> Result<Self, TrayError> {
        let scale = if display_scale >= 1.5 { 2 } else { 1 };
        // A guess until the OS has placed the item and can say what bar it is on.
        let tone = GlyphTone::Light;
        let tray = Tray::new(image(state, tone, scale), tooltip(state))?;
        Ok(Self {
            popover: None,
            tray,
            toggle: TrayToggle::new(),
            shown: (state, tone),
            tone_known: false,
            polls: 0,
            watch: None,
            settled: None,
            scale,
        })
    }

    pub(crate) fn tray(&self) -> &Tray {
        &self.tray
    }

    pub(crate) fn popover(&self) -> Option<&PopoverHost> {
        self.popover.as_ref()
    }

    pub(crate) fn window_id(&self) -> Option<WindowId> {
        self.popover.as_ref().map(PopoverHost::id)
    }

    /// The item's rect, once it is where the OS will keep it. Right after the item is made macOS
    /// can report a rect with an area that lies off every screen (a measured one: x 0, y at the
    /// screen's height), and the bar's appearance with it is wrong; a rect whose centre is on no
    /// monitor is not taken.
    pub(crate) fn placed_rect(&self, event_loop: &ActiveEventLoop) -> Option<TrayRect> {
        let rect = self.tray.rect()?;
        let (cx, cy) = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
        event_loop
            .available_monitors()
            .any(|m| {
                let (p, s) = (m.position(), m.size());
                cx >= f64::from(p.x)
                    && cx < f64::from(p.x) + f64::from(s.width)
                    && cy >= f64::from(p.y)
                    && cy < f64::from(p.y) + f64::from(s.height)
            })
            .then_some(rect)
    }

    /// The rect the item held still at for [`SETTLE`], as of the last [`Self::sync`]; `None` while it
    /// is not yet placed or still moving.
    pub(crate) fn settled_rect(&self) -> Option<TrayRect> {
        self.settled
    }

    /// Watches the rect: it counts as settled once it has held still on a monitor for [`SETTLE`].
    fn observe(&mut self, event_loop: &ActiveEventLoop) {
        match (self.placed_rect(event_loop), self.watch) {
            (Some(now), Some((seen, since))) if now == seen => {
                if since.elapsed() >= SETTLE {
                    self.settled = Some(now);
                }
            }
            (Some(now), _) => {
                self.watch = Some((now, Instant::now()));
                self.settled = None;
            }
            (None, _) => {
                self.watch = None;
                self.settled = None;
            }
        }
    }

    /// Whether the item still waits to learn what bar it is on, so a frame should poll again.
    pub(crate) fn wants_poll(&self) -> bool {
        !self.tone_known && self.polls < TONE_POLLS
    }

    /// Puts the picture right: the state the page's control says, in the tone of the bar once the
    /// item can say what it is.
    pub(crate) fn sync(&mut self, event_loop: &ActiveEventLoop, state: TrayState) {
        let mut tone = self.shown.1;
        if !self.tone_known && self.polls < TONE_POLLS {
            self.polls += 1;
            self.observe(event_loop);
            let dark = self.settled.and_then(|_| self.tray.bar_is_dark());
            if let Some(dark) = dark {
                self.tone_known = true;
                tone = if dark {
                    GlyphTone::Light
                } else {
                    GlyphTone::Dark
                };
            }
        }
        if (state, tone) != self.shown {
            if let Err(e) = self.tray.set_image(image(state, tone, self.scale)) {
                eprintln!("tray image: {e}");
            }
            if state != self.shown.0 {
                let _ = self.tray.set_tooltip(tooltip(state));
            }
            self.shown = (state, tone);
        }
    }

    /// A click on the item, at `now`: the click rules say whether to open, close or ignore it.
    pub(crate) fn click(
        &mut self,
        event_loop: &ActiveEventLoop,
        r: &mut Renderer,
        event: TrayEvent,
        state: TrayState,
        now: Duration,
    ) -> Result<(), String> {
        let TrayEvent::Click { button, rect } = event;
        let left = match button {
            TrayButton::Left => true,
            TrayButton::Right => false,
            TrayButton::Other => return Ok(()),
        };
        match self.toggle.click(tray_surface(platform(), left), now) {
            TrayAction::Open(kind) => {
                if self.popover.is_none() {
                    self.popover = Some(PopoverHost::new(event_loop, &mut r.gpu)?);
                }
                if let Some(p) = &mut self.popover {
                    p.open(r, kind, rect, state);
                }
            }
            TrayAction::Close => {
                if let Some(p) = &mut self.popover {
                    p.close();
                }
            }
            TrayAction::None => {}
        }
        Ok(())
    }

    /// A window event for the popover (the caller checked the window).
    pub(crate) fn window_event(&mut self, event: &WindowEvent, now: Duration) {
        let Some(p) = &mut self.popover else { return };
        if p.window_event(event) == Signal::FocusLost {
            // The surface closes itself on focus loss; the click rules need to know it was this,
            // so the click that took the focus does not reopen it.
            self.toggle.focus_lost(now);
            p.close();
        }
    }

    /// Draws the popover's frame and tells the click rules how the surface went away.
    pub(crate) fn render(
        &mut self,
        r: &mut Renderer,
        state: TrayState,
        now: Duration,
    ) -> Result<Option<Drawn>, String> {
        let Some(p) = &mut self.popover else {
            return Ok(None);
        };
        let drawn = p.render(r, state, now)?;
        if let Some(d) = &drawn
            && (d.frame.dismissed || d.frame.picked.is_some())
        {
            self.toggle.closed();
        }
        Ok(drawn)
    }

    /// The control flow the popover's frame asks for, and whether to draw again now.
    pub(crate) fn flow(drawn: &Drawn, origin: std::time::Instant) -> (ControlFlow, bool) {
        control_flow(drawn.repaint, origin)
    }
}
