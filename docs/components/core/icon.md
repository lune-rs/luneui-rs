---
id: core.icon
title: Codicon glyph constants
area: core
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui-core/src/icon.rs
owner: orch/toolkit-v1
summary: Codicon glyph constants and a glyph helper; the app supplies the icon font
---

## What it is

Constants such as `ADD`, `GEAR`, `ELLIPSIS`, `CHROME_CLOSE`, `CHECK`, `CHROME_MINIMIZE` and `glyph`.

## Needs

- Icon glyphs as constants; the font itself comes from the app (R-core-6, R-text-1).
- Wane — Variants: none; the size comes from `--icon-size` (set by the parent component) (R-core-6)
- Wane — States: none; the colour is `currentColor`, so an icon takes the state of its parent (hover, disabled, tone) (R-core-6)
- Wane — Sizes: 16 px box (`--icon-size`); 14 in the callout, 13 in menu rows, 12 for the checkbox check, the combo chevron, the dropdown check and the table sort arrow. 27 Codicons are used (`js/icons.js`). (R-core-6)
- Wane — Behaviour: Decorative (hidden from assistive tech). An icon-only button carries the label, not the icon. (R-core-6)

## Existing code

`crates/lune-ui-core/src/icon.rs:7-19` (constants), `:22` (`glyph`); 24 lines.

## Spec

### Evidence

Pinned: Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/icon.rs:1-24` (Codicon constants and `glyph`); Wane `docs/ui/components.md:179-192`;
`THIRD_PARTY_NOTICES.md:12-13,24` (Codicons CC BY 4.0; no Codicons font file bundled; glyphs from the app's font).
Lunegit sizes: `src/Lunegit.Desktop/Themes/MenuResources.axaml:13-16` (14 px menu icon), `src/Lunegit.Desktop/Views/IconRailView.axaml:28` (20 px rail icon).
Conflicts: none. Wane 12, 13, 14 and 16 px sizes are the mock's; the repo takes the size as an argument.

### Anatomy

- `glyph(glyph, size, color)` returns a `Spec` of one code point [R crates/lune-ui-core/src/icon.rs:22].
- Codicon code points: 13 named constants, for example `ADD` [R crates/lune-ui-core/src/icon.rs:7] and `CHECK` [R crates/lune-ui-core/src/icon.rs:16].
- Sizes in use: 16 box (`--icon-size`), 14 callout, 13 menu rows, 12 checkbox, combo chevron, dropdown check and table sort arrow [W docs/ui/components.md:179-192]; 14 px menu icon [L src/Lunegit.Desktop/Themes/MenuResources.axaml:13-16]; 12 px dropdown check [R crates/lune-ui-core/src/widgets/dropdown.rs:48].
- Colour: `currentColor` in the mock, so the icon takes its parent's state [W docs/ui/components.md:179-192]; the repo passes the colour in `glyph`.

### States

| state | look | cite |
|---|---|---|
| any | the colour argument; the parent's state in the mock | [W docs/ui/components.md:179-192] |

### Transitions

n/a — an icon has no motion of its own; the owner's transition applies.

### Input and focus

n/a — decorative. An icon-only button carries the label, not the icon [W docs/ui/components.md:179-192].

### Retained state

n/a — stateless.

### Accessibility

- Decorative: hidden from assistive technology; the owning button's name carries the label [W docs/ui/components.md:179-192], [infra.accessibility].

### Strings

n/a — the code points are not strings for the reader; the names are the constants.

### Gallery scenes

- `controls` (existing): icons in the control set, scale 2 and 1. [R apps/gallery/src/scenes.rs:82-93]
- `menu` (existing): menu item icons at 14 px. [R apps/gallery/src/scenes.rs:96-101]
- `dropdown` (existing): the check glyph. [R apps/gallery/src/scenes.rs:117-122]

### Painter primitives

- Existing: icon (a glyph drawn through the text path). [R crates/lune-ui-core/src/icon.rs:22]
- New: none.

### Acceptance

- Existing tests: none in `crates/lune-ui-core/src/icon.rs`.
- Done standard still needs: each constant checked against the Codicons 0.0.45 table, and a pixel probe of the 16, 14, 13 and 12 px sizes.

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: core.painter, core.theme.

## Open questions

- Icon source for Wane's SVG icons: bundle a Codicon font or draw the SVG paths with the painter (Wane native note, [W docs/ui/components.md:179-192]); open.

- The Codicon font licence is not in this repo's plans yet; whether the toolkit bundles the font is open (R-ext-4).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: exists icon.rs:7 (Codicon glyph constants; the app supplies the icon font); Wane priority P0. Native note: The mocks inline Codicon SVG path data. lune-ui draws font glyphs, and RznMono has no Codicon glyphs, so luneui-rs needs an icon source: bundle the Codicon font (as Luneterm bundles a Nerd Font today) or draw the SVG paths with the painter (section 5). Full spec: Wane `docs/ui/components.md`, section `core.icon`; mock: Wane `docs/ui/mocks/components.html`.
