---
id: overlay.popup-panel
title: Popup panel surface
area: overlay
status: in-progress
consumers: luneterm
source: crates/lune-ui-core/src/widgets/mod.rs
owner: orch/toolkit-v1
summary: Shared popup surface: radius-8 fill and 1 px ring, colours passed by the caller (used by the drop-down list)
---

## What it is

`popup_panel`: a filled surface at radius 8 with a 1 px ring; the caller passes the fill and the border [R crates/lune-ui-core/src/widgets/mod.rs:199-203]. Its one caller is the drop-down list [R crates/lune-ui-core/src/widgets/dropdown.rs:26]; the context menu draws its own glass instead [R crates/lune-ui-core/src/widgets/menu.rs:109-111].

## Needs

- Popup surface shared by menu and dropdown (R-overlay-1).
- Wane — the flyout's surface is this entry: "the surface is `overlay.popup-panel`; it must live in its own window" (Wane `docs/ui/components.md:571-584`) (R-overlay-3).

## Existing code

`crates/lune-ui-core/src/widgets/mod.rs:200` (`popup_panel`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/widgets/mod.rs:199-203`;
`crates/lune-ui-core/src/widgets/dropdown.rs:25-26`; `crates/lune-ui-core/src/widgets/menu.rs:109-111`;
`crates/lune-ui-core/src/theme.rs:62,68`; Lunegit `src/Lunegit.Desktop/App.axaml:757-783` (MenuFlyoutPresenter
template: outer radius 12, inner 11); Wane `docs/ui/components.md:571-584`; Luneterm `crates/lune-ui/src/widgets/menu.rs:109-111` at 296cfdc.
Conflicts (winner):
- Radius: Lunegit popups 12 outer, 11 inner [L src/Lunegit.Desktop/App.axaml:764-772] vs 8 in the moved code [R crates/lune-ui-core/src/widgets/mod.rs:201-202]. The moved code wins (pixel gate).
- Flyout fill: Wane `--menu-glass` with blur 30 and saturate 1.8 [W docs/ui/components.md:571-584] vs a caller-passed `bg` with no blur here [R crates/lune-ui-core/src/widgets/mod.rs:200]. The caller's fill wins; the blur is an Open question.

### Anatomy

- Fill: the caller's `bg`, radius 8 [R crates/lune-ui-core/src/widgets/mod.rs:201].
- Ring: 1 px, the caller's `border`, radius 8 [R crates/lune-ui-core/src/widgets/mod.rs:202].
- Dropdown values for reference: `dropdown_bg` #000000F0 and `border` #FFFFFF30 [R crates/lune-ui-core/src/theme.rs:68,62; crates/lune-ui-core/src/widgets/dropdown.rs:26].

### States

| state | look | cite |
|---|---|---|
| rest (the only state) | fill `bg`, ring `border` | [R crates/lune-ui-core/src/widgets/mod.rs:201-202] |

### Transitions

n/a — the surface has no motion; the owner animates it (the dropdown has none, the flyout follows the menu, [W docs/ui/components.md:571-584]).

### Input and focus

n/a — the surface takes no pointer input and no focus. Its owner adds the hit regions (dropdown.rs:51). Layer: whatever layer the owner draws it on.

### Retained state

- Transient: none.
- Persistent: none.
- App-owned: rect, `bg`, `border` (passed every frame) [R crates/lune-ui-core/src/widgets/mod.rs:200].
- Exit retention: no.

### Accessibility

n/a — the surface has no semantic node of its own; the owning list or menu provides the nodes [infra.accessibility].

### Strings

None: the surface draws no text.

### Gallery scenes

- `dropdown` (existing): the surface under an open list, settled. [R apps/gallery/src/scenes.rs:438-442]

### Painter primitives

- Existing: fill, ring. [R crates/lune-ui-core/src/widgets/mod.rs:201-202]
- New: none.

### Acceptance

- Pixel probes at scale 2: radius 16 px; ring 2 px; fill = the caller's `bg`.
- Behaviour: n/a — no input.
- Semantic node: n/a (see Accessibility).

### Build

- Milestone: M4.
- Depends on: core.painter.

## Open questions

- Blur under the surface for the flyout (Wane `--menu-glass` blur 30, saturate 1.8, W docs/ui/components.md:571-584): the moved surface has none; open for M7 (tray flyout).
- Reuse by the tooltip: decided with overlay.tooltip (step 5); open here until that entry lands.
- Whether the flyout's own window uses this surface as-is (Wane section 5 needs a separate borderless window): open, platform work.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it. Only the dropdown calls it today; the menu's surface is separate (menu.rs:109-111).
