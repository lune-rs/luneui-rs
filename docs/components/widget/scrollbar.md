---
id: widget.scrollbar
title: Scrollbars
area: widget
status: stub
consumers: lunegit, wane
source: Lunegit: src/Lunegit.Desktop/App.axaml
owner: —
summary: ScrollViewer, ScrollBar and Thumb overrides in App.axaml (rough stub)
---

## What it is

Rough stub — not analysed. The style survey found ScrollViewer, ScrollBar and Thumb overrides in `App.axaml`; `ScrollOffsetPreserver` and `AutoScrollBehavior` are separate behaviours.

## Needs

- Scrollbar widget (R-widget-2).
- Scroll regions that keep offset (R-layout-2).
- Wane — States: hidden; visible while the pointer is over the scroll area (fade `--dur-fast`); thumb hover (`--scroll-thumb-hover`, accent at .9); content fits (no thumb). (R-widget-2)
- Wane — Sizes: Track 9 wide, transparent; thumb 6 wide at right 1, radius 3 (`--radius-thumb`), minimum length 28, fill `--scroll-thumb` (accent at .55). (R-widget-2)
- Wane — Behaviour: Wheel and touchpad scroll; drag the thumb; PageUp and PageDown in the scroll area. Dragging and the fade are not mocked (headless Chromium hides native bars, so the thumb is drawn and placed by `components.js`). (R-widget-2)

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/App.axaml`

## Open questions

- Auto-hide, overlay or classic?

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

Wane (2026-11-04): lune-ui status for Wane's use: missing; Wane priority P0. Native note: luneui-rs draws its own thumb, so the open points are the fade, the 28 px minimum, dragging and keyboard paging. The code block and the tables reuse it for horizontal overflow. Full spec: Wane `docs/ui/components.md`, section `widget.scrollbar`; mock: Wane `docs/ui/mocks/components.html`.
