//! `gallery --bench`: frame-time probes on a headless device (app code: it reads the clock).
//!
//! Each scenario is built, drawn, waited on and trimmed for `--frames` measured frames after a
//! warm-up of 3, `--runs` times; a line per scenario reports the median over runs of each field.
//! Offscreen only: no window, no event loop, no screen capture.
//!
//! Scenarios: the 15 reference scenes (static), two text-fade probes (alpha moves every frame and
//! every label carries the cycle number, so each fade frame shapes fresh text keys), a scaling and
//! fading dialog over a blurred stripe page, and a self-test (a 4 ms busy-wait in the build phase) that checks the
//! instrument. Simulated time advances 1/60 s per frame.
//!
//! GPU time is the wait of `device.poll` after the draw's submit (submit to done): the frame
//! renderer builds and submits its encoder internally, so encoder timestamps around the draw
//! cannot bracket the GPU work alone. The device is still asked for
//! `TIMESTAMP_QUERY_INSIDE_ENCODERS` so the adapter's support is on record.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use lune_ui::color::Rgba;
use lune_ui::dump;
use lune_ui::frame::FrameParams;
use lune_ui::gpu::{self, Gpu};
use lune_ui::lunegit_dark::{self, THEME};
use lune_ui::paint::{LayerId, Painter, Rect, Xform};
use lune_ui::text::{Spec, Style};
use lune_ui::widgets::dialog::{DialogGlass, DialogHits, DialogSpec, dialog};

use crate::scenes::{BLUR_SATURATION, BLUR_SIGMA, Hit, P, SCENES, Scene};
use crate::{Renderer, offscreen_renderer};

/// Frames drawn and not measured at the start of each run.
const WARMUP: usize = 3;
/// A scenario passes when its median p99 full frame time is within this (60 Hz).
const BUDGET_P99_MS: f64 = 16.6;
/// ... and no measured frame exceeds this.
const BUDGET_MAX_MS: f64 = 33.0;
/// The size the budget is judged at (1040 x 660 logical at 2x).
const DEFAULT_SIZE: (u32, u32) = (2080, 1320);
/// Probe ramp: 250 ms at 60 Hz.
const RAMP_FRAMES: u64 = 15;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Options {
    runs: usize,
    frames: usize,
    size: (u32, u32),
}

impl Options {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        let mut o = Options {
            runs: 3,
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
    Scene(&'static Scene),
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
    for scene in SCENES {
        v.push(Scenario {
            name: format!("scene:{}", scene.name()),
            kind: Kind::Scene(scene),
            judged: at_default,
        });
    }
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
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

// ---- scenario drawing ----

const FILL_TEXT: &str = "the quick brown fox jumps over the lazy dog ";

/// Label `i` of a fade probe: 12 to 40 characters, starting with the cycle number so each cycle
/// shapes text no earlier frame did.
fn label_text(i: usize, cycle: u64) -> String {
    let len = 12 + (i * 7) % 29;
    let mut s = format!("{cycle}-{i} ");
    s.extend(FILL_TEXT.chars().cycle().take(len));
    s.truncate(len);
    s
}

/// `n` base-size labels in a grid of 16 rows under an alpha-only transform that ramps 0 to 1.
fn label_fade(p: &mut P<'_>, n: usize, frame: u64) {
    let (phase, cycle) = (
        (frame % RAMP_FRAMES) as f32 / RAMP_FRAMES as f32,
        frame / RAMP_FRAMES,
    );
    p.on(LayerId::Base);
    let win = p.window;
    p.fill(win, 0.0, [0x1F, 0x1F, 0x1F, 0xFF]);
    p.set_xform(Some(Xform {
        origin: (0.0, 0.0),
        scale: 1.0,
        dy: 0.0,
        alpha: phase,
    }));
    let rows = 16;
    let cols = n / rows;
    let (cw, ch) = (win.w / cols as f32, win.h / rows as f32);
    for i in 0..n {
        let spec = Spec::plain(
            label_text(i, cycle),
            Style::ui(THEME.font_base, THEME.fg_primary),
        );
        p.text(
            &spec,
            (i % cols) as f32 * cw + 8.0,
            (i / cols) as f32 * ch + 8.0,
        );
    }
    p.set_xform(None);
}

/// The stripes of `blur_contrast_scene` on Base, a window-sized backdrop request, and a dialog
/// scaling 0.96 to 1 and fading 0 to 1 about the window centre (the card's centre) over 250 ms,
/// quad-out, repeating.
fn dialog_blur(p: &mut P<'_>, frame: u64) {
    p.on(LayerId::Base);
    let win = p.window;
    let stripes = (win.w / 16.0).ceil() as usize;
    for i in 0..stripes {
        let c: Rgba = if i % 2 == 0 {
            [0, 0, 0, 255]
        } else {
            [255, 255, 255, 255]
        };
        p.fill(Rect::new(i as f32 * 16.0, 0.0, 16.0, win.h), 0.0, c);
    }
    p.on(LayerId::Popup);
    p.backdrop(win, 0.0);
    let phase = (frame % RAMP_FRAMES) as f32 / RAMP_FRAMES as f32;
    let e = 1.0 - (1.0 - phase) * (1.0 - phase);
    p.set_xform(Some(Xform {
        origin: (win.w / 2.0, win.h / 2.0),
        scale: 0.96 + 0.04 * e,
        dy: 0.0,
        alpha: e,
    }));
    let spec = DialogSpec {
        title: "Discard changes",
        message: "The document has unsaved changes. Closing it will discard them.",
        extra_h: 0.0,
        confirm: "Discard",
        cancel: Some("Cancel"),
    };
    let glass = DialogGlass {
        panel: lunegit_dark::PANEL.premultiplied(),
        fill: lunegit_dark::WINDOW.premultiplied(),
        noise: true,
    };
    let hits = DialogHits {
        block: Hit::Block,
        confirm: Hit::Confirm,
        cancel: Hit::Cancel,
    };
    dialog(p, &spec, &glass, &hits, |_, _, _, _| {});
    p.set_xform(None);
}

fn scene_named(id: &str, scale: u32) -> &'static Scene {
    SCENES
        .iter()
        .find(|s| s.id == id && s.scale == scale)
        .expect("reference scene present")
}

/// Paints frame number `frame` of `kind`.
fn build(kind: Kind, p: &mut P<'_>, frame: u64) {
    match kind {
        Kind::Scene(scene) => (scene.draw)(p, scene.time),
        Kind::LabelFade(n) => label_fade(p, n, frame),
        Kind::DialogBlur => dialog_blur(p, frame),
        Kind::SelftestSleep4 => {
            let scene = scene_named("acrylic", 2);
            // A busy-wait of 4 ms, not `thread::sleep`: macOS sleeps 4 ms for about 6.0 ms here
            // (checked with Python's `perf_counter`), so a sleep is not a known answer.
            let start = Instant::now();
            while start.elapsed() < Duration::from_millis(4) {
                std::hint::spin_loop();
            }
            (scene.draw)(p, scene.time);
        }
    }
}

/// One timed frame.
fn frame(
    r: &mut Renderer,
    kind: Kind,
    n: u64,
    target: &wgpu::TextureView,
    size: (u32, u32),
) -> Result<Sample, String> {
    let (s, hover) = match kind {
        Kind::Scene(scene) => (scene.scale as f32, scene.hover),
        _ => (2.0, None),
    };
    let win = Rect::new(0.0, 0.0, size.0 as f32 / s, size.1 as f32 / s);
    let t0 = Instant::now();
    let mut p = Painter::new(s, win, &mut r.text, hover, THEME);
    build(kind, &mut p, n);
    let Painter { scene, .. } = p;
    let t1 = Instant::now();
    let params = FrameParams {
        size,
        scale: s,
        blur_sigma: BLUR_SIGMA,
        saturation: BLUR_SATURATION,
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

fn measure(r: &mut Renderer, sc: &Scenario, o: &Options) -> Result<Outcome, String> {
    let texture = dump::target(&r.gpu.device, r.frame.format(), o.size);
    let view = texture.create_view(&Default::default());
    // Simulated time: frame `n` is at `n / 60` s; the counter keeps running across runs.
    let mut n: u64 = 0;
    let mut runs = Vec::new();
    let mut worst: f64 = 0.0;
    let mut cpu: Vec<f64> = Vec::new();
    for _ in 0..o.runs {
        for _ in 0..WARMUP {
            frame(r, sc.kind, n, &view, o.size)?;
            n += 1;
        }
        let mut samples = Vec::with_capacity(o.frames);
        for _ in 0..o.frames {
            samples.push(frame(r, sc.kind, n, &view, o.size)?);
            n += 1;
        }
        let st = stats(&samples);
        worst = worst.max(st.max);
        cpu.extend(samples.iter().map(Sample::cpu));
        runs.push(st);
    }
    Ok(Outcome {
        median: median_stats(&runs),
        worst,
        cpu_p50: percentile(&sorted(cpu), 0.50),
    })
}

fn passes(o: &Outcome) -> bool {
    o.median.p99 <= BUDGET_P99_MS && o.worst <= BUDGET_MAX_MS
}

/// Runs the bench. Exit 0: every judged scenario passed; 1: some are over budget; 2: usage or
/// setup error, or an instrument check failed.
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
    let mut r = offscreen_renderer(gpu);
    let mut over = 0;
    let (mut controls_p50, mut label_p50) = (None, None);
    for sc in scenarios(o.size) {
        let out = measure(&mut r, &sc, o)?;
        if matches!(sc.kind, Kind::SelftestSleep4) && !(4.0..=6.0).contains(&out.cpu_p50) {
            println!(
                "bench {} cpu_p50={:.2} ms outside [4.0, 6.0]",
                sc.name, out.cpu_p50
            );
            return Err("instrument check failed".into());
        }
        let m = out.median;
        let verdict = match (sc.judged, passes(&out)) {
            (false, _) => "report-only",
            (true, true) => "ok",
            (true, false) => {
                over += 1;
                "OVER"
            }
        };
        println!(
            "bench {} frames={} p50={:.2} p95={:.2} p99={:.2} max={:.2} cpu_p99={:.2} gpu_p99={:.2} gpu=wait worst={:.2} {}",
            sc.name, o.frames, m.p50, m.p95, m.p99, m.max, m.cpu_p99, m.gpu_p99, out.worst, verdict
        );
        match sc.name.as_str() {
            "scene:controls@2x" => controls_p50 = Some(m.p50),
            "probe:label-fade-64" => label_p50 = Some(m.p50),
            _ => {}
        }
    }
    println!("bench: {over} over budget");
    if let (Some(c), Some(l)) = (controls_p50, label_p50)
        && l <= c
    {
        return Err(format!(
            "instrument check failed: label-fade-64 p50 {l:.2} ms is not above controls@2x p50 {c:.2} ms"
        ));
    }
    Ok(u8::from(over > 0))
}
