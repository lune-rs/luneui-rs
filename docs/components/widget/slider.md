---
id: widget.slider
title: Slider
area: widget
status: planned
consumers: any
source: —
owner: orch/toolkit-v1
summary: Slider: 4 px track with accent fill and a 12 px knob (14 hovered); look derived from the toggle knob and the progress track; arrow keys step
---

## What it is

`slider`: a horizontal value slider, 4 px track, accent fill up to the value, a 12 px knob (14 hovered). Arrow keys step the value. Neither Lunegit nor Wane has a slider, so every look value is derived from a sibling entry and cites [D51] (hard rule 8).

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-5).
- Arrow-key stepping and focus (R-widget-3); semantic node with the slider role (R-infra-4).
- Value change under the motion tiers (R-motion-2).

## Existing code

None. The knob geometry is `crates/lune-ui-core/src/widgets/mod.rs:28-47` (`toggle`), the sibling the knob derives from.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/widgets/mod.rs:28-47`; `crates/lune-ui-core/src/theme.rs:62,64,66`; Wane `docs/ui/components.md:403-415,627-640`; `docs/ui/mocks/css/tokens.css:22,40`.
Instrument: `git grep -n "<Slider"` over `src/Lunegit.Desktop` at 33a1dd41 gives no hits; `git grep -i slider` over `docs/ui/components.md` at 69931d6 gives none. No source to conflict with.
Derivation sources: `widget.toggle` (knob), `data.progress` (track and fill), `widget.scrollbar` (thumb colour family) [D51].

### Anatomy

- Track: height 4, radius 4, full width of the rect; fill `surface_overlay` #ffffff15 [W docs/ui/components.md:632], [W docs/ui/mocks/css/tokens.css:22]; derived from `data.progress` [D51].
- Value fill: `accent` from the track start to the knob [W docs/ui/components.md:632], [W docs/ui/mocks/css/tokens.css:40]; `accent` is the toggle's on fill [R crates/lune-ui-core/src/theme.rs:64].
- Knob: circle 12 (14 hovered), centred vertically on the track, centre at the value position [derived: widget.toggle knob geometry, R crates/lune-ui-core/src/widgets/mod.rs:28-29]; fill `fg_primary` (the toggle's off knob) [derived: widget.toggle, R crates/lune-ui-core/src/widgets/mod.rs:45].
- Hit area: the track grown by 4 px across and 6 px up and down, as the toggle's hit area [derived: widget.toggle, R crates/lune-ui-core/src/widgets/mod.rs:47].

### States

| state | look | cite |
|---|---|---|
| rest | track `surface_overlay`, fill `accent`, knob 12 `fg_primary` | [derived: widget.toggle, R crates/lune-ui-core/src/widgets/mod.rs:29,45], [derived: data.progress, W docs/ui/components.md:632] [D51] |
| hover | knob 14 | [derived: widget.toggle, R crates/lune-ui-core/src/widgets/mod.rs:29] [D51] |
| focus-visible | 1 px outline, offset 2, around the knob | [widget.button] [D51] |
| disabled | opacity 0.4, inert | [widget.button], [L src/Lunegit.Desktop/App.axaml:481-483] [D51] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| value by keys | knob x and fill width | value | fast | sine-in-out | snap | [derived: widget.toggle knob move, L .claude/docs/animation-rules.md:84], [D40], [D51] |
| rest ↔ hover | knob size 12 ↔ 14 | hover | instant | linear | size snap | [L .claude/docs/animation-rules.md:102], [D40], [D51] |
| focus-visible on/off | outline opacity | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104], [D40], [D51] |

Never animated: the knob while it follows the pointer during a drag, because that is high-frequency content [L .claude/docs/animation-rules.md:148].

### Input and focus

- Pointer: press on the knob and drag to set the value; the drag follows the pointer without a transition. Click-to-set on the track is not specified (Open questions).
- Keyboard: Right and Up increment, Left and Down decrement, by one `step` [contract: widget.slider role, arrow keys].
- Tab stop: yes [core.focus]; focus-visible after keyboard navigation [core.focus].
- Layer: `Base`; no overlay, no focus scope.
- Pointer capture while dragging [core.input].

### Retained state

- Transient: hover, drag in progress, focus ring [core.ui].
- Persistent: none.
- App-owned: `value`, `min`, `max`, `step` (passed every frame).
- Exit retention: no.

### Accessibility

- Role `slider`; value = the number with min, max and step the owner passes; states focusable, focused, disabled; actions `increment` and `decrement` [infra.accessibility].

### Strings

None: the slider has no text; the label is the owner's.

### Gallery scenes

- `slider-states`: rest, hover, focus-visible, disabled, value mid-track; settled.
- `slider-value-mid`: value one step up by keys, t = 90 ms (half of `fast`).
- `slider-hover-mid`: knob rest → hover, t = 60 ms (half of `instant`).
- `slider-focus-mid`: focus-visible on, t = 90 ms (half of `fast`).

### Painter primitives

- Existing: fill
- New: none

### Acceptance

- Scripted: Right and Left change the value by one step; the value stops at `min` and `max` (Open questions); a disabled slider changes nothing.
- Pixel probes at scale 2: track 8 px high, radius 8 px; knob 24 px (28 hovered); knob centre at the value position.
- Conformance: knob x over 60 Hz samples is monotonic, ends exact, settles within 180 ms + 1 frame; Reduced snaps.
- Semantic node: role slider; value follows `value`; injected `AccessAction` increment and decrement change it by one step.

### Build

- Milestone: M6
- Depends on: widget.toggle, data.progress, widget.scrollbar, core.focus, core.ui, infra.accessibility, motion.transitions

## Open questions

- `pointer-gap`: click-to-set on the track is not in any source; M6 decides.
- `clamp-gap`: whether the slider clamps the step at the ends or the owner does is not in any source; M6 decides.
- `no-consumer`: neither Lunegit nor Wane uses a slider today; the entry is kept for the M6 control set (R-widget-5).

## Notes for the planner

New control with no source. Every look value above is derived and cites [D51]; a value that a reviewer cannot trace to a sibling entry is a defect.
