//! The live docked panels (`docked-panels` page; `platform.desktop`, `platform.window-docking`,
//! `platform.window-group`, `platform.window-shape`, `overlay.menu-window`): three borderless,
//! shaped panel windows that snap, move, raise, minimise and stay on top as one, and the menu one
//! of them opens in a window of its own.
//!
//! [`PanelsHost`] owns the windows, each with a surface, a frame renderer, a `UiStore`, a
//! `Translator` and an `OutputState` of its own on the app's one `Gpu` and `TextCache`, as the
//! tray's `PopoverHost` does. The first panel is the leader of a [`WindowGroup`]; the others are
//! its members. The drawing is the page's (`pages/docked_panels.rs`): a scene and a window are the
//! same drawing.
//!
//! A drag does not use the toolkit's pointer deltas: windows move under the pointer, so the host
//! reads the OS pointer in desktop units on every pointer event and moves the set with
//! `desktop::move_windows` (D313). While the button is held every window of the set sees the same
//! pointer.
//!
//! App code: it reads the clock through the times it is given, reads the OS pointer and talks to
//! the OS. No gate opens a window; the page's scenes draw the same panels headless.

use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};

use lune_ui::desktop::{
    self, DesktopPoint, DesktopRect, Follow, Monitor, ShapeFollower, ShapeMode, monitor_at,
    place_popup, to_units,
};
use lune_ui::dock::{DockDrag, docked_sets, docked_with};
use lune_ui::frame::{FrameParams, FrameRenderer};
use lune_ui::group::{WindowGroup, owned_by};
use lune_ui::image::ImageStore;
use lune_ui::input::{
    FrameOutput, OutputState, Repaint, Translator, WindowCommand, control_flow, keymap,
};
use lune_ui::lunegit_dark::THEME;
use lune_ui::material::{self, Material};
use lune_ui::paint::{Painter, Rect};
use lune_ui::popover::PopoverWindow;
use lune_ui::shortcuts::Shortcuts;
use lune_ui::strings::{DisplayFormat, StringTable};
use lune_ui::surface::{Acquired, WindowSurface};
use lune_ui::ui::{FrameConfig, Ui, UiStore};
use lune_ui::widgets::popover::{PlaceAlign, Side};
use lune_ui::window::{panel_attributes, popover_attributes};

use crate::Renderer;
use crate::pages::{
    METER_FRAME_MS, METER_LIVE, MenuPick, MenuState, PanelFrame, PanelKind, PanelView, draw_menu,
    draw_panel, menu_pick, menu_window_size, meter_pixels, panel_shape,
};
use crate::scenes::{BLUR_SATURATION, BLUR_SIGMA};
use crate::window::merge_flow;

/// How far a dragged set reaches for an edge to snap to, in logical px (the first panel's window
/// converts it into desktop units).
const SNAP: f32 = 10.0;
/// Where the first panel opens, in logical px from its monitor's work area corner on each axis.
const ORIGIN: f32 = 120.0;
/// The period of the host's timer: the levels panel draws a new meter, and a panel whose window
/// lets the pointer through (macOS) is asked about the pointer again.
const TICK: Duration = Duration::from_millis(33);
/// The corner radius of the OS glass behind the menu.
const GLASS_RADIUS: f64 = 9.0;

/// What a window event or a frame asked of the app.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Outcome {
    /// The panels are to be dropped: a close control, a close request or the menu's last row.
    pub(crate) close: bool,
}

/// What the panels are showing and doing, outside their windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Flags {
    /// The first panel is rolled up to its strip.
    shaded: bool,
    /// Every panel is drawn, and sized, twice as large.
    doubled: bool,
    /// The group is kept above other windows.
    on_top: bool,
    /// A dragged set snaps to edges.
    snap: bool,
}

impl Flags {
    /// The factor the panels are drawn at over their base size.
    fn factor(self) -> f32 {
        if self.doubled { 2.0 } else { 1.0 }
    }
}

/// One panel window and everything it draws with.
struct Panel {
    // Fields drop in this order: the surface (it holds a handle to the window) first, then the
    // window handle, then the rest.
    surface: WindowSurface,
    window: Arc<Window>,
    kind: PanelKind,
    frame: FrameRenderer,
    store: UiStore,
    translator: Translator,
    output: OutputState,
    follower: ShapeFollower,
    /// How the platform gives this window its shape.
    mode: ShapeMode,
    /// The control flow this panel's last frame asked for.
    flow: ControlFlow,
    /// Frames drawn into this window.
    frames: u64,
}

/// A drag of the panels, from a press on a strip until the button is released.
#[derive(Debug)]
struct Drag {
    dock: DockDrag,
    /// Every monitor's work area: the edges a set snaps to besides the other panels.
    areas: Vec<DesktopRect>,
    /// How near an edge the set snaps, in desktop units; 0 turns snapping off.
    within: i32,
}

/// The panels: three windows in a group, and the menu window.
pub(crate) struct PanelsHost {
    // Fields drop in this order: the menu window, then the panels, then the group, whose own
    // members and leader follow (members first); then the rest.
    menu: Option<MenuWindow>,
    panels: Vec<Panel>,
    group: WindowGroup,
    strings: StringTable,
    shortcuts: Shortcuts,
    flags: Flags,
    drag: Option<Drag>,
    /// The OS pointer when the last primary press arrived, and in which window.
    press: Option<(WindowId, DesktopPoint)>,
    /// The primary button is down in one of the panels.
    button_down: bool,
    /// Whether the press now in progress moved windows, and whether the one before it did: neither
    /// counts as a click or a double click on a strip.
    moved: bool,
    prev_moved: bool,
    /// The meter frame last written, and how many writes were made and how many were refused.
    meter_frame: Option<u64>,
    meter_writes: u32,
    meter_errors: u32,
    /// The app's time origin, for the control flow the frames ask for.
    start: Instant,
    closing: bool,
    /// The group was minimised by a panel's control and not yet restored: the leader coming back
    /// from the OS's own un-minimise must call `WindowGroup::restore` (macOS orders the detached
    /// members out on a minimise).
    minimised: bool,
}

impl std::fmt::Debug for PanelsHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PanelsHost")
            .field("flags", &self.flags)
            .field("drag", &self.drag)
            .field("meter_writes", &self.meter_writes)
            .field("menu", &self.menu.is_some())
            .finish_non_exhaustive()
    }
}

/// The base size of `kind` in window-logical px at the factor `k`, as a command.
fn size_command(kind: PanelKind, shaded: bool, k: f32) -> WindowCommand {
    let (w, h) = kind.size(shaded);
    WindowCommand::SetSize {
        width: (w * k).round() as u32,
        height: (h * k).round() as u32,
    }
}

/// Whether the command is one a window of a group hands to the group: a window of a group is
/// never minimised, nor kept on top, alone (`lune_ui::group`).
fn goes_to_group(command: &WindowCommand) -> bool {
    matches!(
        command,
        WindowCommand::Minimize | WindowCommand::SetAlwaysOnTop(_)
    )
}

/// The origins of windows `heights` tall, stacked from `first` with equal left edges, each
/// touching the bottom of the one above.
fn stack_origins(first: DesktopPoint, heights: &[i32]) -> Vec<DesktopPoint> {
    let mut y = first.y;
    heights
        .iter()
        .map(|h| {
            let at = DesktopPoint::new(first.x, y);
            y = y.saturating_add(*h);
            at
        })
        .collect()
}

/// Where a window `offset` from the leader goes when the panels double (`doubling`) or halve: the
/// leader's origin plus twice the offset, or half of it.
fn rescaled_origin(leader: DesktopPoint, window: DesktopPoint, doubling: bool) -> DesktopPoint {
    let scale = |d: i32| if doubling { d.saturating_mul(2) } else { d / 2 };
    DesktopPoint::new(
        leader
            .x
            .saturating_add(scale(window.x.saturating_sub(leader.x))),
        leader
            .y
            .saturating_add(scale(window.y.saturating_sub(leader.y))),
    )
}

/// The frames of every panel, in panel order; `None` while the OS cannot say for one.
fn frames_of(panels: &[Panel]) -> Option<Vec<DesktopRect>> {
    panels
        .iter()
        .map(|p| desktop::window_frame(&p.window))
        .collect()
}

impl PanelsHost {
    /// Opens the three panels, hidden, on the monitor of `anchor` (the showcase window), docked in
    /// a stack, gives each its shape and shows them. `anchor` is any window of the app.
    pub(crate) fn new(
        event_loop: &ActiveEventLoop,
        r: &mut Renderer,
        anchor: &Window,
        start: Instant,
    ) -> Result<Self, String> {
        let monitors = desktop::monitors(anchor);
        let from = desktop::window_frame(anchor).map_or(DesktopPoint::default(), |f| f.origin());
        let monitor: Monitor = monitor_at(&monitors, from)
            .and_then(|i| monitors.get(i).copied())
            .ok_or("no monitor")?;
        let upl = monitor.units_per_logical;
        let first = DesktopPoint::new(
            monitor.work.x.saturating_add(to_units(ORIGIN, upl)),
            monitor.work.y.saturating_add(to_units(ORIGIN, upl)),
        );
        let flags = Flags {
            shaded: false,
            doubled: false,
            on_top: false,
            snap: true,
        };

        let attrs = |kind: PanelKind| {
            let (w, h) = kind.size(false);
            panel_attributes(
                &format!("luneui panels - {}", kind.title()),
                (f64::from(w), f64::from(h)),
                false,
            )
        };
        let make = |attrs| -> Result<Arc<Window>, String> {
            event_loop
                .create_window(attrs)
                .map(Arc::new)
                .map_err(|e| format!("panel window: {e}"))
        };
        let leader = make(attrs(PanelKind::Main))?;
        let mut group = WindowGroup::new(leader.clone());
        let mut windows = vec![leader];
        for kind in [PanelKind::Levels, PanelKind::Mini] {
            let member = make(group.member_attributes(attrs(kind)))?;
            group.add(member.clone())?;
            windows.push(member);
        }

        // Every window goes to the monitor first, so its scale settles there; then the stack is
        // laid out from the frames the OS really gave them.
        for w in &windows {
            desktop::set_window_origin(w, first);
        }
        let frames: Vec<DesktopRect> = windows
            .iter()
            .map(|w| desktop::window_frame(w))
            .collect::<Option<_>>()
            .ok_or("the OS cannot say where a panel is")?;
        let heights: Vec<i32> = frames.iter().map(|f| f.h).collect();
        let origins = stack_origins(first, &heights);
        let moves: Vec<(&Window, DesktopPoint)> = windows
            .iter()
            .zip(&origins)
            .map(|(w, at)| (&**w, *at))
            .collect();
        desktop::move_windows(&moves)?;

        let mut panels = Vec::new();
        for (kind, window) in PanelKind::ALL.into_iter().zip(windows) {
            let target = r
                .gpu
                .instance
                .create_surface(window.clone())
                .map_err(|e| format!("panel surface: {e}"))?;
            let size = window.inner_size();
            let surface = WindowSurface::new(&r.gpu, target, (size.width, size.height))?;
            let frame = FrameRenderer::new(&mut r.gpu, surface.format());
            let translator = Translator::new(size, window.scale_factor());
            panels.push(Panel {
                surface,
                window,
                kind,
                frame,
                store: UiStore::default(),
                translator,
                output: OutputState::new(),
                follower: ShapeFollower::default(),
                mode: ShapeMode::Unsupported,
                flow: ControlFlow::Wait,
                frames: 0,
            });
        }
        let mut host = Self {
            menu: None,
            panels,
            group,
            strings: StringTable::default(),
            shortcuts: Shortcuts::default(),
            flags,
            drag: None,
            press: None,
            button_down: false,
            moved: false,
            prev_moved: false,
            meter_frame: None,
            meter_writes: 0,
            meter_errors: 0,
            start,
            closing: false,
            minimised: false,
        };
        host.dock_all();
        for i in 0..host.panels.len() {
            host.reshape(i);
        }
        for p in host.panels.iter().rev() {
            p.window.set_visible(true);
            p.window.request_redraw();
        }
        Ok(host)
    }

    /// Tells the group which members are docked to the leader, from where the windows are.
    fn dock_all(&mut self) {
        let Some(frames) = frames_of(&self.panels) else {
            return;
        };
        let sets = docked_sets(&frames);
        for i in 1..self.panels.len() {
            let id = self.panels[i].window.id();
            if let Err(e) = self.group.set_docked(id, sets[i] == sets[0]) {
                eprintln!("panels: {e}");
            }
        }
    }

    fn panel_index(&self, id: WindowId) -> Option<usize> {
        self.panels.iter().position(|p| p.window.id() == id)
    }

    /// Whether `id` is one of the panels' windows or the menu's.
    pub(crate) fn owns(&self, id: WindowId) -> bool {
        self.panel_index(id).is_some() || self.menu.as_ref().is_some_and(|m| m.id() == id)
    }

    /// The factor the panels are drawn and sized at now.
    fn factor(&self) -> f32 {
        self.flags.factor()
    }

    /// The scale panel `i` draws at: its window's scale factor times the factor.
    fn draw_scale(&self, i: usize) -> f32 {
        self.panels[i].window.scale_factor() as f32 * self.factor()
    }

    /// Gives panel `i` its shape again, for its size and the scale it draws at now: the OS does
    /// not rescale a shape.
    fn reshape(&mut self, i: usize) {
        let scale = self.draw_scale(i);
        let kind = self.panels[i].kind;
        let shape = panel_shape(kind.size(self.flags.shaded));
        let p = &mut self.panels[i];
        match desktop::set_window_shape(&p.window, Some(&shape), scale) {
            Ok(mode) => p.mode = mode,
            Err(e) => eprintln!("panel shape: {e}"),
        }
    }

    /// Asks panel `i`'s follower about the pointer (a platform whose windows are shaped by the
    /// pointer only; elsewhere it does nothing).
    fn follow(&mut self, i: usize, pointer: Option<DesktopPoint>) {
        let scale = self.draw_scale(i);
        let kind = self.panels[i].kind;
        let shape = panel_shape(kind.size(self.flags.shaded));
        let p = &mut self.panels[i];
        if p.mode == ShapeMode::Pointer {
            let _: Follow = p.follower.update(&p.window, Some(&shape), scale, pointer);
        }
    }

    /// The host's timer fired: the levels panel draws its next meter, and the panels that let the
    /// pointer through ask where it is.
    pub(crate) fn tick(&mut self) {
        if self.panels.iter().any(|p| p.mode == ShapeMode::Pointer) {
            let pointer = desktop::pointer();
            for i in 0..self.panels.len() {
                self.follow(i, pointer);
            }
        }
        if let Some(p) = self.panels.get(PanelKind::Levels.index()) {
            p.window.request_redraw();
        }
        if let Some(m) = &self.menu {
            if m.visible {
                m.window().request_redraw();
            }
        }
    }

    /// The control flow of the panels' frames and the timer.
    pub(crate) fn flow(&self) -> ControlFlow {
        let menu = self.menu.as_ref().map(|m| m.flow);
        self.panels
            .iter()
            .map(|p| p.flow)
            .chain(menu)
            .fold(ControlFlow::WaitUntil(Instant::now() + TICK), merge_flow)
    }

    /// Feeds a window event of a panel or of the menu (not a redraw: that is [`Self::render`]).
    pub(crate) fn window_event(&mut self, id: WindowId, event: &WindowEvent) -> Outcome {
        if let Some(m) = &mut self.menu
            && m.id() == id
        {
            m.window_event(event);
            return self.outcome();
        }
        let Some(i) = self.panel_index(id) else {
            return Outcome::default();
        };
        match event {
            WindowEvent::CloseRequested => {
                self.closing = true;
                return self.outcome();
            }
            WindowEvent::Focused(true) => {
                if i == 0 && self.minimised {
                    self.minimised = false;
                    self.group.restore();
                }
                self.group.raise(id);
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => self.reshape(i),
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match state {
                ElementState::Pressed => {
                    self.button_down = true;
                    self.press = desktop::pointer().map(|p| (id, p));
                }
                ElementState::Released => {
                    self.button_down = false;
                    self.end_drag();
                }
            },
            WindowEvent::CursorMoved { .. } => self.drag_to_pointer(),
            _ => {}
        }
        if matches!(
            event,
            WindowEvent::CursorMoved { .. }
                | WindowEvent::CursorLeft { .. }
                | WindowEvent::MouseInput { .. }
        ) {
            self.follow(i, desktop::pointer());
        }
        if !matches!(event, WindowEvent::RedrawRequested) {
            let p = &mut self.panels[i];
            p.translator.window_event(event);
            p.window.request_redraw();
        }
        self.outcome()
    }

    fn outcome(&self) -> Outcome {
        Outcome {
            close: self.closing,
        }
    }

    // ---- the drag -----------------------------------------------------------------------

    /// Starts a drag from the press at `i`'s strip: the first panel takes every panel docked with
    /// it, any other goes alone.
    fn start_drag(&mut self, i: usize) {
        let Some((id, pointer)) = self.press else {
            return;
        };
        if self.panels[i].window.id() == id {
            self.begin_drag(i, pointer);
        }
    }

    /// Starts a drag of panel `i` with the pointer at `pointer` (desktop units): the path a
    /// press takes, with the pointer given. Returns whether a drag began.
    pub(crate) fn begin_drag(&mut self, i: usize, pointer: DesktopPoint) -> bool {
        let Some(frames) = frames_of(&self.panels) else {
            return false;
        };
        if self.drag.is_some() || i >= self.panels.len() {
            return false;
        }
        let moving = if i == 0 {
            docked_with(0, &frames)
        } else {
            vec![i]
        };
        let window = &self.panels[i].window;
        let upl = desktop::units_per_logical(window);
        let within = if self.flags.snap {
            to_units(SNAP, upl)
        } else {
            0
        };
        let areas = desktop::monitors(window).iter().map(|m| m.work).collect();
        self.drag = Some(Drag {
            dock: DockDrag::start(pointer, &frames, &moving),
            areas,
            within,
        });
        true
    }

    /// Moves the dragged set to where the OS pointer puts it now.
    fn drag_to_pointer(&mut self) {
        if let Some(pointer) = desktop::pointer() {
            self.drag_to(pointer);
        }
    }

    /// Moves the dragged set to where `pointer` puts it: one batch, and only when an origin
    /// changed.
    pub(crate) fn drag_to(&mut self, pointer: DesktopPoint) {
        let Some(drag) = &self.drag else {
            return;
        };
        let Some(frames) = frames_of(&self.panels) else {
            return;
        };
        let new = drag.dock.update(pointer, &frames, &drag.areas, drag.within);
        let moves: Vec<(&Window, DesktopPoint)> = new
            .iter()
            .filter(|(i, at)| frames[*i].origin() != *at)
            .map(|(i, at)| (&*self.panels[*i].window, *at))
            .collect();
        if moves.is_empty() {
            return;
        }
        match desktop::move_windows(&moves) {
            Ok(()) => self.moved = true,
            Err(e) => eprintln!("panels: {e}"),
        }
    }

    /// The button came up: the drag is over, and the group learns which members are docked now.
    pub(crate) fn end_drag(&mut self) {
        if self.drag.take().is_some() {
            self.dock_all();
        }
    }

    // ---- commands -----------------------------------------------------------------------

    /// Does what `command` asks for panel `i`: the group's two go to the group, the rest through
    /// the panel's own `OutputState`.
    fn route(&mut self, i: usize, command: WindowCommand) {
        if goes_to_group(&command) {
            match command {
                WindowCommand::Minimize => {
                    self.group.minimise();
                    self.minimised = true;
                }
                WindowCommand::SetAlwaysOnTop(on) => {
                    self.group.set_always_on_top(on);
                    self.flags.on_top = on;
                }
                _ => {}
            }
            return;
        }
        let scale = self.panels[i].window.scale_factor() as f32;
        let p = &mut self.panels[i];
        let mut out = FrameOutput::default();
        out.commands.push(command);
        if p.output.apply(&p.window, &out, scale) {
            self.closing = true;
        }
    }

    /// Rolls the first panel up to its strip, or down again.
    pub(crate) fn toggle_shade(&mut self) {
        self.flags.shaded = !self.flags.shaded;
        let command = size_command(PanelKind::Main, self.flags.shaded, self.factor());
        self.route(0, command);
        self.reshape(0);
    }

    /// Doubles every panel, or halves them again: each is resized, and the others are put at the
    /// leader's origin plus twice (or half) their offset in one batch.
    fn toggle_double(&mut self) {
        let frames = frames_of(&self.panels);
        self.flags.doubled = !self.flags.doubled;
        for i in 0..self.panels.len() {
            let kind = self.panels[i].kind;
            let command = size_command(kind, self.flags.shaded, self.factor());
            self.route(i, command);
        }
        if let Some(frames) = frames {
            let leader = frames[0].origin();
            let doubling = self.flags.doubled;
            let moves: Vec<(&Window, DesktopPoint)> = self
                .panels
                .iter()
                .zip(&frames)
                .skip(1)
                .map(|(p, f)| (&*p.window, rescaled_origin(leader, f.origin(), doubling)))
                .collect();
            if let Err(e) = desktop::move_windows(&moves) {
                eprintln!("panels: {e}");
            }
        }
        for i in 0..self.panels.len() {
            self.reshape(i);
        }
    }

    // ---- frames -------------------------------------------------------------------------

    /// Writes the meter picture for the frame `now` falls in, once per frame, before any panel
    /// draws it.
    fn update_meter(&mut self, r: &mut Renderer, now: Duration) {
        let frame = now.as_millis() as u64 / METER_FRAME_MS;
        if self.meter_frame == Some(frame) {
            return;
        }
        self.meter_frame = Some(frame);
        match r
            .gpu
            .update(METER_LIVE, (0, 0), &meter_pixels(frame * METER_FRAME_MS))
        {
            Ok(()) => self.meter_writes += 1,
            Err(e) => {
                if self.meter_errors == 0 {
                    eprintln!("panels: meter: {e}");
                }
                self.meter_errors += 1;
            }
        }
    }

    /// Draws one frame of the panel or the menu in `id`. `now` is the app's time.
    pub(crate) fn render(
        &mut self,
        event_loop: &ActiveEventLoop,
        r: &mut Renderer,
        id: WindowId,
        now: Duration,
    ) -> Result<Outcome, String> {
        if self.menu.as_ref().is_some_and(|m| m.id() == id) {
            return self.render_menu(r, now);
        }
        let Some(i) = self.panel_index(id) else {
            return Ok(Outcome::default());
        };
        self.update_meter(r, now);
        let kind = self.panels[i].kind;
        let view = PanelView {
            kind,
            shaded: self.flags.shaded,
            on_top: self.flags.on_top,
            meter: METER_LIVE,
        };
        let k = self.factor();
        let start = self.start;
        let drawn = self.panels[i].draw(r, &self.strings, &self.shortcuts, &view, now, k, start)?;
        let Some((frame, commands)) = drawn else {
            return Ok(self.outcome());
        };
        for command in commands {
            self.route(i, command);
        }
        self.react(event_loop, r, i, frame);
        Ok(self.outcome())
    }

    /// What panel `i`'s frame reported.
    fn react(&mut self, event_loop: &ActiveEventLoop, r: &mut Renderer, i: usize, f: PanelFrame) {
        if f.press_started {
            self.prev_moved = self.moved;
            self.moved = false;
            if self.button_down {
                self.start_drag(i);
            }
        }
        if f.minimise {
            self.route(i, WindowCommand::Minimize);
        }
        if f.close {
            self.route(i, WindowCommand::Close);
        }
        if f.zoom {
            self.toggle_double();
        }
        let kind = self.panels[i].kind;
        if f.double_clicked && kind == PanelKind::Main && !self.moved && !self.prev_moved {
            self.toggle_shade();
        }
        if f.secondary_clicked && kind == PanelKind::Mini {
            if let Some(at) = desktop::pointer() {
                if let Err(e) = self.open_menu(event_loop, r, at) {
                    eprintln!("panels: menu: {e}");
                }
            }
        }
    }

    // ---- the menu -----------------------------------------------------------------------

    fn menu_state(&self) -> MenuState {
        MenuState {
            on_top: self.flags.on_top,
            doubled: self.flags.doubled,
            snap: self.flags.snap,
        }
    }

    /// Opens the menu in a window of its own at the pointer `at` (desktop units): the window is
    /// sized first, then moved to [`place_popup`]'s answer, and shown.
    pub(crate) fn open_menu(
        &mut self,
        event_loop: &ActiveEventLoop,
        r: &mut Renderer,
        at: DesktopPoint,
    ) -> Result<DesktopRect, String> {
        let mini = self.panels[PanelKind::Mini.index()].window.clone();
        let state = self.menu_state();
        let logical = {
            let win = Rect::new(0.0, 0.0, 4096.0, 4096.0);
            let scale = mini.scale_factor() as f32;
            let mut p: Painter<'_, ()> = Painter::new(scale, win, &mut r.text, None, THEME);
            menu_window_size(&mut p, state)
        };
        let monitors = desktop::monitors(&mini);
        let monitor = monitor_at(&monitors, at)
            .and_then(|i| monitors.get(i))
            .ok_or("no monitor")?;
        let placed = place_popup(
            DesktopRect::new(at.x, at.y, 0, 0),
            logical,
            Side::Bottom,
            PlaceAlign::Start,
            0.0,
            monitor,
        )
        .ok_or("the menu cannot be placed")?;
        if self.menu.is_none() {
            self.menu = Some(MenuWindow::new(event_loop, &mut r.gpu, &mini, logical)?);
        }
        if let Some(m) = &mut self.menu {
            m.open(logical, placed.origin());
        }
        Ok(placed)
    }

    /// The menu's window, once it has been made.
    pub(crate) fn menu_window(&self) -> Option<&Arc<Window>> {
        self.menu.as_ref().map(MenuWindow::window)
    }

    /// Whether the menu is shown and has not been closed.
    pub(crate) fn menu_open(&self) -> bool {
        self.menu.as_ref().is_some_and(|m| m.open && m.visible)
    }

    /// Closes the menu as a dismissal does: its exit plays and the window hides once idle.
    pub(crate) fn dismiss_menu(&mut self) {
        if let Some(m) = &mut self.menu {
            m.open = false;
            m.window().request_redraw();
        }
    }

    /// The window of panel `i`.
    pub(crate) fn window(&self, i: usize) -> &Arc<Window> {
        &self.panels[i].window
    }

    /// The group the panels are in.
    pub(crate) fn group(&self) -> &WindowGroup {
        &self.group
    }

    /// Every panel's frame in desktop units, in panel order; `None` while the OS cannot say.
    pub(crate) fn frames(&self) -> Option<Vec<DesktopRect>> {
        frames_of(&self.panels)
    }

    /// The distance, in desktop units, within which a drag of panel `i` snaps.
    pub(crate) fn snap_within(&self, i: usize) -> i32 {
        to_units(SNAP, desktop::units_per_logical(&self.panels[i].window))
    }

    /// Frames drawn into panel `i`'s window so far.
    pub(crate) fn frames_drawn(&self, i: usize) -> u64 {
        self.panels[i].frames
    }

    /// How many meter pictures were written and how many writes were refused.
    pub(crate) fn meter_writes(&self) -> (u32, u32) {
        (self.meter_writes, self.meter_errors)
    }

    fn render_menu(&mut self, r: &mut Renderer, now: Duration) -> Result<Outcome, String> {
        let state = self.menu_state();
        let start = self.start;
        let Some(menu) = &mut self.menu else {
            return Ok(self.outcome());
        };
        let drawn = menu.render(r, state, now, start)?;
        if let Some(pick) = drawn.and_then(|d| d.picked).and_then(menu_pick) {
            match pick {
                MenuPick::OnTop => {
                    let on = !self.flags.on_top;
                    self.route(0, WindowCommand::SetAlwaysOnTop(on));
                }
                MenuPick::Double => self.toggle_double(),
                MenuPick::Snap => self.flags.snap = !self.flags.snap,
                MenuPick::Close => self.closing = true,
            }
        }
        Ok(self.outcome())
    }
}

impl Panel {
    /// Draws the panel into its window: returns what the frame reported and the window commands
    /// it asked for, or `None` when no frame was drawn.
    #[allow(clippy::too_many_arguments)]
    fn draw(
        &mut self,
        r: &mut Renderer,
        strings: &StringTable,
        shortcuts: &Shortcuts,
        view: &PanelView,
        now: Duration,
        k: f32,
        start: Instant,
    ) -> Result<Option<(PanelFrame, Vec<WindowCommand>)>, String> {
        let window = self.window.clone();
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
        let target = frame.texture.create_view(&Default::default());
        let size = self.surface.size();
        // The panel draws at a factor over its base size: the translator reads the pointer at the
        // same scale, so the layout stays the base one.
        let scale = window.scale_factor() as f32 * k;
        self.translator.scale_factor(f64::from(scale));
        let input = self.translator.take(now);
        let config = FrameConfig {
            theme: THEME,
            tier: lune_ui::motion::MotionTier::Full,
            busy_cadence: lune_ui::motion::Cadence::BUSY,
            strings,
            arg_format: &DisplayFormat,
            keymap: keymap(),
            shortcuts,
        };
        let mut ui = Ui::begin(&mut self.store, &input, config, &mut r.text);
        let drawn = draw_panel(&mut ui, (0.0, 0.0), view);
        let (display, mut out) = ui.finish();
        let commands = std::mem::take(&mut out.commands);
        self.output.apply(&window, &out, scale);
        let params = FrameParams {
            size,
            scale,
            blur_sigma: BLUR_SIGMA,
            saturation: BLUR_SATURATION,
            ..FrameParams::default()
        };
        self.frame
            .draw_with(&mut r.gpu, &mut r.text, &display, &target, params, &mut [])?;
        window.pre_present_notify();
        self.surface.present(&r.gpu, frame);
        r.gpu.end_frame(&mut r.text);
        self.frames += 1;
        let (flow, redraw) = control_flow(out.repaint, start);
        self.flow = flow;
        if redraw {
            window.request_redraw();
        }
        Ok(Some((drawn, commands)))
    }
}

// ---- the menu window ------------------------------------------------------------------------

/// What one menu frame did.
#[derive(Debug, Clone, Copy)]
struct MenuDrawn {
    /// The index of the row picked, if one was.
    picked: Option<usize>,
}

/// The window the menu fills: popover attributes, owned by the small panel's window, drawn with
/// `Ui::menu_window`. Created on the first open and reused after; it hides once the menu has
/// closed and its frame has gone idle.
struct MenuWindow {
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
    /// The menu is shown; once it is cleared its exit plays and the window hides when idle.
    open: bool,
    visible: bool,
    flow: ControlFlow,
}

impl MenuWindow {
    /// A hidden window of `size` logical px, owned by `owner`.
    fn new(
        event_loop: &ActiveEventLoop,
        gpu: &mut lune_ui::gpu::Gpu,
        owner: &Window,
        size: (f32, f32),
    ) -> Result<Self, String> {
        let attrs = owned_by(
            popover_attributes((f64::from(size.0), f64::from(size.1))),
            owner,
        );
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .map_err(|e| format!("menu window: {e}"))?,
        );
        let guard = PopoverWindow::wrap(window.clone())?;
        if let Err(e) = material::apply_rounded(&window, Material::Vibrancy, GLASS_RADIUS) {
            eprintln!("menu glass: {e}");
        }
        let target = gpu
            .instance
            .create_surface(window.clone())
            .map_err(|e| format!("menu surface: {e}"))?;
        let inner = window.inner_size();
        let surface = WindowSurface::new(gpu, target, (inner.width, inner.height))?;
        let frame = FrameRenderer::new(gpu, surface.format());
        let translator = Translator::new(inner, window.scale_factor());
        Ok(Self {
            surface,
            guard,
            frame,
            store: UiStore::default(),
            translator,
            output: OutputState::new(),
            strings: StringTable::default(),
            shortcuts: Shortcuts::default(),
            open: false,
            visible: false,
            flow: ControlFlow::Wait,
        })
    }

    fn window(&self) -> &Arc<Window> {
        self.guard.window()
    }

    fn id(&self) -> WindowId {
        self.window().id()
    }

    /// Sizes the window, then moves it, then shows it: macOS keeps a resized window's bottom edge
    /// where it was, so a move made before the resize lands a different top edge.
    fn open(&mut self, logical: (f32, f32), origin: DesktopPoint) {
        let window = self.window().clone();
        let _ =
            window.request_inner_size(LogicalSize::new(f64::from(logical.0), f64::from(logical.1)));
        desktop::set_window_origin(&window, origin);
        self.open = true;
        if !self.visible {
            self.guard.show();
            self.visible = true;
        }
        window.request_redraw();
    }

    fn window_event(&mut self, event: &WindowEvent) {
        // The window has no close button, so a close request is ignored. A resize reaches the
        // surface in `render`, which compares it with the window's size.
        if !matches!(
            event,
            WindowEvent::CloseRequested | WindowEvent::RedrawRequested
        ) {
            self.translator.window_event(event);
            self.window().request_redraw();
        }
    }

    /// Draws one frame, if the window is shown.
    fn render(
        &mut self,
        r: &mut Renderer,
        state: MenuState,
        now: Duration,
        start: Instant,
    ) -> Result<Option<MenuDrawn>, String> {
        if !self.visible {
            return Ok(None);
        }
        let window = self.window().clone();
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
        let target = frame.texture.create_view(&Default::default());
        let size = self.surface.size();
        let scale = window.scale_factor() as f32;
        let input = self.translator.take(now);
        let config = FrameConfig {
            theme: THEME,
            tier: lune_ui::motion::MotionTier::Full,
            busy_cadence: lune_ui::motion::Cadence::BUSY,
            strings: &self.strings,
            arg_format: &DisplayFormat,
            keymap: keymap(),
            shortcuts: &self.shortcuts,
        };
        let mut ui = Ui::begin(&mut self.store, &input, config, &mut r.text);
        let mut response = lune_ui::widgets::menu::MenuResponse::default();
        if self.open {
            ui.scope("panels-menu", |ui| {
                let area = ui.painter().window;
                response = draw_menu(ui, area, state);
            });
        }
        let (display, out) = ui.finish();
        self.output.apply(&window, &out, scale);
        let params = FrameParams {
            size,
            scale,
            blur_sigma: BLUR_SIGMA,
            saturation: BLUR_SATURATION,
            ..FrameParams::default()
        };
        self.frame
            .draw_with(&mut r.gpu, &mut r.text, &display, &target, params, &mut [])?;
        window.pre_present_notify();
        self.surface.present(&r.gpu, frame);
        r.gpu.end_frame(&mut r.text);
        if response.dismissed || response.picked.is_some() {
            self.open = false;
        }
        // Closed and still: the exit has played out, so the window can go.
        let hidden = !self.open && out.repaint == Repaint::Idle;
        if hidden {
            self.guard.hide();
            self.visible = false;
        }
        let (flow, redraw) = control_flow(out.repaint, start);
        self.flow = if hidden { ControlFlow::Wait } else { flow };
        if redraw && !hidden {
            window.request_redraw();
        }
        Ok(Some(MenuDrawn {
            picked: response.picked,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lune_ui::dock::touching;

    fn at(x: i32, y: i32) -> DesktopPoint {
        DesktopPoint::new(x, y)
    }

    #[test]
    fn the_stack_has_equal_left_edges_and_each_touches_the_one_above() {
        let heights = [174, 174, 48];
        let origins = stack_origins(at(180, 180), &heights);
        assert_eq!(origins, [at(180, 180), at(180, 354), at(180, 528)]);
        let rects: Vec<DesktopRect> = origins
            .iter()
            .zip(heights)
            .map(|(o, h)| DesktopRect::new(o.x, o.y, 413, h))
            .collect();
        assert!(touching(rects[0], rects[1]) && touching(rects[1], rects[2]));
        assert_eq!(docked_sets(&rects), [0, 0, 0]);
    }

    #[test]
    fn doubling_puts_a_window_at_twice_its_offset_from_the_leader_and_halving_undoes_it() {
        let leader = at(100, 100);
        assert_eq!(rescaled_origin(leader, at(100, 274), true), at(100, 448));
        assert_eq!(rescaled_origin(leader, at(100, 448), false), at(100, 274));
        // A detached window keeps its offset's sign.
        assert_eq!(rescaled_origin(leader, at(40, 60), true), at(-20, 20));
    }

    #[test]
    fn the_size_commands_are_the_panels_base_sizes_times_the_factor() {
        let size = |c| match c {
            WindowCommand::SetSize { width, height } => (width, height),
            other => panic!("{other:?}"),
        };
        assert_eq!(size(size_command(PanelKind::Main, true, 1.0)), (275, 14));
        assert_eq!(size(size_command(PanelKind::Main, false, 1.0)), (275, 116));
        assert_eq!(size(size_command(PanelKind::Main, false, 2.0)), (550, 232));
        assert_eq!(size(size_command(PanelKind::Levels, true, 1.0)), (275, 116));
        assert_eq!(size(size_command(PanelKind::Mini, false, 2.0)), (280, 64));
    }

    #[test]
    fn only_minimise_and_always_on_top_go_to_the_group() {
        assert!(goes_to_group(&WindowCommand::Minimize));
        assert!(goes_to_group(&WindowCommand::SetAlwaysOnTop(true)));
        assert!(goes_to_group(&WindowCommand::SetAlwaysOnTop(false)));
        assert!(!goes_to_group(&WindowCommand::Close));
        assert!(!goes_to_group(&WindowCommand::SetSize {
            width: 1,
            height: 1
        }));
        assert!(!goes_to_group(&WindowCommand::Redraw));
    }

    #[test]
    fn the_factor_doubles_only_when_the_panels_are_doubled() {
        let mut f = Flags {
            shaded: false,
            doubled: false,
            on_top: false,
            snap: true,
        };
        assert_eq!(f.factor(), 1.0);
        f.doubled = true;
        assert_eq!(f.factor(), 2.0);
    }
}
