---
id: widget.spinner
title: Spinner
area: widget
status: planned
consumers: any
source: —
owner: orch/toolkit-v1
summary: Spinner: a 14 px glyph that turns a full revolution every 1.2 s while busy; static under Reduced and Off
---

## What it is

`spinner`: a busy indicator, a glyph that rotates about its centre at one revolution per 1.2 s while busy [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:17,38-53]. Lunegit uses it on the sync icon in the indexing banner [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:15-20].

## Needs

- Drawn through the painter with the rotate primitive (R-widget-5).
- Reduced and Off tiers (R-motion-2, R-motion-3); busy as a semantic state (R-infra-4).

## Existing code

`src/Lunegit.Desktop/Controls/SpinBehavior.cs` (attached property `Enabled` that rotates any control), used at `src/Lunegit.Desktop/Controls/IndexingBanner.axaml:16`. No luneui-rs code yet.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `src/Lunegit.Desktop/Controls/SpinBehavior.cs:9-60`; `src/Lunegit.Desktop/Controls/IndexingBanner.axaml:15-20`; `src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:10-16`; `.claude/docs/animation-rules.md:143-150`.
Instrument: `git grep -n "<Slider\|<ProgressRing"` over `src/Lunegit.Desktop` at 33a1dd41 gives no hits (the positive control `<ProgressBar` gives hits). Wane: `git grep -i spinner` over `docs/ui/components.md` at 69931d6 gives none. The Lunegit rotation is the only source.
Conflict: `.claude/docs/animation-rules.md:150` bars animations longer than 500 ms "except the deliberate flash cue", and this loop is continuous; the rule's exceptions do not name it (Open questions).

### Anatomy

- Glyph: `Sync` codicon, 14 x 14 [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:15-18].
- Rotation about the centre (relative origin 0.5, 0.5) [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:38].
- Period: 360 degrees per 1200 ms [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:17].
- Update: a timer every 16 ms; the angle advances by elapsed time, so a late tick does not slow the spin [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:45,50-53].
- Colour: the caller's; the banner uses `StatusWarningLight` [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:19].
- Timer stops on detach and restarts on attach [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:56-60].

### States

| state | look | cite |
|---|---|---|
| spinning (busy) | glyph rotates, 1.2 s per revolution | [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:17,38-53] |
| stopped (not busy, or detached) | glyph static at its last angle | [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:56-60] |
| Reduced tier | static glyph | [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:10-16] |
| Off tier | static glyph | [motion] |

No hover, focus or disabled look: the spinner takes no input.

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| idle ↔ busy | rotation | spin | slow | linear | snap | [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:17], [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:10-16], [motion] |

The duration column names the nearest token; the loop is 1200 ms, which is not a token (Open questions). Linear easing because the angle advances at a constant rate [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:53]. Off always snaps [motion].

### Input and focus

n/a — the spinner takes no pointer or key input and has no Tab stop.

### Retained state

- Transient: the rotation angle, per instance, reset when detached [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:40,56].
- Persistent: none.
- App-owned: the busy flag (the attached `Enabled` property) and the colour, passed every frame [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:19-21].
- Exit retention: no.

### Accessibility

- Role `progress` (indeterminate); state `busy` while spinning; no value; name = the label the owner passes [infra.accessibility].

### Strings

None: the spinner has no text.

### Gallery scenes

- `spinner-spinning`: glyph at t = 300 ms (a quarter turn), scale 2 and 1.
- `spinner-stopped`: busy off, settled.
- `spinner-reduced`: Reduced tier, glyph static, settled.
- `spinner-start-mid`: idle → busy, t = 250 ms (half of `slow`).

### Painter primitives

- Existing: icon
- New: rotate (`src/Lunegit.Desktop/Controls/SpinBehavior.cs:38-53`)

### Acceptance

- Behaviour: with injected time, the angle advances 360 degrees per 1200 ms (within one frame); stopped holds its angle.
- Pixel probes at scale 2: glyph 28 px; the rotation centre is the glyph centre.
- Conformance: angle over 60 Hz samples is linear, wraps at 360, and is constant under Reduced and Off.
- Semantic node: role progress; busy follows the flag.

### Build

- Milestone: M6
- Depends on: core.painter, core.ui, infra.accessibility, motion.transitions

## Open questions

- `motion-gap`: the continuous spin against animation-rules.md:150 (500 ms cap) and the 1200 ms period, which is not one of the five tokens. The `motion.md` spin rule must settle both before M6; until then the table names `slow`.

## Notes for the planner

Attached-property pattern: the spinner is a behaviour on any icon, not a control. Lunegit-only in use today.
