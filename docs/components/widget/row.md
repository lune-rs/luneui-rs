---
id: widget.row
title: Row (setting row, list item row)
area: widget
status: planned
consumers: wane
source: —
owner: —
summary: Row: setting row (title, description, control at right, min 56 high) or compact item row (32 high, four columns); hover tints the row; rows are not clickable
---

## What it is

`row`: a setting row (title and description, control at the right, min height 56) or an item row (compact, 32 high, four fixed columns). Hover tints the row; the control inside takes the input; rows are not clickable [W docs/ui/components.md:462-465].

## Needs

- Wane controls the toolkit lacks (R-widget-4).

## Existing code

Wane: the settings row exists as app code, not a widget [W docs/ui/components.md:468], at `spikes/glass-ui/src/settings_view.rs:30,280` in Luneterm. Design sources, not code to move: Lunegit `src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:148-165`; Luneterm `spikes/glass-ui/src/settings_view.rs:290-300`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b, Luneterm 296cfdc. Read: `docs/ui/components.md:459-471`; `docs/ui/mocks/css/tokens.css:13,14,21,16`; Lunegit `src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:148-165`, `src/Lunegit.Desktop/Themes/Animations.axaml:111-121`; Luneterm `spikes/glass-ui/src/settings_view.rs:290-300`.
Conflicts: (1) Padding: Lunegit's setting row pads 12 on all sides [L src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:148]; Wane's setting padding is 10 vertical and 12 horizontal [W docs/ui/components.md:464]. The drawn Lunegit row wins: 12 vertical. (2) Luneterm's spike uses 10 top and bottom [LT spikes/glass-ui/src/settings_view.rs:299]; the spike is not spec. (3) Description colour: Lunegit's description uses `ForegroundMuted` [L src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:155]; Wane's is secondary [W docs/ui/components.md:464]; see Open questions.

### Anatomy

- Setting row: radius 6; padding 12; min height 56 [W docs/ui/components.md:464]; Lunegit padding [L src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:148].
- Title 13/20 `fg_primary` [L src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:151-153], [W docs/ui/components.md:464].
- Description 11/16 with wrap [L src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:154-158], [W docs/ui/components.md:464]; top margin 2 [L src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:158].
- Control at the right, 16 px left margin [L src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:160-163].
- Item row: min height 32; padding 0 12; gap 8; rows 2 apart [W docs/ui/components.md:464]. Cols grid 40 / 1fr / 88 / 176, padding 8 12 [W docs/ui/components.md:464,691].
- Meta mono: RznMono 11 secondary [W docs/ui/components.md:462,696]; values RznMono 13 [W docs/ui/components.md:464].

### States

| state | look | cite |
|---|---|---|
| rest | no fill | [W docs/ui/components.md:463] |
| hover | fill `surface_hover` | [W docs/ui/components.md:463], [W docs/ui/mocks/css/tokens.css:21] |
| off (`is-off`) | title secondary, value tertiary | [W docs/ui/components.md:463,691] |
| disabled (`is-disabled`) | opacity 0.4, inert | [W docs/ui/components.md:463], [widget.button] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| rest ↔ hover | background | hover | instant | linear | keep | [L src/Lunegit.Desktop/Themes/Animations.axaml:113-120], [L .claude/docs/animation-rules.md:102] |
| on ↔ off | title and value colour | selection | fast | linear | keep | [L .claude/docs/animation-rules.md:110], [D40] |

Lunegit animates the setting row's background only [L src/Lunegit.Desktop/Themes/Animations.axaml:111-113] and has no press scale on rows. Never animated: the list rows on scroll [L .claude/docs/animation-rules.md:145].

### Input and focus

- Pointer: hover highlights the row [W docs/ui/components.md:465]; rows are not clickable [W docs/ui/components.md:465].
- Keyboard: none of its own; the control inside takes focus and keys.
- Tab stop: no (the control inside is one).
- Layer: `Base`; no overlay, no focus scope.

### Retained state

- Transient: hover background [core.ui].
- Persistent: none.
- App-owned: title, description, meta, value, the `off` and `disabled` flags (passed every frame).
- Exit retention: no.

### Accessibility

- Role `group`; name = the title; description = the description; state `disabled` when the flag is set; the control inside keeps its own node [infra.accessibility].

### Strings

None: title, description and meta text are app data.

### Gallery scenes

- `row-setting`: setting row with a toggle at the right, settled.
- `row-item-cols`: item rows with lead checkbox, title, badge, value in four columns, settled.
- `row-states`: rest, hover, off, disabled; settled.
- `row-hover-mid`: rest → hover, t = 60 ms (half of `instant`).
- `row-off-mid`: on → off text colour, t = 90 ms (half of `fast`).

### Painter primitives

- Existing: fill, text
- New: none

### Acceptance

- Pixel probes at scale 2: setting row min 112 px high; radius 12 px; title starts 24 px from the left edge.
- Conformance: hover background over 60 Hz samples is monotonic, ends exact, settles within 120 ms + 1 frame.
- Semantic node: role group; name equals the title; disabled follows the flag.

### Build

- Milestone: M6
- Depends on: widget.toggle, widget.checkbox, core.painter, text.shaping

## Open questions

- `token-gap`: Lunegit's description uses `ForegroundMuted` [L src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:155] and Wane's is `--text-secondary` [W docs/ui/components.md:464]. The value of `ForegroundMuted` is not read; M6 maps it or confirms Wane's token.

## Notes for the planner

Wane P0. The row is a layout pattern; the controls inside carry the look.
