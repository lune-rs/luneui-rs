//! `Cadence` (D298): the tick grid, `Repaint::paced`, and the frame rule for a busy loop
//! (`Ui::request_busy_frame`) driven through `UiHarness`.

use std::time::Duration;

use lune_ui_core::input::Repaint;
use lune_ui_core::motion::{Cadence, Kind, Motion};
use lune_ui_core::paint::Rect;
use lune_ui_core::testing::UiHarness;
use lune_ui_core::ui::WidgetId;

fn ns(n: u64) -> Duration {
    Duration::from_nanos(n)
}

fn harness() -> UiHarness {
    UiHarness::new((400.0, 300.0))
}

const R: Rect = Rect::new(0.0, 0.0, 10.0, 10.0);

#[test]
fn busy_ticks_at_exact_values() {
    assert_eq!(
        Cadence::BUSY.next_tick(Duration::ZERO),
        Some(ns(33_333_334))
    );
    assert_eq!(
        Cadence::BUSY.next_tick(ns(33_333_333)),
        Some(ns(33_333_334))
    );
    assert_eq!(
        Cadence::BUSY.next_tick(ns(33_333_334)),
        Some(ns(66_666_667))
    );
    assert_eq!(
        Cadence::BUSY.next_tick(Duration::from_secs(1)),
        Some(ns(1_033_333_334))
    );
}

#[test]
fn n_ticks_in_a_second_with_no_drift() {
    for n in [1u16, 7, 24, 30, 59, 60, 144, 1000, 65535] {
        let rate = Cadence::hz(n);
        let start = Duration::from_secs(5);
        let end = Duration::from_secs(6);
        let mut t = start;
        let mut count = 0u32;
        loop {
            let next = rate.next_tick(t).expect("a finite tick");
            assert!(next > t, "{n} Hz: {next:?} not after {t:?}");
            if next > end {
                break;
            }
            count += 1;
            t = next;
        }
        assert_eq!(count, u32::from(n), "{n} Hz: ticks in (5 s, 6 s]");
    }
}

#[test]
fn constructors_and_edges() {
    assert_eq!(Cadence::hz(0), Cadence::EveryFrame);
    assert_eq!(Cadence::hz(30), Cadence::BUSY);
    assert_eq!(Cadence::default(), Cadence::BUSY);
    assert_eq!(Cadence::EveryFrame.next_tick(Duration::from_secs(3)), None);
    assert_eq!(Cadence::hz(1).next_tick(Duration::MAX), None);
    // Must not panic or overflow.
    let _ = Cadence::hz(65535).next_tick(Duration::from_secs(u64::MAX / 2));
}

#[test]
fn paced_moves_only_a_now() {
    let now = Duration::from_secs(1);
    let rate = Cadence::hz(60);
    assert_eq!(
        Repaint::Now.paced(now, rate),
        Repaint::At(ns(1_016_666_667))
    );
    assert_eq!(Repaint::Idle.paced(now, rate), Repaint::Idle);
    let t = Duration::from_millis(1010);
    assert_eq!(Repaint::At(t).paced(now, rate), Repaint::At(t));
    assert_eq!(Repaint::Now.paced(now, Cadence::EveryFrame), Repaint::Now);
}

#[test]
fn a_busy_frame_reports_the_next_tick() {
    let mut h = harness();
    let tick = Repaint::At(ns(1_033_333_334));
    let (_, out) = h.at_ms(1000).frame(|ui| ui.request_busy_frame(R));
    assert_eq!(out.repaint, tick);

    h.busy_cadence = Cadence::EveryFrame;
    let (_, out) = h.at_ms(1000).frame(|ui| ui.request_busy_frame(R));
    assert_eq!(out.repaint, Repaint::Now);
    h.busy_cadence = Cadence::BUSY;

    let (_, out) = h.at_ms(1000).frame(|_| {});
    assert_eq!(out.repaint, Repaint::Idle);
}

#[test]
fn an_earlier_deadline_wins_a_later_one_does_not() {
    let mut h = harness();
    let (_, out) = h.at_ms(1000).frame(|ui| {
        ui.request_busy_frame(R);
        ui.request_repaint_at(Duration::from_millis(1010));
    });
    assert_eq!(out.repaint, Repaint::At(Duration::from_millis(1010)));
    let (_, out) = h.at_ms(1000).frame(|ui| {
        ui.request_busy_frame(R);
        ui.request_repaint_at(Duration::from_millis(1100));
    });
    assert_eq!(out.repaint, Repaint::At(ns(1_033_333_334)));
}

#[test]
fn a_transition_in_flight_is_never_held_to_the_cadence() {
    let mut h = harness();
    let id = WidgetId::ROOT.with("fade");
    let motion = Motion::of(Kind::Appear); // 250 ms under Full
    let (_, out) = h.at_ms(0).frame(|ui| {
        ui.animate_f32(id, 0.0, &motion);
        ui.request_busy_frame(R);
    });
    assert_eq!(
        out.repaint,
        Repaint::At(ns(33_333_334)),
        "first call: settled"
    );
    let (_, out) = h.at_ms(100).frame(|ui| {
        ui.animate_f32(id, 1.0, &motion);
        ui.request_busy_frame(R);
    });
    assert_eq!(out.repaint, Repaint::Now);
    let (_, out) = h.at_ms(200).frame(|ui| {
        ui.animate_f32(id, 1.0, &motion);
        ui.request_busy_frame(R);
    });
    assert_eq!(out.repaint, Repaint::Now);
    let (_, out) = h.at_ms(1000).frame(|ui| {
        ui.animate_f32(id, 1.0, &motion);
        ui.request_busy_frame(R);
    });
    assert_eq!(out.repaint, Repaint::At(ns(1_033_333_334)), "settled");
}

#[test]
fn a_loop_wholly_outside_the_clip_asks_nothing() {
    let mut h = harness();
    let clip = Rect::new(0.0, 0.0, 100.0, 100.0);
    let at = |x: f32| Rect::new(x, 0.0, 10.0, 10.0);
    let tick = Repaint::At(ns(1_033_333_334));

    let (_, out) = h.at_ms(1000).frame(|ui| {
        ui.painter().push_clip(clip);
        ui.request_busy_frame(at(200.0));
    });
    assert_eq!(out.repaint, Repaint::Idle, "outside the clip");
    let (_, out) = h.at_ms(1000).frame(|ui| {
        ui.painter().push_clip(clip);
        ui.request_busy_frame(at(95.0));
    });
    assert_eq!(out.repaint, tick, "partly inside");

    // Under a translate the clip is in the content space: x 200 is now visible.
    let (_, out) = h.at_ms(1000).frame(|ui| {
        ui.painter().push_clip(clip);
        ui.painter().push_translate(-150.0, 0.0);
        ui.request_busy_frame(at(200.0));
    });
    assert_eq!(out.repaint, tick, "translated into view");
    let (_, out) = h.at_ms(1000).frame(|ui| {
        ui.painter().push_clip(clip);
        ui.painter().push_translate(-150.0, 0.0);
        ui.request_busy_frame(at(0.0));
    });
    assert_eq!(out.repaint, Repaint::Idle, "translated out of view");

    let (_, out) = h.at_ms(1000).frame(|ui| ui.request_busy_frame(at(5000.0)));
    assert_eq!(out.repaint, tick, "no clip: always counts");
}
