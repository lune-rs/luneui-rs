//! `gallery --smoke-panels`: a one-shot check of what no gate can run, on a real desktop: the docked
//! panels of the `docked-panels` page (`panels.rs`) and the menu window. It needs a logged-in GUI
//! session. It opens the three panel windows, drives them with synthetic desktop pointers through
//! the same code a real drag takes (`PanelsHost::begin_drag`, `drag_to`, `end_drag`), and asks the OS
//! where the windows are and which window it would hit at a point.
//!
//! It never moves the real cursor and never sends input: the only pointer it reads is
//! `desktop::pointer()`, once, to see that the OS answers.
//!
//! It prints one line per check, `smoke-panels: <check>: ok`, `FAILED (<why>)` or
//! `not checked here (<why>)`, then a verdict. Exit 0 when every applicable check passed, 1 when
//! one failed, 2 (with a line saying why) when there is no GUI session or no monitor to run in.
//! Every stage has a deadline, and so has the whole run.
//!
//! App code: it reads the clock and runs the event loop.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::window::WindowId;

use lune_ui::desktop::{self, DesktopPoint, DesktopRect, Monitor, monitor_at, to_units};
use lune_ui::dock::docked_sets;

use crate::page::register_images;
use crate::pages::{PanelKind, body_probe, notch_probe, panel_pictures};
use crate::panels::PanelsHost;
use crate::popover::{AppEvent, event_loop};
use crate::window::{Shown, setup};

/// The most the whole run may take.
const DEADLINE: Duration = Duration::from_secs(45);
/// The most one stage may take.
const STAGE_WITHIN: Duration = Duration::from_secs(8);
/// How long a state the OS changes by itself (a size, a minimise) may take to show.
const OS_WITHIN: Duration = Duration::from_secs(2);
/// The host's timer, as the app's event loop gives it.
const TICK: Duration = Duration::from_millis(33);

/// What one check found.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Check {
    Ok,
    Failed(String),
    NotHere(String),
}

/// Where the run is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// Wait for every panel to draw a frame.
    Settle,
    Docked,
    Monitor,
    GroupDrag,
    Snap,
    Detach,
    Shape,
    Redock,
    Shade,
    Minimise,
    OnTop,
    Menu,
    Meter,
    /// Drop everything and leave.
    Teardown,
}

impl Stage {
    const ORDER: [Stage; 14] = [
        Stage::Settle,
        Stage::Docked,
        Stage::Monitor,
        Stage::GroupDrag,
        Stage::Snap,
        Stage::Detach,
        Stage::Shape,
        Stage::Redock,
        Stage::Shade,
        Stage::Minimise,
        Stage::OnTop,
        Stage::Menu,
        Stage::Meter,
        Stage::Teardown,
    ];

    fn next(self) -> Stage {
        let at = Self::ORDER.iter().position(|s| *s == self).unwrap_or(13);
        Self::ORDER[(at + 1).min(13)]
    }
}

struct Smoke {
    proxy: EventLoopProxy<AppEvent>,
    shown: Option<Shown>,
    host: Option<PanelsHost>,
    stage: Stage,
    stage_at: Instant,
    /// The stage's sub-step, for a stage that acts, waits for the OS, then acts again.
    sub: u8,
    start: Instant,
    last_tick: Instant,
    /// The monitor the panels opened on.
    monitor: Option<Monitor>,
    /// The rect the menu was placed at.
    placed: Option<DesktopRect>,
    failures: u32,
    /// Set when the run cannot be done here at all (no GUI session).
    unavailable: Option<String>,
}

/// Runs the smoke. See the module docs for the exit status.
pub(crate) fn run() -> ExitCode {
    let event_loop = match event_loop(true) {
        Ok(l) => l,
        Err(e) => {
            println!("smoke-panels: not run: {e} (it needs a logged-in GUI session)");
            return ExitCode::from(2);
        }
    };
    let now = Instant::now();
    let mut app = Smoke {
        proxy: event_loop.create_proxy(),
        shown: None,
        host: None,
        stage: Stage::Settle,
        stage_at: now,
        sub: 0,
        start: now,
        last_tick: now,
        monitor: None,
        placed: None,
        failures: 0,
        unavailable: None,
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        println!("smoke-panels: not run: event loop: {e}");
        return ExitCode::from(2);
    }
    app.verdict()
}

/// The desktop point at `logical` px from the top left of panel `i`, whose frame is `frame`.
fn point_on(host: &PanelsHost, i: usize, frame: DesktopRect, logical: (f32, f32)) -> DesktopPoint {
    let upl = desktop::units_per_logical(host.window(i));
    DesktopPoint::new(
        frame.x.saturating_add(to_units(logical.0, upl)),
        frame.y.saturating_add(to_units(logical.1, upl)),
    )
}

fn shifted(p: DesktopPoint, dx: i32, dy: i32) -> DesktopPoint {
    DesktopPoint::new(p.x.saturating_add(dx), p.y.saturating_add(dy))
}

/// Whether `inner` lies within `outer`.
fn within(inner: DesktopRect, outer: DesktopRect) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.right() <= outer.right()
        && inner.bottom() <= outer.bottom()
}

/// The notch and the body of the panels `which`: the OS says the notch is not the window and the
/// body is (Windows, where the shape is a window region).
fn shape_check(host: &PanelsHost, which: &[usize], shaded: bool) -> Check {
    let Some(frames) = host.frames() else {
        return Check::Failed("the OS cannot say where the panels are".into());
    };
    for &i in which {
        let kind = PanelKind::ALL[i];
        let size = kind.size(shaded && kind == PanelKind::Main);
        let notch = point_on(host, i, frames[i], notch_probe(size));
        let body = point_on(host, i, frames[i], body_probe(size));
        let (at_notch, at_body) = (
            desktop::window_at(host.window(i), notch),
            desktop::window_at(host.window(i), body),
        );
        if at_notch.is_none() || at_body.is_none() {
            return Check::NotHere("the OS cannot say which window is at a point".into());
        }
        if !cfg!(target_os = "windows") {
            return Check::NotHere(
                "the shape is a window region only on Windows; elsewhere the pointer follows it"
                    .into(),
            );
        }
        if at_notch != Some(false) || at_body != Some(true) {
            return Check::Failed(format!(
                "{}: notch {notch:?} -> {at_notch:?} (want Some(false)), body {body:?} -> {at_body:?} (want Some(true))",
                kind.title()
            ));
        }
    }
    Check::Ok
}

impl Smoke {
    fn report(&mut self, name: &str, check: Check) {
        match check {
            Check::Ok => println!("smoke-panels: {name}: ok"),
            Check::Failed(why) => {
                self.failures += 1;
                println!("smoke-panels: {name}: FAILED ({why})");
            }
            Check::NotHere(why) => println!("smoke-panels: {name}: not checked here ({why})"),
        }
    }

    fn go(&mut self, stage: Stage) {
        self.stage = stage;
        self.stage_at = Instant::now();
        self.sub = 0;
    }

    fn verdict(&self) -> ExitCode {
        if let Some(why) = &self.unavailable {
            println!("smoke-panels: not run: {why}");
            return ExitCode::from(2);
        }
        if self.failures == 0 {
            println!("smoke-panels: ok");
            ExitCode::SUCCESS
        } else {
            println!("smoke-panels: FAILED ({} checks)", self.failures);
            ExitCode::from(1)
        }
    }

    /// Whether the stage has been going for longer than the OS may take to answer.
    fn waited(&self) -> bool {
        self.stage_at.elapsed() > OS_WITHIN
    }

    /// Runs the current stage once. Returns whether it is done.
    fn run(&mut self, event_loop: &ActiveEventLoop, host: &mut PanelsHost) -> bool {
        match self.stage {
            Stage::Settle => {
                let drawn = (0..3).all(|i| host.frames_drawn(i) >= 1);
                if !drawn && self.stage_at.elapsed() < STAGE_WITHIN {
                    return false;
                }
                if !drawn {
                    self.report(
                        "1 panels shown",
                        Check::Failed("a panel drew no frames".into()),
                    );
                }
                true
            }
            Stage::Docked => {
                let check = self.docked(host);
                self.report("1 panels shown and docked", check);
                true
            }
            Stage::Monitor => {
                let check = self.monitor_and_pointer(host);
                self.report("2 monitor and pointer", check);
                true
            }
            Stage::GroupDrag => {
                let check = self.group_drag(host);
                self.report("3 group drag", check);
                true
            }
            Stage::Snap => {
                let check = self.snap(host);
                self.report("4 snap to the work area", check);
                true
            }
            Stage::Detach => {
                let (detach, gap) = self.detach(host);
                self.report("5 detach", detach);
                self.report("5 window_at in the gap", gap);
                true
            }
            Stage::Shape => {
                let check = shape_check(host, &[0, 1, 2], false);
                self.report("6 shape", check);
                true
            }
            Stage::Redock => {
                let check = self.redock(host);
                self.report("7 re-dock", check);
                true
            }
            Stage::Shade => self.shade(host),
            Stage::Minimise => self.minimise(host),
            Stage::OnTop => {
                host.group().set_always_on_top(true);
                host.group().set_always_on_top(false);
                println!("smoke-panels: 10 always on top: applied (no getter)");
                true
            }
            Stage::Menu => self.menu(event_loop, host),
            Stage::Meter => {
                let (writes, errors) = host.meter_writes();
                let frames = host.frames_drawn(PanelKind::Levels.index());
                if frames < 10 && self.stage_at.elapsed() < OS_WITHIN {
                    return false;
                }
                let check = if frames >= 10 && writes >= 10 && errors == 0 {
                    Check::Ok
                } else {
                    Check::Failed(format!(
                        "levels drew {frames} frames, {writes} meter writes, {errors} refused"
                    ))
                };
                self.report("12 meter", check);
                true
            }
            Stage::Teardown => true,
        }
    }

    // ---- the checks ---------------------------------------------------------------------

    fn docked(&self, host: &PanelsHost) -> Check {
        let Some(f) = host.frames() else {
            return Check::Failed("the OS cannot say where the panels are".into());
        };
        let mut why = Vec::new();
        if f[1].y != f[0].bottom() {
            why.push(format!(
                "levels.y {} != main.bottom {}",
                f[1].y,
                f[0].bottom()
            ));
        }
        if f[2].y != f[1].bottom() {
            why.push(format!(
                "mini.y {} != levels.bottom {}",
                f[2].y,
                f[1].bottom()
            ));
        }
        if f[0].x != f[1].x || f[1].x != f[2].x {
            why.push(format!("left edges {} {} {}", f[0].x, f[1].x, f[2].x));
        }
        if docked_sets(&f) != [0, 0, 0] {
            why.push(format!("docked sets {:?}", docked_sets(&f)));
        }
        for i in 0..3 {
            if host.window(i).is_visible() == Some(false) {
                why.push(format!("panel {i} is not visible"));
            }
        }
        for i in 1..3 {
            if !host.group().is_docked(host.window(i).id()) {
                why.push(format!("the group does not record panel {i} as docked"));
            }
        }
        if host.group().members().count() != 2 {
            why.push("the group does not hold both members".into());
        }
        if why.is_empty() {
            Check::Ok
        } else {
            Check::Failed(why.join("; "))
        }
    }

    fn monitor_and_pointer(&mut self, host: &PanelsHost) -> Check {
        let monitors = desktop::monitors(host.window(0));
        let Some(frames) = host.frames() else {
            return Check::Failed("the OS cannot say where the panels are".into());
        };
        let Some(m) = monitor_at(&monitors, frames[0].origin()).and_then(|i| monitors.get(i))
        else {
            return Check::Failed("no monitor holds the first panel".into());
        };
        self.monitor = Some(*m);
        if !within(m.work, m.bounds) {
            return Check::Failed(format!(
                "work {:?} is not within bounds {:?}",
                m.work, m.bounds
            ));
        }
        let pointer = desktop::pointer();
        if cfg!(any(target_os = "windows", target_os = "macos")) && pointer.is_none() {
            return Check::Failed("pointer() is None".into());
        }
        Check::Ok
    }

    /// Drags panel `i` from the pointer at `from` by `(dx, dy)`: begin, move, end.
    fn drag(host: &mut PanelsHost, i: usize, from: DesktopPoint, dx: i32, dy: i32) -> bool {
        if !host.begin_drag(i, from) {
            return false;
        }
        host.drag_to(shifted(from, dx, dy));
        host.end_drag();
        true
    }

    fn group_drag(&mut self, host: &mut PanelsHost) -> Check {
        let (Some(m), Some(f0)) = (self.monitor, host.frames()) else {
            return Check::Failed("no frames".into());
        };
        let within_units = host.snap_within(0);
        let (dx, dy) = (137, 61);
        let right = f0.iter().map(|f| f.right()).max().unwrap_or(0);
        let bottom = f0.iter().map(|f| f.bottom()).max().unwrap_or(0);
        let room = f0[0].x + dx - m.work.x > within_units
            && m.work.right() - (right + dx) > within_units
            && f0[0].y + dy - m.work.y > within_units
            && m.work.bottom() - (bottom + dy) > within_units;
        if !room {
            return Check::NotHere("the work area is too small to move clear of its edges".into());
        }
        let from = point_on(host, 0, f0[0], (100.0, 6.0));
        if !Self::drag(host, 0, from, dx, dy) {
            return Check::Failed("no drag began".into());
        }
        let Some(f1) = host.frames() else {
            return Check::Failed("the OS cannot say where the panels are".into());
        };
        let bad: Vec<String> = (0..3)
            .filter(|&i| f1[i].origin() != shifted(f0[i].origin(), dx, dy) || f1[i].w != f0[i].w)
            .map(|i| format!("panel {i}: {:?} -> {:?}", f0[i], f1[i]))
            .collect();
        if bad.is_empty() {
            Check::Ok
        } else {
            Check::Failed(format!("not moved by ({dx}, {dy}): {}", bad.join("; ")))
        }
    }

    fn snap(&mut self, host: &mut PanelsHost) -> Check {
        let (Some(m), Some(f0)) = (self.monitor, host.frames()) else {
            return Check::Failed("no frames".into());
        };
        let reach = host.snap_within(0);
        let target = m.work.x + reach - 2;
        let dx = target - f0[0].x;
        let from = point_on(host, 0, f0[0], (100.0, 6.0));
        if !Self::drag(host, 0, from, dx, 0) {
            return Check::Failed("no drag began".into());
        }
        let Some(f1) = host.frames() else {
            return Check::Failed("the OS cannot say where the panels are".into());
        };
        if f1[0].x == m.work.x {
            Check::Ok
        } else {
            Check::Failed(format!(
                "main.x {} != work.x {} (within {reach}, the set went to {target})",
                f1[0].x, m.work.x
            ))
        }
    }

    fn detach(&mut self, host: &mut PanelsHost) -> (Check, Check) {
        let (Some(m), Some(f0)) = (self.monitor, host.frames()) else {
            let c = Check::Failed("no frames".into());
            return (c.clone(), c);
        };
        let (dx, dy) = (f0[1].w + 250, 150);
        let moved = f0[2].translated(dx, dy);
        if !within(moved, m.work) {
            let c = Check::NotHere("the work area is too small for a free spot".into());
            return (c.clone(), c);
        }
        let from = point_on(host, 2, f0[2], (60.0, 6.0));
        if !Self::drag(host, 2, from, dx, dy) {
            let c = Check::Failed("no drag began".into());
            return (c.clone(), c);
        }
        let Some(f1) = host.frames() else {
            let c = Check::Failed("the OS cannot say where the panels are".into());
            return (c.clone(), c);
        };
        let sets = docked_sets(&f1);
        let mut why = Vec::new();
        if f1[2].origin() == f0[2].origin() {
            why.push("mini did not move".to_string());
        }
        if f1[0] != f0[0] || f1[1] != f0[1] {
            why.push("main or levels moved".to_string());
        }
        if !(sets[0] == sets[1] && sets[2] != sets[0]) {
            why.push(format!("docked sets {sets:?}, want two"));
        }
        let detach = if why.is_empty() {
            Check::Ok
        } else {
            Check::Failed(why.join("; "))
        };
        // Where mini was is now bare desktop (or another app's window): none of ours.
        let gap = DesktopPoint::new(f0[2].x + f0[2].w / 2, f0[2].y + f0[2].h / 2);
        let answers: Vec<Option<bool>> = (0..3)
            .map(|i| desktop::window_at(host.window(i), gap))
            .collect();
        let gap_check = if answers.contains(&None) {
            Check::NotHere("the OS cannot say which window is at a point".into())
        } else if answers.iter().all(|a| *a == Some(false)) {
            Check::Ok
        } else {
            Check::Failed(format!(
                "window_at {gap:?} answered {answers:?}, want all Some(false)"
            ))
        };
        (detach, gap_check)
    }

    fn redock(&mut self, host: &mut PanelsHost) -> Check {
        let Some(f0) = host.frames() else {
            return Check::Failed("no frames".into());
        };
        let (dx, dy) = (f0[1].x + 3 - f0[2].x, f0[1].bottom() + 4 - f0[2].y);
        let from = point_on(host, 2, f0[2], (60.0, 6.0));
        if !Self::drag(host, 2, from, dx, dy) {
            return Check::Failed("no drag began".into());
        }
        let Some(f1) = host.frames() else {
            return Check::Failed("the OS cannot say where the panels are".into());
        };
        let sets = docked_sets(&f1);
        let recorded = host.group().is_docked(host.window(2).id());
        if f1[2].x == f1[1].x && f1[2].y == f1[1].bottom() && sets == [0, 0, 0] && recorded {
            Check::Ok
        } else {
            Check::Failed(format!(
                "mini {:?} under levels {:?}, sets {sets:?}, recorded docked {recorded}",
                f1[2], f1[1]
            ))
        }
    }

    /// Shade the first panel and read its height, then unshade and read it again; the notch holds
    /// after each.
    fn shade(&mut self, host: &mut PanelsHost) -> bool {
        let upl = desktop::units_per_logical(host.window(0));
        let want = |shaded: bool| to_units(PanelKind::Main.size(shaded).1, upl);
        let height = |host: &PanelsHost| host.frames().map(|f| f[0].h);
        match self.sub {
            0 => {
                host.toggle_shade();
                self.sub = 1;
                self.stage_at = Instant::now();
                false
            }
            1 => {
                if height(host) != Some(want(true)) {
                    if self.waited() {
                        let got = height(host);
                        self.report(
                            "8 shade",
                            Check::Failed(format!("main.h is {got:?}, want {}", want(true))),
                        );
                        return true;
                    }
                    return false;
                }
                let check = shape_check(host, &[0], true);
                self.report("8 shade: height and notch", check);
                host.toggle_shade();
                self.sub = 2;
                self.stage_at = Instant::now();
                false
            }
            _ => {
                if height(host) != Some(want(false)) {
                    if self.waited() {
                        let got = height(host);
                        self.report(
                            "8 unshade",
                            Check::Failed(format!("main.h is {got:?}, want {}", want(false))),
                        );
                        return true;
                    }
                    return false;
                }
                let check = shape_check(host, &[0], false);
                self.report("8 unshade: height and notch", check);
                true
            }
        }
    }

    /// Minimise the group and read the members' visibility, then restore it and read it again.
    fn minimise(&mut self, host: &mut PanelsHost) -> bool {
        if !cfg!(target_os = "windows") {
            host.group().minimise();
            host.group().restore();
            self.report(
                "9 minimise and restore",
                Check::NotHere("is_visible tells a minimised member only on Windows".into()),
            );
            return true;
        }
        let members_are =
            |host: &PanelsHost, v: bool| (1..3).all(|i| host.window(i).is_visible() == Some(v));
        match self.sub {
            0 => {
                host.group().minimise();
                self.sub = 1;
                self.stage_at = Instant::now();
                false
            }
            1 => {
                if members_are(host, false) {
                    host.group().restore();
                    self.sub = 2;
                    self.stage_at = Instant::now();
                } else if self.waited() {
                    let seen: Vec<_> = (1..3).map(|i| host.window(i).is_visible()).collect();
                    self.report(
                        "9 minimise",
                        Check::Failed(format!("members' is_visible {seen:?}, want Some(false)")),
                    );
                    host.group().restore();
                    return true;
                }
                false
            }
            _ => {
                if members_are(host, true) {
                    self.report("9 minimise and restore", Check::Ok);
                    true
                } else if self.waited() {
                    let seen: Vec<_> = (1..3).map(|i| host.window(i).is_visible()).collect();
                    self.report(
                        "9 restore",
                        Check::Failed(format!("members' is_visible {seen:?}, want Some(true)")),
                    );
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Open the menu at a pointer near the work area's bottom right corner, read where it landed,
    /// dismiss it and see it hide.
    fn menu(&mut self, event_loop: &ActiveEventLoop, host: &mut PanelsHost) -> bool {
        let Some(m) = self.monitor else {
            self.report("11 menu", Check::Failed("no monitor".into()));
            return true;
        };
        match self.sub {
            0 => {
                let at = DesktopPoint::new(m.work.right() - 4, m.work.bottom() - 4);
                let Some(shown) = &mut self.shown else {
                    self.report("11 menu", Check::Failed("no window".into()));
                    return true;
                };
                match host.open_menu(event_loop, &mut shown.r, at) {
                    Ok(placed) => self.placed = Some(placed),
                    Err(e) => {
                        self.report("11 menu", Check::Failed(format!("open: {e}")));
                        return true;
                    }
                }
                self.sub = 1;
                self.stage_at = Instant::now();
                false
            }
            1 => {
                let (Some(placed), Some(win)) = (self.placed, host.menu_window()) else {
                    self.report("11 menu", Check::Failed("no menu window".into()));
                    return true;
                };
                let frame = desktop::window_frame(win);
                let size_ok = frame.is_some_and(|f| (f.w, f.h) == (placed.w, placed.h));
                // Long enough for a late focus loss at creation to have dismissed it, if it would.
                if (!size_ok && !self.waited())
                    || self.stage_at.elapsed() < Duration::from_millis(500)
                {
                    return false;
                }
                let mini = host.frames().map(|f| f[2]);
                let mut why = Vec::new();
                match frame {
                    Some(f) => {
                        if !within(f, m.work) {
                            why.push(format!("frame {f:?} is not inside work {:?}", m.work));
                        }
                        if (f.w, f.h) != (placed.w, placed.h) {
                            why.push(format!(
                                "size ({}, {}) != place_popup's ({}, {})",
                                f.w, f.h, placed.w, placed.h
                            ));
                        }
                        if let Some(mini) = mini
                            && !(f.w > mini.w || f.h > mini.h)
                        {
                            why.push("the menu is not larger than mini".to_string());
                        }
                    }
                    None => why.push("the OS cannot say where the menu is".to_string()),
                }
                if !host.menu_open() {
                    why.push("the menu closed by itself within 500 ms of opening".to_string());
                }
                if why.is_empty() {
                    self.report("11 menu placed", Check::Ok);
                } else {
                    self.report("11 menu placed", Check::Failed(why.join("; ")));
                }
                host.dismiss_menu();
                self.sub = 2;
                self.stage_at = Instant::now();
                false
            }
            _ => {
                let visible = host.menu_window().and_then(|w| w.is_visible());
                if visible == Some(false) {
                    self.report("11 menu hides after a dismissal", Check::Ok);
                    true
                } else if self.waited() {
                    self.report(
                        "11 menu hides after a dismissal",
                        Check::Failed(format!("is_visible {visible:?}, want Some(false)")),
                    );
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Moves the run on one notch when its stage is done. Called every turn of the loop.
    fn advance(&mut self, event_loop: &ActiveEventLoop) {
        let Some(mut host) = self.host.take() else {
            return;
        };
        if self.last_tick.elapsed() >= TICK {
            self.last_tick = Instant::now();
            host.tick();
        }
        if self.start.elapsed() > DEADLINE && self.stage != Stage::Teardown {
            self.failures += 1;
            println!(
                "smoke-panels: FAILED (not finished after {} s, stage {:?})",
                DEADLINE.as_secs(),
                self.stage
            );
            self.go(Stage::Teardown);
        }
        if self.stage == Stage::Teardown {
            // The menu window, then the panels, then the group: the host's field order.
            drop(host);
            self.shown = None;
            event_loop.exit();
            return;
        }
        let done = self.run(event_loop, &mut host);
        if done {
            self.go(self.stage.next());
        } else if self.stage_at.elapsed() > STAGE_WITHIN {
            self.failures += 1;
            println!("smoke-panels: FAILED (stage {:?} timed out)", self.stage);
            self.go(self.stage.next());
        }
        self.host = Some(host);
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
        // The window is hidden and never shown: it is the app's window, and the panels open on its
        // monitor.
        let mut shown = match setup(event_loop, self.proxy.clone(), false) {
            Ok(shown) => shown,
            Err(e) => {
                self.failures += 1;
                println!("smoke-panels: FAILED (setup: {e})");
                event_loop.exit();
                return;
            }
        };
        if let Err(e) = register_images(&mut shown.r.gpu, panel_pictures()) {
            self.failures += 1;
            println!("smoke-panels: FAILED (pictures: {e})");
            event_loop.exit();
            return;
        }
        match PanelsHost::new(event_loop, &mut shown.r, &shown.window, self.start) {
            Ok(host) => {
                // On top while the checks run, so what the OS says is at a point is the panels.
                host.group().set_always_on_top(true);
                self.host = Some(host);
            }
            Err(e) if e == "no monitor" => {
                self.unavailable = Some(format!("{e}: there is no GUI session (log in and retry)"));
                event_loop.exit();
            }
            Err(e) => {
                self.failures += 1;
                println!("smoke-panels: FAILED (the panels did not open: {e})");
                event_loop.exit();
            }
        }
        self.shown = Some(shown);
        self.stage_at = Instant::now();
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.shown.is_none() || self.host.is_none() {
            return;
        }
        self.advance(event_loop);
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: AppEvent) {}

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.host = None;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if event_loop.exiting() {
            return;
        }
        let now = self.start.elapsed();
        let mut failed = None;
        if let (Some(host), Some(shown)) = (&mut self.host, &mut self.shown) {
            if host.owns(id) {
                if matches!(event, WindowEvent::RedrawRequested) {
                    if let Err(e) = host.render(event_loop, &mut shown.r, id, now) {
                        failed = Some(format!("render: {e}"));
                    }
                } else {
                    // A real close or click during the run is not part of it.
                    let _ = host.window_event(id, &event);
                }
            } else {
                shown.bridge.window_event(&shown.window, &event);
            }
        }
        if let Some(why) = failed {
            self.failures += 1;
            println!("smoke-panels: FAILED ({why})");
            self.go(Stage::Teardown);
        }
    }
}
