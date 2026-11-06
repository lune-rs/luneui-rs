---
id: infra.theme-hot-reload
title: Theme JSON loading and hot reload
area: infra
status: in-progress
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Theming/ThemeApplicator.cs
owner: —
summary: Theme files as JSON mapped onto the Theme value, partial overrides over a baseline, acrylic level, swapped at a frame boundary by the app
---

## What it is

Loading a theme from JSON and swapping it at run time. A theme is a `Theme` value plus glass layer
constants (R-core-4); a file overrides some of them, the rest keep the preset's values; the app
passes the new value on the next frame, so there is no apply step, no event and no file watcher in
the toolkit. Lunegit's `ThemeApplicator` writes colours into Avalonia's live resource dictionary
(which restyles every `DynamicResource`), restores a captured baseline first so a theme is a partial
overlay, scales the acrylic opacities by a user level, and raises a `ThemeApplied` event for
consumers that bake colours into rendered output.

## Needs

- Themes from JSON re-applied at run time (R-infra-3, R-core-4).
- The bounded JSON reader (R-infra-6): `infra.json`.
- Deterministic frames: the toolkit never reads files or the clock (R-nfr-1).

## Existing code

`Theme::from_json`, `ThemeFile`, `Skip`, `ThemeError` in `crates/lune-ui-core/src/theme/file.rs`; the key table `COLOR_KEYS` / `color_slot`, `AcrylicLevel` and `Theme::with_acrylic` in `crates/lune-ui-core/src/theme.rs` (M10 step 2). Tests `crates/lune-ui-core/tests/theme_file.rs`, `apps/gallery/tests/lunegit_files.rs`. Lunegit: `src/Lunegit.Desktop/Theming/ThemeApplicator.cs` (444 lines),
`src/Lunegit.Infrastructure/Theming/JsonThemeRegistry.cs` (147), `src/Lunegit.Core/Theme/ThemeFile.cs` (21), `src/Lunegit.Desktop/SampleThemes/*.json`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/Theming/ThemeApplicator.cs:1-444, src/Lunegit.Infrastructure/Theming/JsonThemeRegistry.cs:1-147, src/Lunegit.Core/Theme/ThemeFile.cs:1-21,
src/Lunegit.Desktop/SampleThemes/README.txt:1-23, SampleThemes/dark-modern.json:1-40, SampleThemes/blue.json:60-100, src/Lunegit.Core/Settings/Enums.cs:75-92, src/Lunegit.Desktop/Themes/DarkTheme.axaml:7-42,152-158,226-235,300-318;
this repo crates/lune-ui-core/src/theme.rs:11-118; Wane docs/ui/components.md:197.
Conflicts: (1) the `AcrylicLevel` doc comments say material opacity 0.3, 0.6, 0.8 and 1.0 [L src/Lunegit.Core/Settings/Enums.cs:81-91], the code scales the baseline: half for less opaque, halfway to 1 for more opaque, 1.0 for opaque [L src/Lunegit.Desktop/Theming/ThemeApplicator.cs:402-426]. The code wins (it is the behaviour).
(2) the inactive-window fallback: the moved preset has `#333333` [R crates/lune-ui-core/src/theme.rs:111-113], Lunegit lightens the tint by 20 percent [L src/Lunegit.Desktop/Theming/ThemeApplicator.cs:284-301], Wane wants near-black `#0b0b0b` [W docs/ui/components.md:197]; see `platform.window`.

### Anatomy

- What a theme file holds: `name`, `author`, `version` (a monotonic integer, 0 when absent, used by the app to refresh bundled themes), `colors` (key to hex string) and `acrylic` (key to number) [L src/Lunegit.Core/Theme/ThemeFile.cs:10-21]. The glass tint colour is a colour key, not an acrylic one [L src/Lunegit.Core/Theme/ThemeFile.cs:5-7].
- Colour syntax: `#AARRGGBB` or `#RRGGBB` in the sample files (440 and 2 occurrences) [L src/Lunegit.Desktop/SampleThemes/dark-modern.json:6-17], [infra.json]. The `Theme` stores straight-alpha RGBA, so `#CCFFFFFF` is `[0xFF, 0xFF, 0xFF, 0xCC]` [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:7], [R crates/lune-ui-core/src/theme.rs:56].
- Mapping onto `Theme` fields: one table, `COLOR_KEYS` (50 keys) and the `match` behind it in `crates/lune-ui-core/src/theme.rs` (it carries Lunegit's key names, so that file holds them). Each key's value in the dark preset equals the `Theme` field's: checked against `DarkTheme.axaml` for all 50 (name and value). One key can set two fields (`SurfaceSeparator`: `surface_separator` and `menu_separator`) and `GlassTintColor` sets the tint of all three glass layers.
  The fields with no Lunegit key follow their source when the file sets it (D208): `accent_hover` (each channel of the accent plus 0x10, saturating), `scroll_thumb`, `scroll_thumb_hover` and `text_selection` (the accent at alpha 0x8C, 0xE6, 0x66), `diff_added_mark` and `diff_removed_mark` (the line fill at alpha 0x40), `menu_text_primary`, `menu_text_tertiary` and `menu_text_quaternary` (the matching `fg_*`). These reproduce the preset exactly. The rest (`on_accent`, `on_accent_text`, `menu_glass`, `caption_close_*`, the data tones, the font sizes, `item_radius`) keep the base value; no toolkit-only keys.
  Lunegit's other colour keys (about fifty per file: file status badges, Markdown tables, tree guides and the rest) have no `Theme` field; the loader reports each as `Skip::UnknownColor` and the app may ignore them.
- Glass layers: `GlassTintDarkOpacity` and `GlassMaterialDarkOpacity` are the window layer (1.0 and 0.7), `GlassTintLightOpacity` and `GlassMaterialLightOpacity` the panel layer (0.8 and 0.7), `GlassMaterialDarkHeaderOpacity` the card layer's material (0.8); `GlassTintColor` is the tint [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:226-235], [R crates/lune-ui-core/src/theme.rs:88-109], [L src/Lunegit.Desktop/SampleThemes/blue.json:65,92-100].
- Partial overlay: a theme is applied by restoring the baseline for every themeable key, then writing the file's keys; a key the file omits keeps the baseline value [L src/Lunegit.Desktop/Theming/ThemeApplicator.cs:253-261], [L src/Lunegit.Desktop/SampleThemes/README.txt:21-22]. In the toolkit the baseline is the preset the app starts from and the overlay is a function from (preset, parsed file) to a new `Theme`; nothing is mutated.
- Acrylic keys: `GlassTintDarkOpacity` and `GlassMaterialDarkOpacity` set the window layer, `GlassTintLightOpacity` and `GlassMaterialLightOpacity` the panel layer, `GlassMaterialDarkHeaderOpacity` the card layer's material; a value outside [0, 1] is `Skip::BadAcrylic`, any other key `Skip::UnknownAcrylic`.
- Acrylic level (`Theme::with_acrylic`, applied last over the file): Default changes nothing; Opaque sets the material opacities to 1.0; Less opaque halves each (0.7 gives 0.35, 0.8 gives 0.4); More opaque moves each halfway to 1 (0.7 gives 0.85, 0.8 gives 0.9); the result is clamped to [0, 1]; tint and solid keys never scale [L src/Lunegit.Desktop/Theming/ThemeApplicator.cs:386-426]. It is a restart-required setting in Lunegit [L src/Lunegit.Core/Settings/Enums.cs:76-78]; the toolkit has no such limit.
- Font scale: the six font sizes times 0.875, 1.0 or 1.125, applied once at startup (restart required) [L src/Lunegit.Desktop/Theming/ThemeApplicator.cs:211-237]. The toolkit takes the already scaled sizes in `Theme`; the multiplier is the app's.
- Inactive-window fallback: a solid colour derived from the tint, replacing the acrylic layers while the window is inactive [L src/Lunegit.Desktop/Theming/ThemeApplicator.cs:284-301], [R crates/lune-ui-core/src/theme.rs:111-113]; see `platform.window`.
- Bad input, as Lunegit forgives it: a file that does not parse or lacks `name`, `colors` or `acrylic` is skipped with a warning [L src/Lunegit.Infrastructure/Theming/JsonThemeRegistry.cs:77-86,105-109]; a duplicate theme name (case-insensitive) is skipped [L src/Lunegit.Infrastructure/Theming/JsonThemeRegistry.cs:88-96]; an unparsable colour skips that key; an unknown acrylic key is skipped [L src/Lunegit.Desktop/Theming/ThemeApplicator.cs:358-369,377-382]. The toolkit loader returns the new `Theme`, or an error for a bad envelope, plus a list of skipped keys (unknown key, bad colour, unknown acrylic key) for the app to show; it never panics ([infra.json]).
- Hot reload: Lunegit has no file watcher; reload is a manual action and the registry raises `ThemesChanged` [L src/Lunegit.Infrastructure/Theming/JsonThemeRegistry.cs:14,30,118]. After an apply it raises `ThemeApplied` so artifacts with colours baked in (rendered Markdown diagrams) re-render [L src/Lunegit.Desktop/Theming/ThemeApplicator.cs:20-34,271-282]. The toolkit has neither: the app reads the file (its own watcher or button), calls the loader and passes the new `Theme` on the next frame; anything the app keeps rendered with baked colours is the app's to refresh.
- Frame boundary: a theme changes between frames, never inside one [derived: a frame is a pure function of its inputs, R-nfr-1].

### States

n/a — a theme has no look of its own and no interaction; the loader is a function from bytes to a `Theme`.

### Transitions

A swap is a hard cut, as Lunegit's is [L src/Lunegit.Desktop/Theming/ThemeApplicator.cs:11-13,247-264] (D209): in the frame `FrameConfig.theme` differs from the previous frame's, `Ui::animate_color` snaps every colour to its target, whether the target moved or not; nothing crossfades. Float transitions are untouched.

### Input and focus

n/a — no pointer, keys, focus or layer; the user action that triggers a reload (a settings button) is the app's.

### Retained state

- Transient: none. Persistent: none in the toolkit. App-owned: the file contents, the registry of available themes, the chosen name, the acrylic level and the font scale.
- Exit retention: no.

### Accessibility

n/a — no node. Contrast between the loaded colours is not checked by the loader (Open questions).

### Strings

None: theme names and warnings are the app's text.

### Gallery scenes

- `controls` (existing): the scene the theme scenes below re-draw.
- `theme-dark-modern`: the `controls` scene under `dark-modern.json` (Lunegit's bundled theme), settled, after M10.
- `theme-blue`: the `controls` scene under `blue.json` (Lunegit's bundled theme), settled, after M10.
- `theme-green`: the `controls` scene under `green.json` (Lunegit's bundled theme), settled, after M10.
- `theme-orange`: the `controls` scene under `orange.json` (Lunegit's bundled theme), settled, after M10.
- `theme-purple`: the `controls` scene under `purple.json` (Lunegit's bundled theme), settled, after M10.
- `theme-partial`: an inline file that overrides only `AccentPrimaryColor`, settled.
- `theme-acrylic-less`, `theme-acrylic-more`, `theme-acrylic-opaque`: the preset under each level: a panel taken from `theme.glass.panel` over bright stripes and a glass menu over the panel, settled.
- `theme-swap`: dark modern, a toggle clicked at 40 ms, a cue at 60 ms that swaps to blue, captured at 80 ms: every colour is already blue (a hard cut) and only the toggle's thumb, which no theme drives, is still moving.
- `theme-swap-b`: the same script captured at 600 ms, settled; a gallery test (`pages::theme::tests`) proves it equals the blue theme's frame after the same click.

### Painter primitives

- Existing: none (every drawing function reads the `Theme`; the loader draws nothing).
- New: none.

### Acceptance

- Known answers (instrument: the real Lunegit files): loading `dark-modern.json` maps `ForegroundPrimary` `"#FFCCCCCC"` to `fg_primary` `[0xCC, 0xCC, 0xCC, 0xFF]` [L src/Lunegit.Desktop/SampleThemes/dark-modern.json:6]; `blue.json` maps `AccentPrimaryColor` `"#FF71B0F7"` to `accent` `[0x71, 0xB0, 0xF7, 0xFF]` [L src/Lunegit.Desktop/SampleThemes/blue.json:64]; `#CCFFFFFF` is `[0xFF, 0xFF, 0xFF, 0xCC]` (ARGB to RGBA).
- Partial overlay: a file with only the `accent` key leaves every other field equal to the preset; an empty `colors` object returns the preset unchanged.
- Preset round trip: the dark preset written through all 50 keys and the five opacity keys, loaded over a base with every table and derived field changed, equals the preset (`the_preset_written_through_the_table_loads_back_equal`).
- Skips: `"#ZZZZZZ"` for a colour skips that key and reports it; an unknown acrylic key is reported; a file without `name` returns an error; none panics.
- Acrylic level: 0.7 gives 0.35 (less), 0.85 (more), 1.0 (opaque); 0.8 gives 0.4 and 0.9; results stay in [0, 1]; Default returns the input [L src/Lunegit.Desktop/Theming/ThemeApplicator.cs:419-425].
- Hot swap: a toggle drawn under theme A, settled, then under theme B in the next frame, gives the Scene of a solo frame under B and no `Repaint::Now` because of the swap; a colour mid-transition at the swap snaps (`tests/theme_file.rs`).
- Pixel probes and conformance: through the `theme-*` scenes; the sample files' values differ from the preset, so their dumps are new baselines, not zero delta; the mapped-pair round trip above is the zero-delta check; no transitions.
- Semantic-node test: n/a.

### Build

- Milestone: M10
- Depends on: infra.json, core.theme, core.color

## Open questions

None open. Answered in M10 step 2 (D207 to D209):

- A theme change does not crossfade: it is a hard cut (D209).
- The `Theme` fields with no Lunegit key derive from the mapped ones (D208); no toolkit keys.
- Contrast and legibility checks on loaded colours: none (Lunegit has none).
- The sample themes' colours differ from the dark preset by design; their dumps are new baselines (step 9's `theme-*` scenes, on the gallery's `theme` page, which also lets the window pick a file and an acrylic level).

## Notes for the planner

Replaces the stub's question "which theme keys map to `Theme` tokens today": the 15 pairs above, equal in value, plus the glass opacities. The rest of Lunegit's keys follow their components. Security review at M10 covers `infra.json` (D12).
