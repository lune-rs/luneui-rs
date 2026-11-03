---
id: platform.window
title: Glass window helpers
area: platform
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui-core/src/window.rs, crates/lune-ui-winit/src/window.rs
owner: orch/toolkit-v1
summary: Glass window attributes, edge resize, double-click detection and caption buttons
---

## What it is

Core (`lune-ui-core`): `EDGE_BORDER`, `ResizeEdge`, `edge_at`, `DoubleClick` and `caption_buttons`, with no windowing type. winit adapter (`lune-ui-winit`): `glass_attributes` (per-platform window attributes from `platform/`) and `resize_direction` (`ResizeEdge` to winit's `ResizeDirection`).

## Needs

- Custom caption, edge resize and double-click handling (R-platform-4).
- Wane — Variants: none (Windows chrome, D9). Parts: brand (wordmark + version), fill (drag area), caption buttons (minimize, maximize, close). (R-platform-4)
- Wane — States: active; inactive (`is-inactive`: solid near-black `--glass-inactive` #0b0b0b, page card `--glass-inactive-card` #141414, no blur, caption glyphs quaternary; user, 2026-11-03: dark black, not lune-ui's #333333 grey); caption hover (`--surface-hover`); close hover (`--caption-close-bg` #c42b1c, white glyph). (R-platform-4)
- Wane — Sizes: Design size 1280 x 800 (`--window-w`, `--window-h`); radius 10 (`--radius-page`); 1 px `--border-subtle`; title strip 32 (`--titlebar-h`) with a 208 wide brand cell (`--sidebar-w`), padding 0 16 0 18, wordmark 14 bold gold, version RznMono 11 tertiary; caption buttons 46 x 32 (`--caption-w`); content grid 208 | 1fr, 10 px gutter (`--gutter`). (R-platform-4)
- Wane — Behaviour: Drag by the title strip, double-click maximizes, edge resize, minimize / maximize / close. The window layer swaps to the solid inactive colour when it loses focus (`overview-unfocused`). Mouse and keyboard (Alt+Space system menu) come from the OS window. (R-platform-4)

## Existing code

`crates/lune-ui-core/src/window.rs:26` (`edge_at`), `:48` (`DoubleClick`), `:70` (`caption_buttons`); 99 lines; `crates/lune-ui-winit/src/window.rs:12` (`glass_attributes`), `:28` (`resize_direction`). Integration test `crates/lune-ui-core/tests/window.rs` (edges, 6 px border, double-click gap under 400 ms); `crates/lune-ui-winit/tests/input.rs` covers the direction mapping.

## Open questions

- None on the seam: the core half has no winit type; the winit mapping is in `lune-ui-winit` (M2 step 8). `edge_at` returns `ResizeEdge` (was winit's `ResizeDirection`).

## Notes for the planner

Moved in the behaviour-preserving extraction (R-ext-1); split across core and winit in M2 step 8.

Wane (2026-11-04): lune-ui status for Wane's use: partial: caption buttons (core window.rs:70) and glass attributes (lune-ui-winit window.rs:12) exist; the title strip and the wordmark are app composition; Wane priority P0. Native note: `backdrop-filter: blur(40px) saturate(1.6)` over a CSS backdrop approximates the window material. luneui-rs uses the real OS material (`platform.material`: acrylic or Mica on Windows, vibrancy or liquid glass on macOS) and the in-app blur pass (`render.blur`). The flattened colours (`--glass-window` rgba 54 54 54 .81) are measured from lune-ui dumps: reuse lune-ui's WINDOW layer, not these numbers. The mocks do not draw macOS traffic lights. Full spec: Wane `docs/ui/components.md`, section `platform.window`; mock: Wane `docs/ui/mocks/components.html`.
