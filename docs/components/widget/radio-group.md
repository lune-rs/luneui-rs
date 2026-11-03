---
id: widget.radio-group
title: Radio group
area: widget
status: planned
consumers: wane
source: —
owner: —
summary: Radio group: vertical 16 px rings, accent ring and 6 px dot when selected; arrow keys move the selection inside the group
---

## What it is

`radio-group`: vertical group of radio rows. Unselected: 16 px ring. Selected: accent fill with a 6 px dot. Click selects; arrow keys move the selection; Tab enters the group once [W docs/ui/components.md:337-339].

## Needs

- Wane controls the toolkit lacks (R-widget-4).
- Keyboard selection and focus (R-widget-3); semantic node (R-infra-4).

## Existing code

None: Wane lists lune-ui as missing [W docs/ui/components.md:342]. Design source, not code to move: `src/Lunegit.Desktop/Themes/DarkTheme.axaml:185-197` (accent ring, white dot).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `docs/ui/components.md:333-345`; `docs/ui/mocks/css/tokens.css:13,27,43`; Lunegit `src/Lunegit.Desktop/Themes/DarkTheme.axaml:185-198`; this repo `crates/lune-ui-core/src/theme.rs:64,66`.
Conflicts: none on the values drawn. Lunegit leaves the inner dot to Fluent's white default [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:188-190]; Wane's dot is `--on-accent` [W docs/ui/components.md:337]; both are white (`on_accent` #FFFFFF [R crates/lune-ui-core/src/theme.rs:66]).

### Anatomy

- Circle 16 px, vertically centred in the row [W docs/ui/components.md:338].
- Unselected: 1 px ring `--text-secondary` [W docs/ui/components.md:338], [W docs/ui/mocks/css/tokens.css:13].
- Selected: fill `accent` [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:191-196], [R crates/lune-ui-core/src/theme.rs:64]; 6 px dot in `on_accent` [W docs/ui/components.md:337].
- Row min height 28; label 13 `--text-secondary`; gap 8 [W docs/ui/components.md:338].
- Vertical only [W docs/ui/components.md:336].

### States

| state | look | cite |
|---|---|---|
| unselected | 1 px ring `fg_secondary`, no fill | [W docs/ui/components.md:338] |
| selected | accent fill, 6 px dot `on_accent` | [W docs/ui/components.md:337], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:191-196] |
| hover | row fill `surface_hover` | [widget.checkbox] (same hover fill; Wane lists no value here) |
| focus-visible | 1 px outline, offset 2 | [widget.button] (same focus ring; Wane lists the state without a value) |
| disabled | opacity 0.4, inert | [W docs/ui/components.md:337], [L src/Lunegit.Desktop/App.axaml:481-483] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| unselected ↔ selected | fill and dot | selection | fast | linear | keep | [L .claude/docs/animation-rules.md:110], [D40] |
| rest ↔ hover | row fill | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102], [D40] |
| focus-visible on/off | outline opacity | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104], [D40] |

Lunegit has no RadioButton motion selector; every row is the general rule under [D40]. Never animated: nothing in the list is a virtualized list.

### Input and focus

- Pointer: click selects [W docs/ui/components.md:339].
- Keyboard: arrow keys move the selection inside the group; Tab enters the group once [W docs/ui/components.md:339]; roving focus inside the group.
- Tab stop: one stop for the group [core.focus].
- Layer: `Base`; no overlay, no focus scope.
- Pointer capture while pressed [core.input].

### Retained state

- Transient: hover, focus ring, selection transition [core.ui].
- Persistent: none.
- App-owned: the selected key (passed every frame), the option labels.
- Exit retention: no.

### Accessibility

- Role `radio-group` with `radio` children; name = the group label the app passes; each radio's name = its label; states `checked`, focusable, focused, disabled; actions `default` selects; arrow keys move focus and selection [infra.accessibility], [W docs/ui/components.md:339].

### Strings

None: option labels are app data.

### Gallery scenes

- `radio-group-states`: unselected, selected, one row hovered, selected row focus-visible, disabled group; settled.
- `radio-select-mid`: unselected → selected, t = 90 ms (half of `fast`).
- `radio-hover-mid`: row rest → hover, t = 60 ms (half of `instant`).
- `radio-focus-mid`: focus-visible on, t = 90 ms (half of `fast`).

### Painter primitives

- Existing: fill, ring, text
- New: none

### Acceptance

- Scripted: arrow down and up move the selection through each row and wrap or stop as specified; Tab enters the group once; a disabled group reports no selection.
- Pixel probes at scale 2: circle 32 px; dot 12 px at the centre of the circle; row min height 56 px.
- Conformance: selection fill over 60 Hz samples is monotonic, ends exact, settles within 180 ms + 1 frame; Reduced keeps the fill.
- Semantic node: role radio-group; checked follows the selected key.

### Build

- Milestone: M6
- Depends on: core.focus, core.ui, infra.accessibility, motion.transitions

## Open questions

- None.

## Notes for the planner

Wane P0. The arrow-key wrap rule is not in Wane's text; the acceptance test needs it, so M6 decides it and records it here.
