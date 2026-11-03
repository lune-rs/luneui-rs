---
id: layout.page
title: Page card (header, scroll body, footer)
area: layout
status: planned
consumers: wane
source: —
owner: —
summary: The card that holds a screen: header with title and actions, scrolling body, optional pinned footer
---

## What it is

The card that holds a screen: header with title and actions, scrolling body, optional pinned footer. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: The body scrolls vertically only (`widget.scrollbar`); the layout responds to width, never to height (`?full=1` grows the window to the content). The footer stays pinned.

## Needs

- Wane — Variants: with footer (`c-page__footer`, the wizard) (R-layout-1)
- Wane — States: inactive window: the card background becomes the solid inactive colour. A disabled primary in the footer keeps its place. (R-layout-1)
- Wane — Sizes: Radius 8 (`--radius-panel`), 1 px `--border-subtle`, fill `--glass-card` (rgba 50 50 50 .808, Lunegit's main-content acrylic: black tint 0.8, material 0.7); margin 0 10 10 6; header padding 16, title 18/24 Medium bright, subtitle 13/20 tertiary, actions right with 8 gap; body padding 0 16 16; footer padding 12 16, top hairline, secondary action left, primary group right. (R-layout-1)
- Wane — Behaviour: The body scrolls vertically only (`widget.scrollbar`); the layout responds to width, never to height (`?full=1` grows the window to the content). The footer stays pinned. (R-layout-1)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: partial: the CARD glass layer exists (theme.rs:105, CARD_CORNER_RADIUS theme.rs:115); header, scroll body and footer are missing). Design source (not code to move): Lunegit Desktop App/src/Lunegit.Desktop/Themes/DarkTheme.axaml:226-233 (GlassTintLightOpacity 0.8, GlassMaterialLightOpacity 0.7) used at Views/RepositoryContentView.axaml:47-58 (content acrylic); Luneterm docs/glass-styling.md section 4 (card layer, radius 8); Luneterm crates/lune-ui/src/theme.rs:101; Luneterm spikes/glass-ui dump 06.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/theme.rs:88-117` (glass layers, radius, inactive fill);
`crates/lune-ui-core/src/paint.rs:167` (rect clip); Lunegit `src/Lunegit.Desktop/Themes/DarkTheme.axaml:226-231`,
`src/Lunegit.Desktop/Views/RepositoryContentView.axaml:47-58`, `src/Lunegit.Desktop/Views/SidePanelView.axaml:9-20`,
`src/Lunegit.Desktop/Views/SettingsPageView.axaml:9-20`, `src/Lunegit.Desktop/Themes/Animations.axaml:295-311`,
`src/Lunegit.Desktop/App.axaml:769-770`; Wane `docs/ui/components.md:235-248`.
Conflicts (winner):
- Card layer: Wane `--glass-card` 50 50 50 .808 is Lunegit's content acrylic (tint 0.8, material 0.7) [W docs/ui/components.md:235-248]; the moved CARD layer is tint 1.0, material 0.8 [R crates/lune-ui-core/src/theme.rs:105-109]; the moved PANEL layer is tint 0.8, material 0.7 [R crates/lune-ui-core/src/theme.rs:98-102] and matches the Lunegit values exactly. The look follows Lunegit (CLAUDE.md), so the page is the PANEL values or a new layer; which one is an Open question (hard rule 8, a DECISIONS row).
- Radius: 8 in Wane [W docs/ui/components.md:235-248], the moved CARD_CORNER_RADIUS [R crates/lune-ui-core/src/theme.rs:115] and Lunegit's side and settings hosts [L src/Lunegit.Desktop/Views/SidePanelView.axaml:9; src/Lunegit.Desktop/Views/SettingsPageView.axaml:10]. All agree.
- Inactive window: the solid inactive fill is `INACTIVE_FALLBACK` #333333FF [R crates/lune-ui-core/src/theme.rs:113], which Wane's "solid inactive colour" names without a value; the moved value wins.

### Anatomy

- Card: radius 8 [R crates/lune-ui-core/src/theme.rs:115]; 1 px `border_subtle` #FFFFFF1A [R crates/lune-ui-core/src/theme.rs:63]; margin 0 10 10 6 [W docs/ui/components.md:235-248].
- Header: padding 16; title 18/24 Medium, bright; subtitle 13/20 tertiary; actions at the right, 8 apart [W docs/ui/components.md:235-248].
- Body: padding 0 16 16; vertical scroll only, with `widget.scrollbar` [W docs/ui/components.md:235-248].
- Footer: padding 12 16; top hairline; secondary action left, primary group right [W docs/ui/components.md:235-248].
- Inactive window: the card is the solid `INACTIVE_FALLBACK` #333333FF instead of the acrylic [R crates/lune-ui-core/src/theme.rs:111-113].
- Lunegit's content acrylic: `ExperimentalAcrylicMaterial`, tint Black, tint 0.8, material 0.7 [L src/Lunegit.Desktop/Views/RepositoryContentView.axaml:52-56].

### States

| state | look | cite |
|---|---|---|
| active | acrylic card (layer: open question, see Evidence) | [R crates/lune-ui-core/src/theme.rs:98-109] |
| inactive window | solid #333333FF; no acrylic | [R crates/lune-ui-core/src/theme.rs:113], [W docs/ui/components.md:235-248] |
| footer primary disabled | the primary keeps its place; the disabled look is the button's (widget.button) | [W docs/ui/components.md:235-248] |
| body scrolling | scrollbar visible while scrolling (widget.scrollbar) | [W docs/ui/components.md:235-248] |
| focus-visible | not on the card; the focused control inside it | [core.focus] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| page content swap in | opacity 0→1 | page-swap | moderate | quad-out | keep (opacity) | [L src/Lunegit.Desktop/Themes/Animations.axaml:295-308], [L .claude/docs/animation-rules.md:107] |
| page content swap in | translateY 6→0 | page-swap | moderate | sine-in-out | fade (dropped under Reduced) | [L src/Lunegit.Desktop/Themes/Animations.axaml:295-311], [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:160-163] |

The card itself does not move or resize; the window's width changes lay it out (Never animated: layout
properties on large surfaces, [L .claude/docs/animation-rules.md:149]). Inactive switch: snaps.

### Input and focus

- Pointer: the card takes the pointer over its controls only; the body scrolls with the wheel and the scrollbar [W docs/ui/components.md:235-248].
- Keyboard: the page has no keys of its own; Tab moves through the header actions, the body and the footer in order [core.focus].
- Tab stop: no (the card is a container) [derived: no interactive surface of its own].
- Layer: Page [derived: the content layer under the popups, `LayerId::Page`, crates/lune-ui-core/src/scene.rs:17-22].
- Focus scope: none. Focus moves to the first control on a page change [core.focus] (open question).
- Pointer capture while pressed: none.

### Retained state

- Transient: scroll momentum and the scrollbar fade [motion].
- Persistent: the scroll offset of the body, owned by the card (survives under `ui.keep_alive`) [core.ui].
- App-owned: title, subtitle, actions, footer content, the window's active flag [R crates/lune-ui-core/src/theme.rs:111-113].
- Exit retention: no.

### Accessibility

- Role `group` for the card, named by the title; the header actions are `button` nodes [infra.accessibility].
- Scroll body: role `scroll-area` with the scroll-into-view action [infra.accessibility].
- Footer: `group` with the primary and secondary actions as `button` nodes [infra.accessibility].
- Live region: none.

### Strings

None: title, subtitle and actions are app text.

### Gallery scenes

- `page-header-footer` (new): card with title, subtitle, two header actions and a footer with a secondary and primary button, settled.
- `page-body-scroll` (new): body taller than the card, scrolled to the middle, settled.
- `page-inactive` (new): the same card with the window inactive (solid fill), settled.
- `page-swap-opacity` (new): page content swap at t = 175 ms (half of moderate, 350 ms) [L src/Lunegit.Desktop/Themes/Animations.axaml:300], [L src/Lunegit.Desktop/App.axaml:154].

### Painter primitives

- Existing: acrylic, ring, fill, text, clip-rect. [R crates/lune-ui-core/src/paint.rs:243,231,215,274,167]
- New: clip-rounded (the body clips its children to the card's radius; Lunegit does the same with `ClipToBounds` on a radius-11 inner border, [L src/Lunegit.Desktop/App.axaml:769-770]).

### Acceptance

- Behaviour (scripted input, injected time): a wheel scroll moves the body offset and clamps at the ends; the footer stays put; Tab order is header, body, footer. None exists.
- Pixel probes at scale 2: radius 16 px; 1 px ring at 2 px; margin 20 px right, 20 px bottom, 12 px left; header padding 32 px; inactive fill #333333FF.
- Conformance: page swap reaches opacity 1 and offset 0 at 350 ms; Reduced fades opacity only.
- Semantic node: `group` named by the title; scroll-area role on the body.

### Build

- Milestone: M7.
- Depends on: core.acrylic, core.theme, core.ui, widget.scrollbar, widget.button, motion.transitions.

## Open questions

- Which layer the card uses: the moved CARD (tint 1.0, material 0.8) or Lunegit's content values (tint 0.8, material 0.7, = PANEL). The look follows Lunegit; a DECISIONS row is needed (hard rule 8).
- Whether the page owns focus on a page change (first control) or the app does: open.
- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: CSS `--glass-card` flattens Lunegit's content acrylic (black tint 0.8, material 0.7; derivation in `mocks/css/tokens.css`), not lune-ui's denser CARD layer (rgba 37 37 37 .89); luneui-rs paints the real layer (`core.acrylic`) and should take these Lunegit levels as its CARD material. The card needs scroll clipping with the radius.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `layout.page`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, local-models, overview, pricing, requests, routes, savings, sessions, settings, upstreams.
