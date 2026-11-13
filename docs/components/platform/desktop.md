---
id: platform.desktop
title: Desktop: units, monitors, pointer, frames and moves
area: platform
status: done
consumers: any
source: crates/lune-ui-core/src/desktop.rs, crates/lune-ui-platform/src/desktop.rs, crates/lune-ui-platform/src/platform/desktop.rs
owner: —
summary: The OS desktop in one unit: monitors and work areas, the pointer, window frames, atomic multi-window moves and the window under a point
---

## What it is

The desktop as the OS reports it, in desktop units: the OS's own global space (physical px on Windows and X11; points from the primary display's top-left, y down, on macOS). The pure half (`lune_ui_core::desktop`) has the types and the maths; the adapter (`lune_ui_platform::desktop`) reads the monitors, the pointer and a window's frame, moves several windows as one batch and says which window is under a point. The core stays pure: the app calls the adapter and passes the values in (D313).

## Needs

- Monitors and their work areas, the pointer and window frames in one unit, so an app can place and snap windows across displays (R-platform-4).
- An atomic move of several windows, so a docked set moves as one (R-platform-4).

## Existing code

The core types are `DesktopPoint`, `DesktopRect`, `Monitor { bounds, work, units_per_logical, primary }`, `to_units` and `monitor_at` in `crates/lune-ui-core/src/desktop.rs` [D309]. The adapter is `crates/lune-ui-platform/src/desktop.rs` with its OS halves in `src/platform/desktop.rs` (and `desktop/`): `monitors`, `pointer`, `window_frame`, `set_window_origin`, `move_windows`, `window_at`, `units_per_logical` [D313, D314]. `lune_ui::desktop` holds both halves. The gallery side is `apps/gallery/src/panels.rs`.

## Spec

### Evidence

Planned and probed on a Windows 11 host in the docked-windows task (D309 to D318): a transparent DirectComposition window is hit even where its pixels are fully transparent, `SetWindowRgn` clips both its drawing and its hit test, and owned windows give one taskbar button and rise, go on top and minimise as one group. No upstream app design source: the shape is generic.

### Anatomy

- Units: `DesktopPoint`/`DesktopRect` are `i32`; `Monitor.units_per_logical` is the scale between a window's logical px and units (the window's scale factor on Windows, 1.0 on macOS).
- `to_units(logical, upl)` rounds halves away from zero, as winit rounds a logical size; `monitor_at` is the monitor holding a point, else the nearest, ties to the lower index.
- `desktop::pointer()` and `desktop::window_at()` read live OS state inside library `src`: an adapter exception, like `frontmost_app` (D313). macOS functions run on the main thread.
- macOS flips the bottom-left origin of AppKit to the top-left of the primary display (`a_rect_flips_from_the_bottom_left_to_the_top_left`).

### States

n/a — the entry is a set of functions and a window helper, not a widget with states. The gallery's panels show a shaded and a doubled form (`docked-panels`).

### Transitions

n/a — windows move and resize at once; there is no animation.

### Input and focus

- A drag reads `pointer()` on every pointer event winit sends while the button is held (windows moving under the pointer make window-relative deltas useless) and calls `move_windows` only when an origin changed; `RawInput` and `FrameOutput` do not change (D313).

### Retained state

n/a — the toolkit keeps none: the app holds the windows, their frames and the group, and passes desktop values in.

### Accessibility

n/a — the panels' controls are named buttons; the OS owns the windows' own accessibility.

### Strings

None: the panels' and the menu's texts are the gallery's own.

### Gallery scenes

- `docked-panels`: the three panels docked in a stack, the canvas version of what the live windows open on the monitor of the showcase window; the page's switch opens them as windows.

### Painter primitives

n/a — nothing here draws; the page's panels use `Painter::image_with` (`core.image`) and fills.

### Acceptance

- Pure maths in `crates/lune-ui-core/tests/desktop.rs`: rounding, monitor choice, containment and saturation. The OS reads and the flip are tested where they are pure (`crates/lune-ui-platform/src/desktop.rs`); the rest is the live check.
- The stack the page opens has equal left edges and each window touching the one above (`the_stack_has_equal_left_edges_and_each_touches_the_one_above`).

### Build

- Milestone: docked windows (0.6.0)
- Depends on: core.painter, platform.window
- Done (D47, M11): tests `crates/lune-ui-core/tests/desktop.rs::to_units_rounds_and_refuses_non_finite, crates/lune-ui-core/tests/desktop.rs::monitor_at_picks_the_containing_or_nearest, crates/lune-ui-platform/src/desktop.rs::a_rect_flips_from_the_bottom_left_to_the_top_left, crates/lune-ui-platform/src/desktop.rs::halves_round_away_from_zero, apps/gallery/src/panels.rs::the_stack_has_equal_left_edges_and_each_touches_the_one_above`; page `docked-panels`; unverified here: macOS is compile-only here (clippy for `aarch64-apple-darwin`, D308): nothing in this entry has run on a Mac; mixed-scale monitors (a window dragged from a 100 % to a 150 % display) are unchecked; and the feel of a real drag (lag between the pointer and the windows, snapping under a hand) is judged by nobody until a person tries it.

### API

- `monitors(window)`, `pointer()`, `window_frame(window)`, `set_window_origin(window, at)`, `move_windows(&[(&Window, DesktopPoint)])`, `window_at(window, at)` and `units_per_logical(window)`. Windows adds `windows-sys` as a direct dependency (D314); macOS adds the `objc2-app-kit` screen and event features.

### Unverified

The live check is `gallery --smoke-panels` (by hand, in a logged-in GUI session; its last reading is in `docs/STATE.md`); the gates run no window. Not run anywhere: a work area with the taskbar on a side or the top, a display left of or above the primary one (negative units), and `window_at` on macOS.

## Open questions

- None.

## Notes for the planner

The pieces are generic (no app words); the gallery's `docked-panels` page and `apps/gallery/src/panels.rs` are the worked example of an app wiring them.
