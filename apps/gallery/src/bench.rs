//! `gallery --bench`: frame-time probes on a headless device (app code: it reads the clock).
//!
//! Each scenario is built, drawn, waited on and trimmed for `--frames` measured frames after a
//! warm-up of 3, `--runs` times (default 5); a line per scenario reports the median over runs of
//! each field. Offscreen only: no window, no event loop, no screen capture.
//!
//! Robust to a busy machine (D85): each run is judged on its own (its p99 and its own worst
//! frame), and a scenario is `OVER` only when at least half its runs are; a minority of over runs
//! is reported beside an `ok`. The self-test is a calibration, not a gate: its spread (p99 and max
//! above its p50) and the load average are printed with every verdict so a reader can see the
//! conditions. Exit 2 is kept for usage and setup errors only.
//!
//! Scenarios: one timeline per page, then the effects page's cost probes and a self-test.
//! A legacy page cycles its static scenes (each held for `HOLD` frames). A Ui page replays, for
//! each of its scenes, frame 0, the script and 60 Hz frames until the frame reports `Idle` (at most
//! `SETTLE_MAX`), then its `bench` steps on a fresh state; the timeline restarts when it ends.
//! The probes: two text-fade grids (alpha moves every frame and every label carries the cycle
//! number, so each fade frame shapes fresh text keys), a scaling and fading dialog over a blurred
//! stripe page, one static reference scene, and a self-test (a 4 ms busy-wait in the build phase)
//! that calibrates the instrument. Probe time advances 1/60 s per frame.
//!
//! GPU time is the wait of `device.poll` after the draw's submit (submit to done): the frame
//! renderer builds and submits its encoder internally, so encoder timestamps around the draw
//! cannot bracket the GPU work alone. The device is still asked for
//! `TIMESTAMP_QUERY_INSIDE_ENCODERS` so the adapter's support is on record.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use lune_ui::dump;
use lune_ui::frame::FrameParams;
use lune_ui::gpu::{self, Gpu};
use lune_ui::input::Repaint;
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::{Painter, Rect};
use lune_ui::scene::Scene;
use lune_ui::text::TextSystem;

use crate::page::{
    Page, PageKind, PageState, Player, Step, page_images, register_images, release_images,
};
use crate::pages::{PAGES, effects};
use crate::scenes::{BLUR_SATURATION, BLUR_SIGMA, EFFECTS, LegacyScene, P, REFERENCE};
use crate::{Renderer, offscreen_renderer};

/// Frames drawn and not measured at the start of each run.
const WARMUP: usize = 3;
/// A scenario passes when its median p99 full frame time is within this (60 Hz).
const BUDGET_P99_MS: f64 = 16.6;
/// ... and no measured frame exceeds this.
const BUDGET_MAX_MS: f64 = 33.0;
/// The size the budget is judged at (1040 x 660 logical at 2x).
const DEFAULT_SIZE: (u32, u32) = (2080, 1320);
/// Frames a legacy page holds each of its scenes.
const HOLD: u64 = 8;
/// Most 60 Hz frames a Ui scene's timeline runs while waiting for `Idle`.
const SETTLE_MAX: u32 = 60;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Options {
    runs: usize,
    frames: usize,
    size: (u32, u32),
}

impl Options {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        let mut o = Options {
            runs: 5,
            frames: 600,
            size: DEFAULT_SIZE,
        };
        let mut it = args.iter();
        while let Some(flag) = it.next() {
            let value = it.next().ok_or_else(|| crate::USAGE.to_owned())?;
            match flag.as_str() {
                "--runs" => o.runs = positive(value)?,
                "--frames" => o.frames = positive(value)?,
                "--size" => {
                    let (w, h) = value
                        .split_once('x')
                        .ok_or_else(|| crate::USAGE.to_owned())?;
                    o.size = (positive(w)? as u32, positive(h)? as u32);
                }
                _ => return Err(crate::USAGE.to_owned()),
            }
        }
        Ok(o)
    }
}

fn positive(s: &str) -> Result<usize, String> {
    match s.parse::<usize>() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(crate::USAGE.to_owned()),
    }
}

#[derive(Clone, Copy)]
enum Kind {
    /// A page's timeline.
    Page(&'static Page),
    /// One static legacy scene (a reference for the instrument check).
    Scene(&'static LegacyScene),
    LabelFade(usize),
    DialogBlur,
    SelftestSleep4,
}

struct Scenario {
    name: String,
    kind: Kind,
    /// Counts toward the exit status (report-only otherwise).
    judged: bool,
}

fn scenarios(size: (u32, u32)) -> Vec<Scenario> {
    let at_default = size == DEFAULT_SIZE;
    let mut v = vec![Scenario {
        name: "probe:selftest-sleep4".into(),
        kind: Kind::SelftestSleep4,
        judged: false,
    }];
    for page in PAGES {
        v.push(Scenario {
            name: format!("page:{}", page.id),
            kind: Kind::Page(page),
            judged: at_default,
        });
    }
    v.push(Scenario {
        name: CONTROLS_PROBE.into(),
        kind: Kind::Scene(scene_named("controls", 2)),
        judged: false,
    });
    v.push(Scenario {
        name: "probe:label-fade-64".into(),
        kind: Kind::LabelFade(64),
        judged: at_default,
    });
    v.push(Scenario {
        name: "probe:label-fade-256".into(),
        kind: Kind::LabelFade(256),
        judged: false,
    });
    v.push(Scenario {
        name: "probe:dialog-blur".into(),
        kind: Kind::DialogBlur,
        judged: at_default,
    });
    v
}

/// The static reference the label-fade probe must exceed (more shaping per frame).
const CONTROLS_PROBE: &str = "probe:controls@2x";

/// One measured frame, in milliseconds.
#[derive(Clone, Copy, Default)]
struct Sample {
    build: f64,
    draw: f64,
    gpu: f64,
    end: f64,
    full: f64,
}

impl Sample {
    fn cpu(&self) -> f64 {
        self.build + self.draw + self.end
    }
}

/// Per-run statistics of one scenario, in milliseconds.
#[derive(Clone, Copy, Default)]
struct RunStats {
    p50: f64,
    p95: f64,
    p99: f64,
    max: f64,
    cpu_p99: f64,
    gpu_p99: f64,
}

/// Nearest-rank percentile of `v` (sorted ascending, non-empty).
fn percentile(v: &[f64], p: f64) -> f64 {
    let rank = (p * v.len() as f64).ceil() as usize;
    v[rank.clamp(1, v.len()) - 1]
}

fn sorted(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(f64::total_cmp);
    v
}

fn stats(samples: &[Sample]) -> RunStats {
    let full = sorted(samples.iter().map(|s| s.full).collect());
    let cpu = sorted(samples.iter().map(Sample::cpu).collect());
    let gpu = sorted(samples.iter().map(|s| s.gpu).collect());
    RunStats {
        p50: percentile(&full, 0.50),
        p95: percentile(&full, 0.95),
        p99: percentile(&full, 0.99),
        max: full[full.len() - 1],
        cpu_p99: percentile(&cpu, 0.99),
        gpu_p99: percentile(&gpu, 0.99),
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// The median over runs of each field.
fn median_stats(runs: &[RunStats]) -> RunStats {
    let m = |f: fn(&RunStats) -> f64| median(runs.iter().map(f).collect());
    RunStats {
        p50: m(|r| r.p50),
        p95: m(|r| r.p95),
        p99: m(|r| r.p99),
        max: m(|r| r.max),
        cpu_p99: m(|r| r.cpu_p99),
        gpu_p99: m(|r| r.gpu_p99),
    }
}

struct Outcome {
    median: RunStats,
    /// The largest measured frame of any run.
    worst: f64,
    cpu_p50: f64,
    /// Runs over budget on their own p99 or their own worst frame.
    runs_over: usize,
    runs: usize,
    /// Measured frames over `BUDGET_MAX_MS`, summed over runs.
    frames_over: usize,
}

impl Outcome {
    /// Over budget in at least half the runs; a minority of over runs is load, not the build.
    fn over(&self) -> bool {
        self.runs_over * 2 >= self.runs
    }
}

/// The 1-minute load average, when the platform reports one (app code may read it).
fn load1() -> String {
    if let Ok(s) = std::fs::read_to_string("/proc/loadavg")
        && let Some(v) = s.split_whitespace().next()
    {
        return v.to_owned();
    }
    std::process::Command::new("sysctl")
        .args(["-n", "vm.loadavg"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            s.split_whitespace()
                .find(|t| t.parse::<f64>().is_ok())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

// ---- scenario drawing ----

fn scene_named(id: &str, scale: u32) -> &'static LegacyScene {
    REFERENCE
        .iter()
        .chain(EFFECTS)
        .find(|s| s.id == id && s.scale == scale)
        .expect("reference scene present")
}

/// One scripted run of a Ui page: a scene's script, or the page's bench steps.
struct Seg {
    page: &'static str,
    state: fn() -> Box<dyn PageState>,
    scale: f32,
    tier: MotionTier,
    steps: &'static [Step],
}

/// A Ui page's frames in order: each segment's frame 0, its steps, then 60 Hz frames until
/// `Idle`.
struct Timeline {
    segs: Vec<Seg>,
    at: usize,
    run: Option<Run>,
    size: (u32, u32),
}

struct Run {
    player: Player,
    /// Frame 0 has run.
    started: bool,
    steps: &'static [Step],
    next: usize,
    /// Time of the last frame, ms.
    ms: u32,
    /// Settle frames run so far; `None` while the script still runs.
    settle: Option<u32>,
}

impl Timeline {
    fn new(page: &'static Page, size: (u32, u32)) -> Self {
        let PageKind::Ui(ui) = page.kind else {
            unreachable!("only Ui pages have timelines")
        };
        let mut segs: Vec<Seg> = ui
            .scenes
            .iter()
            .map(|s| Seg {
                page: page.id,
                state: s.state,
                scale: s.scale,
                tier: s.tier,
                steps: s.script,
            })
            .collect();
        if !ui.bench.is_empty() || segs.is_empty() {
            segs.push(Seg {
                page: page.id,
                state: ui.state,
                scale: 2.0,
                tier: MotionTier::Full,
                steps: ui.bench,
            });
        }
        Timeline {
            segs,
            at: 0,
            run: None,
            size,
        }
    }

    /// The next frame's display list and its scale.
    fn frame(&mut self, text: &mut dyn TextSystem) -> Result<(Scene, f32), String> {
        let seg = &self.segs[self.at];
        let scale = seg.scale;
        let run = self.run.get_or_insert_with(|| {
            let logical = (self.size.0 as f32 / scale, self.size.1 as f32 / scale);
            Run {
                player: Player::new(seg.page, (seg.state)(), logical, scale, seg.tier),
                started: false,
                steps: seg.steps,
                next: 0,
                ms: 0,
                settle: None,
            }
        });
        let (scene, out) = if !run.started {
            run.started = true;
            run.player.frame(text, 0, &[])?
        } else if run.settle.is_none()
            && let Some(step) = run.steps.get(run.next)
        {
            run.next += 1;
            run.ms = step.at_ms;
            run.player.frame(text, step.at_ms, step.events)?
        } else {
            let k = run.settle.unwrap_or(0) + 1;
            run.settle = Some(k);
            run.ms += 1000 / 60;
            run.player.frame(text, run.ms, &[])?
        };
        if run
            .settle
            .is_some_and(|k| out.repaint == Repaint::Idle || k >= SETTLE_MAX)
        {
            self.run = None;
            self.at = (self.at + 1) % self.segs.len();
        }
        Ok((scene, scale))
    }
}
/// What `frame` draws: a probe, a page's static scenes, or a Ui page's timeline.
struct Driver {
    kind: Kind,
    timeline: Option<Timeline>,
}

impl Driver {
    fn new(kind: Kind, size: (u32, u32)) -> Self {
        let timeline = match kind {
            Kind::Page(page) if matches!(page.kind, PageKind::Ui(_)) => {
                Some(Timeline::new(page, size))
            }
            _ => None,
        };
        Driver { kind, timeline }
    }
}

/// Paints one legacy `scene` at its own scale over `size` physical pixels.
fn paint_legacy(text: &mut dyn TextSystem, scene: &LegacyScene, size: (u32, u32)) -> (Scene, f32) {
    let s = scene.scale as f32;
    (scene.paint(text, s, size), s)
}

/// Paints `draw` at scale 2 with no hover.
fn paint_probe(
    text: &mut dyn TextSystem,
    size: (u32, u32),
    draw: impl FnOnce(&mut P<'_>),
) -> (Scene, f32) {
    let win = Rect::new(0.0, 0.0, size.0 as f32 / 2.0, size.1 as f32 / 2.0);
    let mut p = Painter::new(2.0, win, text, None, THEME);
    draw(&mut p);
    (p.scene, 2.0)
}

/// Builds frame number `n` of the driver's scenario: its display list and scale.
fn build(
    d: &mut Driver,
    text: &mut dyn TextSystem,
    n: u64,
    size: (u32, u32),
) -> Result<(Scene, f32), String> {
    match d.kind {
        Kind::Page(page) => match (&mut d.timeline, page.kind) {
            (Some(t), _) => t.frame(text),
            (None, PageKind::Legacy(scenes)) => {
                let scene = &scenes[(n / HOLD) as usize % scenes.len()];
                Ok(paint_legacy(text, scene, size))
            }
            (None, PageKind::Ui(_)) => Err("a Ui page without a timeline".into()),
        },
        Kind::Scene(scene) => Ok(paint_legacy(text, scene, size)),
        Kind::LabelFade(count) => Ok(paint_probe(text, size, |p| {
            effects::label_fade(p, count, n);
        })),
        Kind::DialogBlur => Ok(paint_probe(text, size, |p| effects::dialog_blur(p, n))),
        Kind::SelftestSleep4 => {
            // A busy-wait of 4 ms, not `thread::sleep`: macOS sleeps 4 ms for about 6.0 ms here
            // (checked with Python's `perf_counter`), so a sleep is not a known answer.
            let start = Instant::now();
            while start.elapsed() < Duration::from_millis(4) {
                std::hint::spin_loop();
            }
            Ok(paint_legacy(text, scene_named("acrylic", 2), size))
        }
    }
}

/// One timed frame.
fn frame(
    r: &mut Renderer,
    d: &mut Driver,
    n: u64,
    target: &wgpu::TextureView,
    size: (u32, u32),
) -> Result<Sample, String> {
    let t0 = Instant::now();
    let (scene, s) = build(d, &mut r.text, n, size)?;
    let t1 = Instant::now();
    let params = FrameParams {
        size,
        scale: s,
        blur_sigma: BLUR_SIGMA,
        saturation: BLUR_SATURATION,
        ..FrameParams::default()
    };
    let mut customs: [&mut dyn lune_ui::custom::CustomDraw; 1] = [&mut r.fill];
    r.frame.draw_with(
        &mut r.gpu,
        &mut r.text,
        &scene,
        target,
        params,
        &mut customs,
    )?;
    let t2 = Instant::now();
    r.gpu
        .device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| format!("poll: {e}"))?;
    let t3 = Instant::now();
    r.gpu.end_frame(&mut r.text);
    let t4 = Instant::now();
    Ok(Sample {
        build: ms(t1 - t0),
        draw: ms(t2 - t1),
        gpu: ms(t3 - t2),
        end: ms(t4 - t3),
        full: ms(t4 - t0),
    })
}

/// Measures `sc` with the pictures its page draws registered for the duration.
fn measure(r: &mut Renderer, sc: &Scenario, o: &Options) -> Result<Outcome, String> {
    let images = match sc.kind {
        Kind::Page(page) => register_images(&mut r.gpu, page_images(page))?,
        _ => Vec::new(),
    };
    let outcome = measure_runs(r, sc, o);
    release_images(&mut r.gpu, &images);
    outcome
}

fn measure_runs(r: &mut Renderer, sc: &Scenario, o: &Options) -> Result<Outcome, String> {
    let texture = dump::target(&r.gpu.device, r.frame.format(), o.size);
    let view = texture.create_view(&Default::default());
    // Simulated time: frame `n` is at `n / 60` s; the counter keeps running across runs.
    let mut n: u64 = 0;
    let mut runs = Vec::new();
    let mut worst: f64 = 0.0;
    let mut cpu: Vec<f64> = Vec::new();
    let (mut runs_over, mut frames_over) = (0, 0);
    for _ in 0..o.runs {
        let mut driver = Driver::new(sc.kind, o.size);
        for _ in 0..WARMUP {
            frame(r, &mut driver, n, &view, o.size)?;
            n += 1;
        }
        let mut samples = Vec::with_capacity(o.frames);
        for _ in 0..o.frames {
            samples.push(frame(r, &mut driver, n, &view, o.size)?);
            n += 1;
        }
        let st = stats(&samples);
        worst = worst.max(st.max);
        runs_over += usize::from(!(st.p99 <= BUDGET_P99_MS && st.max <= BUDGET_MAX_MS));
        frames_over += samples.iter().filter(|s| s.full > BUDGET_MAX_MS).count();
        cpu.extend(samples.iter().map(Sample::cpu));
        runs.push(st);
    }
    Ok(Outcome {
        median: median_stats(&runs),
        worst,
        cpu_p50: percentile(&sorted(cpu), 0.50),
        runs_over,
        runs: runs.len(),
        frames_over,
    })
}

/// Runs the bench. Exit 0: every judged scenario passed in more than half its runs; 1: some are
/// over budget in at least half; 2: usage or setup error. Never refuses a busy machine (D85).
pub(crate) fn run(o: &Options) -> ExitCode {
    match run_inner(o) {
        Ok(code) => ExitCode::from(code),
        Err(msg) => {
            eprintln!("bench: {msg}");
            ExitCode::from(2)
        }
    }
}

fn run_inner(o: &Options) -> Result<u8, String> {
    let gpu = pollster::block_on(Gpu::new_with_features(
        gpu::instance(None),
        None,
        wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS,
    ))?;
    let info = gpu.adapter.get_info();
    println!(
        "bench: adapter={} backend={:?} timestamp_inside_encoders={} (adapter) {} (device) gpu=wait",
        info.name,
        info.backend,
        gpu.adapter
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS),
        gpu.device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS),
    );
    println!(
        "bench: runs={} frames={} size={}x{}",
        o.runs, o.frames, o.size.0, o.size.1
    );
    println!("bench: load1={} at start", load1());
    let mut r = offscreen_renderer(gpu);
    let mut over = 0;
    let (mut controls_p50, mut label_p50) = (None, None);
    // The self-test's spread above its own p50 (p99, max): how noisy this machine is right now.
    let mut calib = None;
    for sc in scenarios(o.size) {
        let out = measure(&mut r, &sc, o)?;
        let m = out.median;
        if matches!(sc.kind, Kind::SelftestSleep4) {
            let spread = (m.p99 - m.p50, out.worst - m.p50);
            let in_range = (4.0..=6.0).contains(&out.cpu_p50);
            // D85: load only lengthens a 4 ms wait, so a reading below 4 ms means the
            // instrument under-measures, and no verdict it prints can be trusted.
            let broken = out.cpu_p50 < 4.0;
            println!(
                "bench: calibration {} cpu_p50={:.2} (expected 4.0-6.0: {}) p50={:.2} p99={:.2} worst={:.2} spread_p99=+{:.2} spread_max=+{:.2} load1={}",
                sc.name,
                out.cpu_p50,
                if in_range { "in range" } else { "OUT OF RANGE" },
                m.p50,
                m.p99,
                out.worst,
                spread.0,
                spread.1,
                load1(),
            );
            if broken {
                return Err("instrument check failed: self-test below 4 ms".to_owned());
            }
            calib = Some(spread);
            continue;
        }
        let verdict = match (sc.judged, out.over()) {
            (false, _) => "report-only".to_owned(),
            (true, false) if out.runs_over == 0 => "ok".to_owned(),
            (true, false) => format!("ok (over in {}/{} runs)", out.runs_over, out.runs),
            (true, true) => {
                over += 1;
                format!("OVER (in {}/{} runs)", out.runs_over, out.runs)
            }
        };
        let (sp99, smax) = calib.unwrap_or((0.0, 0.0));
        println!(
            "bench {} frames={} p50={:.2} p95={:.2} p99={:.2} max={:.2} cpu_p99={:.2} gpu_p99={:.2} gpu=wait worst={:.2} frames_over_33={} runs_over={}/{} load1={} calib_spread=+{:.2}/+{:.2} {}",
            sc.name,
            o.frames,
            m.p50,
            m.p95,
            m.p99,
            m.max,
            m.cpu_p99,
            m.gpu_p99,
            out.worst,
            out.frames_over,
            out.runs_over,
            out.runs,
            load1(),
            sp99,
            smax,
            verdict
        );
        match sc.name.as_str() {
            CONTROLS_PROBE => controls_p50 = Some(m.p50),
            "probe:label-fade-64" => label_p50 = Some(m.p50),
            _ => {}
        }
    }
    println!("bench: {over} over budget; load1={} at end", load1());
    if let (Some(c), Some(l)) = (controls_p50, label_p50)
        && l <= c
    {
        println!(
            "bench: calibration warning: label-fade-64 p50 {l:.2} ms is not above {CONTROLS_PROBE} p50 {c:.2} ms"
        );
    }
    Ok(u8::from(over > 0))
}
