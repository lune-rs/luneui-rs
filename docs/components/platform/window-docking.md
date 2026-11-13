---
id: platform.window-docking
title: Window docking: touching, docked sets, snapping and a set drag
area: platform
status: done
consumers: any
source: crates/lune-ui-core/src/dock.rs, crates/lune-ui-core/src/desktop.rs
owner: —
summary: Pure docking maths in desktop units: touching edges, transitive docked sets, snapping to edges and a group drag
---

## What it is

The maths of windows that dock: two windows are docked when they share an edge segment, a docked set is transitive, and a dragged set snaps to the edges of the other windows and of the work areas. It knows no window type: the app says which windows move and applies the answer (D310).

## Needs

- Snap windows to each other and to the work area, and drag a docked set as one (R-platform-4).

## Existing code

`crates/lune-ui-core/src/dock.rs` [D310]: `touching`, `docked_sets`, `docked_with`, `snap` and `DockDrag`. The gallery wires them in `apps/gallery/src/panels.rs`.

## Spec

### Evidence

Planned and probed on a Windows 11 host in the docked-windows task (D309 to D318): a transparent DirectComposition window is hit even where its pixels are fully transparent, `SetWindowRgn` clips both its drawing and its hit test, and owned windows give one taskbar button and rise, go on top and minimise as one group. No upstream app design source: the shape is generic.

### Anatomy

- `touching(a, b)`: an exact shared edge segment of positive length; corners and gaps do not count.
- `docked_sets(rects)` maps each index to the lowest index of its transitive set; `docked_with(i, rects)` lists a set.
- `snap(moving, fixed, areas, within)`: each axis on its own, the nearest edge pair within reach; a tie goes to the first candidate (areas before windows).
- `DockDrag`: the travel since the press, added to the start rects and then snapped, so a snap never accumulates drift and the windows follow the pointer again once it leaves the snap.
- A rect at the edge of the `i32` range does not panic (saturating).

### States

n/a — the entry is a set of functions and a window helper, not a widget with states. The gallery's panels show a shaded and a doubled form (`docked-panels`).

### Transitions

n/a — windows move and resize at once; there is no animation.

### Input and focus

- The press on a strip starts the drag; the pointer comes from the app each event; the release ends it and the app records which members are docked now. The first panel moves the set docked with it, any other panel moves alone.

### Retained state

n/a — the toolkit keeps none: the app holds the windows, their frames and the group, and passes desktop values in.

### Accessibility

n/a — the panels' controls are named buttons; the OS owns the windows' own accessibility.

### Strings

None: the panels' and the menu's texts are the gallery's own.

### Gallery scenes

- `docked-panels`: the three panels docked in a stack: the set a drag of the first one moves; a drag of the small one detaches it.

### Painter primitives

n/a — nothing here draws; the page's panels use `Painter::image_with` (`core.image`) and fills.

### Acceptance

- Known answers in `crates/lune-ui-core/tests/dock.rs`: a small drag does not snap, a drag near another window docks (and then `docked_sets` names one set), an area edge pulls, a tie goes to the area, `within` 0 gives no snap, extreme coordinates do not panic.
- A doubled set is re-placed at the leader's origin plus twice each offset (`doubling_puts_a_window_at_twice_its_offset_from_the_leader_and_halving_undoes_it`).

### Build

- Milestone: docked windows (0.6.0)
- Depends on: platform.desktop
- Done (D47, M11): tests `crates/lune-ui-core/tests/dock.rs::an_exact_shared_edge_touches, crates/lune-ui-core/tests/dock.rs::docked_sets_are_transitive_and_named_by_the_lowest_index, crates/lune-ui-core/tests/dock.rs::a_drag_snaps_to_a_window_and_docks, crates/lune-ui-core/tests/dock.rs::an_area_edge_pulls_a_rect, crates/lune-ui-core/tests/dock.rs::a_tie_goes_to_the_area, crates/lune-ui-core/tests/dock.rs::rects_at_the_edge_of_the_range_do_not_panic, apps/gallery/src/panels.rs::doubling_puts_a_window_at_twice_its_offset_from_the_leader_and_halving_undoes_it`; page `docked-panels`; unverified here: macOS is compile-only here (clippy for `aarch64-apple-darwin`, D308): nothing in this entry has run on a Mac; mixed-scale monitors (a window dragged from a 100 % to a 150 % display) are unchecked; and the feel of a real drag (lag between the pointer and the windows, snapping under a hand) is judged by nobody until a person tries it.

### API

- `DockDrag::start(pointer, rects, moving)` and `DockDrag::update(pointer, rects, areas, within) -> Vec<(usize, DesktopPoint)>` (new origins, ascending by index).

### Unverified

The live check is `gallery --smoke-panels` (by hand, in a logged-in GUI session; its last reading is in `docs/STATE.md`); the gates run no window. Not run anywhere: a drag across displays of different scales, and the snap distance against a real hand (10 logical px converted at the pressed window's scale).

## Open questions

- None.

## Notes for the planner

The pieces are generic (no app words); the gallery's `docked-panels` page and `apps/gallery/src/panels.rs` are the worked example of an app wiring them.
