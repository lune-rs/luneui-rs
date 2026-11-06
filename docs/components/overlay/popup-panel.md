---
id: overlay.popup-panel
title: Popup panel surface
area: overlay
status: done
consumers: luneterm
source: crates/lune-ui-core/src/widgets/popup_panel.rs
owner: —
summary: Shared popup surface: radius-8 fill and 1 px ring, colours passed by the caller (used by the drop-down list)
---

## What it is

`popup_panel`: a filled surface at radius 8 with a 1 px ring; the caller passes the fill and the border [R crates/lune-ui-core/src/widgets/popup_panel.rs:6-11]. Its one caller is the drop-down list [R crates/lune-ui-core/src/widgets/dropdown.rs:26]; the context menu draws its own glass instead [R crates/lune-ui-core/src/widgets/menu.rs:109-111].

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
- Radius: Lunegit popups 12 outer, 11 inner [L src/Lunegit.Desktop/App.axaml:764-772] vs 8 in the moved code [R crates/lune-ui-core/src/widgets/popup_panel.rs:8-9]. The moved code wins (pixel gate).
- Flyout fill: Wane `--menu-glass` with blur 30 and saturate 1.8 [W docs/ui/components.md:571-584] vs a caller-passed `bg` with no blur here [R crates/lune-ui-core/src/widgets/popup_panel.rs:7]. The caller's fill wins; the blur is an Open question.

### Anatomy

- Fill: the caller's `bg`, radius 8 [R crates/lune-ui-core/src/widgets/popup_panel.rs:8].
- Ring: 1 px, the caller's `border`, radius 8 [R crates/lune-ui-core/src/widgets/popup_panel.rs:9].
- Dropdown values for reference: `dropdown_bg` #000000F0 and `border` #FFFFFF30 [R crates/lune-ui-core/src/theme.rs:68,62; crates/lune-ui-core/src/widgets/dropdown.rs:26].

### States

| state | look | cite |
|---|---|---|
| rest (the only state) | fill `bg`, ring `border` | [R crates/lune-ui-core/src/widgets/popup_panel.rs:8-9] |

### Transitions

n/a — the surface has no motion; the owner animates it (the dropdown has none, the flyout follows the menu, [W docs/ui/components.md:571-584]).

### Input and focus

n/a — the surface takes no pointer input and no focus. Its owner adds the hit regions (dropdown.rs:51). Layer: whatever layer the owner draws it on.

### Retained state

- Transient: none.
- Persistent: none.
- App-owned: rect, `bg`, `border` (passed every frame) [R crates/lune-ui-core/src/widgets/popup_panel.rs:7].
- Exit retention: no.

### Accessibility

n/a — the surface has no semantic node of its own; the owning list or menu provides the nodes [infra.accessibility].

### Strings

None: the surface draws no text.

### Gallery scenes

- `dropdown` (existing): the surface under an open list, settled. [R apps/gallery/src/scenes.rs:438-442]

### Painter primitives

- Existing: fill, ring. [R crates/lune-ui-core/src/widgets/popup_panel.rs:8-9]
- New: none.

### Acceptance

- Pixel probes at scale 2: radius 16 px; ring 2 px; fill = the caller's `bg`.
- Behaviour: n/a — no input.
- Semantic node: n/a (see Accessibility).
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_dropdown.rs::the_popup_panel_is_16_px_radius_with_a_2_px_ring_in_the_callers_fill` (the panel the dropdown draws through `popup_panel`: radius 16, ring 2, the caller's fill); the search picker's popup at scale 2: `widget_search_picker.rs::at_scale_2_the_popup_radius_is_16_the_search_bar_8_and_the_dash_is_1_px_in_a_2_4_pattern`; the popover's `popover.rs::pixel_probes_at_scale_2`. It takes no input and emits no node (its host does): `widget_popover.rs::surface_none_draws_nothing_of_its_own`.

### Build

- Milestone: M4.
- Depends on: core.painter.
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_dropdown.rs::the_popup_panel_is_16_px_radius_with_a_2_px_ring_in_the_callers_fill, crates/lune-ui-core/tests/widget_search_picker.rs::at_scale_2_the_popup_radius_is_16_the_search_bar_8_and_the_dash_is_1_px_in_a_2_4_pattern, crates/lune-ui-core/tests/widget_popover.rs::pixel_probes_at_scale_2`; page `dropdown` (the panel on its scenes); unverified here: none.

## Open questions

- Resolved [D143, D145]: blur under the surface for the flyout (Wane `--menu-glass` blur 30, saturate 1.8, W docs/ui/components.md:571-584). `popup_panel` stays opaque and caller-coloured; the glass surface is `popover::glass_panel` (`Surface::Glass`, the glass dropdown, the flyout), and saturate 1.8 is the frame's `FrameParams.saturation`.
- Answered: the tooltip reuses it (`crates/lune-ui-core/src/widgets/tooltip.rs:10`), as do the dropdown, the popover and the search picker's popup.
- Answered (M10): the flyout's own window does not use this surface; `Ui::flyout_window` draws the flyout's own glass card (`overlay.flyout`).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it. Only the dropdown calls it today; the menu's surface is separate (menu.rs:109-111).
