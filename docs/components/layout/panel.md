---
id: layout.panel
title: Panel (group box)
area: layout
status: done
consumers: wane
source: crates/lune-ui-core/src/widgets/panel.rs
owner: —
summary: Outline-only group box with title, meta, body and action footer
---

## What it is

Outline-only group box with title, meta, body and action footer. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Static container. Not collapsible in the mocks.

## Needs

- Wane — Variants: `--flush` (table or list edge to edge, hairline under the header); `--strip` (row of stat cells with hairline dividers); `--card` (a panel inside a panel, 12 padding, children stacked, footer button pinned to the bottom). (R-layout-1)
- Wane — States: unavailable body: one tertiary line, "Unavailable while the proxy is stopped." Panels in a row share a bottom edge (layout rule). (R-layout-1)
- Wane — Sizes: Radius 8, 1 px `--border-subtle`, no fill, no shadow; header padding 12 16, title 14/20 Medium, meta 11/16 tertiary at the right; body padding 0 16 16 (`__body--rows`: 0 4 8); footer padding 12 16, top hairline, note 11/16 tertiary at the left, secondary then primary button at the right. (R-layout-1)
- Wane — Behaviour: Static container. Not collapsible in the mocks. (R-layout-1)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Luneterm spikes/glass-ui/src/settings_view.rs:280 (draw_group), :282-345; glass-ui dump 06.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b, Luneterm 296cfdc. Read: `crates/lune-ui-core/src/theme.rs:55-63`
(`border_subtle`); Wane `docs/ui/components.md:249-262`; Luneterm `spikes/glass-ui/src/settings_view.rs:280-345` (`git show 296cfdc`);
Lunegit `src/Lunegit.Desktop/App.axaml:769-770` (clip to a rounded inner border).
Conflicts: none between sources. Luneterm's spike is a design source only (a spike, not moved code).

### Anatomy

- Outline: radius 8; 1 px `--border-subtle`; no fill; no shadow [W docs/ui/components.md:249-262]. The repo token is `border_subtle` #FFFFFF1A [R crates/lune-ui-core/src/theme.rs:63].
- Header: padding 12 16; title 14/20 Medium, `fg_primary` [W docs/ui/mocks/css/components.css:470 `--text-primary`; LT spikes/glass-ui/src/settings_view.rs:283]; meta 11/16 tertiary at the right [W docs/ui/components.md:249-262].
- Body: padding 0 16 16; rows variant 0 4 8 [W docs/ui/components.md:249-262].
- Footer: padding 12 16; top hairline; note 11/16 tertiary at the left; secondary then primary button at the right [W docs/ui/components.md:249-262].
- Variants: `--flush` body edge to edge with a hairline under the header; `--strip` cells from `data.stat` with hairline dividers; `--card` a nested panel with 12 padding [W docs/ui/components.md:249-262].
- Draw reference: the group draw in Luneterm's spike [LT spikes/glass-ui/src/settings_view.rs:280-345].

### States

| state | look | cite |
|---|---|---|
| rest | outline only | [W docs/ui/components.md:249-262] |
| unavailable body | one tertiary line: "Unavailable while the proxy is stopped." | [W docs/ui/components.md:249-262] |
| row siblings | panels in a row share a bottom edge | [W docs/ui/components.md:249-262] |
| focus-visible | not on the panel; the focused control inside it | [core.focus] |

### Transitions

n/a — the panel is static and not collapsible in the mocks [W docs/ui/components.md:249-262]; the expand of a
collapsible panel is out of scope here (no source).

### Input and focus

- Pointer: none on the panel; its footer buttons take the pointer [W docs/ui/components.md:249-262].
- Keyboard: none on the panel [derived: a container with no keys].
- Tab stop: no; the footer buttons are the tab stops [core.focus].
- Layer: whatever layer the app draws it on [derived: a container].
- Focus scope: none.
- Pointer capture while pressed: none.

### Retained state

- Transient: none.
- Persistent: none.
- App-owned: title, meta, body content, footer note and buttons, the unavailable flag [W docs/ui/components.md:249-262].
- Exit retention: no.

### Accessibility

- Role `group`, name = title, description = meta [infra.accessibility].
- Footer note: `static-text`; buttons: `button` nodes [infra.accessibility].
- Unavailable line: `static-text` with the `status` role when it appears [infra.accessibility].
- Live region: none (the unavailable line is a status, polite, when it changes).

### Strings

None: title, meta, note and button labels are app text (Wane mock content, [W docs/ui/components.md:249-262]).

### Gallery scenes

- `panel-flush` (new): flush body with a list, settled.
- `panel-strip` (new): strip of three `data.stat` cells, settled.
- `panel-card` (new, at 2x and at 1x): a card panel nested in a panel with a pinned footer button, settled.
- `panel-unavailable` (new): the unavailable line in the body, settled.

### Painter primitives

- Existing: ring, text, fill. [R crates/lune-ui-core/src/theme.rs:63; crates/lune-ui-core/src/paint.rs:215,231,274]
- clip-rounded: built in M5 (`Painter::push_clip_rounded`); children clip to the 8 px radius ([W docs/ui/components.md:249-262]; Lunegit clips a rounded inner border with `ClipToBounds`, [L src/Lunegit.Desktop/App.axaml:769-770]). It cuts fills and images, not text, so the panel keeps its own text 16 px in and a flush body's text is the caller's to inset by `panel::TEXT_INSET` (8) [D148].
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): none on the panel; the footer buttons fire once each.
- Pixel probes at scale 2: radius 16 px; 1 px ring at 2 px; header padding 24 px top and bottom, 32 px sides; body bottom padding 32 px.
- Conformance: none (no motion).
- Semantic node: `group` named by the title; the unavailable line announced once when it appears.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_panel.rs`: pixel probes `pixel_probes_at_scale_two`, `it_draws_an_outline_only_at_radius_8_and_last`, `a_plain_panel_fits_header_padding_body_padding_and_returns_the_height`, `rows_pad_0_4_8`, `a_flush_body_is_edge_to_edge_under_a_hairline`, `a_card_pads_12_all_round_and_pins_its_footer_button_with_no_hairline`, `a_filling_panel_pins_its_footer_so_a_row_of_panels_shares_a_bottom_edge`; behaviour `the_note_sits_left_of_the_buttons_and_is_elided_to_the_room`, `tab_order_is_body_then_footer`; node `the_nodes_are_a_group_with_the_note_and_the_buttons_inside`, `unavailable_replaces_the_body_with_one_tertiary_status_line`, `a_disabled_panel_is_disabled_all_the_way_down`.

### Build

- Milestone: M7.
- Depends on: core.theme, core.painter, data.stat, widget.button.
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_panel.rs::pixel_probes_at_scale_two, crates/lune-ui-core/tests/widget_panel.rs::a_plain_panel_fits_header_padding_body_padding_and_returns_the_height, crates/lune-ui-core/tests/widget_panel.rs::a_filling_panel_pins_its_footer_so_a_row_of_panels_shares_a_bottom_edge, crates/lune-ui-core/tests/widget_panel.rs::tab_order_is_body_then_footer, crates/lune-ui-core/tests/widget_panel.rs::the_nodes_are_a_group_with_the_note_and_the_buttons_inside, crates/lune-ui-core/tests/widget_panel.rs::a_flush_body_fill_is_cut_by_the_rounded_clip_at_the_panels_rect`; page `panel`; unverified here: none (documented limit: `fit: true` cuts a flush body's bottom corners only at the final height).

## Open questions

None. Closed by [D148]: the panel is a toolkit component (`Ui::panel`, one closure called for the `Body` and `Footer` slots). The `--card` and `--flush` variants use the `clip-rounded` primitive the page uses, built in M5 (`Painter::push_clip_rounded`), in one row of the `core.painter` table. The outline is the only paint, drawn last; it needs no approximation. With `fit: true` the clip is the rect given, since the final height is known only after the body ran; `fit: false` fills the rect and pins the footer, which is how panels in a row share a bottom edge. With `fit: true` the rounded clip is the rect given, so a flush body's bottom corners are cut only when the rect is the final height (still open, a documented limit of `fit: true`: `panel.rs` says it too; no scene shows it; `fit: false` does cut them, `a_flush_body_fill_is_cut_by_the_rounded_clip_at_the_panels_rect`). A kind with no header gets a top padding of its own (16 plain, 8 rows, 12 card, none flush or strip) so the body does not touch the outline.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `layout.panel`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, local-models, overview, pricing, requests, routes, savings, sessions, settings, tray, upstreams.
