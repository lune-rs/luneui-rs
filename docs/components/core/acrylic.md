---
id: core.acrylic
title: Acrylic colour math and noise opacity
area: core
status: in-progress
consumers: luneterm
source: crates/lune-ui-core/src/acrylic.rs
owner: orch/toolkit-v1
summary: Avalonia acrylic colour math with tests, and the noise opacity
---

## What it is

`AcrylicColors` and `acrylic_colors` (Avalonia's acrylic formulas), `premultiplied`, and `NOISE_OPACITY`.

## Needs

- Avalonia acrylic colour math, tested (R-core-5).
- The Avalonia formulas are bundled third-party logic and need a licence row (R-ext-4).

## Existing code

`crates/lune-ui-core/src/acrylic.rs:16` (`NOISE_OPACITY`), `:20` (`AcrylicColors`), `:30` (`acrylic_colors`), `:52` (`premultiplied`); tests at `:99`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b. Read: `crates/lune-ui-core/src/acrylic.rs:1-16,20-52,106-139` (header, constant, struct, functions, tests);
`crates/lune-ui-core/src/theme.rs:90-113` (layers, inactive fill); `licenses/avalonia-LICENSE`, `THIRD_PARTY_NOTICES.md:8` (licence row);
Lunegit `src/Lunegit.Desktop/Themes/DarkTheme.axaml:226-233` (glass levels), `src/Lunegit.Desktop/Views/RepositoryContentView.axaml:52-56` (content acrylic).
Conflicts: none. The repo CARD layer (material 0.8) and Lunegit's content acrylic (material 0.7) differ; see layout.page.

### Anatomy

| layer | tint | tint opacity | material opacity | cite |
|---|---|---|---|---|
| window | black | 1.0 | 0.7 | [R crates/lune-ui-core/src/theme.rs:91-95], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:227-228] |
| panel | black | 0.8 | 0.7 | [R crates/lune-ui-core/src/theme.rs:98-102], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:230-231] |
| card | black | 1.0 | 0.8 | [R crates/lune-ui-core/src/theme.rs:105-109], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:229] |
| solid (Lunegit only) | black | 1.0 | 0.35 | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:232-233] |

- Noise opacity 0.0225 [R crates/lune-ui-core/src/acrylic.rs:16].
- Inactive window: solid `INACTIVE_FALLBACK` #333333FF instead of the layers [R crates/lune-ui-core/src/theme.rs:111-113].
- White tint suppressed to 20% (test `white_tint_is_suppressed_to_twenty_percent`, acrylic.rs:130).

### States

| state | look | cite |
|---|---|---|
| active window | the layer stack above | [R crates/lune-ui-core/src/theme.rs:90-109] |
| inactive window | solid fallback | [R crates/lune-ui-core/src/theme.rs:113] |

### Transitions

n/a — a colour formula and constants; no motion. The Lunegit glass has none.

### Input and focus

n/a — no input.

### Retained state

n/a — stateless functions and constants (`acrylic_colors`, `premultiplied`, acrylic.rs:30,52).

### Accessibility

n/a — no UI node.

### Strings

n/a — no text.

### Gallery scenes

- `acrylic` (existing): the acrylic layers, settled. [R apps/gallery/src/scenes.rs:180-181]
- `glass-off` (existing): the glass switched off, settled. [R apps/gallery/src/scenes.rs:152-157]
- `blur-contrast` (existing): the blur against contrast content, settled. [R apps/gallery/src/scenes.rs:166-171]
- `unfocused` (existing): the inactive window fallback, settled. [R apps/gallery/src/scenes.rs:159-164]
- `chrome` (existing): the card with its tab outline, settled. [R apps/gallery/src/scenes.rs:173-178]

### Painter primitives

- Existing: acrylic. [R crates/lune-ui-core/src/paint.rs:243]
- New: none.

### Acceptance

- Existing tests (by path, `crates/lune-ui-core/src/acrylic.rs`): `window_layer_matches_avalonia` (:106), `panel_layer_matches_avalonia` (:113), `card_layer_matches_avalonia` (:120 region), `white_tint_is_suppressed_to_twenty_percent` (:130 region).
- Done standard still needs: a pixel probe of the `acrylic` scene at scale 2 against the pixel gate (zero delta), and a probe of the inactive fallback in `unfocused`.

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: core.theme, render.frame, render.gpu.

## Open questions

- Licence text for the Avalonia formulas: Luneterm has `licenses/avalonia-LICENSE`; this repo has none yet.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.
