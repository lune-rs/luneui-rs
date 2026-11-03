---
id: luneterm.terminal-grid
title: Terminal grid renderer
area: luneterm
status: external
consumers: luneterm
source: —
owner: —
summary: Terminal grid drawing stays in Luneterm (lune-termview) and plugs in through the CustomDraw hook
---

## What it is

Luneterm's `lune-termview` draws the terminal grid through `CustomDraw`. It stays in Luneterm (decision D8); the toolkit only provides the hook (`render.custom-draw`).

## Needs

- The hook it builds on must stay stable (R-render-3).
- Not a toolkit deliverable (D8).

## Existing code

External: Luneterm `crates/lune-termview` (depends on lune-ui plus wgpu, cosmic-text, fontdb, etagere, zeno; not on lune-vt). It imports `lune_ui::custom::{CustomDraw, CustomFrame}`, `paint::Rect`, `color::Rgba` and `dump::{target, read_rgba, write_png}`.

## Open questions

- When lune-ui moves here, which of those imports does lune-termview need re-exported?

## Notes for the planner

Listed so the catalogue shows the boundary. Do not build it here.
