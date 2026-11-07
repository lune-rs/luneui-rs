---
id: core.theme
title: Theme tokens and Lunegit dark preset
area: core
status: done
consumers: luneterm
source: crates/lune-ui-core/src/theme.rs, crates/lune-ui-core/src/theme/file.rs
owner: —
summary: Theme token struct and the lunegit_dark preset with the WINDOW, PANEL and CARD glass layers
---

## What it is

`Theme` (tokens the widgets read) and the `lunegit_dark` module: `THEME`, `TINT`, `WINDOW`/`PANEL`/`CARD` layers, `CARD_CORNER_RADIUS`, `TAB_HEADER_HEIGHT`, `BODY_PADDING`.
Since M10 the three glass layers also travel inside the theme: `Theme.glass: ThemeGlass { window, panel, card }` (preset: the three consts, which stay), so a theme file's tint and opacities load with the rest (D207). `Theme::from_json` loads a file over a base theme and `Theme::with_acrylic(AcrylicLevel)` scales the glass material opacities; see `infra.theme-hot-reload`. `COLOR_KEYS` and `Theme::color_key` expose the key table. Every glass surface (menu, list, popover, tooltip, toast, dialog and palette card) draws `menu_glass` and `menu_popup_border`; there is no toast or dialog fill of its own, so `ToastBackground`, `ToastBorder` and `BackdropDark` are unknown file keys (D8).

## Needs

- Look is a `Theme` value; further looks are values, not forks (R-core-4).
- Boundary rule 3 allows the word `lunegit` only in the theme preset module (reserved slot in `scripts/boundaries.allow`; no live entry yet).

## Existing code

`crates/lune-ui-core/src/theme.rs:21` (`Theme`), `:135` (`ThemeGlass`), `:290` (`DARK`, the neutral alias), `:293` (`lunegit_dark`), `:298` (`THEME`); 802 lines, unit tests from `:421`; the JSON loader in `crates/lune-ui-core/src/theme/file.rs`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b. Read: `crates/lune-ui-core/src/theme.rs:11-117` (`Theme` struct, `lunegit_dark::THEME`, glass layers, constants);
Lunegit `src/Lunegit.Desktop/Themes/DarkTheme.axaml:226-233` (glass levels), `src/Lunegit.Desktop/App.axaml:151-155` (animation durations), `src/Lunegit.Desktop/SampleThemes/dark-modern.json:95-96` (the JSON theme format, for M10).
Conflicts: the repo menu padding (6) and Lunegit's (MenuItem 10,6) differ; overlay.menu names the winner (the moved code). The theme's values are the moved code's (pixel-gated).

### Anatomy

- `Theme` struct: colours (`fg_*`, `surface_*`, `border`, `accent`, dialog and menu tokens) and font sizes (`font_s` 11, `font_base` 13, `font_m` 14) [R crates/lune-ui-core/src/theme.rs:11-41].
- `DARK` = `lunegit_dark::THEME` [R crates/lune-ui-core/src/theme.rs:47].
- Glass layers `WINDOW`, `PANEL`, `CARD` [R crates/lune-ui-core/src/theme.rs:91-109]; `CARD_CORNER_RADIUS` 8 [R crates/lune-ui-core/src/theme.rs:115]; `TAB_HEADER_HEIGHT` 32 [R crates/lune-ui-core/src/theme.rs:116]; `BODY_PADDING` 10 [R crates/lune-ui-core/src/theme.rs:117].
- Motion tokens to add in M4 (SF5 names): durations `instant` 120, `fast` 180, `base` 250, `moderate` 350, `slow` 500 [L src/Lunegit.Desktop/App.axaml:151-155]; easings `linear`, `quad-out`, `quad-in`, `sine-in-out`, `back-out`, `cubic-out` (0.215, 0.61, 0.355, 1), `ease-in` (0.42, 0, 1, 1) [R crates/lune-ui-core/src/widgets/menu.rs:46-58]. Kinds are not theme values.
- JSON theme (M10, done): `Theme::from_json` and the 50-key table in `theme.rs` (`COLOR_KEYS`), see `infra.theme-hot-reload`. Widgets still take their `GlassLayer` from their options; the app reads it from `theme.glass`.

### States

| state | look | cite |
|---|---|---|
| dark (the only theme today) | `lunegit_dark::THEME` | [R crates/lune-ui-core/src/theme.rs:47,55] |
| inactive window | the same glass layers (no fallback token, D262) | [D262] |

### Transitions

n/a — the theme is data. A swap between two frames is a hard cut: in the frame `FrameConfig.theme` differs from the last frame's, every `animate_color` snaps to its target (D209); float transitions are untouched.

### Input and focus

n/a — no input.

### Retained state

- App-owned: the theme value, passed in (the toolkit never reads the environment, CLAUDE.md hard rule 2).
- Persistent: none.

### Accessibility

n/a — no UI node.

### Strings

n/a — no text.

### Gallery scenes

- `controls` (existing): all theme tokens in use, scale 2. [R apps/gallery/src/scenes.rs:82-93]

### Painter primitives

- Existing: none.
- Note: The painter takes the theme's colours..
- New: none.

### Acceptance

- The preset against Lunegit's `DarkTheme.axaml` (`crates/lune-ui-core/src/theme.rs`, module `tests`): `every_color_key_holds_the_dark_theme_axaml_value` (47 keys with a line cite each; `GlassTintColor`, `MenuGlassBackground` (the toolkit's own key for `menu_glass`, D8) and `PopupMenuOuterBorder` (the glass ring, white at 0x1F against Lunegit's 0x3A, D11) have none here, the last two are held by `the_glass_ring_and_tint_keys_hold_the_toolkits_values`; the file's brushes sit at `:7-159` and `:298-341`, the glass at `:226-235`), `the_glass_layers_hold_the_dark_theme_axaml_opacities`, `the_tokens_without_an_axaml_key_follow_their_decision_rows` (each field with no key of its own cites the row that sets it: D51, D14, D58, D207, D208), `the_status_and_surface_tokens_hold_their_values`, `the_font_sizes_hold_the_app_axaml_typography_scale` (`App.axaml:159-162`), `the_preset_glass_is_the_old_constants`.
- The motion tokens with a test per token: `anim_duration_instant_is_120_ms`, `fast` 180, `base` 250, `moderate` 350, `slow` 500 (`App.axaml:151-155`); the easings are `motion.curves`.
- The M10 JSON loader with a bounded reader (R-infra-6): `crates/lune-ui-core/tests/theme_file.rs` (`the_key_table_has_50_distinct_keys_each_of_which_resolves`, `the_preset_written_through_the_table_loads_back_equal`, `bad_colours_are_skipped_by_key_and_change_nothing`, `the_envelope_is_checked`, `nothing_panics_on_a_damaged_file`); a swap between two themes is a hard cut for colours (`a_swap_snaps_a_colour_that_is_mid_transition_and_one_whose_target_did_not_move`).
- Every widget takes its colours from the theme: `crates/lune-ui-core/tests/c4.rs` module `m10` (the frame under a theme of distinct colours; `UNREACHED` is empty).

### Build

- Milestone: M4 (motion tokens), M10 (JSON).
- Depends on: core.color, core.painter, motion.transitions.
- Done (D47, M11): tests `crates/lune-ui-core/src/theme.rs::every_color_key_holds_the_dark_theme_axaml_value, crates/lune-ui-core/src/theme.rs::the_glass_layers_hold_the_dark_theme_axaml_opacities, crates/lune-ui-core/src/theme.rs::the_tokens_without_an_axaml_key_follow_their_decision_rows, crates/lune-ui-core/src/theme.rs::anim_duration_fast_is_180_ms, crates/lune-ui-core/tests/theme_file.rs::the_preset_written_through_the_table_loads_back_equal, crates/lune-ui-core/tests/theme_file.rs::nothing_panics_on_a_damaged_file, crates/lune-ui-core/tests/theme_file.rs::a_swap_snaps_a_colour_that_is_mid_transition_and_one_whose_target_did_not_move`; page n/a (data; every scene reads it, `controls` among them); unverified here: none.

## Open questions

- Still open, a placement question: `TAB_HEADER_HEIGHT` and `BODY_PADDING` read as terminal-app values. Checked at M11: nothing in this repo reads either, so they affect no widget. They stay as the preset's constants; moving or dropping them is a public-API change no consumer has asked for (D237).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.
