//! Contract tests for the motion core (`docs/components/motion/*.md` Acceptance).

use std::time::Duration;

use lune_ui_core::motion::*;
use lune_ui_core::theme::DARK;
use lune_ui_core::widgets::menu;

const MS: Duration = Duration::from_millis(1);

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

const ALL_EASINGS: [(&str, Easing); 7] = [
    ("linear", Easing::Linear),
    ("quad-out", Easing::QuadOut),
    ("quad-in", Easing::QuadIn),
    ("sine-in-out", Easing::SineInOut),
    ("back-out", Easing::BackOut),
    ("cubic-out", Easing::CUBIC_OUT),
    ("ease-in", Easing::EASE_IN),
];

// ---- easings ------------------------------------------------------------------------------

#[test]
fn easing_known_answers() {
    let cases = [
        (Easing::QuadOut, 0.75),
        (Easing::QuadIn, 0.25),
        (Easing::SineInOut, 0.5),
        (Easing::BackOut, 1.0876975),
        (Easing::Linear, 0.5),
    ];
    for (e, want) in cases {
        let got = e.eval(0.5);
        assert!((got - want).abs() < 1e-6, "{e:?}(0.5) = {got}, want {want}");
    }
}

#[test]
fn easings_clamp_progress_and_hit_their_ends() {
    for (name, e) in ALL_EASINGS {
        assert_eq!(e.eval(-3.0).to_bits(), e.eval(0.0).to_bits(), "{name}");
        assert_eq!(e.eval(7.0).to_bits(), e.eval(1.0).to_bits(), "{name}");
        assert!(e.eval(0.0).abs() < 1e-5, "{name}");
        assert!((e.eval(1.0) - 1.0).abs() < 1e-5, "{name}");
    }
}

#[test]
fn cubic_out_and_ease_in_are_the_moved_menu_curves_bit_for_bit() {
    for t in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
        let open = menu::cubic_bezier(0.215, 0.61, 0.355, 1.0, t);
        let close = menu::cubic_bezier(0.42, 0.0, 1.0, 1.0, t);
        assert_eq!(Easing::CUBIC_OUT.eval(t).to_bits(), open.to_bits(), "t {t}");
        assert_eq!(Easing::EASE_IN.eval(t).to_bits(), close.to_bits(), "t {t}");
        assert_eq!(
            cubic_bezier(0.215, 0.61, 0.355, 1.0, t).to_bits(),
            open.to_bits()
        );
    }
}

#[test]
fn menu_open_anim_is_unchanged_against_the_moved_function() {
    let e = cubic_bezier(0.215, 0.61, 0.355, 1.0, 0.5);
    let want = (e, 4.0 * (1.0 - e), 0.97 + 0.03 * e);
    let got = menu::menu_open_anim(0.5);
    assert_eq!(got.0.to_bits(), want.0.to_bits());
    assert_eq!(got.1.to_bits(), want.1.to_bits());
    assert_eq!(got.2.to_bits(), want.2.to_bits());
}

#[test]
fn derivative_matches_a_central_difference() {
    for (name, e) in ALL_EASINGS {
        for i in 1..20 {
            let t = i as f32 / 20.0;
            let h = 1e-3f32;
            let fd = (e.eval(t + h) - e.eval(t - h)) / (2.0 * h);
            let d = e.derivative(t);
            assert!(
                (d - fd).abs() <= 2e-2 * fd.abs().max(1.0),
                "{name} at {t}: derivative {d}, central difference {fd}"
            );
        }
    }
    assert_eq!(Easing::Linear.derivative(0.0), 1.0);
    assert_eq!(Easing::QuadOut.derivative(0.0), 2.0);
    assert_eq!(Easing::QuadIn.derivative(0.0), 0.0);
}

#[test]
fn inverse_round_trips_for_monotone_easings() {
    for (name, e) in ALL_EASINGS {
        if e == Easing::BackOut {
            continue;
        }
        for i in 0..=20 {
            let y = i as f32 / 20.0;
            let back = e.eval(e.inverse(y));
            assert!(
                (back - y).abs() < 1e-5,
                "{name}: eval(inverse({y})) = {back}"
            );
        }
    }
}

// ---- lerp, tween, keyframes, spring -------------------------------------------------------

#[test]
fn lerp_is_exact_at_both_ends() {
    let (a, b) = (0.1f32, 0.7f32);
    assert_eq!(a.lerp(b, 0.0).to_bits(), a.to_bits());
    assert_eq!(a.lerp(b, 1.0).to_bits(), b.to_bits());
    let (p, q) = ([0.3f32, -1.7], [12.9f32, 0.01]);
    assert_eq!(p.lerp(q, 0.0), p);
    assert_eq!(p.lerp(q, 1.0), q);
    let (c, d) = ([0.1f32, 0.2, 0.3, 0.9], [0.77f32, 0.0, 1.0, 0.33]);
    assert_eq!(c.lerp(d, 0.0), c);
    assert_eq!(c.lerp(d, 1.0), d);
    assert_eq!(c.sub(c), [0.0; 4]);
    assert!(([0.0f32, 0.0].distance([3.0, 4.0]) - 5.0).abs() < 1e-6);
    assert_eq!(2.0f32.add_scaled(3.0, 0.5), 3.5);
    assert!(![0.0f32, f32::NAN].is_finite());
    assert!(!f32::INFINITY.is_finite());
}

#[test]
fn tween_holds_from_before_start_and_to_exactly_at_the_end() {
    let tw = Tween {
        from: 0.1f32,
        to: 0.7,
        start: ms(100),
        duration: ms(250),
        easing: Easing::CUBIC_OUT,
    };
    assert_eq!(tw.sample(ms(0)), 0.1);
    assert_eq!(tw.sample(ms(99)), 0.1);
    assert!((tw.sample(ms(100)) - 0.1).abs() < 1e-6);
    assert_eq!(tw.sample(ms(350)).to_bits(), 0.7f32.to_bits());
    assert_eq!(tw.sample(ms(10_000)).to_bits(), 0.7f32.to_bits());
    let mid = tw.sample(ms(225));
    let want = 0.1f32.lerp(0.7, Easing::CUBIC_OUT.eval(0.5));
    assert!((mid - want).abs() < 1e-6);
    let instant = Tween {
        duration: Duration::ZERO,
        ..tw
    };
    assert_eq!(instant.sample(ms(99)), 0.1);
    assert_eq!(instant.sample(ms(100)), 0.7);
}

#[test]
fn keyframes_reject_bad_stop_lists() {
    assert!(Keyframes::<f32>::new(vec![]).is_none());
    assert!(
        Keyframes::new(vec![
            (0.0, 0.0f32, Easing::Linear),
            (0.9, 1.0, Easing::Linear)
        ])
        .is_none()
    );
    assert!(
        Keyframes::new(vec![
            (0.6, 0.0f32, Easing::Linear),
            (0.4, 1.0, Easing::Linear),
            (1.0, 1.0, Easing::Linear)
        ])
        .is_none()
    );
    assert!(
        Keyframes::new(vec![
            (-0.1, 0.0f32, Easing::Linear),
            (1.0, 1.0, Easing::Linear)
        ])
        .is_none()
    );
    assert!(
        Keyframes::new(vec![
            (f32::NAN, 0.0f32, Easing::Linear),
            (1.0, 1.0, Easing::Linear)
        ])
        .is_none()
    );
    assert!(Keyframes::new(vec![(1.0, 3.0f32, Easing::Linear)]).is_some());
}

#[test]
fn keyframes_known_answer() {
    let k = Keyframes::new(vec![
        (0.0, 0.0f32, Easing::Linear),
        (0.5, 10.0, Easing::Linear),
        (1.0, 0.0, Easing::QuadOut),
    ])
    .expect("valid stops");
    assert!((k.sample(0.25) - 5.0).abs() < 1e-6);
    assert!((k.sample(0.75) - 2.5).abs() < 1e-6);
    assert_eq!(k.sample(1.0).to_bits(), 0.0f32.to_bits());
    assert_eq!(k.sample(0.0), 0.0);
    assert_eq!(k.sample(f32::NAN), 0.0);
    let colour = Keyframes::new(vec![
        (0.0, [0.0f32; 4], Easing::Linear),
        (1.0, [0.3, 0.6, 0.9, 1.0], Easing::SineInOut),
    ])
    .expect("valid stops");
    assert_eq!(colour.sample(1.0), [0.3, 0.6, 0.9, 1.0]);
}

#[test]
fn spring_known_answer() {
    let s = Spring {
        target: 3.0f32,
        rate: 1.0,
        x0: 4.0,
        v0: 0.0,
        start: ms(500),
    };
    let got = s.sample(ms(1500)) - 3.0;
    assert!((got - 2.0 / std::f32::consts::E).abs() < 1e-6, "{got}");
}

#[test]
fn spring_never_crosses_its_target_when_moving_away_or_at_rest() {
    for d0 in [1.0f32, -1.0, 10.0, -10.0] {
        for v in [0.0f32, 0.5, 5.0] {
            let v0 = v * d0.signum();
            for rate in [1.0f32, 8.0, 30.0] {
                let s = Spring {
                    target: 2.0f32,
                    rate,
                    x0: 2.0 + d0,
                    v0,
                    start: Time::ZERO,
                };
                let mut prev = d0.abs();
                for k in 0..=600u64 {
                    let x = s.sample(Duration::from_nanos(k * 1_000_000_000 / 60));
                    let off = x - 2.0;
                    assert!(off.is_finite());
                    assert!(
                        off * d0 >= 0.0,
                        "d0 {d0} v0 {v0} rate {rate} frame {k}: {off}"
                    );
                    if v0 == 0.0 {
                        assert!(off.abs() <= prev, "d0 {d0} rate {rate} frame {k} grew");
                        prev = off.abs();
                    }
                }
            }
        }
    }
}

#[test]
fn spring_without_a_positive_rate_holds_its_start() {
    for rate in [0.0f32, -2.0, f32::NAN, f32::INFINITY] {
        let s = Spring {
            target: 0.0f32,
            rate,
            x0: 5.0,
            v0: 1.0,
            start: Time::ZERO,
        };
        assert_eq!(s.sample(ms(300)), 5.0);
    }
}

// ---- transition ---------------------------------------------------------------------------

#[test]
fn a_settled_transition_is_never_in_flight() {
    let t = Transition::settled(0.4f32);
    for now in [Time::ZERO, ms(1), ms(100_000)] {
        assert_eq!(t.sample(now), (0.4, false));
        assert_eq!(t.velocity(now), 0.0);
    }
    assert_eq!(t.target(), 0.4);
}

#[test]
fn retarget_from_rest_is_a_plain_tween() {
    for (name, e) in ALL_EASINGS {
        let mut tr = Transition::settled(0.2f32);
        tr.retarget(ms(40), 0.9, ms(250), e);
        let tw = Tween {
            from: 0.2f32,
            to: 0.9,
            start: ms(40),
            duration: ms(250),
            easing: e,
        };
        for k in 0..=20u64 {
            let now = ms(40 + k * 13);
            let (x, _) = tr.sample(now);
            assert_eq!(x.to_bits(), tw.sample(now).to_bits(), "{name} at {now:?}");
        }
        assert_eq!(tr.sample(ms(290)), (0.9, false));
    }
}

/// Retargets a 0→1 run of `easing` over `d` at its half-way point to `target` with the same
/// easing and duration; returns (old, new, instant).
fn half_way_retarget(
    d: Duration,
    e1: Easing,
    e2: Easing,
    target: f32,
) -> (Transition<f32>, Transition<f32>, Time) {
    let mut tr = Transition::settled(0.0f32);
    tr.retarget(Time::ZERO, 1.0, d, e1);
    let now = d / 2;
    let mut next = tr;
    next.retarget(now, target, d, e2);
    (tr, next, now)
}

#[test]
fn an_in_flight_retarget_keeps_position_and_velocity() {
    // The contract's 1 ms finite difference measures the curve's bend as well as its slope (its
    // error grows with the curvature at the retarget instant and shrinks linearly with the
    // step), so it runs on cases whose bend over 1 ms stays under 2 %; the analytic test below
    // checks the velocity itself for every easing pair and target.
    let mut cases = vec![(ms(250), Easing::QuadOut, 2.0f32)];
    for e in [Easing::Linear, Easing::QuadIn, Easing::SineInOut] {
        for target in [0.0f32, 2.0] {
            cases.push((ms(250), e, target));
        }
    }
    for target in [0.0f32, 2.0] {
        cases.push((ms(180), Easing::SineInOut, target));
    }
    for (d, e, target) in cases {
        let (old, new, now) = half_way_retarget(d, e, e, target);
        let (before, moving) = old.sample(now);
        assert!(moving);
        let (at, moving) = new.sample(now);
        assert!(moving);
        assert!(
            (at - before).abs() < 1e-4,
            "{e:?} → {target}: jump {}",
            at - before
        );
        let v_before = (before - old.sample(now - MS).0) / 0.001;
        let v_after = (new.sample(now + MS).0 - at) / 0.001;
        let rel = (v_after - v_before).abs() / v_before.abs().max(v_after.abs());
        assert!(
            rel <= 0.02,
            "{e:?} → {target}: {v_before} vs {v_after} ({rel})"
        );
        assert_eq!(new.sample(now + d), (target, false));
        assert_eq!(new.target(), target);
    }
}

#[test]
fn an_in_flight_retarget_is_continuous_for_every_easing_pair() {
    for d in [ms(180), ms(250)] {
        for (n1, e1) in ALL_EASINGS {
            for (n2, e2) in ALL_EASINGS {
                for target in [0.0f32, 1.0, 2.0] {
                    let (old, new, now) = half_way_retarget(d, e1, e2, target);
                    let (before, at) = (old.sample(now).0, new.sample(now).0);
                    assert!((at - before).abs() < 1e-4, "{n1}→{n2} {target}");
                    let (vb, va) = (old.velocity(now), new.velocity(now));
                    assert!(
                        (va - vb).abs() <= 1e-4 * vb.abs().max(1.0),
                        "{n1}→{n2} {target}: velocity {vb} vs {va}"
                    );
                    assert_eq!(new.sample(now + d), (target, false));
                    assert_eq!(new.sample(now + d + ms(1000)), (target, false));
                }
            }
        }
    }
}

#[test]
fn time_going_backwards_never_panics() {
    let mut tr = Transition::settled(0.0f32);
    tr.retarget(ms(500), 1.0, ms(250), Easing::QuadOut);
    let (x, moving) = tr.sample(ms(10));
    assert_eq!(x, 0.0);
    assert!(moving);
    assert!(tr.velocity(Time::ZERO).is_finite());
    tr.retarget(ms(100), 0.5, ms(180), Easing::SineInOut);
    tr.retarget(Time::ZERO, 0.2, ms(180), Easing::SineInOut);
    assert!(tr.sample(Time::ZERO).0.is_finite());
    assert_eq!(tr.sample(ms(180)), (0.2, false));
    let tw = Tween {
        from: 1.0f32,
        to: 2.0,
        start: ms(900),
        duration: ms(100),
        easing: Easing::Linear,
    };
    assert_eq!(tw.sample(Time::ZERO), 1.0);
}

#[test]
fn snap_and_zero_duration_settle_at_once() {
    let mut tr = Transition::settled([0.0f32; 4]);
    tr.retarget(Time::ZERO, [1.0; 4], ms(250), Easing::Linear);
    tr.snap([0.5; 4]);
    assert_eq!(tr.sample(ms(1)), ([0.5; 4], false));
    tr.retarget(ms(2), [0.0; 4], ms(250), Easing::Linear);
    tr.retarget(ms(3), [0.25; 4], Duration::ZERO, Easing::Linear);
    assert_eq!(tr.sample(ms(3)), ([0.25; 4], false));
}

// ---- tokens and tiers ---------------------------------------------------------------------

#[test]
fn kinds_are_in_motion_md_order() {
    assert_eq!(Kind::ALL.len(), 15);
    assert_eq!(Kind::ALL[0], Kind::Hover);
    assert_eq!(Kind::ALL[12], Kind::Spin);
    assert_eq!(Kind::ALL[14], Kind::Settle);
    assert!(Kind::ALL.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn motion_of_is_the_kinds_table() {
    use Easing::*;
    use Reduce::*;
    use Token::*;
    let table = [
        (Kind::Hover, Instant, Linear, Keep),
        (Kind::Press, Fast, QuadOut, Snap),
        (Kind::Focus, Fast, Linear, Keep),
        (Kind::Appear, Base, QuadOut, Fade),
        (Kind::Dismiss, Fast, QuadIn, Fade),
        (Kind::PageSwap, Moderate, QuadOut, Fade),
        (Kind::Drawer, Moderate, SineInOut, Fade),
        (Kind::Pop, Base, QuadOut, Fade),
        (Kind::Selection, Fast, Linear, Keep),
        (Kind::Move, Fast, SineInOut, Snap),
        (Kind::Expand, Base, SineInOut, Snap),
        (Kind::Value, Fast, QuadOut, Snap),
        (Kind::Spin, Slow, Linear, Keep),
        (Kind::Scroll, Fast, QuadOut, Snap),
        (Kind::Settle, Fast, SineInOut, Snap),
    ];
    for (i, (kind, token, easing, reduced)) in table.into_iter().enumerate() {
        assert_eq!(Kind::ALL[i], kind);
        let m = Motion::of(kind);
        assert_eq!(
            m,
            Motion {
                kind,
                token,
                easing,
                reduced,
                reduced_token: None
            }
        );
    }
}

#[test]
fn the_tokens_and_the_theme_carry_motion_md_values() {
    assert_eq!(
        [
            MOTION.instant,
            MOTION.fast,
            MOTION.base,
            MOTION.moderate,
            MOTION.slow
        ],
        [ms(120), ms(180), ms(250), ms(350), ms(500)]
    );
    assert_eq!(
        MOTION.reduced,
        ReducedMap {
            instant: Token::Instant,
            fast: Token::Instant,
            fast_selection: Token::Fast,
            base: Token::Fast,
            moderate: Token::Fast,
            slow: Token::Base,
        }
    );
    assert_eq!(DARK.motion, MOTION);
    assert_eq!(MotionTier::default(), MotionTier::Full);
}

/// The expected Reduced token, written out from D12 independently of `ReducedMap::map`.
fn d12(token: Token, kind: Kind) -> Token {
    match (token, kind) {
        (Token::Fast, Kind::Selection) => Token::Fast,
        (Token::Instant, _) | (Token::Fast, _) => Token::Instant,
        (Token::Base, _) | (Token::Moderate, _) => Token::Fast,
        (Token::Slow, _) => Token::Base,
    }
}

fn token_ms(t: Token) -> u64 {
    match t {
        Token::Instant => 120,
        Token::Fast => 180,
        Token::Base => 250,
        Token::Moderate => 350,
        Token::Slow => 500,
    }
}

#[test]
fn resolve_table_over_tokens_kinds_and_tiers() {
    let mut checked = 0;
    for token in Token::ALL {
        for kind in Kind::ALL {
            let m = Motion {
                token,
                ..Motion::of(kind)
            };
            for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
                let got = MOTION.resolve(&m, tier);
                let want = match (tier, m.reduced) {
                    (MotionTier::Off, _) | (MotionTier::Reduced, Reduce::Snap) => None,
                    (MotionTier::Full, _) => Some((token_ms(token), true)),
                    (MotionTier::Reduced, Reduce::Keep) => Some((token_ms(d12(token, kind)), true)),
                    (MotionTier::Reduced, Reduce::Fade) => {
                        Some((token_ms(d12(token, kind)), false))
                    }
                };
                let want = want.map(|(d, transforms)| Resolved {
                    duration: ms(d),
                    easing: m.easing,
                    transforms,
                });
                assert_eq!(got, want, "{token:?} {kind:?} {tier:?}");
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 5 * 15 * 3);
}

#[test]
fn an_explicit_reduced_token_overrides_the_map() {
    let fade = Motion {
        reduced_token: Some(Token::Fast),
        ..Motion::of(Kind::Pop)
    };
    let r = MOTION.resolve(&fade, MotionTier::Reduced).expect("fades");
    assert_eq!(r.duration, ms(180));
    assert!(!r.transforms);
    let keep = Motion {
        reduced_token: Some(Token::Slow),
        ..Motion::of(Kind::Hover)
    };
    assert_eq!(
        MOTION
            .resolve(&keep, MotionTier::Reduced)
            .map(|r| r.duration),
        Some(ms(500))
    );
    assert_eq!(MOTION.resolve(&keep, MotionTier::Off), None);
    let snap = Motion {
        reduced_token: Some(Token::Fast),
        ..Motion::of(Kind::Press)
    };
    assert_eq!(MOTION.resolve(&snap, MotionTier::Reduced), None);
}

// ---- spin ---------------------------------------------------------------------------------

#[test]
fn spin_is_periodic_linear_and_finite() {
    assert_eq!(SPIN_PERIOD, ms(1200));
    assert_eq!(spin_angle(Time::ZERO), 0.0);
    assert_eq!(spin_angle(ms(300)), 90.0);
    assert_eq!(spin_angle(ms(600)), 180.0);
    assert_eq!(spin_angle(ms(900)), 270.0);
    assert_eq!(spin_angle(ms(1200)), 0.0);
    for k in 0..200u64 {
        let t = Duration::from_nanos(k * 7_919_000_003);
        let a = spin_angle(t);
        assert!(a.is_finite() && (0.0..360.0).contains(&a), "{t:?}: {a}");
        assert_eq!(a.to_bits(), spin_angle(t + SPIN_PERIOD * 5).to_bits());
    }
    let far = SPIN_PERIOD * 10_000_000;
    assert!((spin_angle(far + ms(150)) - 45.0).abs() < 1e-3);
}

// ---- conformance --------------------------------------------------------------------------

const TIERS: [MotionTier; 3] = [MotionTier::Full, MotionTier::Reduced, MotionTier::Off];

#[test]
fn every_kind_conforms_under_every_tier() {
    for kind in Kind::ALL {
        let m = Motion::of(kind);
        for tier in TIERS {
            let scalar = conformance(&m, &MOTION, tier, 0.0f32, 1.0)
                .unwrap_or_else(|e| panic!("{kind:?} {tier:?}: {e}"));
            let colour = conformance(
                &m,
                &MOTION,
                tier,
                [0.1f32, 0.2, 0.3, 0.0],
                [0.9, 0.5, 0.1, 1.0],
            )
            .unwrap_or_else(|e| panic!("{kind:?} {tier:?} colour: {e}"));
            let end = MOTION.resolve(&m, tier).map_or(Time::ZERO, |r| r.duration);
            for c in [scalar, colour] {
                assert!(
                    c.settled_at >= end && c.settled_at <= end + FRAME,
                    "{kind:?} {tier:?}"
                );
            }
            if end.is_zero() {
                assert_eq!(scalar.settled_at, Time::ZERO, "{kind:?} {tier:?} snaps");
                assert_eq!(scalar.max_step, 0.0);
            }
        }
    }
}

#[test]
fn snapping_tiers_give_the_target_on_the_first_sample() {
    let m = Motion::of(Kind::Press);
    for tier in [MotionTier::Reduced, MotionTier::Off] {
        let c = conformance(&m, &MOTION, tier, 0.0f32, 1.0).expect("snaps");
        assert_eq!(c.settled_at, Time::ZERO);
    }
}

#[test]
fn the_instrument_catches_a_jump() {
    let err = conformance_of(
        |t| {
            if t.is_zero() {
                (0.0f32, true)
            } else if t < ms(250) {
                (1.0, true)
            } else {
                (1.0, false)
            }
        },
        ms(250),
        1.0f32,
        STEP_BOUND,
    )
    .expect_err("jump");
    assert!(
        matches!(err, ConformanceError::Jump { frame: 1, .. }),
        "{err:?}"
    );
}

#[test]
fn the_instrument_catches_an_inexact_end() {
    let tw = Tween {
        from: 0.0f32,
        to: 1.0,
        start: Time::ZERO,
        duration: ms(250),
        easing: Easing::Linear,
    };
    let err = conformance_of(
        |t| {
            if t < ms(250) {
                (tw.sample(t), true)
            } else {
                (0.999, false)
            }
        },
        ms(250),
        1.0f32,
        STEP_BOUND,
    )
    .expect_err("inexact end");
    assert!(matches!(err, ConformanceError::End { .. }), "{err:?}");
    let err = conformance_of(|t| (tw.sample(t), true), ms(250), 1.0f32, STEP_BOUND)
        .expect_err("still in flight");
    assert!(matches!(err, ConformanceError::End { .. }), "{err:?}");
}

#[test]
fn the_instrument_catches_a_nan() {
    let err = conformance_of(
        |t| {
            if t < ms(100) {
                (0.1f32, true)
            } else {
                (f32::NAN, true)
            }
        },
        ms(250),
        1.0f32,
        STEP_BOUND,
    )
    .expect_err("nan");
    assert!(matches!(err, ConformanceError::NotFinite { .. }), "{err:?}");
}

#[test]
fn the_instrument_catches_a_late_restart() {
    let err = conformance_of(
        |t| {
            if t < ms(20) {
                (1.0f32, false)
            } else {
                (1.0, true)
            }
        },
        ms(250),
        1.0f32,
        STEP_BOUND,
    )
    .expect_err("restarted after settling");
    assert!(matches!(err, ConformanceError::Late { .. }), "{err:?}");
}

#[test]
fn the_default_first_steps_sit_inside_the_bound() {
    // Steepest Full-tier first steps, computed rather than assumed (sketch trap): quad-out at
    // `instant` ≈ 0.26 of the distance, back-out at `fast` ≈ 0.38.
    let first = |e: Easing, d: Duration| e.eval(FRAME.as_secs_f32() / d.as_secs_f32());
    assert!(first(Easing::QuadOut, MOTION.instant) < STEP_BOUND);
    assert!(first(Easing::CUBIC_OUT, MOTION.fast) < STEP_BOUND);
    assert!(first(Easing::BackOut, MOTION.fast) < STEP_BOUND_BACK_OUT);
    for kind in Kind::ALL {
        let m = Motion::of(kind);
        let c = conformance(&m, &MOTION, MotionTier::Full, 0.0f32, 1.0).expect("conforms");
        assert!(c.max_step < STEP_BOUND, "{kind:?}: {}", c.max_step);
    }
}

#[test]
fn conformance_is_deterministic() {
    let run = || {
        Kind::ALL
            .iter()
            .flat_map(|&k| TIERS.map(move |tier| (k, tier)))
            .map(|(k, tier)| {
                let c = conformance(&Motion::of(k), &MOTION, tier, [0.0f32, 3.0], [7.0, -2.0])
                    .expect("conforms");
                (c.frames, c.settled_at, c.max_step.to_bits())
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
    let trace = || {
        let mut tr = Transition::settled(0.0f32);
        tr.retarget(Time::ZERO, 1.0, ms(250), Easing::CUBIC_OUT);
        tr.retarget(ms(100), -1.0, ms(180), Easing::SineInOut);
        (0..30u64)
            .map(|k| {
                tr.sample(Duration::from_nanos(k * 1_000_000_000 / 60))
                    .0
                    .to_bits()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(trace(), trace());
}
