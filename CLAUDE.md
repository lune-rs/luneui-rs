# luneui-rs

App-agnostic native Rust glass UI toolkit: immediate-mode painter on wgpu, widgets, OS glass
materials, window helpers, theme tokens. Extracted from Luneterm's `crates/lune-ui` so several Rust
apps share it (Luneterm; a possible Rust rebuild of Lunegit, the Avalonia git client; others). The
look follows Lunegit. No CSS or web engine: styles are Rust tokens. Library only: no deploy.

## Start here

Read `docs/STATE.md` first. Before stopping, update it (now / next / blockers / last updated) in the
same commit as the work. Decisions live in `docs/DECISIONS.md`; do not re-open one unless the user asks.
Source of the code being extracted: `/Users/ijat/Projects/Luneterm/crates/lune-ui` (read-only reference
for the extraction; Luneterm's `docs/architecture.md` sections "lune-ui modules", "Extension points",
"Split points"; D8 overrides its termview line).

## Commands

- Build: `cargo build --workspace`
- Test (all): `cargo test --workspace`
- Test (one): `cargo test -p <crate> <name>`; crates: `lune-ui-core`, `lune-ui-wgpu`, `lune-ui-winit`, `lune-ui-platform`, `lune-ui` (facade), `gallery`, `dump-compare`, `api-docs`. The GPU tests skip without an adapter; `LUNEUI_REQUIRE_GPU=1 cargo test -p lune-ui-wgpu` makes them fail instead
- Format: `cargo fmt --all`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Boundaries: `scripts/check-boundaries.sh`
- Catalogue index (regenerate after adding or editing a component file): `scripts/gen-component-index.sh`
- Acceptance bench (milestone acceptance only; a recorded reading that no gate, merge or fix waits on, and is not re-run to settle load noise, D235; runnable on a busy machine, D85: each run judged on its own p99 <= 16.6 ms and worst frame <= 33 ms, a page OVER only in at least half its runs, the self-test spread and load printed beside every verdict; exit 0): `cargo run --release -p gallery -- --bench` (`--runs 5` is the default; `--size 3456x2234` is report-only)
- Gate (before every commit; must be green): `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo clippy -p lune-ui-winit --all-targets -- -D warnings && cargo test -p lune-ui-winit && scripts/check-boundaries.sh && scripts/gen-component-index.sh --check && scripts/gen-api-docs.sh --check`
- Showcase (the gallery's window; Cmd or Ctrl `]` and `[` walk the pages, Cmd or Ctrl `Q` quits; its global settings include text smoothing, hinting and stem darkening): `cargo run -p gallery`
- Pixel gate (a change to a component's drawing; gallery-only work is checked by eye, D254): `scripts/pixel-gate.sh [--base <rev>] [--allow <id>…] [--removed <id>…]` dumps
  every scene `gallery --list` prints (438 at the library pass's close; the first 14 are the legacy Painter-API scenes) for `<rev>`
  (default `HEAD`) and for the working tree and compares them
  (`dump-compare`); zero delta unless the look change is intended: name its scenes with `--allow <id>`
  and give the reason in a DECISIONS.md row in the same commit. A scene whose subject is gone is named with `--removed <id>` (D260): it must be in
  the base's dump and not in the candidate's list, and its base files move aside; any other missing scene fails. Exit 0 pass, 1 delta, 2 setup error.
  The working tree dumps headless (no window); a base from before M2 step 4 (`52b11d6` and older)
  opens a hidden window, so such a gate needs a logged-in macOS GUI session. Text smoothing is on by default
  since 0.4.0 (D296): `gallery --dump` reads `LUNEUI_GALLERY_TEXT` (`on`, `off`; anything else exits 2), and a gate
  against a base from before it runs as `LUNEUI_GALLERY_TEXT=off scripts/pixel-gate.sh --base <rev>` (smoothing off draws that look).
- Platform smoke (needs a logged-in macOS or Windows GUI session; puts a tray item up for about 2 s; not in the gate; exit 0 held, 1 failed, 2 no session): `cargo run -p gallery -- --smoke-platform`
- Docked panels smoke (the live check of `platform.desktop`, `window-docking`, `window-group`, `window-shape` and `overlay.menu-window`; needs a logged-in macOS or Windows GUI session; opens three shaped panel windows and a menu window for about 10 s, drives them with synthetic desktop pointers, never moves the real cursor or sends input; not in the gate; exit 0 held, 1 a check failed, 2 no session): `cargo run -p gallery -- --smoke-panels`
- Frame pacing probe (needs a logged-in macOS GUI session; a window opens for about 15 s, leave it visible and untouched; not in the gate; exit 0 held, 1 did not hold, 2 could not measure): `cargo run --release -p gallery -- --pace-probe`
- Dump the scenes by hand: `cargo run -p gallery -- --dump <dir>` (`-- --list` prints the scene names)
- M1-only proof against Luneterm's 18-scene spike gate: `scripts/pixel-gate-luneterm.sh` (its full run
  cannot build against the M2 API and exits 2; `--parity-only` checked that `Cargo.lock` holds no
  version Luneterm's lock lacks, and fails by design from M10 on: `arboard` and its tree are new, D213)
- Cross-target (Windows and macOS clippy, Linux in Docker; skips Linux when Docker is not running): `scripts/check-targets.sh`
- Snapshot export for an app that vendors the toolkit (D239; exit 0 done, 1 a check failed, 2 usage or setup; reads `git archive <rev>`, never the working tree; `docs/consuming.md` says how it is used): `scripts/export-snapshot.sh <dest> [--rev <rev>]`
- API reference (D287; a Markdown page per module of the five library crates, generated from rustdoc JSON by `tools/api-docs`, so it cannot go stale: signatures, doc comments, fields, variants, methods, trait impls; regenerate after any public API or doc-comment change; `--check` is in the gate; exit 0 done or current, 1 stale, 2 setup or a failed completeness check): `scripts/gen-api-docs.sh [--check]`; start at `docs/api/INDEX.md`. The pages are generated for `aarch64-apple-darwin` on every host, so they do not depend on the host (D303; the target comes from `rust-toolchain.toml`). It sets `RUSTC_BOOTSTRAP=1` for the rustdoc runs only; a toolchain bump that moves the JSON format exits 2 until the exact `rustdoc-types` pin in `tools/api-docs/Cargo.toml` moves
- Docs (every public function, method, type, trait and module has a doc comment, D242; must build clean): `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --exclude gallery --exclude dump-compare --exclude api-docs`

Toolchain pin `1.92.0` (rustfmt, clippy), edition 2024, `rust-version = 1.85`, resolver 3: in
`rust-toolchain.toml` and the root manifest.

## Architecture rules

- Swap points: text shaping (cosmic-text) with the toolkit's own glyph rasterizer (`GlyphRasterizer`, D294 to D296), GPU renderer (wgpu), OS glass (`Material`), windowing (winit).
  Each sits behind a core type in the pure `lune-ui-core` (no dependencies; no wgpu, winit or cosmic-text
  types in its signatures): `Scene` (display list), `TextSystem`, `ImageStore` (pictures the renderer draws),
  `RawInput`/`FrameOutput`, `Backdrop`.
  Crates: `lune-ui-core`; adapters `lune-ui-wgpu`, `lune-ui-winit`, `lune-ui-platform`; the facade
  `lune-ui` (D9). An adapter depends on the core and its own platform crates only, never on another
  adapter (boundary rules 7 to 9).
- M1 was behaviour-preserving (code moved from `lune-ui` as is); M2 cut the seams and split the
  crates, each step gated at zero pixel delta.
- The toolkit has no composition root: each consuming app wires adapters. The toolkit never starts an
  event loop and never reads the clock or environment; the app passes time, sizes, fonts, theme and hit
  values in, so frames are deterministic.
- Extend, don't patch: new OS glass = a `material` submodule of `lune-ui-platform`; new overlay or widget = a new module plus
  a catalogue file; new look = a `Theme` value. App-specific drawing goes through the `CustomDraw`
  hook, not into the toolkit (D8).
- A widget never builds or runs a context menu: it reports a secondary click and the show-menu action as `context` on its response, and the app opens the menu with `Ui::menu` (D292).
- A composite with one tab stop (radio group, star rating, nav list, tab strip, table header, board cards) uses roving focus: `Sense.skip_tab`,
  `Ui::rove_stop`, `rove_keys`, `rove_focus` (D113); text that must fit a width goes through `text::elide` / `Painter::elide`
  (D114). A drag ghost draws one layer group above its source, out of any `record` (`Ui::drag_ghost`, D168).
- Only the window's background replaces what is below it: `Painter::acrylic` above the fixed `Base` layer, or in an overlay, draws as `Painter::frost`, and a backdrop replaces its region, so no draw opens a hole to the desktop (D258).
- A loaded theme must reach every widget: a widget takes every colour from `ui.theme()` (or its options), never a literal or the preset (`c4::m10` draws the controls under a theme of distinct colours and fails on a preset colour; a fixed colour on purpose is listed in `docs/STATE.md`).
- One text editing engine: `TextEdit` edits through `EditText` and `EditBuffer`, implemented for `str`, `String` and the line-indexed
  `CodeDoc` (D180); a new text kind implements them, it does not get a second engine. A text view draws only the lines in view and
  never shapes, hashes or clones the whole text per frame (D179, D195).
- One selection drawing: every text selection (and the diff view's row selection, in its own colour) is drawn by
  `widgets::selection::paint` from the selected part of each row in view, joined into one shape with rounded outer and
  inner corners (`Painter::fill_joined`, R18, D272); no widget fills a selection itself (the gallery's
  `no_widget_fills_a_selection_itself` walks every scene for it).
- A wheel delta routes per axis: the topmost scroll region live on that axis takes it (D178). A composite with nothing to scroll on
  an axis disables that axis (`Bars::unless_fits`), so the delta reaches the area around it (D176).
- An overlay that opens above an active modal is put on `LayerId::Raised` by `Ui::overlay` and held there only while an older modal is on
  the stack (D137, D255, D263): a widget never names the layer. Each overlay draws on a layer of its own in open order (`Scene::in_order`,
  D256), so a later overlay hides an earlier one's text; every glass request blurs everything drawn before its layer, other glass included
  (one blur per request, at its layer, D257); one routine, `popover::glass_surface`, frosts every floating surface (D260). A surface that
  must stay live and undimmed under a modal goes through `Ui::outside_modals` (`LayerId::Outside`, D259). `Painter::on` changes only the
  layer and keeps the clip stack (`enter`, the overlays, still clears it, D265). A widget in a `with_enabled(false, ..)` scope needs no rule of its own for its nodes (`Ui::node`
  marks them, D140), but it consumes its keys before it gates on `enabled` (`docs/architecture.md`, "Disabled keys", is the one list, D159); `testing::disabled_violations` checks a subtree.
- Each seam has a fake and a contract test in `lune_ui_core::testing` (`RecordingRenderer`, `FakeText`
  with `text_system_contract`, `FakeImages` with `image_store_contract`, `ScriptedInput` with `scripted_input_contract`, `NullMaterial` with
  `backdrop_contract`); a new adapter
  passes the same suite. `docs/architecture.md` holds the module map, the seam table, the multi-window
  model and the pixel gates.

## Hard rules

1. App-agnostic: no app, terminal or domain words in library source. Enforced by
   `scripts/check-boundaries.sh` and its allow list (`scripts/boundaries.allow`; `lunegit` is
   allowed only in the theme preset module, its re-export and the in-crate tests that import the preset; D14).
2. No clock, environment or event-loop access in library `src` (same script; tests and examples may).
3. `unsafe` only in `lune-ui-platform` under `src/platform/` or `src/material/`, each block with a
   `// SAFETY:` comment; `unsafe_op_in_unsafe_fn` denied; every other crate is
   `#![forbid(unsafe_code)]`. Platform code lives in `#[cfg(target_os = ...)]` module files
   (`src/platform/`, `src/material/`) of the adapter crate that implements the seam; shared code never
   branches on OS inline (boundary rules 6, 10).
4. Third-party crates only for platform layers (windowing, GPU, text shaping, OS glass); a new
   dependency needs a DECISIONS.md row.
5. Size budget: when adding a dependency, build the release profile and note the binary-size change in
   the commit message.
6. Every bundled third-party asset or code (the Avalonia acrylic noise texture and colour formulas,
   fonts, shaders from elsewhere) has its licence text in `licenses/` and a row in
   `THIRD_PARTY_NOTICES.md`.
7. Every manifest inherits `license = "MIT OR Apache-2.0"` from the workspace; `publish = false` until
   the user decides otherwise.
8. A change to a component's drawing passes the pixel gate (see Commands; gallery-only work is checked by
   eye, D254). Look values follow Lunegit; a deliberate deviation is a DECISIONS.md row.
9. Library API: `unreachable_pub`, `missing_debug_implementations` warn; `unwrap_used`, `dbg_macro`,
   `todo` warn (workspace lints in the root manifest).
10. Every component has one catalogue file `docs/components/<area>/<id>.md` with front matter; the id
    is an area-prefixed slug (`core.painter`, `data.lane-graph`), no numeric counter. Nobody
    hand-edits `docs/components/INDEX.md`: a script generates it and the gate fails when it is stale
    (`scripts/gen-component-index.sh`; its `--check` is in the gate).

## Git

- Worktree per orchestrated task; commit per change.
- Remote `origin`: github.com/lune-rs/luneui-rs (public, D284). Push only when the user asks; never force-push.
- Every release (a workspace version bump that lands on `main`) gets an annotated tag `v<version>` on its merge commit, message `v<version>: <theme> (<D-rows>)` (as `v0.4.0`, `v0.5.0`); push the tag with the release.

## Read on demand

- `docs/STATE.md` - current work and next steps
- `docs/DECISIONS.md` - decisions and what was rejected
- `docs/requirements.md` - what the toolkit must provide (requirement ids `R-<area>-<n>`)
- `docs/api/INDEX.md` - generated reference of the whole public API (signature, docs, source line of every item, a page per module); open it to find an item by name
- `docs/components/INDEX.md` - generated catalogue index; open a component's file under
  `docs/components/<area>/<slug>.md` for its detail; `docs/components/README.md` explains how to add one
- `docs/architecture.md` - crates, seams, module map, extension points, pixel gates
- `docs/consuming.md` - how an app vendors the toolkit (the snapshot, its duties, release size, the migration map)
