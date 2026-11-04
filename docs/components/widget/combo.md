---
id: widget.combo
title: Combo box
area: widget
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/combo.rs
owner: orch/toolkit-v1
summary: Combo box: value and chevron in an input box; opens overlay.dropdown on click or key
---

## What it is

`combo`: combo box showing the current value and a chevron; opens `overlay.dropdown`. Wane's variants are fill, mono, wide, sm and short [W docs/ui/components.md:378].

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).
- Keyboard open (Enter, Space, Down), Esc closes, Tab stop and focus ring (R-widget-3).
- Semantic node with the combo-box role (R-infra-4).
- Hover and open-border motion under the animation tiers (R-motion-2).

## Existing code

`crates/lune-ui-core/src/widgets/mod.rs:71` (`combo`, 21 lines) and its helper `input_box` at `:50`. Drawn closed in the gallery's `controls` scene (`apps/gallery/src/scenes.rs:307-309`) and open in `dropdown` (`apps/gallery/src/scenes.rs:438-442`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/widgets/mod.rs:50-68,70-91`;
`crates/lune-ui-core/src/theme.rs:62,67,68,85`; `apps/gallery/src/scenes.rs:307-309,438-442`; Wane `docs/ui/components.md:375-387`;
Lunegit `src/Lunegit.Desktop/App.axaml:720-744`, `src/Lunegit.Desktop/Themes/DarkTheme.axaml:278-295`,
`.claude/docs/animation-rules.md:102,104`, `src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:244-246` (use site).
Lunegit has no `ComboBox` motion selector in `Themes/Animations.axaml` or `AnimationsReduced.axaml`.
Conflicts: (1) Wane text inset is 9 px [W docs/ui/components.md:380]; the moved code insets 10 px [R crates/lune-ui-core/src/widgets/combo.rs:52-91]; the code wins. (2) Lunegit keeps the border `#30FFFFFF` when focused [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:294]; the moved code and Wane draw the accent border when open or focused [R crates/lune-ui-core/src/widgets/input_box.rs:29-34], [W docs/ui/components.md:379]; the code wins. (3) Lunegit's ComboBox padding is 11,6 [L src/Lunegit.Desktop/App.axaml:722]; the moved code's text inset and right gutter win for the drawn look.

### Anatomy

- Rect supplied by the caller; 230 x 32 in Wane [W docs/ui/components.md:380]; the gallery passes 210 x 32 [R apps/gallery/src/scenes.rs:308].
- Input box: corner radius `item_radius` 6 [R crates/lune-ui-core/src/widgets/input_box.rs:29-34], [R crates/lune-ui-core/src/theme.rs:85]; 1 px ring `border` (#30FFFFFF) [R crates/lune-ui-core/src/widgets/input_box.rs:29-34], [R crates/lune-ui-core/src/theme.rs:62]; the same ring is Lunegit's combo border [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:291].
- Fill: `surface_input` at rest, `surface_hover` when hovered [R crates/lune-ui-core/src/widgets/input_box.rs:29-34]; Wane fill `--surface-input` [W docs/ui/components.md:380].
- Value text: 13 px UI text `fg_primary`, left 10 px, clipped 30 px short of the right edge, centred vertically [R crates/lune-ui-core/src/widgets/combo.rs:52-91].
- Chevron: `CHEVRON_DOWN` glyph 12 px in `fg_secondary`, in a 20 px slot ending 8 px from the right edge [R crates/lune-ui-core/src/widgets/combo.rs:52-91]; Wane chevron 12 px at right 7 [W docs/ui/components.md:380].
- Dropdown surface: `dropdown_bg` #F0000000 [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:278], [R crates/lune-ui-core/src/theme.rs:68]; dropdown rows hover `SurfaceHover`, selected `SurfaceSelected` [L src/Lunegit.Desktop/App.axaml:736-741] (the list itself is `overlay.dropdown`).

### States

| state | look | cite |
|---|---|---|
| rest | fill `surface_input`, ring `border`, value `fg_primary`, chevron `fg_secondary` | [R crates/lune-ui-core/src/widgets/combo.rs:52-91] [R crates/lune-ui-core/src/widgets/input_box.rs:29-34] |
| hover | fill `surface_hover` | [R crates/lune-ui-core/src/widgets/input_box.rs:29-34] |
| open | ring `accent`; dropdown open | [R crates/lune-ui-core/src/widgets/combo.rs:52-91] [R crates/lune-ui-core/src/widgets/input_box.rs:29-34]; [W docs/ui/components.md:379] |
| focus-visible | ring `accent` (same as open in the moved code); keyboard focus has no separate ring | [R crates/lune-ui-core/src/widgets/input_box.rs:29-34]; [W docs/ui/components.md:379] |
| disabled | opacity 0.4, inert | [W docs/ui/components.md:379], [L src/Lunegit.Desktop/App.axaml:481-483] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| rest ↔ hover | fill | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102], [D40] |
| closed ↔ open | ring colour | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104], [D40] |

The dropdown's open and close motion belongs to `overlay.dropdown`, not to this entry. Lunegit has no ComboBox motion selector, so both rows are the general rule under [D40]. Never animated: the value text while the list scrolls [L .claude/docs/animation-rules.md:145].

### Input and focus

- Pointer: click opens the dropdown [R crates/lune-ui-core/src/widgets/combo.rs:89]; hover over the rect.
- Keyboard: Enter, Space and Down open the dropdown; Esc closes it; the chosen value replaces the label [W docs/ui/components.md:381]; Tab stop yes [core.focus].
- Layer: `Base` for the box; the open list is `Popup` (`overlay.dropdown`) with its own focus scope.
- Focus scope: the combo itself has none; opening pushes the dropdown's scope, and closing restores focus to the combo [overlay.dropdown].
- Dismiss: Esc and an outside click act on the open dropdown only [overlay.dropdown].
- Pointer capture while pressed [core.input].

### Retained state

- Transient: hover, focus-ring colour, fill transition [core.ui].
- Persistent: none.
- App-owned: the value text, `open` (passed every frame) [R crates/lune-ui-core/src/widgets/combo.rs:74-80]; the option list and the highlighted option belong to `overlay.dropdown`.
- Exit retention: no.

### Accessibility

- Role `combo-box`; name = the label the app passes; value = the current value text; states focusable, focused, expanded (while open), disabled; actions `expand` and `collapse`, `default` opens [infra.accessibility], [W docs/ui/components.md:381].

### Strings

None: the value and option text are app data.

### Gallery scenes

- `controls` (existing): combo closed, settled.
- `dropdown` (existing): combo open with the list, settled.
- `combo-states`: rest, hovered, open, focus-visible, disabled; settled.
- `combo-hover-mid`: rest → hover, t = 60 ms (half of `instant`).
- `combo-open-mid`: closed → open border, t = 90 ms (half of `fast`).

### Painter primitives

- Existing: fill, ring, text, icon, clip-rect
- New: none

### Acceptance

- Scripted click, Enter, Space and Down each open the dropdown once; Esc closes it; a disabled combo reports no open [core.ui].
- Pixel probes at scale 2: input-box radius 12 px; ring 2 px; value text starts 20 px from the box's left edge; chevron centre 36 px from the right edge (slot `right - 28`, 20 px wide, centre at 18 px at scale 1).
- Conformance: hover fill over 60 Hz samples is monotonic, ends exact, settles within 120 ms + 1 frame.
- Semantic node: role combo-box; expanded follows `open`; injected `AccessAction` expand opens.

### Build

- Milestone: M4 (retrofit: transitions, focus, semantic node); done by M11.
- Depends on: core.ui, core.focus, infra.accessibility, motion.transitions, overlay.dropdown.

## Open questions

- None.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers the closed combo in `controls` and the open one in `dropdown`. Wane P0; the Wane width variants (wide, sm, short) are M4 additions.
