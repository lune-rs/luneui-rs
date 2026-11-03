---
id: widget.stepper
title: Numeric stepper
area: widget
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/mod.rs
owner: orch/toolkit-v1
summary: Numeric stepper: value with minus and plus buttons; clamping and step size belong to the app
---

## What it is

`stepper`: numeric stepper (NumericUpDown in the Fluent naming) with the value and minus and plus buttons. Clamping and step size belong to the app [W docs/ui/components.md:367].

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).
- Up and Down keys, typed digits, Tab stop and focus-within border (R-widget-3).
- Semantic node with the spin-button role and value bounds (R-infra-4).
- Button hover and focus motion under the animation tiers (R-motion-2).

## Existing code

`crates/lune-ui-core/src/widgets/mod.rs:94` (`stepper`, 24 lines; `input_box` at `:50`). Drawn in the gallery's `controls` scene (`apps/gallery/src/scenes.rs:278-285`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/widgets/mod.rs:50-68,94-117`;
`crates/lune-ui-core/src/theme.rs:62,68,85`; `apps/gallery/src/scenes.rs:278-285`; Wane `docs/ui/components.md:361-373`;
Lunegit `src/Lunegit.Tests/Desktop/Settings/TerminalSettingsViewModelTests.cs:216` (the only trace of an old NumericUpDown in Lunegit's source).
Instrument: `git grep -n NumericUpDown 33a1dd41 -- src/` at Lunegit 33a1dd41 returns two hits, the test comment above and `src/Lunegit.Desktop/STRUCTURE.md:344` (a doc line). No Lunegit control uses the type, so Lunegit gives no look for this entry.
Conflicts: (1) Wane sets the value in RznMono 13 [W docs/ui/components.md:366]; the moved code draws it in the UI font [R crates/lune-ui-core/src/widgets/mod.rs:16,103]; the code wins. (2) Wane's boxes sit 3 px from the top and 31 and 3 px from the right [W docs/ui/components.md:366]; the moved code puts them 4 px from the top and at right 32 and 4 [R crates/lune-ui-core/src/widgets/mod.rs:106-108]; the code wins. (3) Wane's width is 130 [W docs/ui/components.md:366]; the gallery passes 140 [R apps/gallery/src/scenes.rs:281]; the width is the caller's, and 130 is the spec value.

### Anatomy

- Rect supplied by the caller; 130 x 32 in Wane (`--control-w-stepper`, `--control-h`) [W docs/ui/components.md:366].
- Input box: fill `surface_input`, 1 px ring `border`, corner radius `item_radius` 6 [R crates/lune-ui-core/src/widgets/mod.rs:50-68,102], [R crates/lune-ui-core/src/theme.rs:62,85].
- Value: `value` as text in the UI font, left 10 px, `fg_primary` [R crates/lune-ui-core/src/widgets/mod.rs:103].
- Minus and plus boxes: 24 wide, height `r.h - 8` (24 at 32), top 4 px, corner radius 4 [R crates/lune-ui-core/src/widgets/mod.rs:105-110,112]. Minus box spans right 32 to right 56; plus box right 4 to right 28 [R crates/lune-ui-core/src/widgets/mod.rs:106].
- Glyphs: "−" and "+" in 13 px `fg_secondary`, centred in each box [R crates/lune-ui-core/src/widgets/mod.rs:104,114]. The minus is U+2212.
- Button hover: fill `surface_hover` in the hovered box [R crates/lune-ui-core/src/widgets/mod.rs:111-113].

### States

| state | look | cite |
|---|---|---|
| rest | input box as Anatomy; buttons unfilled | [R crates/lune-ui-core/src/widgets/mod.rs:102] |
| button hover | that box's fill `surface_hover` | [R crates/lune-ui-core/src/widgets/mod.rs:111-112], [W docs/ui/components.md:365] |
| focus-within | ring `accent` instead of `border` | [W docs/ui/components.md:365]; not drawn by the moved code (`input_box` gets `focused` false at mod.rs:102) |
| disabled | opacity 0.4, inert | [W docs/ui/components.md:365], [L src/Lunegit.Desktop/App.axaml:481-483] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| button rest ↔ hover | box fill | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102], [D40] |
| focus-within on/off | ring colour | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104], [D40] |

Lunegit has no stepper or NumericUpDown motion; both rows are the general rule under [D40]. Value changes snap: a number is high-frequency content [L .claude/docs/animation-rules.md:148].

### Input and focus

- Pointer: click on minus or plus steps once [R crates/lune-ui-core/src/widgets/mod.rs:115]; hover tracks each button.
- Keyboard: Up and Down step; typed digits set the value [W docs/ui/components.md:367]; Tab stop yes [core.focus].
- Layer: `Base`; no overlay, no focus scope.
- Pointer capture while pressed [core.input].

### Retained state

- Transient: button hover, press, focus ring colour [core.ui].
- Persistent: none; typed text in progress belongs to the app.
- App-owned: `value` (passed every frame), the minimum, maximum and step (clamping is the app's) [R crates/lune-ui-core/src/widgets/mod.rs:94-100], [W docs/ui/components.md:367].
- Exit retention: no.

### Accessibility

- Role `spin-button`; value = the number, with min, max and step the app passes; states focusable, focused, disabled; actions `increment`, `decrement`, `set-value` [infra.accessibility].

### Strings

None: "−" and "+" are symbols drawn from literals [R crates/lune-ui-core/src/widgets/mod.rs:104].

### Gallery scenes

- `controls` (existing): stepper at value 12, scale 2 and 1, settled.
- `stepper-states`: rest, minus hovered, plus hovered, focus-within, disabled; settled.
- `stepper-button-hover-mid`: plus rest → hover, t = 60 ms (half of `instant`).
- `stepper-focus-mid`: focus-within on, t = 90 ms (half of `fast`).

### Painter primitives

- Existing: fill, ring, text
- New: none

### Acceptance

- Scripted: click on plus and minus each report one activation; Up and Down report one step each [core.ui].
- Pixel probes at scale 2: input box radius 12 px; minus box 48 x 48 px; value text starts 20 px from the box edge; `controls` at zero delta.
- Conformance: button hover fill over 60 Hz samples is monotonic, ends exact, settles within 120 ms + 1 frame; Reduced keeps the fill tween.
- Semantic node: role spin-button; value follows `value`; injected `AccessAction` increment and decrement change it by one step.

### Build

- Milestone: M4 (retrofit: transitions, focus, semantic node); done by M11.
- Depends on: core.ui, core.focus, infra.accessibility, motion.transitions.

## Open questions

- `token-gap`: the whole-control hover look is in Wane's state list without a value [W docs/ui/components.md:365], and the moved code never hovers the input box (`mod.rs:102` passes `hovered` false); M4 chooses a value and cites it.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers the stepper in `controls`. Wane P0. The gallery's 140 px width is its own choice, not a spec value.
