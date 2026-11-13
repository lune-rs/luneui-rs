---
id: platform.window-group
title: Window group: windows that raise, minimise and stay on top as one
area: platform
status: done
consumers: any
source: crates/lune-ui-platform/src/group.rs, crates/lune-ui-platform/src/platform/group.rs
owner: —
summary: A leader and its member windows that the OS treats as one: one taskbar entry, raise, on top and minimise together
---

## What it is

A `WindowGroup` makes several windows of one app act as one to the OS. On Windows the members are owned windows of the leader (one taskbar button; the OS raises, keeps on top and minimises them together). On macOS a member docked to the leader is a child window of it, and the detached members are ordered by hand. `owned_by` also serves a popup window (D316).

## Needs

- Several windows that come to the front, stay on top and minimise as one app window (R-platform-4).

## Existing code

`crates/lune-ui-platform/src/group.rs` [D316] with the OS halves in `src/platform/group.rs` (and `group/`). The gallery builds the group in `apps/gallery/src/panels.rs`.

## Spec

### Evidence

Planned and probed on a Windows 11 host in the docked-windows task (D309 to D318): a transparent DirectComposition window is hit even where its pixels are fully transparent, `SetWindowRgn` clips both its drawing and its hit test, and owned windows give one taskbar button and rise, go on top and minimise as one group. No upstream app design source: the shape is generic.

### Anatomy

- `WindowGroup { members, leader }` (members drop first): `new`, `member_attributes`, `add`, `set_docked`, `is_docked`, `raise`, `minimise`, `restore`, `set_always_on_top`, `leader`, `members`, `contains`.
- `owned_by(attrs, owner)`: Windows `with_owner_window`; unchanged elsewhere.
- A member must never be minimised alone (a minimised owned window becomes a stub at the desktop corner): `Minimize` and `SetAlwaysOnTop` from any group window go through the group.
- macOS decisions are pure helpers that run on every host: `raise_order` (the windows to order just below the front one) and `hidden_with_leader` (the detached members `minimise` orders out).

### States

n/a — the entry is a set of functions and a window helper, not a widget with states. The gallery's panels show a shaded and a doubled form (`docked-panels`).

### Transitions

n/a — windows move and resize at once; there is no animation.

### Input and focus

- A panel gaining focus raises the group; a minimise control and the menu's Always on top row go through the group; the group is dropped with its windows (menu, then panels, then the group).

### Retained state

n/a — the toolkit keeps none: the app holds the windows, their frames and the group, and passes desktop values in.

### Accessibility

n/a — the panels' controls are named buttons; the OS owns the windows' own accessibility.

### Strings

None: the panels' and the menu's texts are the gallery's own.

### Gallery scenes

- `docked-panels`: the panels the group holds, drawn in the canvas; the live windows are the group.

### Painter primitives

n/a — nothing here draws; the page's panels use `Painter::image_with` (`core.image`) and fills.

### Acceptance

- Pure: `the_rest_of_the_group_orders_below_the_front_window` and `a_minimised_leader_hides_the_detached_members` (`crates/lune-ui-platform/src/group.rs`) check what macOS orders; the routing rule is `only_minimise_and_always_on_top_go_to_the_group`.
- Live: `add` fails on Windows unless the member was built with `member_attributes`, which `gallery --smoke-panels` exercises with the real windows.

### Build

- Milestone: docked windows (0.6.0)
- Depends on: platform.window, platform.desktop
- Done (D47, M11): tests `crates/lune-ui-platform/src/group.rs::the_rest_of_the_group_orders_below_the_front_window, crates/lune-ui-platform/src/group.rs::a_minimised_leader_hides_the_detached_members, apps/gallery/src/panels.rs::only_minimise_and_always_on_top_go_to_the_group`; page `docked-panels`; unverified here: macOS is compile-only here (clippy for `aarch64-apple-darwin`, D308): nothing in this entry has run on a Mac; mixed-scale monitors (a window dragged from a 100 % to a 150 % display) are unchecked; and the feel of a real drag (lag between the pointer and the windows, snapping under a hand) is judged by nobody until a person tries it.

### API

- Build each member with `group.member_attributes(attrs)` (or `owned_by(attrs, &leader)`), then `group.add(window)`. After a drag call `set_docked(id, docked)` for each member.

### Unverified

The live check is `gallery --smoke-panels` (by hand, in a logged-in GUI session; its last reading is in `docs/STATE.md`); the gates run no window. Not run anywhere: `addChildWindow:ordered:` and the ordering of detached members on macOS (AppKit behaviour the toolkit relies on and cannot test here), and minimise and restore on macOS.

## Open questions

- None.

## Notes for the planner

The pieces are generic (no app words); the gallery's `docked-panels` page and `apps/gallery/src/panels.rs` are the worked example of an app wiring them.
