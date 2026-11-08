---
id: text.shaping
title: Shaped-text cache
area: text
status: done
consumers: luneterm
source: crates/lune-ui-core/src/text.rs, crates/lune-ui-wgpu/src/text_cache.rs
owner: —
summary: TextSystem seam in core; glyphon TextCache behind it, keyed by content and scale; the app registers its fonts
---

## What it is

Core: the `TextSystem` trait (`prepare(spec, scale)` returns a `TextKey` and a logical size; `measure`), plus `Font`, `Style` and `Spec` that describe the text to shape. Adapter: `TextCache` (`lune-ui-wgpu`) implements `TextSystem` over glyphon, with `new`, `split` and `end_frame`; the scale is a per-call argument, so one cache serves every window and entries of several scales coexist until the 180-frame age-out. Fakes: `testing::FakeText` and `testing::text_system_contract`.

## Needs

- Shape and measure text with app-supplied fonts (R-text-1).
- Cache by content and scale; scale is a per-call argument (R-text-2, R-nfr-6).

## Existing code

`crates/lune-ui-core/src/text.rs:172` (`TextSystem`), `:89` (`Spec`), `:125` (`TextKey`); `crates/lune-ui-wgpu/src/text_cache.rs:70` (`TextCache`), `:82` (`TextCache::new`), `:367` (`impl TextSystem`); contract tests in `crates/lune-ui-wgpu/tests/text.rs`.

## Spec

### Evidence

Pinned: this repo fa2958b. Read: `crates/lune-ui-core/src/text.rs:1-118` (`Font` 10, `Style` 17 with `ui` 26, `mono` 36, `medium` 46, `bold` 53, `Spec` 71 with `plain` 77, `wrapped` 84, `cache_key` 91, `TextKey` 105, `TextSystem` 110);
`crates/lune-ui-wgpu/src/text_cache.rs:1-129` (`TextCache` 19, `new` 31, `split` 47, `end_frame` 60); `THIRD_PARTY_NOTICES.md:9-10` (RznSans, RobotoMono Nerd Font Mono and their licences).
Conflicts: none.

### Anatomy

- Two fonts: `Ui` and `Mono` [R crates/lune-ui-core/src/text.rs:10-14].
- Sizes are logical pixels; the scale is an argument of each call [R crates/lune-ui-core/src/text.rs:1-3].
- `Style::ui(size, color)`, `Style::mono(size, line, color)`, `.medium()`, `.bold()` [R crates/lune-ui-core/src/text.rs:26,36,46,53].
- `Spec::plain`, `Spec::wrapped(width)`, and a `cache_key(scale)` [R crates/lune-ui-core/src/text.rs:77,84,91].
- Shaping: glyphon, a shaped-buffer cache behind `TextSystem` [R crates/lune-ui-wgpu/src/text_cache.rs:1,19,31,60].
- Fonts bundled: RznSans (UI, OFL 1.1, `licenses/RznSans-OFL-1.1.txt`) and RobotoMono Nerd Font Mono (terminal, Apache 2.0) [THIRD_PARTY_NOTICES.md:9-10].

### States

n/a — text has no interaction states; the owner's states apply.

### Transitions

n/a — no motion.

### Input and focus

n/a — no input.

### Retained state

- Persistent: the shaped-buffer cache, keyed by `cache_key` [R crates/lune-ui-wgpu/src/text_cache.rs:19-31].
- App-owned: the font data, passed in at `TextCache::new` [R crates/lune-ui-wgpu/src/text_cache.rs:31].

### Accessibility

n/a — text is read by the owner's node.

### Strings

n/a — the strings belong to the caller's `Spec`.

### Gallery scenes

- `controls` (existing): the text in the control set, scale 2. [R apps/gallery/src/scenes.rs:82-93]

### Painter primitives

- Existing: text. [R crates/lune-ui-core/src/paint.rs:274]
- New: none.

### Acceptance

- The contract on the real shaper and on the fake: `crates/lune-ui-wgpu/tests/text.rs::text_cache_with_fonts_meets_the_contract`, `text_cache_without_fonts_meets_the_contract` (no font database never panics, Q4), `text_cache_without_fonts_lays_out_nothing`; `crates/lune-ui-core/src/testing/text.rs::fake_text_meets_the_contract`, `fake_text_sizes_follow_its_formula`.
- Wrapped text at a fixed width: `text.rs::a_wrapped_line_breaks_into_rows_and_the_break_offset_starts_the_next` (width 60), `a_dropped_leading_space_belongs_to_its_lines_row`, `the_rows_stay_in_text_order`; the queries (caret, hit, lines, segments): `the_caret_at_the_end_is_the_width_and_lines_follow_the_endings`, `the_segment_table_matches_unicode_segmentation`, `right_to_left_and_mixed_text_answer_without_panicking`, `generated_text_meets_the_line_properties`. The pixel gate on `controls` (wrapped and unwrapped text at scale 2) at zero delta is the orchestrator's gate.
- The weights (`crates/lune-ui-wgpu/tests/text.rs`): `medium_is_heavier_than_regular` (the same string, more ink). No bold face is bundled (RznSans Regular and Medium only), so `bold_falls_back_to_the_nearest_bundled_weight` pins what happens: bold draws exactly as medium. Anything that asks for bold (a heading, a strong run) shows the Medium face.
- Hit in a cluster (item 40, D280): P21 of `text_system_contract` walks `x` in half-pixel steps on every row and requires `hit` never to go backwards; `a_hit_inside_a_cluster_of_several_glyphs_goes_by_the_clusters_midpoint` pins a mark pair (mono) and a skin tone (UI) at two scales. Inside a cluster that shaped to several glyphs `hit` answers the cluster's start left of the cluster's midpoint and its end right of it (mirrored for a right-to-left glyph); `Buffer::hit` alone halved each glyph on its own.
- Drag right of a cut row (item 41, D280): `a_drag_right_of_a_cut_word_reaches_the_rows_end`; `hit` is unchanged (D112), `EditLayout::extend_hit` does the extending.
- Eliding: `elided_cluster_text_stays_within_the_width`, `elided_mixed_direction_text_stays_within_the_width`.

### Build

- Milestone: M11 (done standard; code exists). M5 added the queries (step 11); the done standard stays with M11.
- Depends on: core.painter, core.theme.
- Done (D47, M11): tests `crates/lune-ui-wgpu/tests/text.rs::text_cache_with_fonts_meets_the_contract, crates/lune-ui-wgpu/tests/text.rs::text_cache_without_fonts_meets_the_contract, crates/lune-ui-wgpu/tests/text.rs::a_wrapped_line_breaks_into_rows_and_the_break_offset_starts_the_next, crates/lune-ui-wgpu/tests/text.rs::generated_text_meets_the_line_properties, crates/lune-ui-wgpu/tests/text.rs::medium_is_heavier_than_regular, crates/lune-ui-wgpu/tests/text.rs::bold_falls_back_to_the_nearest_bundled_weight, crates/lune-ui-core/src/testing/text.rs::fake_text_meets_the_contract`; page n/a (the shaper under every scene; `controls` shows the text at scale 2); unverified here: bold (no face bundled), bidi caret placement on real mixed-direction text beyond the contract's cases, Windows and Linux font fallback.

## Open questions

- Still open, a limit: no bold face is bundled, so bold shows as medium (D237: the bundled-font limit stays; a consumer that wants bold bundles the face and passes it to `TextCache::new`). A shaping feature such as `tnum` is not covered either, and no consumer has asked for it.
- None on the seam (M2 step 5): the trait has no glyphon type in its signature. `R-text-5` (selection, caret) is covered since M5 by the caret, hit, lines and segments queries (D86, D94); a shaping feature such as `tnum` is not covered yet.

## Notes for the planner

The seam landed in M2 step 5 with a fake and a contract suite; the 15 gallery scenes stayed at zero pixel delta.
