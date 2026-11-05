---
id: overlay.dropdown
title: Drop-down list
area: overlay
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/dropdown.rs
owner: toolkit-v1 M7
summary: Combo drop-down option list that flips above its anchor near the window bottom
---

## What it is

`Ui::dropdown` and `Ui::dropdown_with`: an option list under its `anchor`, flipped above it near the window bottom. Rows are 32 high; a check mark marks the selected option. `DropdownOptions { glass, max_rows }` [D143] draws the menu's glass surface and scrolls a long list; both are off by default, so `dropdown` draws what it drew. The Painter-API `dropdown` stays for the legacy scenes.

## Needs

- Drop-down list that stays on screen (R-overlay-1).
- Wane — States: item rest; hover (`--surface-hover`); selected (`--surface-selected`, 12 px check at the right); disabled (tertiary text, inert). (R-overlay-1)
- Wane — Sizes: Panel padding 4, radius 9, 1 px `--popup-border-soft` (white at .08; user, 2026-11-04), the same glass as `overlay.menu` (`--menu-glass`, blur 30; user, 2026-11-03: not lune-ui's opaque `dropdown_bg`); rows 32 high, radius 4, padding 0 6 0 9; 4 to 5 px below the anchor, same width; no shadow. (R-overlay-1)
- Wane — Behaviour: Opens under the combo and flips above it near the window bottom; a click outside or Esc closes it; Up, Down, Enter choose. Not mocked: type-ahead, scrolling a long list. (R-overlay-1)

## Existing code

`crates/lune-ui-core/src/widgets/dropdown.rs` (`dropdown`, `Ui::dropdown`, `Ui::dropdown_with`, `DropdownOptions`); tests in `crates/lune-ui-core/tests/widget_dropdown.rs`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/widgets/dropdown.rs:1-54`;
`crates/lune-ui-core/src/widgets/mod.rs:200-203`; `crates/lune-ui-core/src/theme.rs:60-70`;
`apps/gallery/src/scenes.rs:438-454`; Lunegit `src/Lunegit.Desktop/Themes/Animations.axaml:1-632` (no ComboBox
selector), `src/Lunegit.Desktop/App.axaml:720-745` (ComboBox item states), `src/Lunegit.Desktop/App.axaml:757-763`
(popup corner 12); Wane `docs/ui/components.md:529-542`.
Conflicts (winner):
- Surface: Wane `--menu-glass` blur 30 [W docs/ui/components.md:529-542] vs opaque `dropdown_bg` #000000F0 [R crates/lune-ui-core/src/theme.rs:68]. Both ship [D143]: the opaque list is the default (zero pixel delta), `DropdownOptions.glass` draws the menu's surface (`popover::glass_panel`: radius 9, the blurred scene, `menu_glass`, the `menu_popup_border` ring) and Wane opts in.
- Radius: Wane 9, Lunegit popup 12 [L src/Lunegit.Desktop/App.axaml:761], code 8 [R crates/lune-ui-core/src/widgets/popup_panel.rs:8-9]. The code wins.
- Panel inset: Wane 4 and code 4 agree [R crates/lune-ui-core/src/widgets/dropdown.rs:28-29].

### Anatomy

- Option row: 32 high; x inset 4 and width `panel.w − 8`; row y = panel y + 4 + i × 32 [R crates/lune-ui-core/src/widgets/dropdown.rs:17,28-33].
- Panel: width = anchor width [R crates/lune-ui-core/src/widgets/dropdown.rs:25]; height = options × 32 + 8 [R crates/lune-ui-core/src/widgets/dropdown.rs:18]; fill `dropdown_bg` #000000F0 at radius 8, 1 px ring `border` #FFFFFF30 [R crates/lune-ui-core/src/widgets/mod.rs:200-202, crates/lune-ui-core/src/theme.rs:62,68].
- Placement: 4 below the anchor [R crates/lune-ui-core/src/widgets/dropdown.rs:19]; flips above, at `anchor.y − 4 − h`, minimum 8, when `below + h > window h − 8` [R crates/lune-ui-core/src/widgets/dropdown.rs:20-24].
- Label: 10 px inset, `fg_primary` when enabled, `fg_tertiary` when disabled [R crates/lune-ui-core/src/widgets/dropdown.rs:40-45, crates/lune-ui-core/src/theme.rs:56,58].
- Check mark (selected): 12 px glyph, `fg_secondary`, in a 20 px box 26 px from the row's right edge [R crates/lune-ui-core/src/widgets/dropdown.rs:46-48].
- Row radius 4 [R crates/lune-ui-core/src/widgets/dropdown.rs:36,38].
- Glass: the same panel with the menu's surface at radius 9; the rows, the check and the fills are unchanged [D143].
- Long list: `max_rows` shows at most that many rows (at least one) and the panel is `max_rows` × 32 + 8 high; the rows scroll in the 32-high-row viewport between the 4 px paddings through `Ui::scroll_area` (wheel, trackpad and the overlay thumb on the right edge; no horizontal bar). The list opens with the selected row in view, centred when it was below the fold; a key that moves the highlight brings that row into view (`Reveal::Nearest`). The cost is the unscrolled list's: every row is drawn each frame and clipped, so a list of thousands wants the search picker or a virtual list.

### States

| state | look | cite |
|---|---|---|
| rest | no row fill; label `fg_primary` | [R crates/lune-ui-core/src/widgets/dropdown.rs:35-45] |
| hover (enabled row) | fill `surface_hover` #FFFFFF10 | [R crates/lune-ui-core/src/widgets/dropdown.rs:37-38, crates/lune-ui-core/src/theme.rs:60] |
| selected | fill `surface_selected` #FFFFFF18, check mark | [R crates/lune-ui-core/src/widgets/dropdown.rs:35-36,46-48, crates/lune-ui-core/src/theme.rs:61] |
| selected and hovered | selected fill (the hover fill is skipped) | [R crates/lune-ui-core/src/widgets/dropdown.rs:35-38] |
| disabled option | label `fg_tertiary`; no hit region, so no hover | [R crates/lune-ui-core/src/widgets/dropdown.rs:40-44,50-52] |
| focus-visible (keyboard row) | the hover fill on the highlighted row | [D76] |
| open | the list drawn by the caller | [R crates/lune-ui-core/src/widgets/dropdown.rs:9-15] |
| flipped | list above the anchor | [R crates/lune-ui-core/src/widgets/dropdown.rs:20-24] |

### Transitions

n/a — no motion: the moved code has none (dropdown.rs:9-54), and Lunegit's Animations.axaml has no ComboBox
or popup-list selector [L src/Lunegit.Desktop/Themes/Animations.axaml:1-632]. Open and close snap.
No transition is given for row hover or selection; open question.

### Input and focus

- Pointer: hover an enabled row, click picks it [R crates/lune-ui-core/src/widgets/dropdown.rs:37,50-52]; a disabled row has no hit region. An outside click closes [W docs/ui/components.md:529-542].
- Keyboard: Up, Down, Enter choose; Esc closes [W docs/ui/components.md:529-542] (requirements, not mocked); built as [D76].
- Tab stop: the combo owns the tab stop; the rows do not take focus (open question).
- Layer: Popup (the Wane native note: the list is drawn above scrolling content) [W docs/ui/components.md:529-542]. The moved code does not set a layer (dropdown.rs:9-54).
- Focus scope: push on open; pop restores focus to the anchor combo [core.focus]. Escape and outside click act on the top overlay only [core.focus].
- Pointer capture while pressed: none in the moved code.

### Retained state

- Transient: row hover; keyboard row highlight [D76].
- Persistent: the scroll offset of a long list, kept by the scroll area for the list's id.
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
- `dropdown-glass`: the glass surface, settled.
- `dropdown-scroll`: thirty options in eight rows, glass, opened on the twenty-first, a row hovered, settled.

### Painter primitives

- Existing: fill, ring, text, icon. [R crates/lune-ui-core/src/widgets/mod.rs:200-202, crates/lune-ui-core/src/widgets/dropdown.rs:45,48]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): Up, Down and Enter choose the enabled row; Esc and an outside click close; a disabled row never fires; the glass list emits the backdrop and the glass fill; thirty options with `max_rows` 8 scroll, Down past the last visible row reveals it, a selected row 20 opens in view, a click after a wheel step picks the row under the pointer; in a disabled scope the keys are taken, nothing is picked and `disabled_violations` is empty (`tests/widget_dropdown.rs`).
- Placement: when `below + h > window h − 8` the list flips above the anchor (dropdown.rs:20-24).
- Pixel probes at scale 2: row 64 px high, panel radius 16 px, check glyph 24 px, row radius 8 px. Source values in Anatomy.
- Conformance: none (no motion).
- Semantic node: `list` with `list-item` rows; selected row has the `selected` state.

### Build

- Milestone: M4 (retrofit), M7 (the glass surface and the scrolling list, step 4).
- Depends on: core.focus, infra.accessibility, overlay.popup-panel, widget.combo, layout.scroll-area.

## Open questions

- Resolved [D143]: glass look: both, the opaque list by default and `glass` for the menu's surface.
- Resolved [D143]: long option lists scroll through `max_rows`.
- Type-ahead: no source; not built [D143].
- Resolved [D76]: the list takes focus; a keyboard row highlight (Up and Down skip disabled rows, Enter picks, it starts at the selected row) is drawn with the hover fill.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: partial: widgets/dropdown.rs:9 draws an opaque `dropdown_bg` panel; the menu glass (theme.rs `menu_glass`) is missing on it; Wane priority P0. Native note: Drawn on the modal or popup layer, above scrolling content; the mock's `position: absolute` must become a popup layer with window-edge flipping. Full spec: Wane `docs/ui/components.md`, section `overlay.dropdown`; mock: Wane `docs/ui/mocks/components.html`.
