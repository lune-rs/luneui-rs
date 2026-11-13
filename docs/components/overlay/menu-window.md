---
id: overlay.menu-window
title: Context menu in its own window
area: overlay
status: done
consumers: any
source: crates/lune-ui-core/src/desktop.rs, crates/lune-ui-core/src/widgets/menu.rs
owner: —
summary: A menu that opens as a window of its own, placed from the pointer on the right monitor and kept inside its work area
---

## What it is

A context menu for a borderless window that is too small to hold it: the menu is a window of its own (`popover_attributes` plus `owned_by` the panel), drawn with `Ui::menu_window`, placed by `desktop::place_popup` (`widgets::popover::place` in desktop units, on the monitor under the pointer, inside its work area) and hidden once dismissed and idle (D311). A widget never builds the menu: it reports the secondary click and the app opens it (D292).

## Needs

- A menu that can be larger than the window that opens it (R-overlay-4).

## Existing code

`place_popup` in `crates/lune-ui-core/src/desktop.rs` [D311] over `widgets::popover::place`; the menu itself is `overlay.menu` (`Ui::menu_window`, `menu_size`). The gallery's window is `MenuWindow` in `apps/gallery/src/panels.rs`, drawn by `draw_menu` in `apps/gallery/src/pages/docked_panels.rs`.

## Spec

### Evidence

Planned and probed on a Windows 11 host in the docked-windows task (D309 to D318): a transparent DirectComposition window is hit even where its pixels are fully transparent, `SetWindowRgn` clips both its drawing and its hit test, and owned windows give one taskbar button and rise, go on top and minimise as one group. No upstream app design source: the shape is generic.

### Anatomy

- `place_popup(anchor, size, side, align, gap, monitor)`: the size and gap are multiplied by the monitor's `units_per_logical` and rounded the way winit rounds, the result is flipped and shifted inside the monitor's work area; `None` for a bad scale, size or gap.
- The window is sized first, then moved (macOS keeps a resized window's bottom edge), then shown; it is owned by the panel that opened it, so it is dropped before it.
- Its focus loss, Escape and an outside press dismiss it (`Ui::menu_window`).

### States

n/a — the entry is a set of functions and a window helper, not a widget with states. The gallery's panels show a shaded and a doubled form (`docked-panels`).

### Transitions

n/a — windows move and resize at once; there is no animation.

### Input and focus

- The menu's rows: Always on top and Snap to edges (checked when on), Double size, Close panels. A pick acts on the group; a dismissal hides the window once the exit has played.

### Retained state

n/a — the toolkit keeps none: the app holds the windows, their frames and the group, and passes desktop values in.

### Accessibility

n/a — the panels' controls are named buttons; the OS owns the windows' own accessibility.

### Strings

None: the panels' and the menu's texts are the gallery's own.

### Gallery scenes

- `docked-panels-menu`: the menu through `menu_window_in` in a pane over a flat stand-in for the desktop, next to the small panel; settled after the open's motion.

### Painter primitives

n/a — nothing here draws; the page's panels use `Painter::image_with` (`core.image`) and fills.

### Acceptance

- Placement known answers in `crates/lune-ui-core/tests/desktop.rs`: flipped above and shifted left near the bottom-right corner, unchanged near the top-left, the gap scaled, bad numbers refused.
- The small panel is smaller than its menu on both axes (`the_small_panel_is_the_smallest_and_smaller_than_the_menu_on_both_axes`).

### Build

- Milestone: docked windows (0.6.0)
- Depends on: overlay.menu, overlay.popover, platform.desktop, platform.window-group
- Done (D47, M11): tests `crates/lune-ui-core/tests/desktop.rs::a_popup_flips_and_shifts_inside_the_work_area, crates/lune-ui-core/tests/desktop.rs::a_popup_gap_is_scaled, crates/lune-ui-core/tests/desktop.rs::a_popup_refuses_bad_numbers, apps/gallery/src/pages/docked_panels.rs::the_small_panel_is_the_smallest_and_smaller_than_the_menu_on_both_axes`; page `docked-panels`; unverified here: macOS is compile-only here (clippy for `aarch64-apple-darwin`, D308): nothing in this entry has run on a Mac; mixed-scale monitors (a window dragged from a 100 % to a 150 % display) are unchecked; and the feel of a real drag (lag between the pointer and the windows, snapping under a hand) is judged by nobody until a person tries it.

### API

- `Ui::menu_window(key, rows)` and `Ui::menu_window_in(key, area, rows)` fill the window; `menu_size(painter, rows)` is the window's size in logical px.

### Unverified

The live check is `gallery --smoke-panels` (by hand, in a logged-in GUI session; its last reading is in `docs/STATE.md`); the gates run no window. The OS glass behind the menu window, and whether an owned popup keeps the focus rules of the tray's (`Focused(false)` at creation) are not checked on any OS.

## Open questions

- None.

## Notes for the planner

The pieces are generic (no app words); the gallery's `docked-panels` page and `apps/gallery/src/panels.rs` are the worked example of an app wiring them.
