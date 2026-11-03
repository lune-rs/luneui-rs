---
id: widget.text-field
title: Text field
area: widget
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/mod.rs
owner: orch/toolkit-v1
summary: TextBox with placeholder and a caret when focused
---

## What it is

`field`: TextBox with a placeholder and a caret when focused.

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).
- Text selection, caret movement and clipboard (R-text-5).
- Wane — Variants: fill (100% width); sm (24 high); mono (ids, URLs, paths); password (fixed 16-dot mask); search (16 px search icon at the left); date (calendar icon at the right, mono) (R-widget-1)
- Wane — States: rest; hover (`--surface-hover`); focus (border `--accent`); disabled (opacity .4); error (set by `widget.form-field`: border `--err`). (R-widget-1)
- Wane — Sizes: 240 x 32 (`--control-w-field`, `--control-h`), radius 6, 1 px `--border`, fill `--surface-input`, input padding 0 9, text 13, placeholder tertiary; search padding-left 33; date padding-right 28. (R-widget-1)
- Wane — Behaviour: Caret, selection, clipboard (cut, copy, paste), Esc, Tab. The password mask has a fixed length so it does not leak the key length, and the value cannot be copied. IME composition is a requirement (`luneterm.ime-composition` is planned). Not mocked: caret, selection, validation timing. (R-widget-1)

## Existing code

`crates/lune-ui-core/src/widgets/mod.rs:120`; the glass-ui spike calls it from `src/widgets.rs`. `widgets/mod.rs` has 203 lines and mostly no tests.

## Open questions

- Keyboard focus and key handling for this control are not in the inventory (R-widget-3).
- Text selection, caret movement and clipboard behaviour are unverified (R-text-5); IME is not covered (R-text-4).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: partial: the text field exists (widgets/mod.rs:120, box widgets/mod.rs:50); masked, search-icon, date and mono variants are missing; Wane priority P1. Native note: Native text input is not CSS-approximable: caret, selection, IME and clipboard are real work in luneui-rs. The mock's dots are U+2022 at 24 px in RznSans; luneui-rs draws its own mask glyphs. Full spec: Wane `docs/ui/components.md`, section `widget.text-field`; mock: Wane `docs/ui/mocks/components.html`.
