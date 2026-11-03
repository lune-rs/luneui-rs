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

## Open questions

- None on the seam (M2 step 5): the trait has no glyphon type in its signature. `R-text-5` (selection, caret) and a shaping feature such as `tnum` are not covered yet.

## Notes for the planner

The seam landed in M2 step 5 with a fake and a contract suite; the 15 gallery scenes stayed at zero pixel delta.
