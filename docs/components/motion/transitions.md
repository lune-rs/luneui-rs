---
id: motion.transitions
title: Motion system and transitions
area: motion
status: planned
consumers: any
source: —
owner: —
summary: Time, Easing, Tween, Spring, Keyframes and Transition evaluated in closed form, tier-aware, with a 60 Hz conformance harness
---

## What it is

The pure-core motion model every component animates through. The app passes `Time` each frame;
values are evaluated in closed form at `t`, so they depend on neither the frame rate nor a clock and
a frame is reproducible. Lunegit's tokens and curves are the defaults (`docs/motion.md`); springs
are critically damped and never bouncy (Wane's rule: no bouncy motion, no hover lifts
[W docs/ui/README.md:92]). A `Transition<T>` that is retargeted mid-flight continues from its current
value and velocity, so a quick click never waits on an animation. The surface is `ui.animate_*(id,
target, token)`, which reports `Repaint::Now` while any transition is in flight.

## Needs

- General transitions under the motion tiers, built from Lunegit's duration and easing tokens (R-motion-2, R-motion-3).
- Time comes from the app; no clock read (R-nfr-1).
- The moved menu curves become two of the seven easings, bit-identical (R-motion-1).
- Every transition is checked at 60 Hz in `cargo test`; settled pages go Idle (R-motion-3).

## Existing code

None for the model. The only moved motion code is `motion.curves`
(`crates/lune-ui-core/src/widgets/menu.rs:21-57`). Lunegit has the tokens and the Full and Reduced
stylesheets (`src/Lunegit.Desktop/App.axaml:151-155`, `Themes/Animations.axaml`,
`Themes/AnimationsReduced.axaml`) but no code-side model: its transitions are Avalonia's.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `docs/motion.md` (the vocabulary);
`crates/lune-ui-core/src/widgets/menu.rs:21-57`; `crates/lune-ui-core/src/input.rs:1-30` (time in the
input); Lunegit .claude/docs/animation-rules.md:1-172, src/Lunegit.Desktop/App.axaml:147-155,
src/Lunegit.Desktop/Controls/SpinBehavior.cs:15-64, src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:36-44;
Wane docs/ui/README.md:92, docs/ui/mocks/css/tokens.css:115-119.
Conflict: Lunegit animates through Avalonia style transitions and a per-frame `DispatcherTimer`
(the spinner adds elapsed milliseconds, so it depends on the frame rate) [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:42-53];
the toolkit evaluates closed-form values of `Time` instead, so a frame is reproducible [R crates/lune-ui-core/src/input.rs:5].

### Anatomy

- `Time`: the app's monotonic instant, carried in the frame input as a duration since an app-chosen origin [R crates/lune-ui-core/src/input.rs:5,22].
- `Easing`: the seven names of `docs/motion.md` as `Linear`, `QuadOut`, `QuadIn`, `SineInOut`, `BackOut`, `CUBIC_OUT`, `EASE_IN`, plus `CubicBezier(x1, y1, x2, y2)`; maps progress 0..1 to eased 0..1, clamping outside [motion], [motion.curves].
- Known easing values (the test table): `quad-out(0.5) = 0.75`, `quad-in(0.5) = 0.25`, `sine-in-out(0.5) = 0.5`, `back-out(0.5) = 1.0876975` (overshoots 1 before settling), `cubic-out` and `ease-in` equal the moved `cubic_bezier` [derived: the formulas of `docs/motion.md`], [R crates/lune-ui-core/src/widgets/menu.rs:46-56].
- `Lerp`: linear interpolation for `f32`, colours (straight-alpha RGBA, per channel, premultiplied for blending) and points; `Tween<T: Lerp> { from, to, start, duration, easing }` evaluates `from.lerp(to, easing(clamp((t - start) / duration)))`.
- `Spring`: analytic critically damped: with distance `d0 = x0 - target` and velocity `v0`, `x(t) = target + (d0 + (v0 + w d0) t) e^(-w t)` for angular rate `w` [derived: the closed form of a critically damped oscillator; the repo has no spring today].
- `Keyframes<T>`: a list of `(progress, value, easing)` stops sampled in closed form; stops sorted, the last at progress 1.
- `Transition<T>`: current value and velocity, a target and a token (duration and easing); `retarget(now, new_target)` restarts from the sampled current value and its velocity (the derivative of the closed form at that instant); `sample(now)` returns the value and whether it is still in flight.
- `MotionTier::{Full, Reduced, Off}` and theme motion tokens: the five durations (120, 180, 250, 350, 500 ms), the easing per kind, the Reduced duration map [motion], [motion.animation-tiers]. Components name a token (`instant`..`slow`), never milliseconds [L src/Lunegit.Desktop/App.axaml:147-150].
- `ui.animate_f32(id, target, token)`, `ui.animate_color(id, target, token)`: look up the `Transition` retained under `id`, retarget when `target` changed, return the current value; the id's state follows `core.ui` lifetimes; reports `Repaint::Now` while in flight and nothing once settled, `At(t)` for a known single end time [core.ui].
- Tier handling: Off returns `target` at once; Reduced applies the per-kind rule (`keep`, `fade`, `snap`) and the mapped duration [motion].
- `motion::conformance(transition, tier)`: samples at 60 Hz (16.667 ms steps) from the start; passes when no step jumps more than its bound (an argument of the harness; M4 sets it per kind), the end value is exact, it settles within duration plus one frame, and no sample is NaN or infinite [D41], [derived: C4 names the checks and leaves the bound to M4].
- Indicators (`spin`) are not transitions: one revolution per 1200 ms, `angle = 360 x frac(t / 1200 ms)` from the app's `Time`, so no per-frame accumulation [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:17,45-53], [D40].

### States

n/a — no visual states; the per-kind behaviour is in `docs/motion.md` and each component's own Transitions table.

### Transitions

n/a — this entry is the machinery; the tokens and kinds table is `docs/motion.md` [motion], and each component entry lists its own changes in its Transitions table.

### Input and focus

n/a — no input. Time and tier arrive as frame inputs [R crates/lune-ui-core/src/input.rs:5,22].

### Retained state

- Transient: one `Transition` per animated id (current value, velocity, target, start time); dropped after one frame without use [core.ui].
- Persistent: none.
- Exit retention: yes: an id whose exit transition is still running stays alive and keeps `Repaint::Now` until it settles (a closing dialog, popover, menu or toast: Full 180 / Reduced 120 / Off 0 ms) [core.ui], [L src/Lunegit.Desktop/Services/DialogService.cs:87-92].
- App-owned: `Time`, `MotionTier`, the theme's motion tokens.

### Accessibility

n/a — no widget. Reduced and Off are the reduced-motion hook [motion.animation-tiers], [infra.accessibility].

### Strings

None: no user-visible text.

### Gallery scenes

- `motion-easings`: the seven easings plotted as curves over a unit square, each labelled with its token name; settled.
- `motion-kinds`: one swatch per kind of `docs/motion.md` showing its change at t = half its default duration (hover 60 ms, press 90 ms, focus 90 ms, appear 125 ms, dismiss 90 ms, page-swap 175 ms, drawer 175 ms, pop 125 ms, selection 90 ms, move 90 ms, expand 125 ms, value 90 ms, scroll 90 ms, settle 90 ms; spin at 300 ms).
- `motion-kinds-settled`: the same swatches settled.
- `motion-retarget`: one `move` retargeted at t = 60 ms of its 180 ms run, drawn at t = 120 ms, showing the path leaves from the current value without a jump.

### Painter primitives

- Existing: fill, stroke, text, xform.
- New: none.

### Acceptance

- Known-answer easing table above, to 1e-6; `Easing::CUBIC_OUT` and `EASE_IN` bit-equal the moved `cubic_bezier` values [R crates/lune-ui-core/src/widgets/menu.rs:167-193].
- A tween ends at exactly its `to`; a retarget at progress 0.5 continues with no jump: the first sample after equals the last sample before to within 1e-4.
- `Spring` known answer: `d0 = 1, v0 = 0, w = 1` gives `x(1) - target = 2 / e = 0.7357589`; no overshoot for any `d0`, `v0` of the same sign; settles toward target monotonically [derived: closed form above].
- Conformance (60 Hz): all 15 kinds under Full, Reduced and Off: no jump above the bound, exact end, settled within duration plus one frame, no NaN [D41].
- `ui.animate_*` reports `Repaint::Now` while in flight and `Idle` once every id settled; an unused transient id is gone after one frame; an exit-retained id completes then reports Idle [core.ui].
- Determinism: two runs with the same input script yield identical frames (no clock read; boundary script rule 4 stays green).

### Build

- Milestone: M4.
- Depends on: core.ui, motion.curves, motion.animation-tiers.

## Open questions

- `Spring` stiffness and default `w`: no catalogue entry needs a spring today (every kind uses a tween token), and neither Lunegit nor Wane has one. M4 builds the type per the animation model and picks no default rate until a component needs one.
- `ui.animate_*` return types beyond `f32` and colour (points, rects): added per component as its entry needs them; none is required by an entry yet.

## Notes for the planner

The vocabulary is fixed in `docs/motion.md` and DECISIONS D40 and D50; this entry only specifies the
Rust shape. The conformance harness is the D41 gate for every later component, so build and test it
before any component animates.
