---
id: text.shaping
title: Shaped-text cache
area: text
status: in-progress
consumers: luneterm
source: crates/lune-ui-core/src/text.rs, crates/lune-ui-wgpu/src/text_cache.rs
owner: orch/toolkit-v1
summary: TextSystem seam in core; glyphon TextCache behind it, keyed by content and scale; the app registers its fonts
---

## What it is

Core: the `TextSystem` trait (`prepare(spec, scale)` returns a `TextKey` and a logical size; `measure`), plus `Font`, `Style` and `Spec` that describe the text to shape. Adapter: `TextCache` (`lune-ui-wgpu`) implements `TextSystem` over glyphon, with `new`, `split` and `end_frame`; the scale is a per-call argument, so one cache serves every window and entries of several scales coexist until the 180-frame age-out. Fakes: `testing::FakeText` and `testing::text_system_contract`.

## Needs

- Shape and measure text with app-supplied fonts (R-text-1).
- Cache by content and scale; scale is a per-call argument (R-text-2, R-nfr-6).

## Existing code

`crates/lune-ui-core/src/text.rs:110` (`TextSystem`), `:71` (`Spec`), `:105` (`TextKey`); `crates/lune-ui-wgpu/src/text_cache.rs:19` (`TextCache`), `:31` (`TextCache::new`), `:68` (`impl TextSystem`); contract tests in `crates/lune-ui-wgpu/tests/text.rs`.

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

- Existing tests (by path): `crates/lune-ui-wgpu/tests/text.rs` `text_cache_with_fonts_meets_the_contract`, `text_cache_without_fonts_meets_the_contract`; `crates/lune-ui-core/src/testing/text.rs` `fake_text_meets_the_contract`, `fake_text_sizes_follow_its_formula`.
- Done standard still needs: a wrapped-text test at a fixed width against the pixel gate, and the medium and bold face probes.

### Build

- Milestone: M11 (done standard; code exists). M5 added the queries (step 11); the done standard stays with M11.
- Depends on: core.painter, core.theme.

## Open questions

- None on the seam (M2 step 5): the trait has no glyphon type in its signature. `R-text-5` (selection, caret) is covered since M5 by the caret, hit, lines and segments queries (D86, D94); a shaping feature such as `tnum` is not covered yet.

## Notes for the planner

The seam landed in M2 step 5 with a fake and a contract suite; the 15 gallery scenes stayed at zero pixel delta.
