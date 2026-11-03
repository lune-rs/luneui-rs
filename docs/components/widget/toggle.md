---
id: widget.toggle
title: Toggle switch
area: widget
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/mod.rs
owner: orch/toolkit-v1
summary: On/off switch: 40x20 track, accent fill when on, knob slides between ends
---

## What it is

`toggle`: an on/off switch, 40 by 20 track, accent fill when on, knob 12 px (14 px hovered). Used
for every on/off setting; a checkbox is for picking items from a list [W docs/ui/components.md:311].

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).
- Keyboard focus and Space activation (R-widget-3).
- Semantic node with the switch role (R-infra-4).
- Animated state change under the motion tiers (R-motion-2).

## Existing code

`crates/lune-ui-core/src/widgets/mod.rs:20` (`toggle`, 29 lines, no tests). Drawn in the gallery's
`controls` scene (`apps/gallery/src/scenes.rs:254`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/widgets/mod.rs:20-48`;
`crates/lune-ui-core/src/theme.rs:13-23`; Lunegit src/Lunegit.Desktop/Themes/DarkTheme.axaml:200-210,
src/Lunegit.Desktop/Themes/Animations.axaml (no ToggleSwitch selector), .claude/docs/animation-rules.md:82-110,
src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:148-163; Wane docs/ui/components.md:305-317.
Conflict: Wane puts the knob "at 3 px" [W docs/ui/components.md:310]; the moved code insets it 4 px
[R crates/lune-ui-core/src/widgets/mod.rs:33,44], which the pixel gate pins. The code wins.

### Anatomy

- Track 40 x 20, radius 10, vertically centred in the given rect [R crates/lune-ui-core/src/widgets/mod.rs:28].
- Knob: circle 12 (14 hovered), inset 4 from the track end [R crates/lune-ui-core/src/widgets/mod.rs:29,33,44].
- Hit area: track grown 4 px left and right, 6 px up and down (48 x 32) [R crates/lune-ui-core/src/widgets/mod.rs:47].
- Off: 1 px ring `fg_secondary`, knob `fg_primary` [R crates/lune-ui-core/src/widgets/mod.rs:43,45]. On: track `accent`, knob `on_accent` [R crates/lune-ui-core/src/widgets/mod.rs:31,38]; Lunegit's on fill and stroke are AccentPrimaryColor in every pointer state [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:203-208].
- Optional label: 13 px secondary text, gap 12, min height 24 [W docs/ui/components.md:310].

### States

| state | look | cite |
|---|---|---|
| off | ring `fg_secondary`, knob left `fg_primary` | [R crates/lune-ui-core/src/widgets/mod.rs:40-45] |
| on | fill `accent`, knob right `on_accent` | [R crates/lune-ui-core/src/widgets/mod.rs:30-38] |
| hover (off) | track fill `surface_hover`, knob 14 | [R crates/lune-ui-core/src/widgets/mod.rs:29,40-42] |
| hover (on) | knob 14, fill unchanged | [R crates/lune-ui-core/src/widgets/mod.rs:29], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:204] |
| pressed | as hover (Lunegit's pressed fill equals rest) | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:205,208] |
| focus-visible | 1 px outline, offset 2 | [W docs/ui/components.md:309] |
| disabled | opacity 0.4, inert | [W docs/ui/components.md:309], [L src/Lunegit.Desktop/App.axaml:481-483] (Lunegit's disabled toolbar action) |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| off ↔ on | knob x | move | fast | sine-in-out | snap | [W docs/ui/components.md:310], [L .claude/docs/animation-rules.md:84], [D40] |
| off ↔ on | track fill and ring colour | selection | fast | linear | keep | [L .claude/docs/animation-rules.md:110], [D40] |
| rest ↔ hover | track fill, knob size 12 ↔ 14 | hover | instant | linear | fill keep, size snap | [L .claude/docs/animation-rules.md:102] |
| focus-visible on/off | outline opacity | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104] |

Lunegit leaves the switch to Fluent's default template (no selector in Animations.axaml), so the
knob slide is a D40 deviation. Never animated: nothing beyond the motion.md list.

### Input and focus

- Pointer: click toggles; hover over the hit area [R crates/lune-ui-core/src/widgets/mod.rs:47].
- Keyboard: Space toggles; Tab stop in call order; focus-visible after keyboard navigation [W docs/ui/components.md:311], [core.focus].
- Layer: whatever layer the app draws it on; no overlay, no focus scope.

### Retained state

- Transient: hover, press, knob position and colour transitions, focus-ring opacity [core.ui].
- Persistent: none. App-owned: `on` (passed every frame), the label text.
- Exit retention: no.

### Accessibility

- Role `switch`; name = the label or the row title the app passes, description = the row description (Lunegit puts both beside the switch in a settings row [L src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:148-163]); states checked, focusable, focused, disabled; action `default` toggles [infra.accessibility].

### Strings

None: the label is app text.

### Gallery scenes

- `controls` (existing): on and off, scale 1 and 2.
- `toggle-states`: off, on, hover off, hover on, focus-visible, disabled, with label; settled.
- `toggle-switching`: off → on at t = 90 ms (half of `fast`).
- `toggle-hover-mid`: off rest → hover, t = 60 ms (half of `instant`).
- `toggle-focus-mid`: focus-visible on, t = 90 ms (half of `fast`).

### Painter primitives

- Existing: fill, ring
- New: none.

### Acceptance

- Scripted click and Space each flip `changed` once; a disabled toggle reports no click [core.ui].
- Pixel probes at scale 2: track 80 x 40 px, knob 24 px (28 hovered), knob centre 20 px from the track end (unhovered), on fill = `accent`.
- Conformance: knob x over 60 Hz samples is monotonic, ends exact, settles within 180 ms + 1 frame; Reduced snaps the knob, Off snaps everything.
- Semantic node: role switch, checked follows `on`, injected `AccessAction` default toggles.

### Build

- Milestone: M4 (retrofit: transitions, focus, semantic node); done by M11.
- Depends on: core.ui, core.focus, infra.accessibility, motion.transitions.

## Open questions

- None.

## Notes for the planner

Moves from the extraction (R-ext-1); `controls` pins today's look at zero delta, so the retrofit must
keep the settled frames identical.
