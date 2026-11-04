---
id: widget.list-row-badge
title: List rows with status badges
area: widget
status: in-progress
consumers: lunegit, wane
source: Lunegit: src/Lunegit.Desktop/App.axaml
owner: toolkit-v1 M6
summary: Master list row: name, value and sub-line with a badge in the head; rest transparent, hover surface-hover, selected surface-selected
---

## What it is

`list-row-badge`: master list row with a head (name and a badge), a value, and a sub-line or meta. Rest is transparent; hover is `surface_hover`; selected is `surface_selected` with a bright name, never a border or accent bar. Click selects and swaps the detail pane; Up and Down move the selection [W docs/ui/components.md:477-479].

## Needs

- Wane controls the toolkit lacks (R-widget-4); list selection and keys (R-widget-3); semantic node (R-infra-4).

## Existing code

Lunegit side-panel list styles: `src/Lunegit.Desktop/App.axaml:352-365,687-698`. `src/Lunegit.Desktop/ViewModels/SidePanelRow.cs` is named in the survey and was not read. Luneterm has no list-row widget.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `docs/ui/components.md:473-485`; `docs/ui/mocks/css/tokens.css:13-16,20-23`; Lunegit `src/Lunegit.Desktop/App.axaml:209,352-365,687-698`, `src/Lunegit.Desktop/Themes/Animations.axaml:123-125`.
Conflicts: (1) Padding: Lunegit's side-panel item pads 8 horizontal and 6 vertical [L src/Lunegit.Desktop/App.axaml:360]; Wane pads 8 12 [W docs/ui/components.md:478]. Lunegit draws this list, so Lunegit wins for the side-panel list. The Wane 12 applies to master lists outside the side panel and is an Open question. (2) Selection: Lunegit's list item is transparent and the `Border.side-panel-item-active` carries the fill [L src/Lunegit.Desktop/App.axaml:687-695,357-358]; Wane's selected fill is `--surface-selected` [W docs/ui/components.md:477]; they agree on the fill. (3) Radius 6 agrees [L src/Lunegit.Desktop/App.axaml:209].

### Anatomy

- Row radius 6 [L src/Lunegit.Desktop/App.axaml:209], rows 2 apart and gap 4 [W docs/ui/components.md:478].
- Padding 8 horizontal, 6 vertical in the side panel [L src/Lunegit.Desktop/App.axaml:360].
- Name 13/20 Medium [W docs/ui/components.md:478]; Medium weight as the gallery draws it [R apps/gallery/src/scenes.rs:328].
- Value RznMono 13; sub-line RznMono 11/16 tertiary; meta 11/16 tertiary [W docs/ui/components.md:478].
- Badge in the head: `widget.badge`; a lead dot: `widget.status-dot` [W docs/ui/components.md:483].

### States

| state | look | cite |
|---|---|---|
| rest | transparent fill; secondary name | [L src/Lunegit.Desktop/App.axaml:687-690], [W docs/ui/components.md:477] |
| hover | fill `surface_hover` | [L src/Lunegit.Desktop/App.axaml:352-353], [W docs/ui/components.md:477] |
| selected | fill `surface_selected`; bright name; no bar | [L src/Lunegit.Desktop/App.axaml:357-358], [W docs/ui/components.md:477] |
| selected + hover | fill `surface_active` | [L src/Lunegit.Desktop/App.axaml:363-364] |
| focus-visible | 1 px outline, offset 2 | [widget.button] (Wane gives no focus value for rows) |
| disabled | opacity 0.4, inert | [widget.button], [L src/Lunegit.Desktop/App.axaml:481-483] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| rest ↔ hover | background | hover | instant | linear | keep | [L src/Lunegit.Desktop/Themes/Animations.axaml:123-125], [L .claude/docs/animation-rules.md:102] |
| unselected ↔ selected | background and name | selection | fast | linear | keep | [L .claude/docs/animation-rules.md:110], [D40] |
| focus-visible on/off | outline opacity | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104], [D40] |

Never animated: the rows of a virtualized list on scroll [L .claude/docs/animation-rules.md:145].

### Input and focus

- Pointer: click selects and swaps the detail pane [W docs/ui/components.md:479]; hover over the row.
- Keyboard: Up and Down move the selection [W docs/ui/components.md:479]; Enter selects.
- Tab stop: the list is one stop; the selected row takes it [core.focus].
- Layer: `Base`; no overlay, no focus scope.
- Pointer capture while pressed [core.input].

### Retained state

- Transient: hover, press, focus ring [core.ui].
- Persistent: selection as a set of row keys, scroll offset (owned by the list container) [core.ui].
- App-owned: items, the detail pane content, the selected key when the owner sets it.
- Exit retention: no.

### Accessibility

- Role `list-item` inside a `list`; name = the name; value = the value text; states selected, focusable, focused, disabled; actions `default` selects and `scroll-into-view` [infra.accessibility], [W docs/ui/components.md:479].

### Strings

None: names, values and meta are app data.

### Gallery scenes

- `list-row-badge-states`: rest, hover, selected, selected + hover, focus-visible, disabled; settled.
- `list-row-badge-hover-mid`: rest → hover, t = 60 ms (half of `instant`).
- `list-row-badge-select-mid`: unselected → selected, t = 90 ms (half of `fast`).
- `list-row-badge-focus-mid`: focus-visible on, t = 90 ms (half of `fast`).

### Painter primitives

- Existing: fill, text
- New: none

### Acceptance

- Scripted: Up and Down move the selection through the rows; click selects once and reports the key.
- Pixel probes at scale 2: row radius 12 px; padding 16 horizontal, 12 vertical in the side-panel form; selected fill = `surface_selected`.
- Conformance: hover background over 60 Hz samples is monotonic, ends exact, settles within 120 ms + 1 frame.
- Semantic node: role list-item; selected follows the selection key; injected `AccessAction` default selects.

### Build

- Milestone: M6
- Depends on: widget.badge, widget.status-dot, core.focus, core.ui, infra.accessibility, motion.transitions

## Open questions

- `conflict-padding`: the 12 px horizontal padding (Wane master list) against Lunegit's 8 px side-panel padding. Lunegit wins for the side panel; M6 decides whether the master list uses 12.

## Notes for the planner

Wane P0. Lunegit's side-panel list is the drawn source; `SidePanelRow.cs` is not read, so its behaviour is not specified here.
