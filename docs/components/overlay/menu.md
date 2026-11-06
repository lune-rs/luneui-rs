---
id: overlay.menu
title: Glass context menu
area: overlay
status: done
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/menu.rs
owner: —
summary: Glass context menu with open and close motion
---

## What it is

`Ui::menu` and `Ui::menu_with` over `MenuRow`, built from Luneium's ContextMenu.css and Popup.css; `menu_open_anim` and `menu_close_anim` drive its motion (`MENU_IN`, `MENU_OUT`). Rows are `Item`, `Entry(MenuItem)` (disabled, danger, chevron), `Header` and `Separator` [D141]; `MenuPlace` opens the menu at a point or next to an anchor [D142]. The Painter-API `menu` with `MenuEntry` stays for the legacy scenes. `menu_rows` is the row frame the flyout shares.

## Needs

- Glass context menu (R-overlay-1).
- Open and close motion with the app passing time in (R-motion-1).
- Wane — Variants: plain; with a two-line header (`__header`: name with a status dot, status line); danger item (`c-menu__item--danger`, `--err` text; Quit Wane); chevron hints (submenu); shortcut hints (11 px quaternary) (R-overlay-1)
- Wane — States: item rest; hover (`--popup-item-hover`); pressed (`--popup-item-pressed`); disabled (`--menu-disabled`); menu opening and closing (motion below). (R-overlay-1)
- Wane — Sizes: Min width 190, padding 5, radius 9 (`--radius-menu`), 1 px `--popup-border-soft` (white at .08; user, 2026-11-04), fill `--menu-glass` (rgb 44 44 44 / .616); items 28 high, radius 4, padding 0 8, label 12 (`--font-menu`), icon slot 14 (13 px glyph), hint 11; separator slot 11 high with a 1 px line. (R-overlay-1)
- Wane — Behaviour: Opens at the pointer; click or Enter picks an item; Esc or an outside click closes. Motion: open .18 s ease-out from 4 px lower at 97%, close .12 s ease-in to 2 px lower at 98%, origin top-left (menu.rs:21-22). Arrow keys and shortcut routing are requirements, not mocked. (R-overlay-1)

## Existing code

`crates/lune-ui-core/src/widgets/menu.rs`: `Ui::menu`, `Ui::menu_with`, `menu_rows`, `menu_layout_at`, `menu_size`, `Ui::menu_window` (the menu filling a window of its own, M10 D220), `menu_paint`, the free `menu`, `menu_open_anim`, `menu_close_anim`. Tests: `crates/lune-ui-core/tests/widget_menu.rs`, `tests/c4.rs`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/widgets/menu.rs:1-192`;
`crates/lune-ui-core/src/theme.rs:71-82`; `crates/lune-ui-core/src/scene.rs:17-27`; `apps/gallery/src/scenes.rs:95-108,366-392`;
Lunegit `src/Lunegit.Desktop/Themes/Animations.axaml:250-265`, `src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:167-184`,
`src/Lunegit.Desktop/App.axaml:757-815,818-849`, `src/Lunegit.Desktop/Themes/MenuResources.axaml:13-16`,
`.claude/docs/animation-rules.md:102-110`; Wane `docs/ui/components.md:543-556`.
Conflicts (winner):
- Menu padding: Wane 5 [W docs/ui/components.md:543-556] vs 6 [R crates/lune-ui-core/src/widgets/menu.rs:71]. The moved code wins (pixel gate).
- Row height and radius: Lunegit MenuItem MinHeight 30, radius 8, padding 10,6 [L src/Lunegit.Desktop/App.axaml:787-790] vs code 28 rows, radius 4, 8 px label inset [R crates/lune-ui-core/src/widgets/menu.rs:72,89,124,126]. The code wins.
- Label weight: Lunegit Medium [L src/Lunegit.Desktop/App.axaml:792] vs regular 12 px [R crates/lune-ui-core/src/widgets/menu.rs:76]. The code wins.
- Open duration: Lunegit Base 250 ms quad-out [L src/Lunegit.Desktop/Themes/Animations.axaml:250-256] vs moved MENU_IN 180 ms cubic-out [R crates/lune-ui-core/src/widgets/menu.rs:21,46-49]. The moved values win; the Lunegit fade is the source of the opacity-only Reduced rule.
- Pressed and disabled looks: Lunegit's values win [L src/Lunegit.Desktop/App.axaml:802-810, src/Lunegit.Desktop/Themes/DarkTheme.axaml:305,307] [D141].
- Chevron colour: Wane quaternary [W docs/ui/mocks/css/components.css:676] vs `menu_text_tertiary` [D141]. The decision wins.
- Header name: 13 (`font_base`) Medium, as Wane [W docs/ui/mocks/css/components.css:674]; the menu's label stays 12 [D141]. Settled in M7 step 13 (ruling C15); the 20 and 16 px lines and the 6 px padding are Wane's.

### Anatomy

- Panel: min width 190; width = max(190, 6 + 8 + 14 + 8 + label + 16 + hint + 8 + 6) when a shortcut exists [R crates/lune-ui-core/src/widgets/menu.rs:78,89]; height = 2 × 6 + 28 per item + 11 per separator [R crates/lune-ui-core/src/widgets/menu.rs:71-73,79-80,90-92].
- Placement: x clamped to `[8, window w − w − 8]`; y is the pointer y, or flipped above (`max(y − h, 8)`) when `y + h > window h − 8` [R crates/lune-ui-core/src/widgets/menu.rs:95-100].
- Glass: backdrop blur at radius 9 [R crates/lune-ui-core/src/widgets/menu.rs:109]; fill `menu_glass` #2C2C2C9D [R crates/lune-ui-core/src/widgets/menu.rs:110, crates/lune-ui-core/src/theme.rs:79]; 1 px ring `menu_popup_border` #FFFFFF3A [R crates/lune-ui-core/src/widgets/menu.rs:111, crates/lune-ui-core/src/theme.rs:75]; radius 9 [R crates/lune-ui-core/src/widgets/menu.rs:74].
- Header row: `menu::HEADER` 48 high: 6 padding, a 20 px line (8 px `status_dot::paint`, 8 gap, name `font_base` 13 medium `menu_text_primary`), a 16 px line (status `font_s`, `menu_text_tertiary`, under the dot), 6 padding, 8 in from the row [W docs/ui/mocks/css/components.css:673-675] [D141]. It widens the panel like an item.
- Chevron: `icon::CHEVRON_RIGHT` 12 px, 8 in from the right, `menu_text_quaternary`; a row may carry a hint as well, right-aligned 2 px before the chevron, and the row counts 16 + hint + 2 + 12 in the width (16 + 12 without a hint) [W docs/ui/mocks/css/components.css:676] [D141] (ruling C16; D141 does not say the chevron wins).
- Item row: 28 high, x inset 6, width panel − 12 [R crates/lune-ui-core/src/widgets/menu.rs:72,121]; hover fill `menu_item_hover` #FFFFFF16 at radius 4 [R crates/lune-ui-core/src/widgets/menu.rs:123-124, crates/lune-ui-core/src/theme.rs:76].
- Icon: 13 px glyph in a 14 px slot 8 px in from the row, `menu_text_tertiary` [R crates/lune-ui-core/src/widgets/menu.rs:126-128, crates/lune-ui-core/src/theme.rs:73]; label 8 px after the slot, 12 px, `menu_text_primary` [R crates/lune-ui-core/src/widgets/menu.rs:76,130-133, crates/lune-ui-core/src/theme.rs:72].
- Shortcut hint: `font_s` 11 px, `menu_text_quaternary` #FFFFFF66, right-aligned 8 px in [R crates/lune-ui-core/src/widgets/menu.rs:77,135-138, crates/lune-ui-core/src/theme.rs:74,82].
- Separator: 11 high; 1 px line at +5, inset 2 px each side, `menu_separator` #FFFFFF22 [R crates/lune-ui-core/src/widgets/menu.rs:145-156, crates/lune-ui-core/src/theme.rs:77].
- Lunegit's icon size 14 for every MenuItem icon [L src/Lunegit.Desktop/Themes/MenuResources.axaml:13-16] matches the 14 px slot.

### States

| state | look | cite |
|---|---|---|
| item rest | label `menu_text_primary`, no fill | [R crates/lune-ui-core/src/widgets/menu.rs:121-134] |
| item hover | fill `menu_item_hover` (only while `interactive`) | [R crates/lune-ui-core/src/widgets/menu.rs:123-124] |
| item pressed | fill `menu_item_pressed` #FFFFFF23 while held, in place of the hover fill, also dragged off the row; gives way when a key lights another row | [L src/Lunegit.Desktop/App.axaml:808-810], [D141] |
| item disabled | text, glyph, hint and chevron `menu_text_disabled` #FFFFFF58, no fill; blocks, takes nothing | [L src/Lunegit.Desktop/App.axaml:802-806], [D141] |
| item danger | text and glyph `err` | [W docs/ui/mocks/css/components.css:383], [D141] |
| focus-visible | the highlight: the hover fill on the lit row, no ring | [D141] |
| menu opening | alpha, offset and scale from `menu_open_anim` | [R crates/lune-ui-core/src/widgets/menu.rs:46-49,102-108] |
| menu closing | alpha, offset and scale from `menu_close_anim`; not interactive | [R crates/lune-ui-core/src/widgets/menu.rs:52-58,62-63,140] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| open | opacity 0→1, y offset 4→0, scale 0.97→1, origin top-left | pop | fast | cubic-out | fade (opacity only, Fast) | [R crates/lune-ui-core/src/widgets/menu.rs:21,46-49], [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:167-175] |
| close | opacity 1→0, y offset 0→2, scale 1→0.98 | dismiss | instant | ease-in | fade | [R crates/lune-ui-core/src/widgets/menu.rs:22,52-57] |
| item rest ↔ hover | row fill, `animate_color` at `menu::HOVER`; a key moves the highlight at once, so one row shows it the frame the key arrives; a row disabled while lit loses its fill at once; snapped in the exit painter | hover | instant | linear | keep | [D40], [D141], [docs/motion.md] |

Open and close use the moved curves (the Lunegit ease-out family is the same kind of curve,
[L .claude/docs/animation-rules.md:82-86]).
Never animated: the rows themselves, the pressed fill, and the panel's size (the menu sizes once per open).

### Input and focus

- Pointer: a click on an enabled item picks it; separators, the header and disabled items are not picked, and the panel blocks them, so a press on one is not an outside press. The menu opens at a point or next to an anchor. A press outside closes and reaches nothing, a press on the trigger included, so it does not reopen [D77].
- Keyboard: Up and Down wrap over the enabled items, Home and End go to the ends, Enter and Space pick, Escape closes [D141]. Tab stays on the rows. A Control, Alt or Command chord on an arrow stays a shortcut. No type-ahead (no source).
- Highlight: at most one row is lit, the row of the last input: the pointer entering a row, the pointer leaving its own row, a press, an arrow, Home, End, a keyboard open (the first enabled row). A pointer open lights nothing. A key moves from the lit row, not the focused one: with nothing lit, Down and Home light the first enabled row and Up and End the last. A pointer resting on a row does not relight it after an arrow, and leaves a key's row lit. Enter and Space pick the lit row, else the focused one. A row disabled while lit goes out; Enter then does nothing until a key or the pointer lights a row. Enter on a focused disabled row, and every key in a disabled scope, is consumed and does nothing.
- Tab stop: one, roving: focus lands on the first enabled item at open and follows the keys [D141].
- Layer: Popup; the menu does not set its layer (menu.rs:64-161), and the gallery draws it on `LayerId::Popup` [R apps/gallery/src/scenes.rs:369]. Layers under popups go to an offscreen scene so the glass can blur them [R crates/lune-ui-core/src/scene.rs:26-27].
- Focus scope: push on open; pop restores focus to the previous widget or anchor [core.focus]. Esc and outside click act on the top overlay only [core.focus].
- Pointer capture while pressed: none in the moved code.

### Retained state

- Transient: the highlight (lit row, whether a key moved it, the row under the pointer, whether focus was on a row) and the row fades; the open and close progress [motion].
- Persistent: none (no submenus, no selection).
- App-owned: rows, placement, the name, whether the menu is called.
- Exit retention: yes: Full 120 / Reduced 120 / Off 0 ms [core.ui] (Full is MENU_OUT, menu.rs:22).

### Accessibility

- Role `menu`, named by `MenuOptions.name`, else `luneui.menu.name`; each item `menu-item` with name = item text and description = shortcut text [D142] [infra.accessibility].
- A header is `static-text`: name = its name, description = its status line.
- Item states: `focusable`, `focused`; a disabled item is `disabled`, with no `focusable` and no action. In a disabled scope every node is [infra.accessibility].
- Actions: default (Enter, click), dismiss (Esc) [infra.accessibility].
- Separators and the glyph are not nodes. Live region: none.

### Strings

`luneui.menu.name` "Menu", the accessible name when the app gives none. Item text, shortcut and header text come from the app.

### Gallery scenes

- `menu` (existing): six entries (three items, a separator, two more), item 1 hovered, settled. [R apps/gallery/src/scenes.rs:95-101,366-392]
- `menu-opening` (existing): t = 90 ms (half of MENU_IN, 180 ms). [R apps/gallery/src/scenes.rs:102-108]
- `menu-closing`: t = 60 ms (half of MENU_OUT, 120 ms).
- `menu-keyboard`: a keyboard open, Down twice: the third row lit.
- `menu-variants`: header, separator, a plain, a disabled and a danger row, chevrons on an enabled and a disabled row.
- `menu-pressed`: held on the third row, #FFFFFF23.
- `menu-anchored`: under its button, `Side::Bottom`, `PlaceAlign::Start`, gap 4.
- `menu-anchored-flipped`: the same near the window's bottom, flipped above.
- `tray-menu-window` (M10, `platform.tray-icon`): `Ui::menu_window`, the menu filling a window of its own from its corner, over a flat stand-in for the desktop, settled.

### Painter primitives

- Existing: backdrop, fill, ring, text, icon, xform. [R crates/lune-ui-core/src/widgets/menu.rs:103-111,124,128,130]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): click, Enter and Space pick once and close; Esc and an outside click close; during closing, clicks reach nothing; every row of Highlight above, in `tests/widget_menu.rs`.
- Pixel probes at scale 2: panel min width 380 px, row 56 px high, panel radius 18 px, separator 22 px high, row fill #FFFFFF16, pressed fill #FFFFFF23. Source values in Anatomy.
- Conformance: open runs 180 ms, cubic-out, from alpha 0 / offset 4 / scale 0.97 to rest; close runs 120 ms, ease-in, to alpha 0; Reduced fades opacity only; Off snaps.
- Semantic node: role `menu`, always named; item names and shortcut descriptions match the rows; the default action fires an enabled item and does nothing on a disabled one; `disabled_violations` is empty in a disabled scope.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_menu.rs`: behaviour `a_click_picks_the_item_once_and_a_separator_cannot_be_picked`, `enter_picks_the_hovered_item_else_the_focused_one`, `space_picks_as_enter_does`, `escape_and_an_outside_press_dismiss_and_the_press_reaches_nothing`, `a_closing_menu_hits_nothing`, the highlight rows `a_keyboard_open_lights_the_first_enabled_row_and_a_pointer_open_none`, `down_and_up_wrap_and_skip_disabled_rows_header_and_separators`, `a_press_takes_the_highlight_from_the_keys_and_shows_the_pressed_fill_while_held`; pixel probes at scale 2 `at_scale_2_the_panel_is_380_wide_with_radius_18_rows_56_high_and_separators_22`, `the_variants_draw_their_colours`, `a_row_draws_its_hint_and_its_chevron`; conformance `the_curves_conform_under_every_tier_and_opacity_is_monotone_through_the_ui`, `exit_retention_is_120_ms_full_120_reduced_and_none_off`, `reduced_fades_opacity_only`, `the_row_hover_conforms_under_every_tier_and_fades_through_the_ui`; node `the_semantic_tree_has_a_menu_of_menu_items`, `the_menu_node_is_always_named`; disabled `a_disabled_scope_takes_the_keys_picks_nothing_and_marks_every_node`, `a_disabled_row_takes_no_pointer_no_action_and_no_fill_but_blocks`.

### Build

- Milestone: M4 (retrofit), M7 (variants, keyboard, anchored menus).
- Depends on: core.ui, core.focus, infra.accessibility, motion.transitions, render.blur.
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_menu.rs::a_click_picks_the_item_once_and_a_separator_cannot_be_picked, crates/lune-ui-core/tests/widget_menu.rs::escape_and_an_outside_press_dismiss_and_the_press_reaches_nothing, crates/lune-ui-core/tests/widget_menu.rs::a_closing_menu_hits_nothing, crates/lune-ui-core/tests/widget_menu.rs::down_and_up_wrap_and_skip_disabled_rows_header_and_separators, crates/lune-ui-core/tests/widget_menu.rs::at_scale_2_the_panel_is_380_wide_with_radius_18_rows_56_high_and_separators_22, crates/lune-ui-core/tests/widget_menu.rs::the_curves_conform_under_every_tier_and_opacity_is_monotone_through_the_ui, crates/lune-ui-core/tests/widget_menu.rs::exit_retention_is_120_ms_full_120_reduced_and_none_off, crates/lune-ui-core/tests/widget_menu.rs::the_semantic_tree_has_a_menu_of_menu_items, crates/lune-ui-core/tests/widget_menu.rs::a_disabled_scope_takes_the_keys_picks_nothing_and_marks_every_node`; page `menu`; unverified here: the OS tray menu surface (`platform.tray-icon`).

## Open questions

- Answered: shortcut routing of a menu's hints: the hint is text drawn from the chord (`Chord::hint`); the app binds its own `Binding` (`infra.shortcut-routing`).
- Answered (D8, D48; M10): the tray menu is the toolkit's glass menu in its own window (`Ui::menu_window`), not a native NSMenu.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: exists widgets/menu.rs:64 (the macOS variant adds a two-line header and chevron hints); Wane priority P0. Native note: `backdrop-filter: blur(30px) saturate(1.8)` approximates lune-ui's backdrop pass (`render.blur`); luneui-rs draws the real blur. On macOS the tray menu may be the native NSMenu instead (D8 chose the glass surface, so the stats header fits). Full spec: Wane `docs/ui/components.md`, section `overlay.menu`; mock: Wane `docs/ui/mocks/components.html`.
