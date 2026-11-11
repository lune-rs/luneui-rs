//! `gallery --pace-probe`: times the frame pacing on the real renderer (D299). It needs a
//! logged-in GUI session. About 15 s long, it opens one visible window and draws a real toolkit
//! frame per redraw (a tab strip with a shining tab, a spinner and an indeterminate bar), in four
//! phases of 0.5 s warm-up and 3 s measured each:
//!
//! 1. `0.4.0`: every busy loop asks for the next frame at once, default surface options;
//! 2. `cadence`: busy loops tick at `Cadence::BUSY`, default surface options;
//! 3. `paced-fifo`: busy loops ask at once, the app puts each `Now` on a grid at the display's
//!    rate (`Repaint::paced`), default surface options;
//! 4. `paced-novsync`: as 3 under `PresentMode::AutoNoVsync`.
//!
//! It asks for a redraw only on its own schedule (what a frame's repaint maps to, and a deadline
//! wake) and forwards no input, so a stray event cannot add frames. Leave the window visible and
//! untouched: covering, minimising, moving it off screen or resizing it makes the run
//! inconclusive. Exit 0 held, 1 did not hold, 2 could not measure (no GUI session, a window that
//! was hidden or resized while measured, or an instrument check that failed). The verdict
//! ([`judge`]) is a pure function of the four phases' numbers; every bound is on something load
//! cannot move.
//!
//! App code: it reads the clock and runs the event loop.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};

use lune_ui::input::{control_flow, keymap};
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::{Cadence, MotionTier};
use lune_ui::paint::Rect;
use lune_ui::shortcuts::Shortcuts;
use lune_ui::strings::{DisplayFormat, StringTable};
use lune_ui::surface::{Acquired, SurfaceOptions};
use lune_ui::ui::{FrameConfig, Ui, UiStore};
use lune_ui::widgets::progress::ProgressOptions;
use lune_ui::widgets::spinner::SpinnerOptions;
use lune_ui::widgets::tab_strip::{Tab, TabKey, TabStripOptions};
use lune_ui::widgets::tone::Tone;

use crate::bench::{load1, percentile, sorted};
use crate::draw_with;
use crate::page::FrameTweaks;
use crate::popover::{AppEvent, event_loop};
use crate::window::{Shown, setup};

/// A phase's warm-up and its measured time.
const WARM: Duration = Duration::from_millis(500);
const MEASURE: Duration = Duration::from_secs(3);
/// The most the whole run may take; a window that stays hidden ends it here (exit 2).
const DEADLINE: Duration = Duration::from_secs(30);
/// How long to wait before asking again after a hidden window answered `Skip`.
const RETRY: Duration = Duration::from_millis(50);

/// Phase 2 may draw at most this times `Cadence::BUSY`'s rate: a broken cadence draws at the
/// display's rate, and load only lowers a grid's count (a late frame still asks for the next
/// tick, never an extra one).
const CADENCE_SLACK: f64 = 1.2;
/// Phase 4 must draw at least this fraction of the display's rate: a halving reads 0.5, and load
/// skips a tick only when a frame starts a whole interval late.
const PACED_FLOOR: f64 = 0.75;
/// The most an acquire's p95 may take in phases 2 and 4, ms: not waiting for the display.
const ACQUIRE_P95_MAX: f64 = 2.0;
/// Phase 1 must show 0.4.0's wait: acquire p50 at least this fraction of the display interval.
/// Load lengthens the frame's own work and shortens the wait, so below it the run cannot tell a
/// wait from none.
const WAIT_SEEN: f64 = 0.25;

/// One phase: what its frames are drawn with.
#[derive(Debug, Clone, Copy)]
struct Plan {
    name: &'static str,
    cadence: Cadence,
    /// The frame's `Now` is put on a grid at the display's rate.
    paced: bool,
    no_vsync: bool,
}

const PLANS: [Plan; 4] = [
    Plan {
        name: "0.4.0",
        cadence: Cadence::EveryFrame,
        paced: false,
        no_vsync: false,
    },
    Plan {
        name: "cadence",
        cadence: Cadence::BUSY,
        paced: false,
        no_vsync: false,
    },
    Plan {
        name: "paced-fifo",
        cadence: Cadence::EveryFrame,
        paced: true,
        no_vsync: false,
    },
    Plan {
        name: "paced-novsync",
        cadence: Cadence::EveryFrame,
        paced: true,
        no_vsync: true,
    },
];

/// What one phase measured.
#[derive(Debug, Clone, Default)]
pub(crate) struct PhaseStats {
    pub(crate) name: &'static str,
    /// Measured frames the probe asked for.
    pub(crate) scheduled: u32,
    /// Measured frames the OS asked for (first show, expose, resize).
    pub(crate) os: u32,
    /// Time in `WindowSurface::acquire` for every measured frame, ms.
    pub(crate) acquire_ms: Vec<f64>,
    /// Time inside the redraw handler for every measured frame, acquire included, ms.
    pub(crate) busy_ms: f64,
    /// The start-to-start time of consecutive scheduled frames, ms.
    pub(crate) intervals_ms: Vec<f64>,
    /// Measured acquires that answered `Skip` and `Redraw`.
    pub(crate) skips: u32,
    pub(crate) redraws: u32,
    /// A `Resized` came while measuring.
    pub(crate) resized: bool,
}

impl PhaseStats {
    fn fps(&self) -> f64 {
        f64::from(self.scheduled) / MEASURE.as_secs_f64()
    }

    /// The acquire time at percentile `p`, ms; `None` with no frame measured.
    fn acquire_at(&self, p: f64) -> Option<f64> {
        (!self.acquire_ms.is_empty()).then(|| percentile(&sorted(self.acquire_ms.clone()), p))
    }

    /// The phase's printed line.
    fn line(&self, n: usize) -> String {
        let at = |p| {
            self.acquire_at(p)
                .map_or("-".to_owned(), |v| format!("{v:.2}"))
        };
        let max = self
            .acquire_ms
            .iter()
            .copied()
            .reduce(f64::max)
            .map_or("-".to_owned(), |v| format!("{v:.2}"));
        let iv = sorted(self.intervals_ms.clone());
        let (i50, i95) = if iv.is_empty() {
            ("-".to_owned(), "-".to_owned())
        } else {
            (
                format!("{:.2}", percentile(&iv, 0.50)),
                format!("{:.2}", percentile(&iv, 0.95)),
            )
        };
        format!(
            "pace-probe: phase {n} {}: scheduled {:.1}/s ({} frames), os {}, acquire p50 {} p95 {} p99 {} max {max} ms, busy {:.0} ms/s, interval p50 {i50} p95 {i95} ms, skip {} redraw {}{}",
            self.name,
            self.fps(),
            self.scheduled,
            self.os,
            at(0.50),
            at(0.95),
            at(0.99),
            self.busy_ms / MEASURE.as_secs_f64(),
            self.skips,
            self.redraws,
            if self.resized { ", resized" } else { "" },
        )
    }
}

/// The verdict: the exit code and one reason per failed check (or the held bounds).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Verdict {
    /// 0 held, 1 did not hold, 2 could not measure.
    pub(crate) code: u8,
    pub(crate) reasons: Vec<String>,
}

/// `Cadence::BUSY`'s rate a second.
fn busy_hz() -> f64 {
    match Cadence::BUSY {
        Cadence::Hz(n) => f64::from(n.get()),
        Cadence::EveryFrame => 0.0,
    }
}

/// Judges the four phases at a display of `display_hz`. Exit 2 (could not measure) is checked
/// first; then exit 1 names each failed bound; p99 and max are printed, never judged, and phase 3
/// is printed, not judged (under `Fifo` the queue fills only after about 13 s).
pub(crate) fn judge(phases: &[PhaseStats; 4], display_hz: u32) -> Verdict {
    let hz = f64::from(display_hz.max(1));
    let interval = 1000.0 / hz;
    let mut unmeasured = Vec::new();
    for (i, p) in phases.iter().enumerate() {
        let n = i + 1;
        if p.skips > 0 || p.redraws > 0 {
            unmeasured.push(format!(
                "phase {n}: acquire answered Skip {} and Redraw {} (the window was covered, minimised or moved off screen)",
                p.skips, p.redraws
            ));
        }
        if p.resized {
            unmeasured.push(format!("phase {n}: the window was resized"));
        }
        if p.acquire_ms.is_empty() {
            unmeasured.push(format!("phase {n}: no frame was measured"));
        }
    }
    if unmeasured.is_empty()
        && let Some(p50) = phases[0].acquire_at(0.50)
        && p50 < WAIT_SEEN * interval
    {
        unmeasured.push(format!(
            "instrument check failed: phase 1 acquire p50 {p50:.2} ms is under {:.2} ms (0.4.0's wait was not seen, so a zero cannot be trusted)",
            WAIT_SEEN * interval
        ));
    }
    if !unmeasured.is_empty() {
        return Verdict {
            code: 2,
            reasons: unmeasured,
        };
    }
    let mut failed = Vec::new();
    let fps2 = phases[1].fps();
    if fps2 > CADENCE_SLACK * busy_hz() {
        failed.push(format!(
            "phase 2 drew {fps2:.1}/s, above {:.1}/s",
            CADENCE_SLACK * busy_hz()
        ));
    }
    for i in [1, 3] {
        if let Some(p95) = phases[i].acquire_at(0.95)
            && p95 > ACQUIRE_P95_MAX
        {
            failed.push(format!(
                "phase {} acquire p95 {p95:.2} ms is above {ACQUIRE_P95_MAX} ms",
                i + 1
            ));
        }
    }
    let fps4 = phases[3].fps();
    if fps4 < PACED_FLOOR * hz {
        failed.push(format!(
            "phase 4 drew {fps4:.1}/s, below {:.1}/s",
            PACED_FLOOR * hz
        ));
    }
    if failed.is_empty() {
        Verdict {
            code: 0,
            reasons: vec![format!(
                "phase 2 {fps2:.1}/s with acquire p95 within {ACQUIRE_P95_MAX} ms; phase 4 {fps4:.1}/s of a {hz:.0} Hz display with acquire p95 within {ACQUIRE_P95_MAX} ms"
            )],
        }
    } else {
        Verdict {
            code: 1,
            reasons: failed,
        }
    }
}

/// The probe's window, its frames and what it measured.
struct Probe {
    proxy: winit::event_loop::EventLoopProxy<AppEvent>,
    shown: Option<Shown>,
    store: UiStore,
    strings: StringTable,
    shortcuts: Shortcuts,
    /// App time 0.
    origin: Instant,
    display_hz: u32,
    phase: usize,
    phase_start: Instant,
    /// The probe asked for the redraw that is pending.
    asked: bool,
    /// The scheduled frames' start times in the measured part of this phase.
    starts: Vec<Instant>,
    cur: PhaseStats,
    done: Vec<PhaseStats>,
    /// Set when the run cannot be done here (no GUI session, a closed window, the deadline).
    unavailable: Option<String>,
}

impl Probe {
    /// Whether `age` into the phase is in its measured part.
    fn measuring(age: Duration) -> bool {
        age >= WARM && age < WARM + MEASURE
    }

    fn fail(&mut self, why: String, event_loop: &ActiveEventLoop) {
        if self.unavailable.is_none() {
            self.unavailable = Some(why);
        }
        event_loop.exit();
    }

    /// Draws one frame, records it and sets what asks for the next.
    fn render(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let started = Instant::now();
        if started.duration_since(self.origin) > DEADLINE {
            self.fail(
                "not finished in time (is the window hidden?)".into(),
                event_loop,
            );
            return Ok(());
        }
        let scheduled = std::mem::take(&mut self.asked);
        let age = started.duration_since(self.phase_start);
        let measuring = Self::measuring(age);
        let plan = PLANS[self.phase];
        let now = started.duration_since(self.origin);
        let Some(shown) = &mut self.shown else {
            return Ok(());
        };
        let Shown {
            window,
            surface,
            r,
            translator,
            ..
        } = shown;
        let t0 = Instant::now();
        let acquired = surface.acquire(&r.gpu)?;
        let acquire_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let frame = match acquired {
            Acquired::Frame(f) => f,
            Acquired::Skip => {
                if measuring {
                    self.cur.skips += 1;
                    self.cur.acquire_ms.push(acquire_ms);
                }
                event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + RETRY));
                return Ok(());
            }
            Acquired::Redraw => {
                if measuring {
                    self.cur.redraws += 1;
                    self.cur.acquire_ms.push(acquire_ms);
                }
                window.request_redraw();
                return Ok(());
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let size = surface.size();
        let scale = window.scale_factor();
        let input = translator.take(now);
        let config = FrameConfig {
            theme: THEME,
            tier: MotionTier::Full,
            busy_cadence: plan.cadence,
            strings: &self.strings,
            arg_format: &DisplayFormat,
            keymap: keymap(),
            shortcuts: &self.shortcuts,
        };
        let mut ui = Ui::begin(&mut self.store, &input, config, &mut r.text);
        draw_content(&mut ui);
        let (display, out) = ui.finish();
        let mut repaint = out.repaint;
        if plan.paced {
            let rate = Cadence::hz(u16::try_from(self.display_hz).unwrap_or(60));
            repaint = repaint.paced(now, rate);
        }
        let tweaks = FrameTweaks::DEFAULT;
        draw_with(r, &display, &view, tweaks.params(size, scale as f32))?;
        window.pre_present_notify();
        surface.present(&r.gpu, frame);
        r.gpu.end_frame(&mut r.text);
        let (flow, redraw) = control_flow(repaint, self.origin);
        event_loop.set_control_flow(flow);
        if redraw {
            window.request_redraw();
            self.asked = true;
        }
        if measuring {
            if scheduled {
                self.cur.scheduled += 1;
                if let Some(prev) = self.starts.last() {
                    self.cur
                        .intervals_ms
                        .push(started.duration_since(*prev).as_secs_f64() * 1000.0);
                }
                self.starts.push(started);
            } else {
                self.cur.os += 1;
            }
            self.cur.acquire_ms.push(acquire_ms);
            self.cur.busy_ms += started.elapsed().as_secs_f64() * 1000.0;
        }
        if started.duration_since(self.phase_start) >= WARM + MEASURE {
            self.next_phase(event_loop);
        }
        Ok(())
    }

    /// Ends the phase; between frames, with no texture alive, starts the next or ends the run.
    fn next_phase(&mut self, event_loop: &ActiveEventLoop) {
        let mut stats = std::mem::take(&mut self.cur);
        stats.name = PLANS[self.phase].name;
        println!("{}", stats.line(self.phase + 1));
        self.done.push(stats);
        self.starts.clear();
        self.phase += 1;
        let Some(plan) = PLANS.get(self.phase) else {
            event_loop.exit();
            return;
        };
        if let Some(shown) = &mut self.shown {
            if plan.no_vsync {
                shown.surface.set_options(
                    &shown.r.gpu,
                    SurfaceOptions {
                        present_mode: wgpu::PresentMode::AutoNoVsync,
                        ..SurfaceOptions::default()
                    },
                );
            }
            shown.window.request_redraw();
        }
        self.asked = true;
        self.phase_start = Instant::now();
        event_loop.set_control_flow(ControlFlow::Wait);
    }
}

/// The probe's frame: a tab strip with three tabs, the second shining, a spinner and an
/// indeterminate bar. Nothing else moves.
fn draw_content(ui: &mut Ui<'_>) {
    let tabs = [
        Tab {
            key: TabKey(0),
            title: "Sessions",
            closable: true,
            progress: None,
        },
        Tab {
            key: TabKey(1),
            title: "Building",
            closable: true,
            progress: Some(Tone::Neutral),
        },
        Tab {
            key: TabKey(2),
            title: "Notes",
            closable: true,
            progress: None,
        },
    ];
    let opts = TabStripOptions {
        active: Some(TabKey(0)),
        panel_focused: true,
        busy: None,
        busy_tone: Tone::Neutral,
        card: Rect::new(40.0, 40.0, 700.0, 300.0),
        name: "Probe",
        header_tint: None,
    };
    ui.tab_strip("strip", Rect::new(40.0, 40.0, 700.0, 32.0), &tabs, &opts);
    ui.spinner(
        "spinner",
        (80.0, 400.0),
        &SpinnerOptions {
            name: "Working",
            ..SpinnerOptions::default()
        },
    );
    ui.progress(
        "bar",
        Rect::new(40.0, 460.0, 400.0, 0.0),
        &ProgressOptions {
            value: None,
            name: "Loading",
            ..ProgressOptions::default()
        },
    );
}

/// The display's rate, Hz: the window's monitor, rounded; 60 when it does not say.
fn display_hz(window: &Window) -> u32 {
    window
        .current_monitor()
        .and_then(|m| m.refresh_rate_millihertz())
        .map_or(60, |mhz| (f64::from(mhz) / 1000.0).round() as u32)
        .max(1)
}

/// Runs the probe. See the module docs for the exit status.
pub(crate) fn run() -> ExitCode {
    let event_loop = match event_loop(true) {
        Ok(l) => l,
        Err(e) => {
            println!("pace-probe: not run: {e} (it needs a logged-in GUI session)");
            return ExitCode::from(2);
        }
    };
    let now = Instant::now();
    let mut app = Probe {
        proxy: event_loop.create_proxy(),
        shown: None,
        store: UiStore::default(),
        strings: StringTable::default(),
        shortcuts: Shortcuts::default(),
        origin: now,
        display_hz: 60,
        phase: 0,
        phase_start: now,
        asked: false,
        starts: Vec::new(),
        cur: PhaseStats::default(),
        done: Vec::new(),
        unavailable: None,
    };
    println!(
        "pace-probe: about 15 s; leave the window visible and untouched (do not cover, minimise, move or resize it)"
    );
    if let Err(e) = event_loop.run_app(&mut app) {
        println!("pace-probe: not run: event loop: {e}");
        return ExitCode::from(2);
    }
    app.verdict()
}

impl Probe {
    fn verdict(self) -> ExitCode {
        let load = load1();
        if let Some(why) = &self.unavailable {
            println!("pace-probe: verdict: could not measure: {why}; load1={load}");
            return ExitCode::from(2);
        }
        let Ok(phases) = <[PhaseStats; 4]>::try_from(self.done) else {
            println!("pace-probe: verdict: could not measure: the run ended early; load1={load}");
            return ExitCode::from(2);
        };
        let v = judge(&phases, self.display_hz);
        let word = match v.code {
            0 => "held",
            1 => "did not hold",
            _ => "could not measure",
        };
        println!(
            "pace-probe: verdict: {word}: {}; display {} Hz; load1={load}",
            v.reasons.join("; "),
            self.display_hz
        );
        ExitCode::from(v.code)
    }
}

impl ApplicationHandler<AppEvent> for Probe {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.shown.is_some() {
            return;
        }
        if event_loop.available_monitors().next().is_none() {
            self.fail(
                "no monitor: there is no GUI session (log in and retry)".into(),
                event_loop,
            );
            return;
        }
        match setup(event_loop, self.proxy.clone(), true) {
            Ok(shown) => {
                self.display_hz = display_hz(&shown.window);
                // The first show's frame is the OS's, not the probe's.
                shown.window.request_redraw();
                self.shown = Some(shown);
                self.origin = Instant::now();
                self.phase_start = self.origin;
            }
            Err(e) => self.fail(format!("setup: {e}"), event_loop),
        }
    }

    /// An accessibility wake or a tray click asks nothing of the probe.
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: AppEvent) {}

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        // A deadline the probe asked for has come.
        if matches!(cause, StartCause::ResumeTimeReached { .. })
            && let Some(shown) = &self.shown
        {
            shown.window.request_redraw();
            self.asked = true;
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if event_loop.exiting() {
            return;
        }
        match event {
            WindowEvent::CloseRequested => self.fail("the window was closed".into(), event_loop),
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.render(event_loop) {
                    self.fail(format!("render: {e}"), event_loop);
                }
            }
            WindowEvent::Resized(size) => {
                let measuring = Self::measuring(self.phase_start.elapsed());
                if measuring {
                    self.cur.resized = true;
                }
                if let Some(shown) = &mut self.shown {
                    shown.surface.resize(&shown.r.gpu, size.width, size.height);
                    // An `os` frame: the probe did not schedule it.
                    shown.window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A phase of `scheduled` frames with `samples` as its acquire times.
    fn phase(scheduled: u32, samples: Vec<f64>) -> PhaseStats {
        PhaseStats {
            name: "t",
            scheduled,
            acquire_ms: samples,
            ..PhaseStats::default()
        }
    }

    /// Inputs shaped like the planner's probe 2 at 60 Hz.
    fn probe_shaped() -> [PhaseStats; 4] {
        [
            phase(181, vec![16.5; 181]),
            phase(91, vec![0.05; 91]),
            phase(181, vec![0.03; 181]),
            phase(181, vec![0.03; 181]),
        ]
    }

    #[test]
    fn inputs_shaped_like_the_probe_hold() {
        let v = judge(&probe_shaped(), 60);
        assert_eq!(v.code, 0, "{:?}", v.reasons);
    }

    #[test]
    fn a_resize_or_a_skip_while_measuring_cannot_be_measured() {
        let mut p = probe_shaped();
        p[1].resized = true;
        assert_eq!(judge(&p, 60).code, 2);
        let mut p = probe_shaped();
        p[1].skips = 1;
        assert_eq!(judge(&p, 60).code, 2);
    }

    #[test]
    fn a_cadence_that_draws_at_the_display_rate_did_not_hold() {
        let mut p = probe_shaped();
        p[1] = phase(180, vec![0.05; 180]);
        let v = judge(&p, 60);
        assert_eq!(v.code, 1);
        assert!(v.reasons[0].contains("phase 2"), "{:?}", v.reasons);
    }

    #[test]
    fn a_paced_run_at_half_the_display_rate_did_not_hold() {
        let mut p = probe_shaped();
        p[3] = phase(90, vec![0.03; 90]);
        let v = judge(&p, 60);
        assert_eq!(v.code, 1);
        assert!(v.reasons[0].contains("phase 4"), "{:?}", v.reasons);
    }

    #[test]
    fn a_phase_1_that_did_not_wait_is_an_instrument_failure_even_when_phase_2_fails() {
        let mut p = probe_shaped();
        p[0] = phase(181, vec![0.05; 181]);
        p[1] = phase(180, vec![0.05; 180]);
        assert_eq!(judge(&p, 60).code, 2);
    }

    #[test]
    fn the_acquire_bound_is_the_p95_not_the_largest_sample() {
        // Nearest rank: the 86th of 90 is the 95th percentile.
        let with = |slow: usize| {
            let mut s = vec![0.05; 90 - slow];
            s.extend(vec![17.0; slow]);
            let mut p = probe_shaped();
            p[1] = phase(90, s);
            judge(&p, 60).code
        };
        assert_eq!(with(4), 0);
        assert_eq!(with(5), 1);
    }
}
