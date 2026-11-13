---
id: platform.window-shape
title: Window shape: the part of a window that is drawn and takes the pointer
area: platform
status: done
consumers: any
source: crates/lune-ui-core/src/desktop.rs, crates/lune-ui-platform/src/desktop.rs
owner: —
summary: A window shaped as a union of polygons: a window region on Windows, a pointer follower on macOS
---

## What it is

A `WindowShape` is a union of simple polygons in a frame's logical px (each polygon even-odd). `set_window_shape` gives a window that shape: on Windows a window region clips both its drawing and its hit test; macOS has no region API, so a `ShapeFollower` toggles whether the window takes clicks from the polled pointer. The app supplies the shape and sets it again after the window is resized or its scale changes (D315).

## Needs

- Borderless windows that are not rectangles: the desktop shows, and takes clicks, between the parts (R-platform-4).

## Existing code

`WindowShape` and `ShapeError` in `crates/lune-ui-core/src/desktop.rs` [D309]; `set_window_shape`, `ShapeMode`, `ShapeFollower` and `Follow` in `crates/lune-ui-platform/src/desktop.rs` [D315].

## Spec

### Evidence

Planned and probed on a Windows 11 host in the docked-windows task (D309 to D318): a transparent DirectComposition window is hit even where its pixels are fully transparent, `SetWindowRgn` clips both its drawing and its hit test, and owned windows give one taskbar button and rise, go on top and minimise as one group. No upstream app design source: the shape is generic.

### Anatomy

- `WindowShape::{new, with_polygon, with_rect, polygons, is_empty, bounds, contains}`: a union, not even-odd across polygons; a polygon needs 3 finite points.
- Windows: each point `round(x * scale)` plus the content's offset in the outer frame, a `CreatePolygonRgn` per polygon combined with `RGN_OR`, then `SetWindowRgn` (`ShapeMode::Region`); `None` removes the region.
- macOS: `ShapeMode::Pointer`: the follower makes the window click-through while the pointer is over its frame but outside the shape, and asks for `Follow::Poll`. It never calls winit on Windows.
- Elsewhere `ShapeMode::Unsupported`.
- The gallery's panels each have a notch cut out of a corner and draw nothing in it.

### States

n/a — the entry is a set of functions and a window helper, not a widget with states. The gallery's panels show a shaded and a doubled form (`docked-panels`).

### Transitions

n/a — windows move and resize at once; there is no animation.

### Input and focus

- Inside the shape the window takes clicks; in the notch they reach what is below. The app re-applies the shape after `Resized` and `ScaleFactorChanged`, and (macOS) asks the follower on the panel's pointer events and every 33 ms.

### Retained state

n/a — the toolkit keeps none: the app holds the windows, their frames and the group, and passes desktop values in.

### Accessibility

n/a — the panels' controls are named buttons; the OS owns the windows' own accessibility.

### Strings

None: the panels' and the menu's texts are the gallery's own.

### Gallery scenes

- `docked-panels`: each panel drawn with its notch empty; the live windows have the shape.

### Painter primitives

n/a — nothing here draws; the page's panels use `Painter::image_with` (`core.image`) and fills.

### Acceptance

- Pure: the union and edge cases in `crates/lune-ui-core/tests/desktop.rs`; `region_points_scale_round_and_offset` and `the_pointer_is_inside_through_or_outside_the_shape` in `crates/lune-ui-platform/src/desktop.rs`; every panel's shape leaves its notch out and keeps its body (`the_notch_is_cut_out_of_every_panels_shape`).
- Live: `gallery --smoke-panels` asks the OS which window is at a point in a notch and on a body.

### Build

- Milestone: docked windows (0.6.0)
- Depends on: platform.desktop, platform.window
- Done (D47, M11): tests `crates/lune-ui-core/tests/desktop.rs::a_polygon_needs_three_finite_points, crates/lune-ui-core/tests/desktop.rs::a_triangle_contains_its_inside, crates/lune-ui-core/tests/desktop.rs::separate_rects_leave_the_gap_out, crates/lune-ui-core/tests/desktop.rs::overlapping_polygons_are_a_union_not_even_odd, crates/lune-ui-platform/src/desktop.rs::region_points_scale_round_and_offset, crates/lune-ui-platform/src/desktop.rs::the_pointer_is_inside_through_or_outside_the_shape, apps/gallery/src/pages/docked_panels.rs::the_notch_is_cut_out_of_every_panels_shape`; page `docked-panels`; unverified here: macOS is compile-only here (clippy for `aarch64-apple-darwin`, D308): nothing in this entry has run on a Mac; mixed-scale monitors (a window dragged from a 100 % to a 150 % display) are unchecked; and the feel of a real drag (lag between the pointer and the windows, snapping under a hand) is judged by nobody until a person tries it.

### API

- `set_window_shape(window, Some(&shape), scale) -> Result<ShapeMode, String>`; `scale` is the frame's physical px per shape unit (`RawInput::scale`), so a window drawn at a zoom keeps its shape in step. A non-finite or non-positive scale is an `Err`.
- `ShapeFollower::update(window, shape, scale, pointer) -> Follow` (one per window).

### Unverified

The live check is `gallery --smoke-panels` (by hand, in a logged-in GUI session; its last reading is in `docs/STATE.md`); the gates run no window. The follower is not run anywhere: `set_cursor_hittest` on a shaped macOS window, and a Windows window region on a display left of the primary one.

## Open questions

- None.

## Notes for the planner

The pieces are generic (no app words); the gallery's `docked-panels` page and `apps/gallery/src/panels.rs` are the worked example of an app wiring them.
