---
id: layout.panel
title: Panel (group box)
area: layout
status: in-progress
consumers: wane
source: —
owner: toolkit-v1 M7
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
- Header: padding 12 16; title 14/20 Medium; meta 11/16 tertiary at the right [W docs/ui/components.md:249-262].
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

- Behaviour (scripted input, injected time): none on the panel; the footer buttons fire once each. None exists.
- Pixel probes at scale 2: radius 16 px; 1 px ring at 2 px; header padding 24 px top and bottom, 32 px sides; body bottom padding 32 px.
- Conformance: none (no motion).
- Semantic node: `group` named by the title; the unavailable line announced once when it appears.

### Build

- Milestone: M7.
- Depends on: core.theme, core.painter, data.stat, widget.button.

## Open questions

None. Closed by [D148]: the panel is a toolkit component (`Ui::panel`, one closure called for the `Body` and `Footer` slots). The `--card` and `--flush` variants use the `clip-rounded` primitive the page uses, built in M5 (`Painter::push_clip_rounded`), in one row of the `core.painter` table. The outline is the only paint, drawn last; it needs no approximation. With `fit: true` the clip is the rect given, since the final height is known only after the body ran; `fit: false` fills the rect and pins the footer, which is how panels in a row share a bottom edge. A kind with no header gets a top padding of its own (16 plain, 8 rows, 12 card, none flush or strip) so the body does not touch the outline.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `layout.panel`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, local-models, overview, pricing, requests, routes, savings, sessions, settings, tray, upstreams.
