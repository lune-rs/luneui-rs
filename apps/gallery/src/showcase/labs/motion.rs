//! The motion lab: the seven easings and the fifteen animation kinds, each with a replay button,
//! its curve or swatch, its token and the time it takes. The page runs on the frame's clock and
//! tier, so the settings pane's speed, tier and durations change it at once: a slower clock
//! stretches every run, Reduced shortens or fades them, Off snaps them.
//!
//! A change here is timed by hand from the resolved row (`MotionTokens::resolve` and
//! `Easing::eval`): the runs are one-shot, begin on a press and rest at their end.

use std::time::Duration;

use lune_ui::motion::{Easing, Kind, Motion, Reduce, Resolved, SPIN_PERIOD, Token, spin_angle};
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, ButtonOptions};

use super::text;
use crate::page::{Page, PageKind, PageState, Stage, UiPage};
use crate::pages::motion::{EASINGS, Tones, easing_name, kind_name, plot_in, swatch, token_name};
use crate::pages::{card, heading};

pub(crate) static PAGE: Page = Page {
    id: "lab-motion",
    title: "Motion lab",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: &[],
        bench: &[],
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Lab::new())
}

/// The easings' columns, their plot and the track under it.
const E_PITCH: f32 = 138.0;
const PLOT: f32 = 72.0;
const TRACK_W: f32 = 120.0;
const DOT: f32 = 12.0;
/// The kinds' columns and rows, and a swatch.
const K_COLS: usize = 5;
const K_PITCH: (f32, f32) = (190.0, 104.0);
const SWATCH: (f32, f32) = (64.0, 48.0);
/// A replay button.
const REPLAY: (f32, f32) = (64.0, 24.0);

struct Lab {
    /// When each run began; none is at rest, at its end.
    easing: [Option<Duration>; 7],
    kind: [Option<Duration>; 15],
}

impl Lab {
    fn new() -> Self {
        Self {
            easing: [None; 7],
            kind: [None; 15],
        }
    }
}

/// The row an easing is shown with: a move over the slow token with its Reduced rule kept.
fn easing_row(e: Easing) -> Motion {
    Motion {
        kind: Kind::Move,
        token: Token::Slow,
        easing: e,
        reduced: Reduce::Keep,
        reduced_token: None,
    }
}

/// How far a run that began at `start` has got at `now` under `run` (what the row resolves to
/// under the tier; none snaps): the eased value, and whether it is still on its way. A run with
/// no start is at its end.
fn progress(start: Option<Duration>, now: Duration, run: Option<Resolved>) -> (f32, bool) {
    let (Some(start), Some(run)) = (start, run) else {
        return (1.0, false);
    };
    let total = run.duration.as_secs_f32();
    if total <= 0.0 {
        return (1.0, false);
    }
    let t = now.saturating_sub(start).as_secs_f32() / total;
    if t >= 1.0 {
        (1.0, false)
    } else {
        (run.easing.eval(t), true)
    }
}

/// What a row takes, as a line: its token and the milliseconds it resolves to.
fn time_line(motion: &Motion, run: Option<Resolved>) -> String {
    match run {
        Some(r) => format!(
            "{}, {} ms",
            token_name(motion.token),
            r.duration.as_millis()
        ),
        None => format!("{}, snaps", token_name(motion.token)),
    }
}

/// A small button `label`led, `width` wide, at `at`; whether it was pressed.
fn replay_button(ui: &mut Ui<'_>, key: &str, label: &str, at: (f32, f32), width: f32) -> bool {
    let options = ButtonOptions {
        kind: ButtonKind::Secondary,
        small: true,
        icon: None,
    };
    let r = Rect::new(at.0, at.1, width, REPLAY.1);
    ui.button_with(key, r, label, &options).clicked
}

impl PageState for Lab {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Motion lab");
        let t = *ui.theme();
        let (tier, now) = (ui.tier(), ui.time());
        let tones = Tones {
            dim: t.surface_selected,
            hot: t.accent,
            ring: t.border_subtle,
            faint: t.fg_faint,
        };
        let (x, y) = (card.x + 24.0, card.y + 20.0);
        let mut moving = false;

        heading(ui, "EASINGS, EACH OVER THE SLOW TOKEN", x, y);
        for (i, e) in EASINGS.into_iter().enumerate() {
            let (cx, cy) = (x + i as f32 * E_PITCH, y + 22.0);
            let motion = easing_row(e);
            let run = t.motion.resolve(&motion, tier);
            let key = format!("ease-{}", easing_name(e));
            if replay_button(ui, &key, "Replay", (cx, cy + 128.0), REPLAY.0) {
                self.easing[i] = Some(now);
            }
            let (v, going) = progress(self.easing[i], now, run);
            moving |= going;
            plot_in(
                ui.painter(),
                Rect::new(cx, cy, PLOT, PLOT),
                e,
                tones.ring,
                tones.hot,
            );
            text(
                ui,
                easing_name(e),
                cx,
                cy + PLOT + 4.0,
                t.font_s,
                t.fg_primary,
            );
            let track = Rect::new(cx, cy + PLOT + 24.0, TRACK_W, DOT);
            let p = ui.painter();
            p.fill(track, DOT / 2.0, tones.dim);
            let dot = Rect::new(cx + v * (TRACK_W - DOT), track.y, DOT, DOT);
            p.fill(dot, DOT / 2.0, tones.hot);
            let line = time_line(&motion, run);
            text(ui, &line, cx, track.bottom() + 6.0, t.font_s, t.fg_tertiary);
        }

        heading(
            ui,
            "KINDS, EACH WITH ITS OWN TOKEN AND EASING",
            x,
            y + 192.0,
        );
        let all = replay_button(
            ui,
            "replay-all",
            "Replay all",
            (x + 960.0 - 96.0, y + 186.0),
            96.0,
        );
        if all {
            self.easing = [Some(now); 7];
            self.kind = [Some(now); 15];
        }
        let spin = spin_angle(now);
        for (i, kind) in Kind::ALL.into_iter().enumerate() {
            let cx = x + (i % K_COLS) as f32 * K_PITCH.0;
            let cy = y + 220.0 + (i / K_COLS) as f32 * K_PITCH.1;
            let motion = Motion::of(kind);
            let run = t.motion.resolve(&motion, tier);
            let key = format!("kind-{}", kind_name(kind));
            if replay_button(ui, &key, "Replay", (cx, cy + 56.0), REPLAY.0) {
                self.kind[i] = Some(now);
            }
            let (v, going) = if kind == Kind::Spin {
                (1.0, false)
            } else {
                progress(self.kind[i], now, run)
            };
            moving |= going;
            // A snap shows the end state at once, transforms included.
            let transforms = run.is_none_or(|r| r.transforms);
            swatch(
                ui.painter(),
                Rect::new(cx, cy, SWATCH.0, SWATCH.1),
                kind,
                v,
                transforms,
                spin,
                &tones,
            );
            let name_x = cx + SWATCH.0 + 8.0;
            text(
                ui,
                kind_name(kind),
                name_x,
                cy + 2.0,
                t.font_base,
                t.fg_primary,
            );
            let (first, second) = if kind == Kind::Spin {
                (
                    format!("{} ms period", SPIN_PERIOD.as_millis()),
                    "linear".to_owned(),
                )
            } else {
                (
                    time_line(&motion, run),
                    easing_name(run.map_or(motion.easing, |r| r.easing)).to_owned(),
                )
            };
            text(ui, &first, name_x, cy + 22.0, t.font_s, t.fg_tertiary);
            text(ui, &second, name_x, cy + 36.0, t.font_s, t.fg_tertiary);
        }
        // The spinner turns all the time; a run in flight wants every frame.
        let due = if moving {
            Duration::from_millis(1)
        } else {
            Duration::from_millis(16)
        };
        ui.request_repaint_at(now + due);
    }
}

#[cfg(test)]
mod tests {
    use super::super::rig::{CANVAS, player};
    use super::*;
    use crate::page::{Ev, Player};
    use lune_ui::access::Role;
    use lune_ui::motion::MotionTier;
    use lune_ui::scene::LayerId;
    use lune_ui::testing::FakeText;
    use lune_ui::theme::Theme;

    const FULL: Theme = lune_ui::lunegit_dark::THEME;

    fn resolve(motion: &Motion, tier: MotionTier) -> Option<Resolved> {
        FULL.motion.resolve(motion, tier)
    }

    #[test]
    fn a_run_goes_from_nothing_to_its_end_on_the_easing() {
        let m = easing_row(Easing::QuadOut);
        let run = resolve(&m, MotionTier::Full);
        let ms = Duration::from_millis;
        // At rest and before it starts, the end.
        assert_eq!(progress(None, ms(10), run), (1.0, false));
        // Begun now: nothing yet, and on its way.
        assert_eq!(progress(Some(ms(100)), ms(100), run), (0.0, true));
        // A quarter of 500 ms in.
        let (v, going) = progress(Some(ms(100)), ms(225), run);
        assert!(going);
        assert!((v - Easing::QuadOut.eval(0.25)).abs() < 1e-6, "{v}");
        // Done.
        assert_eq!(progress(Some(ms(100)), ms(600), run), (1.0, false));
        // Off snaps: no run, the end at once.
        assert_eq!(
            progress(Some(ms(100)), ms(100), resolve(&m, MotionTier::Off)),
            (1.0, false)
        );
    }

    #[test]
    fn the_tier_and_the_theme_set_the_time_a_row_takes() {
        let fade = Motion::of(Kind::Appear);
        assert_eq!(
            time_line(&fade, resolve(&fade, MotionTier::Full)),
            "base, 250 ms"
        );
        // Reduced maps base to fast (180 ms), Off snaps.
        assert_eq!(
            time_line(&fade, resolve(&fade, MotionTier::Reduced)),
            "base, 180 ms"
        );
        assert_eq!(
            time_line(&fade, resolve(&fade, MotionTier::Off)),
            "base, snaps"
        );
        let mut slow = FULL;
        slow.motion.base = Duration::from_millis(900);
        assert_eq!(
            time_line(&fade, slow.motion.resolve(&fade, MotionTier::Full)),
            "base, 900 ms"
        );
    }

    fn replays(out: &lune_ui::input::FrameOutput) -> usize {
        out.tree
            .nodes
            .iter()
            .filter(|n| n.role == Role::Button && n.name == "Replay")
            .count()
    }

    /// The base layer's shapes of a frame at `ms` (a press made at `events`).
    fn shapes(p: &mut Player, ms: u32, events: &[Ev]) -> usize {
        let (scene, _) = p.frame(&mut FakeText, ms, events).expect("frame");
        scene.layers[LayerId::Base as usize].geometry.under.len()
            + scene.layers[LayerId::Base as usize].geometry.over.len()
    }

    #[test]
    fn every_kind_and_easing_has_a_replay_and_the_page_keeps_a_frame_coming() {
        let mut p = player("lab-motion", live());
        p.frame(&mut FakeText, 0, &[]).expect("frame");
        let (_, out) = p.frame(&mut FakeText, 16, &[]).expect("frame");
        assert_eq!(replays(&out), 7 + 15);
        assert!(
            out.tree
                .nodes
                .iter()
                .any(|n| n.name == "Replay" && n.bounds.right() <= CANVAS.right())
        );
        // The spinner turns: never idle.
        assert_ne!(out.repaint, lune_ui::input::Repaint::Idle);
    }

    /// A press on one kind's replay starts its run: its swatch is at its start for a frame, and
    /// back at its end when the run is over. Everything else, and the hover of the press, is the
    /// same as without it.
    #[test]
    fn a_replay_restarts_its_kind_and_it_comes_to_rest() {
        let run = |press: bool, tier: MotionTier| {
            let mut p = Player::new("lab-motion", live(), (1600.0, 900.0), 1.0, tier);
            p.set_stage(Stage::at(CANVAS));
            p.frame(&mut FakeText, 0, &[]).expect("frame");
            p.frame(&mut FakeText, 1000, &[]).expect("frame");
            let events: &[Ev] = if press {
                &[Ev::PointerOver(&["kind-appear"]), Ev::Click]
            } else {
                &[Ev::PointerOver(&["kind-appear"])]
            };
            let mut counts = vec![shapes(&mut p, 1016, events)];
            for ms in [1032, 1048, 1064, 1500, 2000] {
                counts.push(shapes(&mut p, ms, &[]));
            }
            counts
        };
        let (plain, pressed) = (run(false, MotionTier::Full), run(true, MotionTier::Full));
        assert_ne!(plain[..4], pressed[..4], "the run is under way");
        assert_eq!(plain[4..], pressed[4..], "and over by 500 ms");
        // Off snaps: the press changes nothing.
        assert_eq!(run(false, MotionTier::Off), run(true, MotionTier::Off));
    }
}
