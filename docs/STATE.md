<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-07 (gallery showcase task closed)

## Now

Program toolkit-v1 is closed: M11, the last milestone, merged at 10f503e. The toolkit is five crates (`lune-ui-core`, the adapters
`lune-ui-wgpu`, `lune-ui-winit`, `lune-ui-platform`, and the facade `lune-ui`), workspace version 0.2.0 (D238). `apps/gallery` had 74 pages and
431 scene names at M11's close; the showcase task added seven labs and the shell page (82 pages, 436 scene names). The catalogue holds 98 entries: 92 `done`, 2 `in-progress`, 4 `external`. Milestone merges to main: M4 d9fe71f,
M5 1aeaf1b, M6 a75a708, M7 662d310, M8 2474248, M9 8f499d0, M10 55cfaaf, M11 10f503e (M1 to M3: the extraction, the seams and the catalogue).
`docs/architecture.md` has the module map, the pages and "API changes since M1"; `docs/consuming.md` how an app vendors the toolkit.

M11 (DECISIONS D234 to D245) did, in order: the catalogue's still-needs tests; `Gpu::is_lost` and a surface stage in the platform smoke; the caption
buttons through the `Ui` (`Ui::caption_buttons`, the page `window`); the key readers that leaked when disabled or took a chord, and F11; the theme
reach audit to every colour field (`UNREACHED` is empty); the 1.5x scene `ui-controls@1.5x` and `apps/gallery/src/catalogue.rs`, the check that
gallery and catalogue agree; `scripts/export-snapshot.sh`, `docs/consuming.md` and the migration map; the rustdoc bar; the done-standard audit of all
93 non-external entries (91 `done`, each with a `- Done (D47, M11):` line naming its tests; the two Class B ones below); this close.

Class B (D234): `platform.material` and `render.surface` stay `in-progress` by their own text. The first needs Luneterm's runs of the OS materials
on real machines (`owner: Luneterm M9 to M11 runs`); the second needs a pixel gate on a presented frame (`owner: consumer's window run`).
`catalogue.rs` holds the list and a test that no other entry is `in-progress`.

## The gallery showcase

`cargo run -p gallery` opens the showcase: a 1600 x 900 glass window with a searchable sidebar (groups, pages, scenes), the chosen page live in the
middle, and on the right that page's knobs and the global settings (theme, motion tier, speed, zoom, glass layers, blur, noise, OS material, durations).
Seven labs (glass, blur, effects, motion, tokens, clipboard, JSON) cover what no component page shows. Keys: Cmd or Ctrl `]` and `[` walk the pages,
Cmd or Ctrl `Q` quits. `--list`, `--dump <dir>` and `--bench` are unchanged; the shell's own scenes are the five `showcase-*`. `docs/components/infra/showcase.md`
is the entry (what it shows, the four pages with no knob and why, what is unverified); decisions D246 to D254; the code is `apps/gallery/src/showcase/`.
The task changed nothing under `crates/` (D246).

Gallery work is checked by eye, not by the pixel gate: the gate runs when a component's design changes (D254). A hands-on run in a real window is what
proves clicks, the OS material swap and the feel of speed and zoom; the tests only drive the shell with scripted pointer events.

## Readings (M11; each run once; nothing acts on them)

- Final gate at ace3648, the merged tip: fmt, clippy, boundaries, index, `cargo doc -D warnings`: clean. `cargo test --workspace`: 2,620 passed, 0 failed, 111 binaries.
  `LUNEUI_REQUIRE_GPU=1 cargo test -p lune-ui-wgpu`: 73 passed. `scripts/check-targets.sh`: Windows passed, Linux skipped (Docker not running). Export and an offline consumer: passed.
- Pixel gate: `scripts/pixel-gate.sh --base 55cfaaf` at ace3648: 413 files identical, 0 differ, max delta 0; 18 new scenes (`ui-controls@1.5x`, seven `window-*`, and ten
  disabled or focus states from review), nothing allowed. Viewed by eye once, no director: the ten review scenes, and `ui-controls@1.5x` (1560 x 990), `window-caption-hover-close`, `window-edges`, `window-inactive-wane`. The other four `window-*` scenes were not viewed.
- Release size: `cargo build --release -p gallery` at a9d1292, `wc -c target/release/gallery` (macOS, strip, fat LTO): 9,869,520 bytes, against 9,869,408 at
  the M10 tip (+112) and 9,448,688 at the M9 merge. Also in `docs/consuming.md`.
- Bench (`cargo run --release -p gallery -- --bench`, per D85) at d14069f, one reading, 2080 x 1320, 5 runs, M1 Pro, no GUI session: 0 of 74 pages over budget at load
  2.0, self-test in range with a spread of +1.13/+2.81. Highest p99 5.99 ms and worst frame 13.03 ms, both `code-editor-view`; `flyout` p99 4.85. The M10 reading (295aadc,
  15 pages over at load 10 to 36, spread +17.01/+45.46) was on a loaded machine; the same pages read clean here, which fits load as its cause. Nothing waits on this reading (D235).
- Platform smoke (`cargo run -p gallery -- --smoke-platform`) at a9d1292: not run, "no monitor: there is no GUI session". The last runs, on a logged-in session: M10 at
  b8faaf1 (exit 0, a real tray item, its rect, a synthetic click, the surface in its window, the frontmost app unchanged, the panel key, teardown) and M11 step 2
  at 15c471e (exit 0, plus the surface resized (2080, 1320) to (2144, 1352) and back, acquire answered skip because the window is hidden).

## Carried limits

Each is a recorded deviation or a choice with a DECISIONS row or an entry, and an API change no consumer has asked for (D237, D245). Paid in M11 and gone from this list:
the disabled-key readers and the chord takers, F11, the audit's 33 unreached colour fields.

- A `with_enabled(false, ..)` scope does not dim an overlay opened inside it (`command-palette-disabled` shows it).
- Layers and clips: overlays above a modal go one level deep, those stacked further share `Raised` and a third level that overlaps its parent interleaves (D137; two edges of it:
  a child held on `Raised` after its parent closed sits above a dialog opened in that window, and a child in a nested dialog's body registers before that dialog's footer buttons).
  One blur per frame (D138). Text cannot be cut by a rounded clip (the bounding rect; the page and panel inset their text by the radius, `panel::TEXT_INSET` is the caller's, D148).
  Text draws above every fill of its layer, so labels show through a sliding tab.
- Fonts: the gallery bundles no CJK face, so the IME scenes' "かな" draws as boxes (D90). No bold face is bundled, so bold draws exactly as medium (`text.shaping`). No italic face, so italic
  is the shaper's skew (D189). Inside a cluster of several glyphs a click lands on either edge non-monotonically; a drag stopped right of a row cut inside a word leaves one grapheme out
  until the pointer reaches the next row (P20, D112).
- Audit rulings (Q23, Q25): the tab strip closes its open menu when disabled, so that frame's menu keys reach a `Global` binding (`disabling_the_strip_closes_an_open_menu`); a ring's width
  follows the device scale, not the `Xform`'s (harmless at a popup's 0.9 to 1). `TAB_HEADER_HEIGHT` and `BODY_PADDING` (consts of `lunegit_dark`) are read by nothing in the repo.
- `menu_separator` is `#FFFFFF22`, not Lunegit's `#18FFFFFF` (D244).
- API gaps, marked in the entries: the table has no column resize or reorder (D163, Q15, deferred); `StrokeStyle` has no cap mode, so the lane graph's lines end butt (D172); the access tree has no
  keyboard-move action for the board (D55, D169); the callout has no title line or inline figure and the empty state's actions have no glyph (D136); the sidebar footer is one line in one family;
  a flyout route has no state note; `Ui::combo` has no glass list; toast history rows have no hover or expand; `Ui::filter_bar` has no option for the palette's glyph, text and radius;
  `PaletteRow` has no icon tone; the side panel's show moves only the content; a panel with `fit: true` cuts a flush body's bottom corners only at the final height.
- `role-gap`: tab to panel association (D122), multi-selectable (D100), composition range, the editor's full-text query (D217).
- Tree view: `cells` is not called on rows of a moving block; rows have no ShowMenu action; no folder glyph; `CHEVRON_GAP` and `GLYPH_GAP` have no source (accepted).
  Split panes draw with the size the frame began with, so a size a frame's input changed shows in the panes on the catch-up frame.
- M8: the table has no horizontal scroll; `page` and `settings_page` nested in another area keep a delta even when they fit (D176); an Escape before the release in one frame is lost and
  the drop commits (D176); a `Measured` row height after a sort takes effect on the next frame; a control in a row scrolled out of view loses focus (rows out of view are not built); a wrapping
  cell ignores `num`; the sorted header's focus ring snaps; bars are keyed by column index, so a rolling window morphs in place; the board moves one card a frame by Shift and arrow and takes
  no landing in a disabled scope.
- M9: a very long single line costs in proportion to its length (D194); a line inserted into a wrapped editor leaves stale heights below it until built; a word motion in the editor stops at
  its window's edge (Q16); a markdown nested block is built whole and only blocks in view are Tab stops; no folding and no in-editor search (D181); a one-line code block scrolled fully right has
  its thumb under the copy button by about 4 px; markdown underline, strike and pill are fills from row boxes (D188); a page key or Home in the frame right after a far jump in a wrapped view lands off
  by what that frame measured; a pane pulled in the frame of its own wheel publishes nothing.
- M10: the text field and text area refuse a paste or set-value over 1 MiB whole (D232, D233), the code editor sets no limit; assistive clients get the lines in view of an editor (D217);
  `Dismiss` is exposed only on a tooltip, `Dragging` and a mixed toggle are not sent (D217); a locale file written for a newer toolkit fails on an older one (D210); the Windows popover activates
  (D224, Q17); a surface is placed from the tray item's rect, which on Windows may sit a few px inside the taskbar (D219); the macOS rect settles only after 300 ms (D225); the tray icon has no
  automatic inversion while the item is highlighted (D218); no Windows row scene; a loaded theme's `WindowFallbackBackground` sets only `window_fallback` (the app recomputes the inactive fill);
  no drop-target widget; `--parity-only` of `scripts/pixel-gate-luneterm.sh` fails by design (the lock has grown).
- The real pointer feel of the tab drag, slider, splitter, tree and board in window mode is not testable with scripted input.
- Review nits left open (M6, M8): `toolbar_label` repeats `button_inner_colored` (F2); colour helpers exist twice (F3); a per-frame `to_owned` of a tab name (F4); a press held across a disabled frame
  activates a tab twice (F7); a ghost's step on a press mid-slide and a second Shift move during a landing (F5, F6).

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
- The showcase in a real window: clicks on the sidebar, the knobs and the settings, a live swap of the OS material, the feel of speed and zoom, Copy and Paste against the OS clipboard (`docs/components/infra/showcase.md`).

## Next

1. Luneterm's M9 adopts the toolkit through `docs/consuming.md`: vendor `scripts/export-snapshot.sh`'s output, switch crate by crate (version 0.2.0 lets the in-tree `lune-ui` and the snapshot stand
   in one lock), use the migration map; its own OS runs close `platform.material` and `render.surface`. Luneterm itself is not changed by this repo.
2. Before any public release: license or remove Lunegit's theme and locale fixtures, which are bundled with no open licence (D206, Q19); the export already leaves them out.
3. The user's calls, all reversible and taken at the recommended option: the final tray glyph (`platform.tray-icon`), column resize and reorder in the table (D163, Q15), and three taken calls to
   confirm: the Windows popover activates (D224, Q17), the tray icon is a CPU raster (D218, Q18), the Lunegit fixtures stay bundled (Q19).
4. CI stays out (D15): the repo has no remote.
5. Task `luneui-rs/library-pass`: the user reviewed the components, and the full list with its hand-off is `~/.claude/orchestrations/luneui-rs/library-pass/scope.md`. It covers the library follow-ups above. Planning starts in a fresh session; read that file first.

## Library follow-ups (found by the gallery showcase; nothing under `crates/` changed, D246)

For the next milestone, library and API. The first is the plan the user asked for after reviewing the showcase.

- Noise strength and luminosity constants are fixed: expose them (D246; its own plan, after the user's review). The blur kernel cap (48 px, so sigma 16) is fixed too; the
  gallery's noise control is on/off by post-processing the scene's acrylic params (`params[3]`), which covers library widgets but not a strength.
- Tray surfaces cannot sit in a content area: the library places those overlays at the window corner, and an `Xform` does not move overlay content.
- `Ui::caption_buttons` places from x = 0, y = 0 and ignores its area's origin.
- A gallery test cannot list hit regions: `UiStore::prev_hits` is `pub(crate)`.
- Scene scripts cannot address a widget by a numeric key (`Ev::PointerOver` takes `&'static [&'static str]`; a tree row's id is `id(u64)`): the `showcase-table` scene aims at a computed point instead.
- Tab order is registration order with no way to scope it: a hosted page's scripted Tab needs its widgets registered before the shell's, so the shell draws its content pane before its sidebar.
- `OutputState::apply` sends the IME area in the window's own scale: under the gallery's zoom the candidate window sits off by the zoom factor.
- A scrolled host's pane clip has to be rebuilt in the content space (pane + offset) and re-applied by every page that calls `Painter::on`: `on` drops the clip. A stack-preserving layer switch in the toolkit would remove the need.
- `Painter::on` also clears the clip stack, so the `pop_clip` of a scroll area that a hosted page switched layers in restores nothing and the pane's clip stays on for everything drawn after (found with the glass lab: the sidebar and settings vanished). The showcase resets with `on(Base)` after its content pane; a stack-preserving layer switch in the toolkit would remove the trap.

## Blockers / open questions

- None.
- Wane's OS behaviours (close-to-tray, single instance, autostart, notifications) are assumed app-side and documented (answer 2026-11-03, reversible).
