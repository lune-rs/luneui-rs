---
id: layout.sidebar
title: Sidebar
area: layout
status: in-progress
consumers: wane
source: —
owner: toolkit-v1 M7
summary: 208 px column on the window glass: nav list (or wizard steps) and a status footer
---

## What it is

208 px column on the window glass: nav list (or wizard steps) and a status footer. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Nav items navigate; the footer is static text. The wizard sidebar has no nav and no links (Back and Continue only).

## Needs

- Wane — Variants: main (nav items); wizard (`widget.steps` fills the nav slot, `role=navigation`, label "Setup steps") (R-layout-1)
- Wane — States: footer ok (ok dot, "Proxy on 127.0.0.1:4646"); footer error (err dot, "Proxy stopped", no address; the page notice carries the detail). (R-layout-1)
- Wane — Sizes: 208 wide (`--sidebar-w`), no card or fill of its own (it sits on the window glass); nav padding 8, items 2 apart (`nav_item::PITCH` 34 = 32 + 2); footer padding 12 16, top hairline `--border-subtle`, text 11/16 tertiary, addresses and `~/.wane` in RznMono; 10 px bottom margin (`--gutter`). (R-layout-1)
- Wane — Behaviour: Nav items navigate; the footer is static text. The wizard sidebar has no nav and no links (Back and Continue only). (R-layout-1)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Luneterm spikes/glass-ui/src/settings_view.rs:209 (nav column, 190 wide there); `docs/ui/mocks/css/components.css` (layout.sidebar block).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b, Luneterm 296cfdc. Read: Wane `docs/ui/components.md:221-234`;
Luneterm `spikes/glass-ui/src/settings_view.rs:209` (`git show 296cfdc`); `crates/lune-ui-core/src/theme.rs:63`
(`border_subtle`). Lunegit has no 208 px sidebar in the sources read: its side panel column width was not read (Open questions).
Conflicts: the nav width is 208 in Wane [W docs/ui/components.md:221-234] and 190 in the Luneterm spike
[LT spikes/glass-ui/src/settings_view.rs:209]. The spike is a draft, and Wane's 208 is the value the mock fixes; Wane wins for this entry.

### Anatomy

- Column: 208 wide; no card, no fill of its own; shows the window glass [W docs/ui/components.md:221-234].
- Nav: padding 8; items 2 apart [W docs/ui/components.md:221-234].
- Footer: padding 12 16; top hairline `--border-subtle` (repo `border_subtle` #FFFFFF1A, [R crates/lune-ui-core/src/theme.rs:63]); text 11/16 tertiary; addresses and `~/.wane` in RznMono [W docs/ui/components.md:221-234].
- Bottom margin 10 (`--gutter`) [W docs/ui/components.md:221-234].
- Nav column geometry in the spike: 10 px inset, 12 px top, 190 wide [LT spikes/glass-ui/src/settings_view.rs:209].

### States

| state | look | cite |
|---|---|---|
| nav, rest and selected | nav items from `widget.nav-item` | [W docs/ui/components.md:221-234] |
| footer ok | ok dot, "Proxy on 127.0.0.1:4646" | [W docs/ui/components.md:221-234] |
| footer error | err dot, "Proxy stopped", no address; the page notice carries the detail | [W docs/ui/components.md:221-234] |
| wizard | `widget.steps` in the nav slot; no nav links | [W docs/ui/components.md:221-234] |
| focus-visible | the focused nav item (widget.nav-item) | [core.focus] |

### Transitions

n/a — no motion in Wane or the spike. The content it switches (the page) animates in `layout.page`.

### Input and focus

- Pointer: a click on a nav item navigates [W docs/ui/components.md:221-234]; the footer takes no pointer input.
- Keyboard: the nav list is one roving stop; Up and Down move between items without wrapping, Enter and Space navigate (keys of `widget.nav-item`, [D121]) [core.focus].
- Tab stop: the nav list once (the selected row); the footer is not a stop [derived: static text, W docs/ui/components.md:221-234].
- Layer: Base (the window glass) [R crates/lune-ui-core/src/scene.rs:17-22].
- Focus scope: none.
- Pointer capture while pressed: none.

### Retained state

- Transient: nav hover.
- Persistent: none.
- App-owned: the selected nav item, the footer status (ok or error) and its text, the wizard step [W docs/ui/components.md:221-234].
- Exit retention: no.

### Accessibility

- Nav: a `Group` named by the caller over the nav list (`List` of `ListItem`, the rows of `widget.nav-item`). The wizard's nav is the steps' `Navigation` named by the caller (Wane `role=navigation` [W docs/ui/components.md:221-234]) [infra.accessibility], [D55].
- Items: `ListItem` nodes; the selected one has `Selected` and `Current` [infra.accessibility].
- Footer: a `Status` node (polite) for the ok or error text, a sibling after the group [infra.accessibility].

### Strings

None: nav labels and footer text are app text (Wane mock content, [W docs/ui/components.md:221-234]).

### Gallery scenes

- `sidebar-main` (new): six nav items, one selected, footer ok (mono text), settled.
- `sidebar-footer-error` (new): footer error, settled.
- `sidebar-wizard` (new): the wizard steps in the nav slot (one done, one current, two upcoming) with the ok footer, settled.

### Painter primitives

- Existing: fill, text. [R crates/lune-ui-core/src/paint.rs:215,274]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): a click on a nav item fires navigate once; the footer ignores input; the wizard has no nav links. None exists.
- Pixel probes at scale 2: column 416 px wide; nav padding 16 px; footer padding 24 px top and bottom, 32 px sides; hairline 2 px.
- Conformance: none (no motion here).
- Semantic node: nav `group`; items `button` with `selected`; footer `status`.

### Build

- Milestone: M7.
- Depends on: widget.nav-item, widget.steps, widget.status-dot, core.theme, core.painter.

## Open questions

None. Closed by [D149]: the sidebar is a toolkit component (`Ui::sidebar` with `SidebarBody::{Nav, Steps}` and an optional `SidebarFooter`); it has no collapse width of its own (the app owns the width and the rect it passes); the arrows are the nav list's (one stop, no wrap); the footer's bottom sits 10 px above the rect's bottom, its hairline spans the rect's width. Lunegit's side panel column width is the app's (it sets the window's split, not the host). Native rendering: no blur of its own: it shows the window material.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `layout.sidebar`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, local-models, overview, pricing, requests, routes, savings, sessions, settings, upstreams.
