---
id: core.icon
title: Codicon glyph constants
area: core
status: done
consumers: luneterm, wane
source: crates/lune-ui-core/src/icon.rs
owner: —
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

`crates/lune-ui-core/src/icon.rs:7-37` (29 constants), `:39` (`glyph`); the glyphs are the app's font, the toolkit holds code points only.

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

- The 29 constants against the Codicons 0.0.45 table, as literals with the source named (`crates/lune-ui-core/tests/icon.rs::every_constant_is_its_codicons_code_point`; `gear_is_settings_gear`, `the_table_covers_distinct_glyphs_in_the_private_use_block`, `a_glyph_spec_uses_the_mono_role_at_its_size`).
- Pixel probe: `crates/lune-ui-wgpu/tests/text.rs::an_icon_glyph_has_ink_that_grows_with_its_size` (the plus at 12, 13, 14 and 16 px: ink exists at each, the box never shrinks and 16 px is larger than 12 px; an empty string leaves the target empty).

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: core.painter, core.theme.
- Done (D47, M11): tests `crates/lune-ui-core/tests/icon.rs::every_constant_is_its_codicons_code_point, crates/lune-ui-core/tests/icon.rs::gear_is_settings_gear, crates/lune-ui-core/tests/icon.rs::a_glyph_spec_uses_the_mono_role_at_its_size, crates/lune-ui-wgpu/tests/text.rs::an_icon_glyph_has_ink_that_grows_with_its_size`; page n/a (constants; drawn by the legacy scenes `controls`, `menu`, `dropdown`); unverified here: none (the glyph shapes depend on the app's icon font).

## Open questions

- Answered (R-core-6, `THIRD_PARTY_NOTICES.md:13-14`): the icon source is the app's font; the toolkit holds Codicons code points only and bundles no Codicons font, so the SVG-path alternative is not built.
- Answered (`THIRD_PARTY_NOTICES.md:25`): the Codicons licence is CC BY 4.0, noted there; no font file is bundled, so R-ext-4 asks for no licence text.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: exists icon.rs:7 (Codicon glyph constants; the app supplies the icon font); Wane priority P0. Native note: The mocks inline Codicon SVG path data. lune-ui draws font glyphs, and RznMono has no Codicon glyphs, so luneui-rs needs an icon source: bundle the Codicon font (as Luneterm bundles a Nerd Font today) or draw the SVG paths with the painter (section 5). Full spec: Wane `docs/ui/components.md`, section `core.icon`; mock: Wane `docs/ui/mocks/components.html`.
