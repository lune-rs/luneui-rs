---
id: core.acrylic
title: Acrylic colour math and noise opacity
area: core
status: done
consumers: luneterm
source: crates/lune-ui-core/src/acrylic.rs
owner: —
summary: Avalonia acrylic colour math with tests, and the noise opacity
---

## What it is

`AcrylicColors` and `acrylic_colors` (Avalonia's acrylic formulas), `premultiplied`, and `NOISE_OPACITY`.

## Needs

- Avalonia acrylic colour math, tested (R-core-5).
- The Avalonia formulas are bundled third-party logic and need a licence row (R-ext-4).

## Existing code

`crates/lune-ui-core/src/acrylic.rs:16` (`NOISE_OPACITY`), `:20` (`AcrylicColors`), `:30` (`acrylic_colors`), `:52` (`premultiplied`); tests at `:100`.

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
- Inactive window: `window_fallback` (#333333FF, the theme's `window_fallback`) instead of the layers [R crates/lune-ui-core/src/theme.rs:111-113]. The background still draws two acrylic items, the window and the card, both in `window_fallback` with noise off, so the card's corners antialias against the panel; it draws no glass layer's colour and no noise.
- White tint suppressed to 20% (test `white_tint_is_suppressed_to_twenty_percent`, acrylic.rs:130).

### States

| state | look | cite |
|---|---|---|
| active window | the layer stack above | [R crates/lune-ui-core/src/theme.rs:90-109] |
| inactive window | two acrylic items (window and card) in `window_fallback`, noise off | [R crates/lune-ui-core/src/theme.rs:113] |

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
- The inactive fallback: `apps/gallery/src/scenes.rs`, module `tests`: `the_unfocused_scene_draws_the_window_fallback_and_no_glass_layer` (two acrylic items, both `premul(window_fallback)`, `params[3]` 0 for no noise; the focused background beside it is the glass layers with noise).
- Pixel probes (`crates/lune-ui-wgpu/tests/acrylic_fade.rs::fade_blends_between_the_base_and_the_card`: opacity 1 replaces the base with the premultiplied card colour, within one level; `primitives.rs::acrylic_radii_cuts_only_the_corner_it_names`, scale 2) and the pixel gate on the `acrylic`, `glass-off`, `unfocused` and `chrome` scenes at zero delta (the orchestrator's gate; M11's pre-audit run held 413 of 413).

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: core.theme, render.frame, render.gpu.
- Done (D47, M11): tests `crates/lune-ui-core/src/acrylic.rs::window_layer_matches_avalonia, crates/lune-ui-core/src/acrylic.rs::panel_layer_matches_avalonia, crates/lune-ui-core/src/acrylic.rs::card_layer_matches_avalonia, crates/lune-ui-core/src/acrylic.rs::white_tint_is_suppressed_to_twenty_percent, apps/gallery/src/scenes.rs::the_unfocused_scene_draws_the_window_fallback_and_no_glass_layer, crates/lune-ui-wgpu/tests/acrylic_fade.rs::fade_blends_between_the_base_and_the_card`; page n/a (a colour formula; drawn by the legacy scenes `acrylic`, `glass-off`, `unfocused`, `chrome`); unverified here: none (what the OS composites under the layers is `platform.material`).

## Open questions

- Answered: the licence text for the Avalonia formulas is `licenses/avalonia-LICENSE`, with its row in `THIRD_PARTY_NOTICES.md:8`.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.
