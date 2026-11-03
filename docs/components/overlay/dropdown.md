---
id: overlay.dropdown
title: Drop-down list
area: overlay
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/dropdown.rs
owner: orch/toolkit-v1
summary: Combo drop-down option list that flips above its anchor near the window bottom
---

## What it is

`dropdown`: an option list under its `anchor`, flipped above it near the window bottom [R crates/lune-ui-core/src/widgets/dropdown.rs:7-24]. Rows are 32 high; a check mark marks the selected option [R crates/lune-ui-core/src/widgets/dropdown.rs:17,46-49].

## Needs

- Drop-down list that stays on screen (R-overlay-1).
- Wane — States: item rest; hover (`--surface-hover`); selected (`--surface-selected`, 12 px check at the right); disabled (tertiary text, inert). (R-overlay-1)
- Wane — Sizes: Panel padding 4, radius 9, 1 px `--popup-border-soft` (white at .08; user, 2026-11-04), the same glass as `overlay.menu` (`--menu-glass`, blur 30; user, 2026-11-03: not lune-ui's opaque `dropdown_bg`); rows 32 high, radius 4, padding 0 6 0 9; 4 to 5 px below the anchor, same width; no shadow. (R-overlay-1)
- Wane — Behaviour: Opens under the combo and flips above it near the window bottom; a click outside or Esc closes it; Up, Down, Enter choose. Not mocked: type-ahead, scrolling a long list. (R-overlay-1)

## Existing code

`crates/lune-ui-core/src/widgets/dropdown.rs:9` (`dropdown`); 54 lines.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/widgets/dropdown.rs:1-54`;
`crates/lune-ui-core/src/widgets/mod.rs:200-203`; `crates/lune-ui-core/src/theme.rs:60-70`;
`apps/gallery/src/scenes.rs:438-454`; Lunegit `src/Lunegit.Desktop/Themes/Animations.axaml:1-632` (no ComboBox
selector), `src/Lunegit.Desktop/App.axaml:720-745` (ComboBox item states), `src/Lunegit.Desktop/App.axaml:757-763`
(popup corner 12); Wane `docs/ui/components.md:529-542`.
Conflicts (winner):
- Surface: Wane `--menu-glass` blur 30 [W docs/ui/components.md:529-542] vs opaque `dropdown_bg` #000000F0 [R crates/lune-ui-core/src/theme.rs:68, crates/lune-ui-core/src/widgets/dropdown.rs:26]. The moved code wins (pixel gate); the glass look is an Open question for a DECISIONS row.
- Radius: Wane 9, Lunegit popup 12 [L src/Lunegit.Desktop/App.axaml:761], code 8 [R crates/lune-ui-core/src/widgets/mod.rs:201-202]. The code wins.
- Panel inset: Wane 4 and code 4 agree [R crates/lune-ui-core/src/widgets/dropdown.rs:28-29].

### Anatomy

- Option row: 32 high; x inset 4 and width `panel.w − 8`; row y = panel y + 4 + i × 32 [R crates/lune-ui-core/src/widgets/dropdown.rs:17,28-33].
- Panel: width = anchor width [R crates/lune-ui-core/src/widgets/dropdown.rs:25]; height = options × 32 + 8 [R crates/lune-ui-core/src/widgets/dropdown.rs:18]; fill `dropdown_bg` #000000F0 at radius 8, 1 px ring `border` #FFFFFF30 [R crates/lune-ui-core/src/widgets/mod.rs:200-202, crates/lune-ui-core/src/theme.rs:62,68].
- Placement: 4 below the anchor [R crates/lune-ui-core/src/widgets/dropdown.rs:19]; flips above, at `anchor.y − 4 − h`, minimum 8, when `below + h > window h − 8` [R crates/lune-ui-core/src/widgets/dropdown.rs:20-24].
- Label: 10 px inset, `fg_primary` when enabled, `fg_tertiary` when disabled [R crates/lune-ui-core/src/widgets/dropdown.rs:40-45, crates/lune-ui-core/src/theme.rs:56,58].
- Check mark (selected): 12 px glyph, `fg_secondary`, in a 20 px box 26 px from the row's right edge [R crates/lune-ui-core/src/widgets/dropdown.rs:46-48].
- Row radius 4 [R crates/lune-ui-core/src/widgets/dropdown.rs:36,38].

### States

| state | look | cite |
|---|---|---|
| rest | no row fill; label `fg_primary` | [R crates/lune-ui-core/src/widgets/dropdown.rs:35-45] |
| hover (enabled row) | fill `surface_hover` #FFFFFF10 | [R crates/lune-ui-core/src/widgets/dropdown.rs:37-38, crates/lune-ui-core/src/theme.rs:60] |
| selected | fill `surface_selected` #FFFFFF18, check mark | [R crates/lune-ui-core/src/widgets/dropdown.rs:35-36,46-48, crates/lune-ui-core/src/theme.rs:61] |
| selected and hovered | selected fill (the hover fill is skipped) | [R crates/lune-ui-core/src/widgets/dropdown.rs:35-38] |
| disabled option | label `fg_tertiary`; no hit region, so no hover | [R crates/lune-ui-core/src/widgets/dropdown.rs:40-44,50-52] |
| focus-visible (keyboard row) | not in the moved code | open question |
| open | the list drawn by the caller | [R crates/lune-ui-core/src/widgets/dropdown.rs:9-15] |
| flipped | list above the anchor | [R crates/lune-ui-core/src/widgets/dropdown.rs:20-24] |

### Transitions

n/a — no motion: the moved code has none (dropdown.rs:9-54), and Lunegit's Animations.axaml has no ComboBox
or popup-list selector [L src/Lunegit.Desktop/Themes/Animations.axaml:1-632]. Open and close snap.
No transition is given for row hover or selection; open question.

### Input and focus

- Pointer: hover an enabled row, click picks it [R crates/lune-ui-core/src/widgets/dropdown.rs:37,50-52]; a disabled row has no hit region. An outside click closes [W docs/ui/components.md:529-542].
- Keyboard: Up, Down, Enter choose; Esc closes [W docs/ui/components.md:529-542] (requirements, not mocked).
- Tab stop: the combo owns the tab stop; the rows do not take focus (open question).
- Layer: Popup (the Wane native note: the list is drawn above scrolling content) [W docs/ui/components.md:529-542]. The moved code does not set a layer (dropdown.rs:9-54).
- Focus scope: push on open; pop restores focus to the anchor combo [core.focus]. Escape and outside click act on the top overlay only [core.focus].
- Pointer capture while pressed: none in the moved code.

### Retained state

- Transient: row hover; keyboard row highlight (none yet, open question).
- Persistent: none. Scroll offset for long lists is not in the moved code [layout.scroll-area].
- App-owned: options (text and enabled flag), selected index, the anchor rect, the open flag [R crates/lune-ui-core/src/widgets/dropdown.rs:9-15].
- Exit retention: no (no motion).

### Accessibility

- Owner: the combo (`combo-box`, widget.combo); the list is `list`, each row `list-item` with states `selected`, `disabled`, `focusable` [infra.accessibility].
- Name: the combo's label (app text); value: the selected option's text [infra.accessibility].
- Actions: default (choose), dismiss (Esc) [infra.accessibility].
- Live region: none.

### Strings

None: option texts come from the app (dropdown.rs:9-12).

### Gallery scenes

- `dropdown` (existing): list open under the combo with a selected row, settled. [R apps/gallery/src/scenes.rs:438-442]
- `dropdown-flipped` (existing): list open above the combo near the bottom edge, settled. [R apps/gallery/src/scenes.rs:444-454]
- `dropdown-hover`: an enabled row hovered, settled (new).
- `dropdown-disabled`: one option disabled, settled (new).

### Painter primitives

- Existing: fill, ring, text, icon. [R crates/lune-ui-core/src/widgets/mod.rs:200-202, crates/lune-ui-core/src/widgets/dropdown.rs:45,48]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): Up, Down and Enter choose the enabled row; Esc and an outside click close; a disabled row never fires. None exists yet.
- Placement: when `below + h > window h − 8` the list flips above the anchor (dropdown.rs:20-24).
- Pixel probes at scale 2: row 64 px high, panel radius 16 px, check glyph 24 px, row radius 8 px. Source values in Anatomy.
- Conformance: none (no motion).
- Semantic node: `list` with `list-item` rows; selected row has the `selected` state.

### Build

- Milestone: M4 (retrofit), M7 (variants, keyboard, anchored menus).
- Depends on: core.focus, infra.accessibility, overlay.popup-panel, widget.combo, layout.scroll-area.

## Open questions

- Glass look (Wane `--menu-glass`, blur 30) vs the moved opaque `dropdown_bg`: a look change needs a DECISIONS row (hard rule 8); open.
- Long option lists scroll: not in the moved code and not mocked (W docs/ui/components.md:529-542); open for layout.scroll-area.
- Type-ahead: not mocked; open.
- Keyboard focus-visible on a row and the Tab stop: not in any source; open.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: partial: widgets/dropdown.rs:9 draws an opaque `dropdown_bg` panel; the menu glass (theme.rs `menu_glass`) is missing on it; Wane priority P0. Native note: Drawn on the modal or popup layer, above scrolling content; the mock's `position: absolute` must become a popup layer with window-edge flipping. Full spec: Wane `docs/ui/components.md`, section `overlay.dropdown`; mock: Wane `docs/ui/mocks/components.html`.
