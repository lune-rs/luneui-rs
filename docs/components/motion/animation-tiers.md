---
id: motion.animation-tiers
title: Animation tiers and reduced motion
area: motion
status: in-progress
consumers: any
source: crates/lune-ui-core/src/motion/tokens.rs
owner: orch/toolkit-v1
summary: Full, Reduced and Off motion tiers passed in by the app, with the per-token Reduced duration map
---

## What it is

Three tiers that every transition obeys: **Full** (everything in `docs/motion.md`), **Reduced**
(opacity and colour only, no transforms, slides or scales, no press-scale, durations stepped down by
a fixed map) and **Off** (every change snaps). Lunegit mounts one of two stylesheets or none at run
time [L src/Lunegit.Desktop/Theming/AnimationApplicator.cs:10-13,45-56]; the toolkit has no
stylesheets, so the tier is a value the app passes into the frame (`MotionTier`) and every
`ui.animate_*` call reads it. The toolkit never reads an OS reduced-motion setting (hard rule 2); the
app decides, for example from its own settings.

## Needs

- A reduced-motion tier and a tier that turns transitions off (R-motion-2).
- The tier arrives as a frame input; nothing reads the OS or the clock (R-nfr-1).
- Every transition is conformance-tested under each tier (R-motion-3).

## Existing code

None in this repo. Lunegit: `src/Lunegit.Desktop/Theming/AnimationApplicator.cs` (mounts the Full or
Reduced style file, or neither, hot, on the UI thread), `src/Lunegit.Desktop/Themes/Animations.axaml`
(632 lines, Full tier), `src/Lunegit.Desktop/Themes/AnimationsReduced.axaml` (Reduced tier),
`src/Lunegit.Desktop/Services/DialogService.cs:87-92` (the exit retention per tier).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit .claude/docs/animation-rules.md:1-172,
src/Lunegit.Desktop/Theming/AnimationApplicator.cs:1-60, src/Lunegit.Desktop/Themes/AnimationsReduced.axaml
(all), src/Lunegit.Desktop/Themes/Animations.axaml (selectors and durations), src/Lunegit.Desktop/Services/DialogService.cs:81-92,
.claude/docs/dialog-exit-timing.md:14-44, src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:413-440,
src/Lunegit.Infrastructure/Localization/en.json:65-67,618-620; `docs/motion.md`.
Conflict: Lunegit names the third tier None; the toolkit calls it Off (the same behaviour)
[L src/Lunegit.Desktop/Theming/AnimationApplicator.cs:53-55]. Lunegit's doc says "durations effectively
halved" [L .claude/docs/animation-rules.md:157], but the stylesheets do not halve literally: they step
a token down (Base to Fast, Moderate to Fast, Fast to Instant for exits, selection stays Fast); the
stylesheets win and `docs/motion.md` records the map.

### Anatomy

- Tiers: Full, Reduced, Off [L .claude/docs/animation-rules.md:152-158]; the toolkit value is `MotionTier::{Full, Reduced, Off}` passed in the frame input [motion].
- Full: every kind and magnitude of `docs/motion.md` [L .claude/docs/animation-rules.md:156].
- Reduced: transforms dropped, press-scale dropped, hover and selection colour tweens kept [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:10-17,20-29], the transform-bearing surfaces (page swap, side panel, dialog card, menus) keep an opacity fade only [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:158-164,198-209,298-310].
- Off: nothing is mounted, so every state flips instantly [L src/Lunegit.Desktop/Theming/AnimationApplicator.cs:53-55].
- Reduced duration map: instant to instant; fast to instant (exits, scrollbar fade) with selection staying fast; base to fast; moderate to fast; slow to base [derived: no `slow` use exists in either stylesheet]; the evidence rows are in `docs/motion.md` [motion].
- Dialog exit retention: Full 180 ms, Reduced 120 ms, Off 0 ms [L src/Lunegit.Desktop/Services/DialogService.cs:87-92].
- The tab-reorder slide is Full only: Reduced and Off snap [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:421].
- Hot application: changing the level takes effect without a restart [L src/Lunegit.Desktop/Theming/AnimationApplicator.cs:10-13]; in the toolkit the new tier applies from the next frame.
- Spinners and busy indicators are state indicators, not transitions; Lunegit's spin timer never consults the tier [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:34-63], so they run in every tier [D40].

### States

n/a — a configuration value, not a component. The three tiers are the states; each is a column of the Transitions table below.

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| hover colour | background | hover | instant | linear | keep | [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:24-26] |
| press | scale 0.98 | press | fast | quad-out | snap | [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:15-17] |
| page swap | opacity and translateY | page-swap | moderate | quad-out | fade | [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:198-209], [L src/Lunegit.Desktop/Themes/Animations.axaml:295-304] |
| dialog open | opacity and scale 0.96 | pop | base | quad-out | fade | [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:298-310], [L src/Lunegit.Desktop/Themes/Animations.axaml:454-463] |
| tab-reorder neighbour slide | translateX | settle | fast | sine-in-out | snap | [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:421-431] |

One row per behaviour that differs by tier; the full per-kind matrix is `docs/motion.md`. Never
animated: nothing beyond the motion.md list.

### Input and focus

n/a — no pointer or keyboard behaviour. The tier is chosen by the app (Lunegit exposes it as the
Appearance setting `AnimationLevel` [L .claude/docs/animation-rules.md:154-155]).

### Retained state

- Transient: none held here; in-flight transitions live in `core.ui`'s retained store [core.ui].
- Persistent: none. App-owned: the current `MotionTier`, passed every frame.
- Exit retention: it defines the retention per tier (180 / 120 / 0 ms for a dialog exit) [L src/Lunegit.Desktop/Services/DialogService.cs:87-92].

### Accessibility

n/a — not a widget. The tier is the toolkit's reduced-motion hook; `Reduced` and `Off` serve motion-sensitive users [L .claude/docs/animation-rules.md:13,157]. The app maps the OS setting to a tier; the toolkit reads no environment [D3].

### Strings

None: the tier names and the setting row are app text (Lunegit keeps them as `settings.appearance.animation.level.title`, `...desc` and `enum.animationLevel.full|reduced|none` [L src/Lunegit.Infrastructure/Localization/en.json:66,67,618-620]).

### Gallery scenes

- `motion-tiers`: the kind swatches of `motion.transitions`' `motion-kinds` scene rendered three times (Full, Reduced, Off) at t = 90 ms (half of `fast`), one panel per tier.
- `motion-tiers-settled`: the same panels, settled; they must be pixel-identical, since a tier changes motion and never the resting look.

### Painter primitives

- Existing: fill, text, xform.
- New: none.

### Acceptance

- Under Full, Reduced and Off, each transition kind reaches its exact end value: Full within duration plus one frame, Reduced within the mapped duration plus one frame, Off on the same frame [motion].
- Under Reduced no transform component changes (a scripted sample of offset and scale stays at rest); under Off every animated property equals its target on the frame of the change.
- A tier change between frames takes effect on the next frame, and a settled page reports `Repaint::Idle` under all three tiers [D41].
- Dialog exit retention is 180, 120 and 0 ms by tier, then Idle [L src/Lunegit.Desktop/Services/DialogService.cs:87-92], [core.ui].
- Semantic tree and strings are unaffected by the tier.

### Build

- Milestone: M4.
- Depends on: motion.transitions, core.ui.

## Open questions

- None. Resolved: a tier change snaps everything in flight on the next frame [D69].

## Notes for the planner

The Reduced duration map is the one place the tier rule is data, not code; keep it as a theme motion
token so an app can retune it. The stub's open question (which properties animate, which curves) is
answered by `docs/motion.md`.
