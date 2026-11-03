---
id: widget.nav-item
title: Navigation item
area: widget
status: planned
consumers: wane
source: —
owner: —
summary: Navigation item: 32 px row, secondary text at rest, surface-selected fill and bright text when active, no accent bar
---

## What it is

`nav-item`: 32 px navigation row with a 16 px icon and a 13 px label. Rest is secondary text; hover tints the row; the active row (the current screen) is surface-selected with bright text and no accent bar. Up and Down move focus; Click navigates [W docs/ui/components.md:421-423].

## Needs

- Wane controls the toolkit lacks (R-widget-4).
- Up and Down keys and focus (R-widget-3); semantic node (R-infra-4).

## Existing code

None in luneui-rs or Luneterm's `lune-ui` [W docs/ui/components.md:426]. Design sources, not code to move: Lunegit `src/Lunegit.Desktop/App.axaml:368-396` (side-panel item styles) and Luneterm `spikes/glass-ui/src/settings_view.rs:229-242` (nav row, selection fill).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b, Luneterm 296cfdc. Read: `docs/ui/components.md:417-429`; `docs/ui/mocks/css/tokens.css:13,16,23,24,146`; Lunegit `src/Lunegit.Desktop/App.axaml:209,368-396`, `src/Lunegit.Desktop/Themes/Animations.axaml:123-125`; Luneterm `spikes/glass-ui/src/settings_view.rs:229-242`.
Conflicts: (1) Padding: Lunegit's side-panel button pads 8 horizontal and 6 vertical [L src/Lunegit.Desktop/App.axaml:371]; Wane fixes the height at 32 with padding 0 8 [W docs/ui/components.md:422]; the horizontal 8 agrees; Wane's height governs the vertical size. (2) Label size: Luneterm's spike draws the label at 12 [LT spikes/glass-ui/src/settings_view.rs:240]; Wane's spec is 13 [W docs/ui/components.md:422]; the spike is not spec, so 13 is used. (3) Radius 6 agrees [L src/Lunegit.Desktop/App.axaml:209], [W docs/ui/components.md:422]. Row pitch 34 in the spike (32 plus 2) agrees with Wane's rows 2 apart [LT spikes/glass-ui/src/settings_view.rs:242].

### Anatomy

- Row 32 high (`--nav-h`) [W docs/ui/components.md:422], [W docs/ui/mocks/css/tokens.css:146]; rows 2 apart [W docs/ui/components.md:422].
- Radius 6 [L src/Lunegit.Desktop/App.axaml:209]; padding 0 8; gap 8; icon 16; label 13 [W docs/ui/components.md:422].
- Active fill and label: spike uses the selected fill over the full row [LT spikes/glass-ui/src/settings_view.rs:231].
- Order: Overview, Sessions, Requests, Upstreams, Routes, Pricing, Local models, Token savings, Settings [W docs/ui/components.md:423] (app data, not the entry's).

### States

| state | look | cite |
|---|---|---|
| rest | label `fg_secondary`, no fill | [W docs/ui/components.md:421], [W docs/ui/mocks/css/tokens.css:13] |
| hover | fill `surface_hover`; label primary | [L src/Lunegit.Desktop/App.axaml:379-381], [W docs/ui/components.md:421] |
| active | fill `surface_selected`; label bright, weight 500; no accent bar | [L src/Lunegit.Desktop/App.axaml:390-392], [W docs/ui/components.md:421], [W docs/ui/mocks/css/tokens.css:16,23] |
| active + hover | fill `surface_active` | [L src/Lunegit.Desktop/App.axaml:394-396] |
| focus-visible | 1 px outline, offset 2 | [widget.button] (Wane lists "focus" with no value here) |
| pressed | fill `surface_active` | [widget.button], [W docs/ui/mocks/css/tokens.css:24] |
| disabled | opacity 0.4, inert | [widget.button], [L src/Lunegit.Desktop/App.axaml:481-483] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| rest ↔ hover | fill | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102], [L src/Lunegit.Desktop/Themes/Animations.axaml:123-125], [D40] |
| inactive ↔ active | fill and label | selection | fast | linear | keep | [L .claude/docs/animation-rules.md:110], [D40] |
| focus-visible on/off | outline opacity | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104], [D40] |

Lunegit's tween is on the Border side-panel item; the Button variant has none, so the rows carry [D40]. Never animated: the list rows on scroll [L .claude/docs/animation-rules.md:145].

### Input and focus

- Pointer: click navigates [W docs/ui/components.md:423]; hover over the row.
- Keyboard: Up and Down move focus between items [W docs/ui/components.md:423]; Enter activates.
- Tab stop: the nav list is one stop; the active item takes it [core.focus].
- Layer: `Base`; no overlay, no focus scope.
- Pointer capture while pressed [core.input].

### Retained state

- Transient: hover, press, focus ring [core.ui].
- Persistent: none.
- App-owned: the current screen (`selected`, passed every frame), the label and the icon.
- Exit retention: no.

### Accessibility

- Role `list-item` inside a `list`; name = the screen name; state `selected` for the current screen (`aria-current` in Wane) [W docs/ui/components.md:423]; states focusable, focused, disabled; action `default` navigates [infra.accessibility].

### Strings

None: screen names are app data.

### Gallery scenes

- `nav-item-states`: rest, hover, active, active + hover, focus-visible, pressed, disabled; settled.
- `nav-item-hover-mid`: rest → hover, t = 60 ms (half of `instant`).
- `nav-item-select-mid`: inactive → active, t = 90 ms (half of `fast`).
- `nav-item-focus-mid`: focus-visible on, t = 90 ms (half of `fast`).

### Painter primitives

- Existing: fill, icon, text
- New: none

### Acceptance

- Scripted: Up and Down move focus through the rows; Enter and click each navigate once.
- Pixel probes at scale 2: row 64 px high; radius 12 px; icon 32 px; rows 4 px apart.
- Conformance: active fill over 60 Hz samples is monotonic, ends exact, settles within 180 ms + 1 frame; Reduced keeps the fill.
- Semantic node: role list-item; selected follows the current screen.

### Build

- Milestone: M6
- Depends on: core.focus, core.ui, infra.accessibility, motion.transitions

## Open questions

- None.

## Notes for the planner

Wane P0. The Luneterm spike is a reference for the selection fill only; the spec values come from Wane and Lunegit.
