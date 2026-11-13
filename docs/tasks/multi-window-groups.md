# Task: docked multi-window groups (done, 0.6.0)

Status: done. Shipped as version 0.6.0 by the task `luneui-rs/multi-window-groups`: D304 to D306 (image parts and sampling), D307 (`ImageStore::update`), D308 (macOS compile check), D309 to D311 (desktop units, docking, popup placement), D312 (window commands, `panel_attributes`), D313 and D314 (the app-run drag, `windows-sys`), D315 (window shapes), D316 (window groups), D317 and D318 (the gallery page and its smoke), D319 (the release). Catalogue: `platform.desktop`, `platform.window-docking`, `platform.window-group`, `platform.window-shape`, `overlay.menu-window`. The text below is the original brief, kept as written.

Optional toolkit support, discussed 2026-11-12. **Nothing is waiting on this.** It comes from a
possible full Rust rewrite of whipspot (`~/Projects/archived/whipspot`, a Winamp-style player), but
the toolkit stays app-agnostic: no skin, theme-file or Winamp concept enters luneui-rs. The app
parses its skins and draws its panels as its own widgets (`Painter`, images, hit nodes,
`CustomDraw`); the toolkit only supplies the generic pieces below.

Goal: several long-lived, borderless, transparent windows that dock together like separate panels,
so the space between undocked panels is the desktop and clicks there reach whatever is behind.
Today only the tray popover exercises a second window.

## Pieces (all generic)

1. **Image source rect**: draw part of a registered picture (sprite sheets, atlases), for example
   `Painter::image_part`. Without it an app cuts each sprite into its own `ImageId`.
2. **Window group**: attach and detach windows, move them together, raise them together, show one
   Dock or taskbar entry, minimise and close as a group. macOS: native child windows
   (`addChildWindow:ordered:`, `unsafe` in `lune-ui-platform`), which also stops panels drifting
   apart mid-drag. Windows: owned windows; the app moves each one itself.
3. **Edge snapping maths** in `lune-ui-core`: snap to screen work areas and other windows' edges,
   work out which windows are docked. Decide whether it works in logical or physical pixels
   (mixed-scale monitors).
4. **Screen-space input for an app-run drag** (`DragMove` hands the move to the OS, so nothing can
   snap): window position on screen, pointer in screen coordinates while the button is held, the
   monitors and their work areas, and a way to set a window's position (a `WindowCommand` variant,
   or a winit helper). Unverified: whether winit keeps reporting the pointer once it leaves the
   window mid-drag (Windows may need mouse capture).
5. **A menu or popover in its own window, anchored to a point on screen**: generalise the tray's
   `Ui::menu_window` plus `popover_attributes` and `place_surface`, so a context menu can extend
   past a small window.
6. Optional, unverified: **click-through on transparent pixels** of a shaped window. Check first
   whether macOS already passes clicks through for a non-opaque wgpu/Metal window; if not, use a hit
   region in platform code.

## Compatibility

All of it can be additive. `WindowCommand` is `#[non_exhaustive]`. `ImageItem`, `RawInput` and
`FrameOutput` are not, so a new field breaks struct literals (custom renderers, tests): ship as a
minor bump (0.x) with a `docs/consuming.md` migration note, or mark them `#[non_exhaustive]` in the
same release. A picture drawn without a source rect draws as now, so the pixel gate stays at zero
delta. Apps that vendor a snapshot (D239) see nothing until they pull it.

## Done when

A gallery page opens three docked panels as separate windows: they snap and move as one, detach
and leave real desktop between them, raise together, show one Dock or taskbar entry, and a context
menu from the smallest panel shows in full. Each piece gets a DECISIONS.md row and a catalogue file.
