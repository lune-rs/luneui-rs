//! `gallery --smoke-platform`: a one-shot check of what no gate can run. It needs a logged-in GUI
//! session. About two seconds long, it puts an item in the tray, reads its rect, opens the menu
//! surface through the same path a click takes (a synthetic [`TrayEvent`] in place of a real click),
//! draws at least ten frames of it, closes it, opens and closes it again, and tears everything down.
//! The accessibility bridge is built on a hidden window. It uses no clipboard.
//!
//! It prints each fact it measured. Exit 0 only if the frames drew, the frontmost app stayed the
//! same and teardown finished; exit 1 if any of those failed; exit 2 (with a line saying why) if
//! there is no GUI session to run in.
//!
//! App code: it reads the clock and runs the event loop.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::window::WindowId;

use lune_ui::popover::frontmost_app;
use lune_ui::tray::{TrayButton, TrayError, TrayEvent, TrayState};

use crate::popover::{AppEvent, TrayHost, event_loop, route_tray_events};
use crate::window::{Shown, setup};

/// The most the whole run may take.
const DEADLINE: Duration = Duration::from_secs(20);
/// How long the item may take to be placed by the OS.
const PLACE_WITHIN: Duration = Duration::from_secs(3);
/// Frames the first open must draw, and how long it stays open at least.
const FRAMES: u64 = 10;
const OPEN_FOR: Duration = Duration::from_millis(400);
/// How long a close may take to hide the window.
const HIDE_WITHIN: Duration = Duration::from_secs(2);

/// Where the run is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// Wait for the OS to place the item.
    Place,
    /// Open the surface and let it draw.
    Open1,
    /// Close it and wait for the window to hide.
    Close1,
    /// Open it again and let it draw a little.
    Open2,
    Close2,
    /// Drop everything and leave.
    Teardown,
}

/// What the run found out.
#[derive(Debug, Default)]
struct Facts {
    frames_first_open: u64,
    front_before: Option<String>,
    front_after: Vec<Option<String>>,
    is_key: Option<bool>,
    placed: bool,
    teardown: bool,
    failure: Option<String>,
}

struct Smoke {
    proxy: EventLoopProxy<AppEvent>,
    shown: Option<Shown>,
    host: Option<TrayHost>,
    step: Step,
    step_at: Instant,
    /// The popover's frame count when the current open began.
    frames_at: u64,
    start: Instant,
    facts: Facts,
    /// Set when the run cannot be done here at all (no GUI session, no tray).
    unavailable: Option<String>,
}

/// Runs the smoke. See the module docs for the exit status.
pub(crate) fn run() -> ExitCode {
    let event_loop = match event_loop(true) {
        Ok(l) => l,
        Err(e) => {
            println!("smoke: not run: {e} (it needs a logged-in GUI session)");
            return ExitCode::from(2);
        }
    };
    route_tray_events(event_loop.create_proxy());
    let now = Instant::now();
    let mut app = Smoke {
        proxy: event_loop.create_proxy(),
        shown: None,
        host: None,
        step: Step::Place,
        step_at: now,
        frames_at: 0,
        start: now,
        facts: Facts::default(),
        unavailable: None,
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        println!("smoke: not run: event loop: {e}");
        return ExitCode::from(2);
    }
    app.verdict()
}

impl Smoke {
    fn fail(&mut self, why: String) {
        if self.facts.failure.is_none() {
            self.facts.failure = Some(why);
        }
    }

    fn go(&mut self, step: Step) {
        self.step = step;
        self.step_at = Instant::now();
    }

    fn verdict(&self) -> ExitCode {
        if let Some(why) = &self.unavailable {
            println!("smoke: not run: {why}");
            return ExitCode::from(2);
        }
        let f = &self.facts;
        let front_ok = f.front_after.iter().all(|a| *a == f.front_before);
        println!(
            "smoke: frames drawn in the first open: {}",
            f.frames_first_open
        );
        println!(
            "smoke: frontmost app before: {:?}, after each step: {:?}",
            f.front_before, f.front_after
        );
        println!("smoke: popover was key while open: {:?}", f.is_key);
        println!("smoke: teardown finished: {}", f.teardown);
        let ok = f.failure.is_none() && f.frames_first_open >= FRAMES && front_ok && f.teardown;
        if let Some(why) = &f.failure {
            println!("smoke: FAILED: {why}");
        }
        if !front_ok {
            println!("smoke: FAILED: the frontmost app changed");
        }
        if ok {
            println!("smoke: ok");
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        }
    }

    /// A synthetic right click on the item, through the same path a real one takes.
    fn click(&mut self, event_loop: &ActiveEventLoop) {
        let now = self.start.elapsed();
        let (Some(host), Some(shown)) = (&mut self.host, &mut self.shown) else {
            return;
        };
        let Some(rect) = host.settled_rect() else {
            self.fail("the tray item has no rect".into());
            return;
        };
        let click = TrayEvent::Click {
            button: TrayButton::Right,
            rect,
        };
        if let Err(e) = host.click(event_loop, &mut shown.r, click, TrayState::Serving, now) {
            self.fail(format!("click: {e}"));
        }
    }

    fn popover_frames(&self) -> u64 {
        self.host
            .as_ref()
            .and_then(TrayHost::popover)
            .map_or(0, |p| p.frames())
    }

    fn popover_visible(&self) -> bool {
        self.host
            .as_ref()
            .and_then(TrayHost::popover)
            .is_some_and(|p| p.is_visible())
    }

    /// Moves the run on one notch when its step is done. Called every turn of the loop.
    fn advance(&mut self, event_loop: &ActiveEventLoop) {
        let waited = self.step_at.elapsed();
        if self.start.elapsed() > DEADLINE && self.facts.failure.is_none() {
            self.fail(format!(
                "not finished after {} s (step {:?})",
                DEADLINE.as_secs(),
                self.step
            ));
            self.go(Step::Teardown);
        }
        match self.step {
            Step::Place => {
                let rect = self.host.as_ref().and_then(TrayHost::settled_rect);
                if let Some(rect) = rect {
                    let dark = self.host.as_ref().and_then(|h| h.tray().bar_is_dark());
                    println!("smoke: tray rect {rect:?}, bar dark: {dark:?}");
                    if let Some(h) = &mut self.host {
                        h.sync(event_loop, TrayState::Serving);
                    }
                    self.facts.placed = true;
                    self.click(event_loop);
                    self.frames_at = self.popover_frames();
                    self.go(Step::Open1);
                } else if waited > PLACE_WITHIN {
                    self.fail("the tray item was not placed by the OS".into());
                    self.go(Step::Teardown);
                }
            }
            Step::Open1 => {
                let drawn = self.popover_frames() - self.frames_at;
                if drawn >= FRAMES && waited >= OPEN_FOR {
                    self.facts.frames_first_open = drawn;
                    self.facts.is_key = self
                        .host
                        .as_ref()
                        .and_then(TrayHost::popover)
                        .map(|p| p.window().has_focus());
                    self.facts.front_after.push(frontmost_app());
                    if let Some(p) = self.host.as_ref().and_then(TrayHost::popover) {
                        let w = p.window();
                        println!(
                            "smoke: surface window at {:?}, {:?} physical, scale {}",
                            w.outer_position().ok(),
                            w.outer_size(),
                            w.scale_factor()
                        );
                        for m in w.available_monitors() {
                            println!(
                                "smoke: monitor {:?} at {:?}, {:?}, scale {}",
                                m.name(),
                                m.position(),
                                m.size(),
                                m.scale_factor()
                            );
                        }
                    }
                    self.click(event_loop);
                    self.go(Step::Close1);
                } else if waited > Duration::from_secs(4) {
                    self.fail(format!("only {drawn} frames drew in the first open"));
                    self.go(Step::Teardown);
                }
            }
            Step::Close1 | Step::Close2 => {
                if !self.popover_visible() {
                    self.facts.front_after.push(frontmost_app());
                    if self.step == Step::Close1 {
                        self.click(event_loop);
                        self.frames_at = self.popover_frames();
                        self.go(Step::Open2);
                    } else {
                        self.go(Step::Teardown);
                    }
                } else if waited > HIDE_WITHIN {
                    self.fail("the window did not hide after a close".into());
                    self.go(Step::Teardown);
                }
            }
            Step::Open2 => {
                if self.popover_frames() - self.frames_at >= 5
                    && waited >= Duration::from_millis(200)
                {
                    self.click(event_loop);
                    self.go(Step::Close2);
                } else if waited > Duration::from_secs(4) {
                    self.fail("the second open drew too few frames".into());
                    self.go(Step::Teardown);
                }
            }
            Step::Teardown => {
                // The popover's window goes through its guard here, on the main thread, before
                // the window it was made beside.
                self.host = None;
                self.shown = None;
                self.facts.teardown = true;
                event_loop.exit();
            }
        }
    }
}

impl ApplicationHandler<AppEvent> for Smoke {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.shown.is_some() {
            return;
        }
        if event_loop.available_monitors().next().is_none() {
            self.unavailable =
                Some("no monitor: there is no GUI session (log in and retry)".into());
            event_loop.exit();
            return;
        }
        self.facts.front_before = frontmost_app();
        // The window is hidden and never shown: the bridge needs one, and the smoke wants nothing
        // on screen but the item and its surface.
        match setup(event_loop, self.proxy.clone(), false) {
            Ok(shown) => {
                println!("smoke: accessibility bridge built on a hidden window");
                self.shown = Some(shown);
            }
            Err(e) => {
                self.fail(format!("setup: {e}"));
                event_loop.exit();
                return;
            }
        }
        let scale = self.shown.as_ref().map_or(1.0, |s| s.window.scale_factor());
        match TrayHost::new(TrayState::Serving, scale) {
            Ok(host) => self.host = Some(host),
            Err(TrayError::Unsupported) => {
                self.unavailable = Some("this platform has no tray".into());
                self.shown = None;
                event_loop.exit();
            }
            Err(e) => {
                self.unavailable = Some(format!("the OS refused the tray item: {e}"));
                self.shown = None;
                event_loop.exit();
            }
        }
        self.step_at = Instant::now();
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.shown.is_none() || self.host.is_none() {
            return;
        }
        // Keep the item's picture right once the OS says what bar it is on.
        if let Some(h) = &mut self.host {
            h.sync(event_loop, TrayState::Serving);
        }
        self.advance(event_loop);
        // Frames are forced while the surface is up, so the count does not depend on how still it
        // gets.
        if let Some(p) = self.host.as_ref().and_then(TrayHost::popover)
            && p.is_visible()
        {
            p.window().request_redraw();
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: AppEvent) {
        // A real click during the run is not part of it: the run clicks for itself.
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.host = None;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if event_loop.exiting() {
            return;
        }
        let now = self.start.elapsed();
        if let (Some(host), Some(shown)) = (&mut self.host, &mut self.shown) {
            if host.window_id() == Some(id) {
                if matches!(event, WindowEvent::RedrawRequested) {
                    if let Err(e) = host.render(&mut shown.r, TrayState::Serving, now) {
                        self.fail(format!("render: {e}"));
                        self.go(Step::Teardown);
                    }
                } else {
                    host.window_event(&event, now);
                }
                return;
            }
            shown.bridge.window_event(&shown.window, &event);
        }
    }
}
