<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-11 (tab header tint fix, version 0.5.1 (D301, D302), tagged `v0.5.1`; frame pacing, version 0.5.0, merged at b767f51 and tagged `v0.5.0`; font smoothing, version 0.4.0, tagged `v0.4.0`; the tab strip's `context` (0.3.0, tagged at 2c0e25c), tab-strip-plus-and-slide, library pass and dark-glass are on main, published at github.com/lune-rs/luneui-rs)

## Now

Tab header tint (D301, D302; version 0.5.1, a look fix, no API change): the tab strip's header tint stops at the outline stroke's outer edge, so it no longer lies under the stroke on the tab's arcs (a light wash brightened each tab corner, a dark one dimmed the arcs). The pixel gate moved only the `tab-strip-*` scenes with a tint. Next: Luneterm moves its pin from `v0.5.0` to `v0.5.1`.

Frame pacing (task `luneui-rs/frame-pacing`, D298 to D300; version 0.5.0, breaking; merged at b767f51, tagged `v0.5.0`): a busy loop (a tab's progress shine, the busy cue, a spinner, an indeterminate bar) reports `At` on the grid of `FrameConfig::busy_cadence` instead of `Now` every frame; `Cadence::BUSY` is 30 a second by default (D1; the feel is the user's by-eye check). A transition, input or a catch-up frame still gives `Now`, and a loop wholly outside its clip asks nothing. The window surface takes `SurfaceOptions` (present mode, frame latency; default `Fifo`, 2, as 0.4.0), and `Repaint::paced` puts a run of `Now` frames on a display-rate grid: on macOS/Metal with `AutoNoVsync`, the main thread did not wait in `acquire` (`gallery --pace-probe`, exit 0; the reading is under Readings). `acquire` drops a suboptimal texture before it reconfigures (D11; fixed by reading, never run). Migration table: `docs/consuming.md`, "0.4.0 to 0.5.0". Entry: `render.surface` (still `in-progress`, as before).

Program toolkit-v1 closed with M11 (merged at 10f503e); the gallery showcase task and the library pass followed. The toolkit is five crates (`lune-ui-core`, the adapters
`lune-ui-wgpu`, `lune-ui-winit`, `lune-ui-platform`, and the facade `lune-ui`), workspace version 0.5.1 (D302; 0.5.0 was D300, 0.4.0 was D297, 0.3.0 was D293, 0.2.0 D238, kept while no consumer had adopted the toolkit). `apps/gallery` has 82 pages and 439 scene names (the first 14 are the legacy Painter-API scenes). The catalogue holds 98 entries: 92 `done`, 2 `in-progress`,
4 `external`. Milestone merges to main: M4 d9fe71f, M5 1aeaf1b, M6 a75a708, M7 662d310, M8 2474248, M9 8f499d0, M10 55cfaaf, M11 10f503e (M1 to M3: the extraction, the seams and the catalogue).
`docs/architecture.md` has the module map, the pages and "API changes since M1"; `docs/consuming.md` how an app vendors the toolkit and, for the library pass, the migration map of its API changes.

Font smoothing (task `luneui-rs/font-smoothing`, D294 to D297; merged at cebd7d7, tagged `v0.4.0`): the toolkit rasterizes glyphs itself and text is smoothed by default, like macOS: no hinting and stem darkening
(Pathfinder's numbers), no blend gamma, the same on every OS. `FrameParams.text: TextRendering` is the per-frame setting (`SMOOTH` default, `OFF` is the 0.3.0 look), `CustomFrame.text` hands the effective value to an app's draws,
and `lune_ui::text::GlyphRasterizer` is the one rasterizer an app's own glyph renderer shares (key its cache by `GlyphParams`, no `DISABLE_HINTING` of its own). glyphon's renderer is vendored into `lune-ui-wgpu` as the private
`glyph` module and `glyphon` is out of the tree (D294); the glyph atlas and the rotated-text rasters are keyed by the setting, so a toggle never serves a stale glyph (D296). The gallery's showcase has a TEXT section
in the global settings, and `gallery --dump` reads `LUNEUI_GALLERY_TEXT` (`on`, `off`). Breaking, hence 0.4.0; migration table in `docs/consuming.md`, "0.3.0 to 0.4.0" (D297). Entry: `text.rendering`.
Next: tell Luneterm the rev (`v0.4.0`, cebd7d7) so it can bump its pin.

Tab context menu (task `luneui-rs/tab-context-menu`, D292, D293; merged, tagged `v0.3.0` at 2c0e25c): no widget builds a context menu (D292); `Ui::tab_strip` reports `TabStripResponse::context: Option<(TabKey, Rect)>` and the app opens its own `Ui::menu`. The gallery's tab-strip page builds an icon menu from `context`. Migration table in `docs/consuming.md`, "0.2.0 to 0.3.0".

Tab strip (task `luneui-rs/tab-strip-plus-and-slide`, D289 to D291; merged at 32d1904): a closable tab reserves its close column, so a hover or an activation moves no tab (D289, a Lunegit deviation); the new-tab button follows the last tab and is pinned at the right end only on overflow, and the viewport is fitted to the tabs, so the strip's empty space reaches the app under it (D290); the active tab's notch slides on a change of the active key (`tab_strip::SLIDE`, D291; scene `tab-strip-switch-mid`, 439 scene names). The pixel gate (`--base main`) moved exactly the `tab-strip-*` scenes (14 by the button's place, `tab-strip-content-swap` also by the slide) and left `tab-strip-overflow` identical; those 15 are the `--allow` list of D290. The feel of the slide is the user's to check with `cargo run -p gallery`.

Dark glass (task `luneui-rs/dark-glass`, D285 to D288; merged at 9f1421f): the OS glass is data (`Material::Custom(OsMaterial)`, `Material::os_material`); a theme file may carry an optional `glass` section (layers, curve, material); `theme::near_black` is the approved near-black look as one file, floating surfaces included (`MENU_GLASS`, D288). The gallery lists it in the materials and themes panes, and its tab header uses `near_black::HEADER_TINT` under that glass. Lunegit's preset is unchanged. The pixel gate was not run (no component's drawing changed, D254): the five `showcase-*` scenes move by design and are checked by eye. Unverified here: `UltraDark` under a Light system appearance on macOS, and every Windows result (the Windows code compiles; nothing ran there). Step 7 added `docs/api/` (D287): the whole public API as generated Markdown, a page per module, from rustdoc JSON by `tools/api-docs`; `scripts/gen-api-docs.sh --check` is in the gate, so it cannot go stale. Start at `docs/api/INDEX.md`.

The library pass (task `luneui-rs/library-pass`, DECISIONS D255 to D282, one milestone by the user's word) did, in order: real stacked glass (a draw layer per overlay, a blur per
glass request, frost, and the window's own background as the only draw that replaces what is below; D255 to D259); `Ui::outside_modals` for a surface that stays outside a modal; one
glass surface for every popup, list, card and toast (D260, D261); an inactive window that stays glass (D262); then the API items (frame values, `Painter::on` that keeps the clip, small
APIs, scoped tab order; D264 to D268), the review's notes R3, R4, R11, R14 to R18 (D269 to D275) and the faults of items 22 to 44 (D276 to D281). Two review rounds ran; the third was held (D25 of the plan).
The reading D6 gave: the window's OS glass, real OS windows and the window's own background on `Base` (with the structural panels that tint it) stay acrylic; everything above `Base` is frost.

Class B (D234): `platform.material` and `render.surface` stay `in-progress` by their own text. The first needs Luneterm's runs of the OS materials
on real machines (`owner: Luneterm M9 to M11 runs`); the second needs a pixel gate on a presented frame (`owner: consumer's window run`).
`catalogue.rs` holds the list and a test that no other entry is `in-progress`.

## The gallery showcase

`cargo run -p gallery` opens the showcase: a 1600 x 900 glass window with a searchable sidebar (groups, pages, scenes), the chosen page live in the
middle, and on the right that page's knobs and the global settings (theme, motion tier, speed, zoom, glass layers, noise strength, luminosity curve, blur and its limits, OS material, durations,
the floating surfaces' tint, opacity and border, text smoothing). Seven labs (glass, blur, effects, motion, tokens, clipboard, JSON) cover what no component page shows. Keys: Cmd or Ctrl `]` and `[` walk the pages,
Cmd or Ctrl `Q` quits. `--list`, `--dump <dir>` and `--bench` are unchanged; the shell's own scenes are the five `showcase-*`. `docs/components/infra/showcase.md`
is the entry (what it shows, the four pages with no knob and why, what is unverified); decisions D246 to D254; the code is `apps/gallery/src/showcase/`.
The showcase task changed nothing under `crates/` (D246); the library follow-ups it found were paid by the library pass.

Gallery work is checked by eye, not by the pixel gate: the gate runs when a component's design changes (D254). A hands-on run in a real window is what
proves clicks, the OS material swap and the feel of speed and zoom; the tests only drive the shell with scripted pointer events.

## Readings (each run once; nothing acts on them)

- Library pass, the gate at 435a7ca (the branch tip when review round 2 ran): fmt, clippy (also `-p lune-ui-winit`), boundaries, index, `cargo doc -D warnings`, `LUNEUI_REQUIRE_GPU=1 cargo test -p lune-ui-wgpu`,
  `scripts/check-targets.sh`: all exit 0. The test totals of `cargo test --workspace` are the orchestrator's final gate's; step 26 did not re-run them (docs only).
- Library pass, the pixel gate: `scripts/pixel-gate.sh --base 04dd5c9` at 435a7ca: 438 files identical, 0 differ. Against the pass's base, `--base 1340d15` with the nine `--removed` ids and the allow list of D282:
  the result is the orchestrator's final gate's (`scripts/pixel-gate.sh` header and regex now accept a scene that exists only at a fractional scale, as `ui-controls@1.5x`; step 26's own run was not finished when this was written).
- Look values settled by the user after a live run (step 25, Q2): `menu_popup_border` #FFFFFF1F (R8), `menu_glass` alpha 157 (R9), the tab shine 1800 ms and a 48 px band (R3), `tree_guide` #FFFFFF14 (R15). Then, from the same run, D283: a tree guide drops from the centre of its parent's mark, and a leaf's stub reaches its label (`GuideStub::Label`, default; `Short` kept as an option).
  A pick changes one constant or token and re-runs the gate once with the same allow lists.
- Font smoothing, G0 (the vendored renderer, zero delta): `scripts/pixel-gate.sh --base 2c0e25c` at a8f75cf (step 1): 439 of 439 scenes identical. G1 at ed23193 (`LUNEUI_GALLERY_TEXT=off`, `--base 2c0e25c`): 434 of 439 scenes identical; the five `showcase-*` scenes are
  allowed (D254) and also read 5 of 5 identical. G2 at ed23193 (the default smooth dump against G1's base, a record): 434 files, 0 identical, 434 differ, max delta 195 (every scene draws text).
- Frame pacing, `cargo run --release -p gallery -- --pace-probe` at 9250138 (macOS/Metal, Apple M1 Pro, one 60 Hz display, the window left untouched): exit 0 on the first run, no re-run. Verbatim:
  `reading: pace-probe 9250138 60 Hz load1=5.20 exit 0`;
  `pace-probe: phase 1 0.4.0: scheduled 60.0/s (180 frames), os 0, acquire p50 16.21 p95 16.38 p99 16.87 max 16.88 ms, busy 1000 ms/s, interval p50 16.68 p95 16.86 ms, skip 0 redraw 0`;
  `pace-probe: phase 2 cadence: scheduled 30.0/s (90 frames), os 0, acquire p50 0.04 p95 0.06 p99 0.07 max 0.07 ms, busy 14 ms/s, interval p50 33.35 p95 34.70 ms, skip 0 redraw 0`;
  `pace-probe: phase 3 paced-fifo: scheduled 60.0/s (180 frames), os 0, acquire p50 0.03 p95 0.05 p99 0.07 max 0.08 ms, busy 24 ms/s, interval p50 16.67 p95 18.48 ms, skip 0 redraw 0`;
  `pace-probe: phase 4 paced-novsync: scheduled 60.0/s (180 frames), os 0, acquire p50 0.03 p95 0.04 p99 0.05 max 0.06 ms, busy 28 ms/s, interval p50 16.67 p95 18.28 ms, skip 0 redraw 0`;
  `pace-probe: verdict: held: phase 2 30.0/s with acquire p95 within 2 ms; phase 4 60.0/s of a 60 Hz display with acquire p95 within 2 ms; display 60 Hz; load1=5.20`.
  Phase 4's `AutoNoVsync` resolved to `Immediate` on Metal (the planner's listing; the probe does not print it). The planner's probes at 0b46176 (load 6 to 8) are the figures in D298 and D299.
- Release size: `cargo build --release -p gallery`, `wc -c target/release/gallery` (macOS, strip, fat LTO): 10,118,576 bytes at 2c0e25c, 10,118,544 after step 1 (the vendored renderer), 10,118,544 after step 3
  (the rasterizer; nothing called it yet), 10,267,264 at ed23193 (steps 4 to 6: the rasterizer in use). Before that: 9,869,520 at a9d1292 (M11), 9,869,408 at the M10 tip and 9,448,688 at the M9 merge. Also in `docs/consuming.md`.
- Bench (M11, `cargo run --release -p gallery -- --bench`, per D85) at d14069f, one reading, 2080 x 1320, 5 runs, M1 Pro, no GUI session: 0 of 74 pages over budget at load
  2.0, self-test in range with a spread of +1.13/+2.81. Highest p99 5.99 ms and worst frame 13.03 ms, both `code-editor-view`; `flyout` p99 4.85. Nothing waits on a bench reading (D235);
  the first reading of stacked blur in the pages (about 0.25 ms a level at 3456 x 2234 in the plan's probe, D257) is the final gate's, not recorded here.
- Platform smoke (`cargo run -p gallery -- --smoke-platform`) at a9d1292: not run, "no monitor: there is no GUI session". The last runs, on a logged-in session: M10 at
  b8faaf1 (exit 0, a real tray item, its rect, a synthetic click, the surface in its window, the frontmost app unchanged, the panel key, teardown) and M11 step 2
  at 15c471e (exit 0, plus the surface resized (2080, 1320) to (2144, 1352) and back, acquire answered skip because the window is hidden).

## Carried limits

Each is a recorded deviation or a choice with a DECISIONS row or an entry, and an API change no consumer has asked for (D237, D245). Paid in the library pass and gone from this list: D137's layer
limits and D138's single blur, an overlay in a disabled scope, the ring width under a transform, `TAB_HEADER_HEIGHT` and `BODY_PADDING`, the cluster and drag hit faults, the table and list
faults (a sort, a wrapping `num` cell, the header ring, a focused row out of view), the editor's stale heights, word motions, Home and the page keys, the one-line block's thumb, split panes' size, the
fit panel's clip, the nits F2 to F7 of M6 and M8, and the Library follow-ups the showcase found.

- Clips and text: text cannot be cut by a rounded clip (the bounding rect; the page and panel inset their text by the radius, `panel::TEXT_INSET` is the caller's, D148). Text draws above every
  fill of its layer, so labels show through a sliding tab.
- Glass (D255 to D263): the structural panels (`Ui::page`, toolbar, side panel, settings page, board) are acrylic only on `Base`; one an app draws above it, in an overlay or inside
  `Ui::outside_modals`, is frost over what is under it (D6). The live shell's panes draw no acrylic. The panes of the gallery stay undimmed under a modal, as the user accepted (D259);
  `Ui::outside_modals` is not recorded, so it belongs at a shell's top level, not inside `Ui::record`. A hit registered straight through `Painter::hit` (not `interact`) has Tab order 0 (D267).
  What the OS draws for Windows acrylic and for vibrancy that follows the window when the window is inactive is unverified here, and the app draws no fallback now (D262).
- Fonts: the gallery bundles no CJK face, so the IME scenes' "かな" draws as boxes (D90). No bold face is bundled, so bold draws exactly as medium (`text.shaping`). No italic face, so italic
  is the shaper's skew (D189).
- Audit ruling Q25 stands; Q23 (the strip closing its own menu when disabled) is gone with the strip's menu (D293).
- `menu_separator` is `#FFFFFF22`, not Lunegit's `#18FFFFFF` (D244). `menu_popup_border` is `#FFFFFF1F`, a deliberate deviation from Lunegit's `PopupMenuOuterBorder` (D260).
- API gaps, marked in the entries: the table has no column resize or reorder (D163, Q15, deferred); `StrokeStyle` has no cap mode, so the lane graph's lines end butt (D172); the access tree has no
  keyboard-move action for the board (D55, D169); the callout has no title line or inline figure and the empty state's actions have no glyph (D136); the sidebar footer is one line in one family;
  a flyout route has no state note; toast history rows have no hover or expand; `Ui::filter_bar` has no option for the palette's glyph, text and radius;
  `PaletteRow` has no icon tone; the side panel's show moves the card only with `SidePanelOptions::card_moves` (D281); the toolkit has no lane assigner, the app owns the lane layout (a hand-built
  graph shows the widget in `lane-graph-advanced`); a theme file cannot set `tree_guide`, `selection_radius` or `selection_pad`.
- `role-gap`: tab to panel association (D122), multi-selectable (D100), composition range, the editor's full-text query (D217).
- Tree view: `cells` is not called on rows of a moving block; rows have no ShowMenu action; no folder glyph; `CHEVRON_GAP` and `GLYPH_GAP` have no source (accepted).
- M8: the table has no horizontal scroll; `page` and `settings_page` nested in another area keep a delta even when they fit (D176); bars are keyed by column index, so a rolling window morphs in place;
  the board moves one card a frame by Shift and arrow and takes no landing in a disabled scope. The focus a list keeps for a row out of view starts one frame after the focus enters the row, and a list
  without `index_of` whose focused row changed place while out of view loses it (D278).
- M9: a very long single line costs in proportion to its length (D194); a markdown nested block is built whole and only blocks in view are Tab stops; no folding and no in-editor search (D181);
  markdown underline, strike and pill are fills from row boxes (D188); a pane pulled in the frame of its own wheel publishes nothing; in an editable wrapped view PageUp after Cmd+End can rest
  one row off (within a line, D279). The diff view's selection is clipped at the pane edge, which hides its rounded corners at the edge (D272).
- M10: the text field and text area refuse a paste or set-value over 1 MiB whole (D232, D233), the code editor sets no limit; assistive clients get the lines in view of an editor (D217);
  `Dismiss` is exposed only on a tooltip, `Dragging` and a mixed toggle are not sent (D217); a locale file written for a newer toolkit fails on an older one (D210); the Windows popover activates
  (D224, Q17); a surface is placed from the tray item's rect, which on Windows may sit a few px inside the taskbar (D219); the macOS rect settles only after 300 ms (D225); the tray icon has no
  automatic inversion while the item is highlighted (D218); no Windows row scene; no drop-target widget; `--parity-only` of `scripts/pixel-gate-luneterm.sh` fails by design (the lock has grown).
- The real pointer feel of the tab drag, slider, splitter, tree and board in window mode is not testable with scripted input.
- Review nits left open (library pass, round 2): the doc at `caption.rs:56` (F7), `button::paint` keeps an unused `_kind` argument (F8). The tab shine draws up to 48 clipped text items per shining title
  per frame; the implementer said buffers are cached by colour, and nothing measured it (D270).

## Unverified here

`done` is a statement about scripted input and the code that runs here. What only a real OS, hardware or person shows is in each entry's `unverified here` and is not ticked:

- The two Class B entries: what the OS composites for each material on real machines (Windows acrylic and Mica, macOS vibrancy and liquid glass), `platform.material`; a pixel gate on a presented
  frame, `render.surface`.
- Real typing with an input method (roadmap R1), bidi caret placement, screen readers and an assistive client of the AccessKit bridge (its tree is replayed into `accesskit_consumer`, no client ran).
- Windows and Linux run time (the lint runs through `scripts/check-targets.sh`, Linux only with Docker): nothing of the tray, the popover, the clipboard or the bridge has run on Windows; Linux has no tray.
- The tray beyond what the smoke measured on this Mac: a real click on the item, real hover and keys in the panel, the OS shadow and corners, a menu bar on a second display, the highlighted item.
  The panel rests on AppKit behaviour Apple does not document and is proved only on this macOS version (D223); run the smoke again after a macOS update.
- The OS clipboard beyond the one guarded `real_round_trip` (macOS, plain text; `#[ignore]`, writes the user's clipboard, not re-run in M11); a real file dragged over a window.
- A real GPU loss (a driver reset, an unplugged adapter): the tests stand in with `Device::destroy`.
- Font smoothing (D296): the look at 1x on a real 100%-scaled Windows display (nothing ran on Windows; the user's), colour glyphs through the rasterizer (no colour font is bundled, so the pass-through
  is untested on real data), and a hands-on toggle of the TEXT section in `cargo run -p gallery`. Unhinted text at 1x may read softer than ClearType; an app can pass `hinting: true` per window.
- The showcase in a real window: clicks on the sidebar, the knobs and the settings, a live swap of the OS material, the feel of speed and zoom, Copy and Paste against the OS clipboard (`docs/components/infra/showcase.md`).
- The library pass in a real window (the user's hands-on run of `cargo run -p gallery`): a dialog over a page, a list over the dialog, a tooltip and a toast; a click on the sidebar between dialog scenes (R7's path);
  an inactive window (R13); a tab in progress (R3, the feel of the shine); the glass sliders. OS glass and an inactive window are live-only.
- Frame pacing (D298 to D300): the user's by-eye check of the 30 a second loops (the shine, the busy cue, the spinner, the indeterminate bar; D1, one constant `Cadence::BUSY`); `paced` and `AutoNoVsync` on DX12, Vulkan and GL (unmeasured); tearing under `AutoNoVsync` on any platform (unchecked); `acquire`'s suboptimal path (D11: fixed by reading, never run; Metal never reports suboptimal, Vulkan does).
- Dark glass (D285, D286): the near-black look on a real macOS window in Light appearance (the fix if it fails is `appearance` in `near_black::MATERIAL`, one constant); the Windows acrylic tint and dark flag on builds 22523 and later; the tab header under the near-black glass (`near_black::HEADER_TINT`, a proposal for the user's eye).

## Next

1. Frame pacing: after the merge, tell Luneterm the rev so it drops its `IDLE_ANIMATION_HZ` stopgap and maps `Repaint` through `paced` (D298, D299); the by-eye check in Unverified here (the user's). Font smoothing: the by-eye checks in Unverified here (the user's), and `v0.4.0` after the merge with the user's yes. Dark glass: the by-eye checks in Unverified here. After any public API or doc-comment change run `scripts/gen-api-docs.sh` (the gate's `--check` fails on a stale `docs/api/`).
2. The rest of a hands-on run of `cargo run -p gallery` (see Unverified here); the look values are settled.
3. Luneterm's M9 adopts the toolkit through `docs/consuming.md`: vendor `scripts/export-snapshot.sh`'s output, switch crate by crate (version 0.2.0 and later let the in-tree `lune-ui` and the snapshot stand
   in one lock), use the migration maps (the Luneterm one, "API changes of the library pass", "0.2.0 to 0.3.0" and "0.3.0 to 0.4.0"); its own OS runs close `platform.material` and `render.surface`. Luneterm itself is not changed by this repo.
4. The Lunegit fixtures are licensed MIT OR Apache-2.0 (D284); the export still leaves them out.
5. The user's calls, all reversible and taken at the recommended option: the final tray glyph (`platform.tray-icon`), column resize and reorder in the table (D163, Q15), and two taken calls to
   confirm: the Windows popover activates (D224, Q17), the tray icon is a CPU raster (D218, Q18).
6. CI stays out (D15) until the user asks; the remote is `origin`, github.com/lune-rs/luneui-rs (public).

## Blockers / open questions

- None.
- Wane's OS behaviours (close-to-tray, single instance, autostart, notifications) are assumed app-side and documented (answer 2026-11-03, reversible).
