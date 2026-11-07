---
id: platform.window
title: Glass window helpers
area: platform
status: done
consumers: luneterm, wane
source: crates/lune-ui-core/src/window.rs, crates/lune-ui-core/src/widgets/caption.rs, crates/lune-ui-winit/src/window.rs
owner: —
summary: Glass window attributes, edge resize, double-click detection and caption buttons
---

## What it is

Core (`lune-ui-core`): `EDGE_BORDER`, `ResizeEdge`, `edge_at`, `DoubleClick` and `caption_buttons`, with no windowing type. winit adapter (`lune-ui-winit`): `glass_attributes` (per-platform window attributes from `platform/`) and `resize_direction` (`ResizeEdge` to winit's `ResizeDirection`).

## Needs

- Custom caption, edge resize and double-click handling (R-platform-4).
- Wane — Variants: none (Windows chrome, D9). Parts: brand (wordmark + version), fill (drag area), caption buttons (minimize, maximize, close). (R-platform-4)
- Wane — States: active; inactive (`is-inactive`: solid near-black `--glass-inactive` #0b0b0b, page card `--glass-inactive-card` #141414, no blur, caption glyphs quaternary; not ported: an inactive window stays glass, R13, D262; only the caption's glyphs change); caption hover (`--surface-hover`); close hover (`--caption-close-bg` #c42b1c, white glyph). (R-platform-4)
- Wane — Sizes: Design size 1280 x 800 (`--window-w`, `--window-h`); radius 10 (`--radius-page`); 1 px `--border-subtle`; title strip 32 (`--titlebar-h`) with a 208 wide brand cell (`--sidebar-w`), padding 0 16 0 18, wordmark 14 bold gold, version RznMono 11 tertiary; caption buttons 46 x 32 (`--caption-w`); content grid 208 | 1fr, 10 px gutter (`--gutter`). (R-platform-4)
- Wane — Behaviour: Drag by the title strip, double-click maximizes, edge resize, minimize / maximize / close. The window layer does not swap to a solid colour when it loses focus (`overview-unfocused`): the glass stays (R13, D262). Mouse and keyboard (Alt+Space system menu) come from the OS window. (R-platform-4)

## Existing code

`crates/lune-ui-core/src/window.rs` (`edge_at`, `DoubleClick`, the Painter-form `caption_buttons` and the shared `CAPTION_BUTTON_WIDTH`, `CAPTION_GLYPH_SIZE`, `caption_rect`); since M11 `Ui::caption_buttons` over `CaptionButton` and `CaptionOptions` in `crates/lune-ui-core/src/widgets/caption.rs` (D240: the Painter form stays, `unfocused` and `chrome` draw through it at zero delta); `crates/lune-ui-winit/src/window.rs:12` (`glass_attributes`), `:28` (`resize_direction`). Integration test `crates/lune-ui-core/tests/window.rs` (edges, 6 px border, double-click gap under 400 ms); `crates/lune-ui-winit/tests/input.rs` covers the direction mapping.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b, Luneterm 296cfdc. Read: this repo crates/lune-ui-core/src/window.rs:1-99, crates/lune-ui-core/tests/window.rs:1-60, crates/lune-ui-winit/src/window.rs:1-39, crates/lune-ui-winit/src/platform/macos.rs:1-9, platform/windows.rs:1-13, platform/other.rs:1-5,
crates/lune-ui-core/src/input.rs:61-62,166-181,183-223, crates/lune-ui-core/src/theme.rs:78-81,111-113; Lunegit src/Lunegit.Desktop/Theming/ThemeApplicator.cs:284-301, Views/DiffView.axaml:73-83; Wane docs/ui/components.md:193-205,714, docs/ui/mocks/css/components.css:30-50; Luneterm docs/glass-styling.md:7-28.
Conflicts: (1) caption glyph colour at rest: the moved code draws `fg_primary` [R crates/lune-ui-core/src/window.rs:89], Wane `--text-secondary` [W docs/ui/mocks/css/components.css:43]; the moved code wins (pixel-gated).
(2) inactive fill: gone (R13, D262). Lunegit gave a solid `#333333`, the tint lightened 20 percent [L src/Lunegit.Desktop/Theming/ThemeApplicator.cs:284-301], [LT docs/glass-styling.md:24-28], and Wane asked for near-black `#0b0b0b` [W docs/ui/components.md:197]; the user's note R13 says an inactive window stays glass, so no widget has a fill that depends on the window's state.
The preset keeps `#333333` (a pixel-gated gallery value); Wane's look is a theme value that overrides it (`infra.theme-hot-reload`). A user decision is recorded in the Wane entry, not overruled here.
Caption sizes agree: 46 x 32 [R crates/lune-ui-core/src/window.rs:87], [W docs/ui/components.md:198]; the close fill `#c42b1c` agrees [R crates/lune-ui-core/src/theme.rs:80], [W docs/ui/components.md:197].

### Anatomy

- Resize border: 6 logical px around a borderless window; `edge_at(point, w, h)` returns the corner first (north-west, north-east, south-west, south-east), else the side (west, east, north, south), else none; the right and bottom bounds are `>= w - 6` and `>= h - 6` [R crates/lune-ui-core/src/window.rs:9-41].
- Double click: two clicks closer than 400 ms (strictly less) are a double click; the caller passes the click time [R crates/lune-ui-core/src/window.rs:43-65], [R crates/lune-ui-core/tests/window.rs:40-47].
- Caption buttons: three 46 px wide buttons along the top right of the window rect, each as tall as the caller says (Wane 32), minimize, maximize (restore while maximized) and close, with 10 px Codicon glyphs in `fg_primary`; hover fills `surface_hover`, and the close button fills `caption_close_hover` with a white glyph [R crates/lune-ui-core/src/window.rs:68-98], [R crates/lune-ui-core/src/theme.rs:80-81], [W docs/ui/components.md:197-198].
- Window attributes (winit adapter): a transparent window with the given logical size and minimum size and a visible flag; macOS: transparent full-size title bar with the title hidden; Windows: no decorations, no redirection bitmap, undecorated shadow, round corners, dark theme; other platforms: unchanged [R crates/lune-ui-winit/src/window.rs:10-25], [R crates/lune-ui-winit/src/platform/macos.rs:4-9], [R crates/lune-ui-winit/src/platform/windows.rs:4-13], [R crates/lune-ui-winit/src/platform/other.rs:3-5].
- Resize direction: each `ResizeEdge` maps to winit's `ResizeDirection` [R crates/lune-ui-winit/src/window.rs:27-39].
- Window commands the app performs, in order: drag move, drag resize from an edge, minimize, toggle maximize, close, redraw [R crates/lune-ui-core/src/input.rs:166-181]. The helpers do not call them; the app wires hits and edges to commands.
- Title strip, brand cell and wordmark: app composition; the strip is 32 px high with a 208 px brand cell, padding 0 16 0 18 [W docs/ui/components.md:198,202]; Wane's own status line says the title strip and the wordmark are app composition. Window radius 10 and the 1 px `--border-subtle` are the page card's look [W docs/ui/components.md:198].
- Active and inactive: Lunegit swaps the acrylic layers for a solid fill while the window is inactive, with `IsVisible` bound to the window's active state, a hard cut [L src/Lunegit.Desktop/Views/DiffView.axaml:73-83], [LT docs/glass-styling.md:24-28]; the toolkit does not (R13, D262). The caption's own `active` option dims its glyphs. The core delivers the focus change as `Event::Focus` [R crates/lune-ui-core/src/input.rs:61-62].
- Backdrop: the OS material behind the window is `platform.material`; the in-app blur is `render.blur` [W docs/ui/components.md:204].
- macOS traffic lights: the mocks do not draw them [W docs/ui/components.md:204]; with a transparent full-size title bar the OS keeps them, so the app's title strip must leave room (Open questions).

### States

| state | look | cite |
|---|---|---|
| caption button rest | transparent, glyph `fg_primary` | [R crates/lune-ui-core/src/window.rs:89,96] |
| caption hover (minimize, maximize) | fill `surface_hover` | [R crates/lune-ui-core/src/window.rs:93-95], [W docs/ui/mocks/css/components.css:47] |
| close hover | fill `caption_close_hover` (`#C42B1C`), glyph white | [R crates/lune-ui-core/src/window.rs:90-92], [R crates/lune-ui-core/src/theme.rs:80-81] |
| maximized | the middle glyph is "restore" | [R crates/lune-ui-core/src/window.rs:79-84] |
| pressed | as hover | [derived: the moved code draws no pressed look] |
| focus-visible | none: the caption buttons are pointer targets; the keyboard path is the OS (Alt+Space system menu) | [W docs/ui/components.md:199] |
| disabled | not applicable to the helpers | [derived: no source] |
| window active | acrylic layers shown | [LT docs/glass-styling.md:7-14] |
| window inactive | the layers stay; caption glyphs quaternary (Wane) | [D262], [W docs/ui/mocks/css/components.css:49] |
| edge hover | the matching resize cursor | [R crates/lune-ui-core/src/input.rs:204-216] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| caption hover in and out | button fill | hover | instant | linear | keep | [W docs/ui/mocks/css/components.css:45], [L .claude/docs/animation-rules.md:102] |

The Wane mock eases with `--ease-out`; the animation rules give Linear for hover and colour, which wins (Lunegit wins over Wane, PD6) [L .claude/docs/animation-rules.md:85]. The active and inactive swap is a hard cut in Lunegit [L src/Lunegit.Desktop/Views/DiffView.axaml:73-83]; it stays a snap. Never animated: window size, position, edge resizing.

### Input and focus

- Pointer: a press on an edge (from `edge_at`) starts `DragResize(edge)` and the cursor over it is the matching resize icon; a press on the title strip's free area starts `DragMove`; a double click there toggles maximize; a click on a caption button runs its command [R crates/lune-ui-core/src/window.rs:26-41,43-65], [R crates/lune-ui-core/src/input.rs:166-181], [W docs/ui/components.md:199].
- Keys: none of the toolkit's; Alt+Space (the system menu) and the OS window shortcuts come from the OS window [W docs/ui/components.md:199]. Tab stops: the caption buttons are not Tab stops [derived: no source].
- Layer: the window chrome sits on `Page` above the window glass [R crates/lune-ui-core/src/scene.rs:17-22]. Windows snap layouts (the hover menu on the maximize button) need OS hit-testing the helpers do not provide [W docs/ui/components.md:714], (Open questions).

### Retained state

- Transient: caption hover, the double-click time of the last click.
- Persistent: none. App-owned: maximized flag, window active flag, the resize and maximize decisions.
- Exit retention: no.

### Accessibility

- The three caption buttons are role `button` with names `luneui.window.minimize`, `luneui.window.maximize` (`luneui.window.restore` while maximized) and `luneui.window.close`; action default runs the command [infra.accessibility]. On Windows the OS also exposes the window's own system buttons when it draws them; here the app draws them [W docs/ui/components.md:714].

### Strings

- `luneui.window.minimize`: "Minimize", `luneui.window.maximize`: "Maximize", `luneui.window.restore`: "Restore", `luneui.window.close`: "Close" [derived: Windows caption button names; no Lunegit key; Lunegit draws no caption buttons of its own in the files read].

### Gallery scenes

- `chrome` (existing): the tab-outline chrome in a window.
- `glass-off` (existing): glass turned off.
- Page `window` (M11) holds these seven. `window-caption`: the three caption buttons, rest, settled; `window-caption-maximized` with the restore glyph; `window-caption-hover-min` and `window-caption-hover-close` with the pointer over the button, settled.
- `window-caption-hover-fade`: the pointer entering a caption button, t = 60 ms (half of `instant`).
- `window-edges`: the 6 px resize border outlined with its eight regions, settled.
- `window-caption-inactive`: the caption of a window that is not the active one (glyphs in `fg_quaternary`), settled; the glass behind it is the same.

### Painter primitives

- Existing: fill, fill-radii, text (glyphs), icon, ring, clip-rect.
- New: none.

### Acceptance

- M11 step 3 (`crates/lune-ui-core/tests/widget_caption.rs`): `at_rest_and_active_the_scene_is_the_painter_forms`, `a_settled_hover_is_the_painter_forms_hover`, `a_pointer_over_close_fills_caption_close_hover`, `a_click_returns_the_button_once`, `a_press_takes_no_focus_and_tab_finds_no_stop`, `maximized_draws_the_restore_glyph`, `each_button_is_92_device_px_wide_at_scale_2`, `the_hover_fill_passes_motion_conformance_in_every_tier`, `the_fill_fades_in_over_instant_and_off_snaps`, `an_inactive_window_draws_its_glyphs_in_fg_quaternary`, `the_nodes_are_buttons_with_their_names_and_the_default_action_clicks`, `the_names_come_from_the_string_table_and_default_to_english`, `a_disabled_scope_leaves_no_violation_and_no_click`; the gallery page `window` holds the seven scenes below.
- Existing tests stay green: `corners`, `edges`, `border_is_six_pixels`, `interior_is_none`, `double_click_gap_is_strictly_under_400ms` (`crates/lune-ui-core/tests/window.rs`); the direction mapping test in `crates/lune-ui-winit/tests/input.rs`.
- Edge known answers (window 400 x 300): (5.9, 100) is west and (6.0, 100) none; (0, 0) north-west; (399, 299) south-east [R crates/lune-ui-core/tests/window.rs:11-37].
- Caption: a scripted pointer over the close button hovers it and fills `caption_close_hover`; a click returns the close hit; `maximized` draws the restore glyph; at scale 2 each button is 92 device px wide.
- Behaviour: a press on an edge reports the resize edge; a double click within 399 ms toggles maximize and 400 ms does not (`crates/lune-ui-core/tests/window.rs::edges`, `double_click_gap_is_strictly_under_400ms`, and for the `Ui` form `ui_core.rs::double_click_is_strictly_under_400_ms`; `crates/lune-ui-winit/tests/input.rs::resize_edges_map_to_the_same_named_direction`). The toolkit reports the edge and the double click; the app turns them into a `WindowCommand`.
- Conformance: the hover fill passes `motion::conformance`; Reduced keeps it, Off snaps [motion].
- Semantic node: the three buttons exist with the names above and `default` runs the command.
- Real OS behaviour (transparent windows, round corners, snap layouts, traffic lights) is unverified here: no VM or GUI run (D15); Luneterm's M9 and M10 runs cover it. The scenes and the semantic node are built (M11 step 3) and the M11 audit passed them (the `Done` line below).

### Build

- Milestone: M11 (done standard; code exists)
- Depends on: core.painter, core.input, core.theme, platform.input, infra.accessibility, motion.transitions
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_caption.rs::at_rest_and_active_the_scene_is_the_painter_forms, crates/lune-ui-core/tests/widget_caption.rs::a_pointer_over_close_fills_caption_close_hover, crates/lune-ui-core/tests/widget_caption.rs::a_click_returns_the_button_once, crates/lune-ui-core/tests/widget_caption.rs::maximized_draws_the_restore_glyph, crates/lune-ui-core/tests/widget_caption.rs::each_button_is_92_device_px_wide_at_scale_2, crates/lune-ui-core/tests/widget_caption.rs::the_hover_fill_passes_motion_conformance_in_every_tier, crates/lune-ui-core/tests/widget_caption.rs::the_nodes_are_buttons_with_their_names_and_the_default_action_clicks, crates/lune-ui-core/tests/widget_caption.rs::a_disabled_scope_leaves_no_violation_and_no_click, crates/lune-ui-core/tests/window.rs::edges, crates/lune-ui-core/tests/window.rs::double_click_gap_is_strictly_under_400ms, crates/lune-ui-winit/tests/input.rs::resize_edges_map_to_the_same_named_direction`; page `window` (seven scenes); unverified here: real OS windows (transparent windows, round corners, snap layouts, traffic lights); Luneterm's M9 and M10 runs cover them (D15).

## Open questions

- None on the seam: the core half has no winit type; the winit mapping is in `lune-ui-winit` (M2 step 8). `edge_at` returns `ResizeEdge` (was winit's `ResizeDirection`).
- Answered (M4, `core.ui`): the `Ui` detects double clicks from the frame's own time (`Response::double_clicked`, `ui_core.rs::double_click_is_strictly_under_400_ms`), so scripted tests need no real clock. `DoubleClick` with an `Instant` stays for an app that has no `Ui`.
- Still open, a limit: Windows 11 snap layouts on the maximize button need non-client hit-testing from the OS window procedure [W docs/ui/components.md:714]; no helper exists; unverified here (an OS window-procedure hook, an API no consumer has asked for, D237).
- Still open: space for the macOS traffic lights in the app's title strip: no source gives the inset, and the toolkit draws none.
- A loaded theme's `WindowFallbackBackground` is an unknown key (it set `Theme::window_fallback` in M10, D207; the token went with the inactive look, D262), listed in `skipped`.

## Notes for the planner

Moved in the behaviour-preserving extraction (R-ext-1); split across core and winit in M2 step 8.

Wane (2026-11-04): lune-ui status for Wane's use: partial: caption buttons (core window.rs:70) and glass attributes (lune-ui-winit window.rs:12) exist; the title strip and the wordmark are app composition; Wane priority P0. Native note: `backdrop-filter: blur(40px) saturate(1.6)` over a CSS backdrop approximates the window material. luneui-rs uses the real OS material (`platform.material`: acrylic or Mica on Windows, vibrancy or liquid glass on macOS) and the in-app blur pass (`render.blur`). The flattened colours (`--glass-window` rgba 54 54 54 .81) are measured from lune-ui dumps: reuse lune-ui's WINDOW layer, not these numbers. The mocks do not draw macOS traffic lights. Full spec: Wane `docs/ui/components.md`, section `platform.window`; mock: Wane `docs/ui/mocks/components.html`.
