---
id: widget.steps
title: Wizard steps
area: widget
status: planned
consumers: wane
source: —
owner: —
summary: Wizard steps: four 32 px rows in the sidebar slot with upcoming, current and done states; not clickable
---

## What it is

`steps`: wizard step list in the sidebar slot, on the same rows as the nav list. Upcoming, current and done states. Not clickable: Back and Continue move through the steps [W docs/ui/components.md:505-507].

## Needs

- Wane controls the toolkit lacks (R-widget-4).

## Existing code

None in luneui-rs or Luneterm's `lune-ui` [W docs/ui/components.md:510]. Wane is the only source for this entry.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `docs/ui/components.md:501-513,695`; `docs/ui/mocks/css/tokens.css:14,16,23,27,40,43`.
No conflict: one source. The row language is `widget.nav-item` [W docs/ui/components.md:509].

### Anatomy

- Rows 32 high, radius 6, 2 apart; padding 0 8; gap 8 [W docs/ui/components.md:506].
- Number circle 20 px with a 1 px `--border` ring; number 11 Medium [W docs/ui/components.md:506], [W docs/ui/mocks/css/tokens.css:27].
- Label 13 in a 148 px column that never wraps; a long label is shortened [W docs/ui/components.md:506,695].
- Lead line 13/20 tertiary, 12 px below the first row [W docs/ui/components.md:695].

### States

| state | look | cite |
|---|---|---|
| upcoming | tertiary label; ring number | [W docs/ui/components.md:505], [W docs/ui/mocks/css/tokens.css:14] |
| current | fill `surface_selected`; bright label, weight 500 | [W docs/ui/components.md:505], [W docs/ui/mocks/css/tokens.css:16,23] |
| done | accent circle with a 12 px check in `on_accent`; secondary label | [W docs/ui/components.md:505], [W docs/ui/mocks/css/tokens.css:40,43] |

Not interactive, so no hover, press, focus or disabled look [W docs/ui/components.md:507].

### Transitions

n/a — Wane specifies no motion for the step rows [W docs/ui/components.md:505-507]; state changes come from the owner's Back and Continue.

### Input and focus

n/a — not clickable [W docs/ui/components.md:507]. Back and Continue are the owner's buttons (`widget.button`).

### Retained state

- Transient: none.
- Persistent: none.
- App-owned: the current step index and each step's label (passed every frame) [W docs/ui/components.md:507].
- Exit retention: no.

### Accessibility

- Role `list` of `list-item`s; the current row is state `selected`; name = the step label; `done` has no state in the vocabulary, see Open questions [infra.accessibility].

### Strings

None: step labels are app data.

### Gallery scenes

- `steps-states`: four rows, one each upcoming, current and done, settled.

### Painter primitives

- Existing: fill, ring, icon, text
- New: none

### Acceptance

- Pixel probes at scale 2: row 64 px high; number circle 40 px; label column 296 px and never wraps.
- Conformance: n/a — no transition.
- Semantic node: role list; the current item is selected; a long label is shortened, not wrapped.

### Build

- Milestone: M6
- Depends on: widget.nav-item, core.painter, text.shaping

## Open questions

- `role-gap`: the done state needs a semantic state (for example `checked`) that the vocabulary lacks; M6 picks a state from the list or extends the list.

## Notes for the planner

Wane P2 in the mocks. Lives in the sidebar slot, so the sidebar owner places it.
