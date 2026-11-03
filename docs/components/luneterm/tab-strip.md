---
id: luneterm.tab-strip
title: Terminal tab strip
area: luneterm
status: planned
consumers: luneterm
source: —
owner: —
summary: Tab strip for terminal sessions: new, close, rename, switch, close-confirm (Luneterm M10)
---

## What it is

Luneterm M10 lists tabs: new, close, rename, switch and close-confirm via foreground process. The tab outline geometry already exists (`core.tab-outline`); a strip built from it is not in `lune-ui` today per the module inventory.

## Needs

- Tab outline geometry as the visual base (R-core-7).
- Close-confirm uses the modal dialog (R-overlay-1); rename needs a text field (R-widget-1).
- Widgets the strip may need beyond today's set (R-widget-2).

## Existing code

Geometry: `crates/lune-ui/src/chrome.rs` (see `core.tab-outline`). The glass-ui spike imports `chrome` in `spikes/glass-ui/src/terminal_view.rs:5` (rest of the file not read for this entry). Lunegit's equivalent: `src/Lunegit.Desktop/Controls/Terminal/TerminalTabChrome.cs`, `TerminalTabDragController.cs`, `TerminalChromeAnimation.cs`.

## Open questions

- Luneterm's roadmap names new, close, rename, switch and close-confirm; drag reorder is Lunegit behaviour, not named for M10.
- Does the strip belong in the toolkit or in Luneterm? It reads terminal-shaped; the planner decides how much generalises.

## Notes for the planner

Needed by Luneterm M10 (blocked on the extraction, M9 first). Roadmap scope: `Luneterm real-build roadmap.md` M10 row.
