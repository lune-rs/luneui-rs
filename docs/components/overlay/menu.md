---
id: overlay.menu
title: Glass context menu
area: overlay
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/menu.rs
owner: toolkit-v1 M7
summary: Glass context menu with open and close motion
---

## What it is

`menu` with `MenuEntry`, built from Luneium's ContextMenu.css and Popup.css; `menu_open_anim` and `menu_close_anim` drive its motion (`MENU_IN`, `MENU_OUT`) [R crates/lune-ui-core/src/widgets/menu.rs:1,9-22,46-58,64].

## Needs

- Glass context menu (R-overlay-1).
- Open and close motion with the app passing time in (R-motion-1).
- Wane — Variants: plain; with a two-line header (`__header`: name with a status dot, status line); danger item (`c-menu__item--danger`, `--err` text; Quit Wane); chevron hints (submenu); shortcut hints (11 px quaternary) (R-overlay-1)
- Wane — States: item rest; hover (`--popup-item-hover`); pressed (`--popup-item-pressed`); disabled (`--menu-disabled`); menu opening and closing (motion below). (R-overlay-1)
- Wane — Sizes: Min width 190, padding 5, radius 9 (`--radius-menu`), 1 px `--popup-border-soft` (white at .08; user, 2026-11-04), fill `--menu-glass` (rgb 44 44 44 / .616); items 28 high, radius 4, padding 0 8, label 12 (`--font-menu`), icon slot 14 (13 px glyph), hint 11; separator slot 11 high with a 1 px line. (R-overlay-1)
- Wane — Behaviour: Opens at the pointer; click or Enter picks an item; Esc or an outside click closes. Motion: open .18 s ease-out from 4 px lower at 97%, close .12 s ease-in to 2 px lower at 98%, origin top-left (menu.rs:21-22). Arrow keys and shortcut routing are requirements, not mocked. (R-overlay-1)

## Existing code

`crates/lune-ui-core/src/widgets/menu.rs:64` (`menu`), `:9` (`MenuEntry`), `:46` (`menu_open_anim`), `:52` (`menu_close_anim`); 192 lines; tests at `:163`.

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
- Pressed and disabled fills: Lunegit has them [L src/Lunegit.Desktop/App.axaml:802-810]; the moved code has neither (`MenuEntry` has no disabled flag, menu.rs:9-17). Open question.

### Anatomy

- Panel: min width 190; width = max(190, 6 + 8 + 14 + 8 + label + 16 + hint + 8 + 6) when a shortcut exists [R crates/lune-ui-core/src/widgets/menu.rs:78,89]; height = 2 × 6 + 28 per item + 11 per separator [R crates/lune-ui-core/src/widgets/menu.rs:71-73,79-80,90-92].
- Placement: x clamped to `[8, window w − w − 8]`; y is the pointer y, or flipped above (`max(y − h, 8)`) when `y + h > window h − 8` [R crates/lune-ui-core/src/widgets/menu.rs:95-100].
- Glass: backdrop blur at radius 9 [R crates/lune-ui-core/src/widgets/menu.rs:109]; fill `menu_glass` #2C2C2C9D [R crates/lune-ui-core/src/widgets/menu.rs:110, crates/lune-ui-core/src/theme.rs:79]; 1 px ring `menu_popup_border` #FFFFFF3A [R crates/lune-ui-core/src/widgets/menu.rs:111, crates/lune-ui-core/src/theme.rs:75]; radius 9 [R crates/lune-ui-core/src/widgets/menu.rs:74].
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
| item pressed | not in the moved code; Lunegit has a pressed fill | open question [L src/Lunegit.Desktop/App.axaml:808-810] |
| item disabled | not in the moved code (no flag on `MenuEntry`) | open question [R crates/lune-ui-core/src/widgets/menu.rs:9-17] |
| focus-visible | not in the moved code | open question |
| menu opening | alpha, offset and scale from `menu_open_anim` | [R crates/lune-ui-core/src/widgets/menu.rs:46-49,102-108] |
| menu closing | alpha, offset and scale from `menu_close_anim`; not interactive | [R crates/lune-ui-core/src/widgets/menu.rs:52-58,62-63,140] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| open | opacity 0→1, y offset 4→0, scale 0.97→1, origin top-left | pop | fast | cubic-out | fade (opacity only, Fast) | [R crates/lune-ui-core/src/widgets/menu.rs:21,46-49], [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:167-175] |
| close | opacity 1→0, y offset 0→2, scale 1→0.98 | dismiss | instant | ease-in | fade | [R crates/lune-ui-core/src/widgets/menu.rs:22,52-57] |
| item rest ↔ hover | row fill | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102]; none in the moved code or Lunegit (no MenuItem selector in Animations.axaml), so open question |

Open and close use the moved curves (the Lunegit ease-out family is the same kind of curve,
[L .claude/docs/animation-rules.md:82-86]). The item-hover row is a snap until the open question is answered.
Never animated: the rows and their hover fill, and the panel's size (the menu sizes once per open).

### Input and focus

- Pointer: a click on an item picks it [R crates/lune-ui-core/src/widgets/menu.rs:140-141]; hover over an item fills it [R crates/lune-ui-core/src/widgets/menu.rs:123]; separators are not hit targets. The menu opens at the pointer [R crates/lune-ui-core/src/widgets/menu.rs:67]. An outside click closes [W docs/ui/components.md:543-556]; the gallery covers the window with a dismiss hit [R apps/gallery/src/scenes.rs:371].
- Keyboard: Enter picks; Esc closes [W docs/ui/components.md:543-556]. Enter picks the hovered item [D76]. Arrow keys and shortcut routing: requirements, not mocked (open question).
- Tab stop: no (open question).
- Layer: Popup; the menu does not set its layer (menu.rs:64-161), and the gallery draws it on `LayerId::Popup` [R apps/gallery/src/scenes.rs:369]. Layers under popups go to an offscreen scene so the glass can blur them [R crates/lune-ui-core/src/scene.rs:26-27].
- Focus scope: push on open; pop restores focus to the previous widget or anchor [core.focus]. Esc and outside click act on the top overlay only [core.focus].
- Pointer capture while pressed: none in the moved code.

### Retained state

- Transient: item hover; the open and close progress, passed in by the app as elapsed/duration [R crates/lune-ui-core/src/widgets/menu.rs:45-58] [motion].
- Persistent: none (no submenus, no selection).
- App-owned: entries (`MenuEntry`), anchor point, `interactive`, the animation value [R crates/lune-ui-core/src/widgets/menu.rs:9-17,64-69].
- Exit retention: yes: Full 120 / Reduced 120 / Off 0 ms [core.ui] (Full is MENU_OUT, menu.rs:22).

### Accessibility

- Role `menu`; each item `menu-item` with name = item text and description = shortcut text [R crates/lune-ui-core/src/widgets/menu.rs:82-87] [infra.accessibility].
- Item states: `focusable`, `focused`; `disabled` once the flag exists (open question) [infra.accessibility].
- Actions: default (Enter, click), dismiss (Esc) [infra.accessibility].
- Separators and the glyph are not nodes. Live region: none.

### Strings

None: item text and shortcut come from the app (`MenuEntry`, [R crates/lune-ui-core/src/widgets/menu.rs:9-17]).

### Gallery scenes

- `menu` (existing): six entries (three items, a separator, two more), item 1 hovered, settled. [R apps/gallery/src/scenes.rs:95-101,366-392]
- `menu-opening` (existing): t = 90 ms (half of MENU_IN, 180 ms). [R apps/gallery/src/scenes.rs:102-108]
- `menu-closing`: t = 60 ms (half of MENU_OUT, 120 ms) (new).

### Painter primitives

- Existing: backdrop, fill, ring, text, icon, xform. [R crates/lune-ui-core/src/widgets/menu.rs:103-111,124,128,130]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): click and Enter pick once and close; Esc and an outside click close; during closing, clicks reach nothing. Existing tests: `crates/lune-ui-core/src/widgets/menu.rs` `tests` module (`cubic_bezier_endpoints`, `open_curve_ends_at_rest`, `close_curve_ends_at_none`, menu.rs:163-191) cover the curves only.
- Pixel probes at scale 2: panel min width 380 px, row 56 px high, panel radius 18 px, separator 22 px high, row fill #FFFFFF16. Source values in Anatomy.
- Conformance: open runs 180 ms, cubic-out, from alpha 0 / offset 4 / scale 0.97 to rest; close runs 120 ms, ease-in, to alpha 0; Reduced fades opacity only; Off snaps.
- Semantic node: role `menu`; item names and shortcut descriptions match the entries; the default action fires the item.

### Build

- Milestone: M4 (retrofit), M7 (variants, keyboard, anchored menus).
- Depends on: core.ui, core.focus, infra.accessibility, motion.transitions, render.blur.

## Open questions

- Submenus (chevron hint), the two-line header and the danger item are Wane variants, not in the moved code; open for M7.
- Arrow-key navigation (Enter on the hovered item is built, D76) and shortcut routing: requirements, not mocked (W docs/ui/components.md:543-556); open.
- Pressed and disabled looks (Wane `--popup-item-pressed`, `--menu-disabled`; Lunegit App.axaml:802-810): not in the moved code; open for a look decision (hard rule 8).
- On macOS the tray menu may be the native NSMenu instead (W docs/ui/components.md:543-556; D8 chose the glass surface); open for the tray work.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: exists widgets/menu.rs:64 (the macOS variant adds a two-line header and chevron hints); Wane priority P0. Native note: `backdrop-filter: blur(30px) saturate(1.8)` approximates lune-ui's backdrop pass (`render.blur`); luneui-rs draws the real blur. On macOS the tray menu may be the native NSMenu instead (D8 chose the glass surface, so the stats header fits). Full spec: Wane `docs/ui/components.md`, section `overlay.menu`; mock: Wane `docs/ui/mocks/components.html`.
