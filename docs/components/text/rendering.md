---
id: text.rendering
title: Text rendering (font smoothing)
area: text
status: done
consumers: any
source: crates/lune-ui-core/src/text/rendering.rs, crates/lune-ui-wgpu/src/raster.rs, crates/lune-ui-wgpu/src/text_cache.rs
owner: —
summary: How glyphs are rasterized, a frame value (smoothing, hinting, stem darkening) on by default; one public rasterizer shared with an app's own glyph renderer
---

## What it is

Font smoothing as a text-rendering setting the app passes each frame: `FrameParams.text: TextRendering` (`smoothing`, `hinting`, `stem_darkening: StemDarkening { factors, max_px, max_ppem }`), handed to app draws as `CustomFrame.text`. The default, `TextRendering::SMOOTH`, draws macOS-like text: no hinting and stem darkening (Pathfinder's numbers, which aim at macOS 10.13), no blend gamma. `TextRendering::OFF` is the look before 0.4.0 (hinted, no darkening). The same on every OS: the glyphs are rasterized on the CPU and blended by one shader, so no per-OS default exists.

The core holds the pure values (`TextRendering`, `StemDarkening`, `GlyphParams`, the per-glyph effective values). `GlyphRasterizer::image(fonts, key, params)` in `lune-ui-wgpu` (re-exported as `lune_ui::text::GlyphRasterizer`) turns a cosmic-text `CacheKey` and `GlyphParams` into a coverage image: `GlyphParams::OFF` returns cosmic-text's image unchanged; otherwise an outline glyph is filled again from its outline, grown by the dilation a side and re-centred; a colour glyph is returned as cosmic-text drew it. The toolkit's own renderer (the vendored `glyph` module, D294) and an app's own glyph renderer share it, so both draw the same glyphs.

The hinting flag is the toolkit's rasterization hinting. It is distinct from the text shaper's metrics hinting (cosmic-text `Buffer::set_hinting`, layout rounding, off by default and never set by the toolkit).

## Needs

- Glyphs rasterized with a setting the app chooses per frame and per window, readable by an app's own glyph renderer (R-text-6).
- Keyed caches: no cached glyph image or text raster is drawn under a setting other than the one it was made with (R-text-6).

## Existing code

`crates/lune-ui-core/src/text/rendering.rs` (`TextRendering`, `StemDarkening`, `GlyphParams`); `crates/lune-ui-wgpu/src/raster.rs` (`GlyphRasterizer`); `crates/lune-ui-wgpu/src/glyph/` (the atlas keyed by `GlyphKey { CacheKey, GlyphParams }`); `crates/lune-ui-wgpu/src/text_cache.rs` (`TextCache::rasterize`, the rotated-text path); `crates/lune-ui-wgpu/src/gpu.rs` (`Gpu::glyph_atlas_size`, a diagnostic). Decisions D294 to D297.

## Spec

### Evidence

Pinned: this repo at the font-smoothing task. Read: swash 0.2.10 `scale/mod.rs:846-882` (the outline path) and `scale/outline.rs:99-105` (`embolden`); cosmic-text 0.19.0 `SwashCache`; Pathfinder `content/src/effects.rs` (the stem-darkening constants; licence note in `THIRD_PARTY_NOTICES.md`).
Conflicts: none.

### Anatomy

- `TextRendering { smoothing, hinting, stem_darkening }`; `SMOOTH` (default) and `OFF`; `effective()` makes a bad value fall to `MACOS`'s component (never truncates) and caps a dilation at `StemDarkening::MAX_PX` = 1.0 px a side.
- `StemDarkening::MACOS`: 0.0121 px per ppem in x, 1.25 times that in y, at most 0.3 px a side, none above 72 ppem.
- `GlyphParams { hinting, dilation }`: the per-glyph effective values, `TextRendering::glyph_params(ppem)`; the atlas key.
- `GlyphRasterizer`: three rules, `OFF` unchanged, outline re-filled dilated and re-centred, colour glyphs as cosmic-text drew them.
- Rotated text: `TextCache::rasterize(key, &mut GlyphRasterizer, TextRendering)` keeps the `Buffer::draw` code at `OFF` and draws per glyph through the rasterizer otherwise.

### States

n/a — a setting has no interaction states.

### Transitions

n/a — no motion; a change of setting between frames takes effect on the next draw and never serves a stale glyph.

### Input and focus

n/a — no input. The gallery's showcase has a TEXT section in its global settings (Font smoothing, Hinting, the five darkening numbers).

### Retained state

- Persistent: the glyph atlas (entries of several settings coexist until eviction) and the rotated-text rasters, keyed by `(TextKey, setting)`; `Gpu::end_frame` drops the rasters of a setting no draw of the app frame used.
- App-owned: the `TextRendering` value, passed in `FrameParams.text` each frame.

### Accessibility

n/a — drawing only.

### Strings

n/a — no strings.

### Gallery scenes

n/a — the setting changes every scene that draws text; the showcase's TEXT section toggles it live, and `gallery --dump` honours `LUNEUI_GALLERY_TEXT` (`on`, `off`).

### Painter primitives

- Existing: text.
- New: none.

### Acceptance

- The rasterizer (`crates/lune-ui-wgpu/tests/raster.rs`): `t1_off_is_cosmic_texts_image`, `t2_zero_dilation_through_the_outline_path_is_cosmic_texts_unhinted_image`, `t3_a_stem_gains_twice_the_dilation`, `t4_a_dilated_glyph_stays_centred`, `t5_coverage_grows_with_the_dilation`, `t6_the_same_inputs_give_the_same_bytes`.
- The renderer (`crates/lune-ui-wgpu/tests/text_rendering.rs`): a toggle never serves a cached glyph or raster of the other setting; a custom draw gets the effective setting; the rotated loop agrees with `Buffer::draw` and smooth text is heavier; the atlas grows under one setting and keeps the glyphs of another; a scene without text reads the same at every setting; rasters of settings no longer drawn are dropped.
- The values (`crates/lune-ui-core/src/text/rendering.rs`, inline tests): `macos_darkening_at_the_listed_sizes`, `no_darkening_above_the_limit_or_for_a_bad_size`, `smoothing_off_is_always_the_old_look`, `a_bad_factor_is_the_macos_one_for_that_component`, `max_px_is_capped_and_bad_ones_fail_to_the_default`, `keys_follow_the_effective_per_glyph_values`.
- Pixel gate: with `LUNEUI_GALLERY_TEXT=off` the dumps equal 0.3.0's on every scene except the five `showcase-*` scenes (D296).

### Build

- Milestone: font smoothing (0.4.0).
- Depends on: text.shaping, render.custom-draw.
- Done (D47, M11): tests `crates/lune-ui-wgpu/tests/raster.rs::t1_off_is_cosmic_texts_image, crates/lune-ui-wgpu/tests/raster.rs::t4_a_dilated_glyph_stays_centred, crates/lune-ui-wgpu/tests/text_rendering.rs::a_toggle_never_serves_a_cached_glyph_or_raster_of_the_other_setting, crates/lune-ui-wgpu/tests/text_rendering.rs::the_rotated_loop_agrees_with_buffer_draw_and_smooth_text_is_heavier, crates/lune-ui-core/src/text/rendering.rs::keys_follow_the_effective_per_glyph_values`; page n/a (every text scene; the showcase's TEXT section toggles it); unverified here: the look at 1x on a real 100%-scaled Windows display, colour glyphs (no colour font is bundled, so the pass-through is untested on real data), a hands-on toggle in a real window.

## Open questions

- Blend gamma is left out (D296): it depends on each glyph's colour and an assumed background and needs a shader uniform; adding it later is one field and a uniform in the vendored `Params` pad.
- Unhinted text at 1x may read softer than ClearType; an app that wants hinting at 1x passes `hinting: true` for that window's frames.

## Notes for the planner

An app's own glyph renderer rasterizes with `GlyphRasterizer::image(fonts, key, frame.text.glyph_params(ppem))`, keys its glyph cache by `GlyphParams` and leaves `DISABLE_HINTING` to the rasterizer (`docs/consuming.md`, "0.3.0 to 0.4.0").
