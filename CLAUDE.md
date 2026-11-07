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
- Test (one): `cargo test -p <crate> <name>`; crates: `lune-ui-core`, `lune-ui-wgpu`, `lune-ui-winit`, `lune-ui-platform`, `lune-ui` (facade), `gallery`, `dump-compare`. The GPU tests skip without an adapter; `LUNEUI_REQUIRE_GPU=1 cargo test -p lune-ui-wgpu` makes them fail instead
- Format: `cargo fmt --all`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Boundaries: `scripts/check-boundaries.sh`
- Catalogue index (regenerate after adding or editing a component file): `scripts/gen-component-index.sh`
- Acceptance bench (milestone acceptance only; a recorded reading that no gate, merge or fix waits on, and is not re-run to settle load noise, D235; runnable on a busy machine, D85: each run judged on its own p99 <= 16.6 ms and worst frame <= 33 ms, a page OVER only in at least half its runs, the self-test spread and load printed beside every verdict; exit 0): `cargo run --release -p gallery -- --bench` (`--runs 5` is the default; `--size 3456x2234` is report-only)
- Gate (before every commit; must be green): `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo clippy -p lune-ui-winit --all-targets -- -D warnings && cargo test -p lune-ui-winit && scripts/check-boundaries.sh && scripts/gen-component-index.sh --check`
- Showcase (the gallery's window; Cmd or Ctrl `]` and `[` walk the pages, Cmd or Ctrl `Q` quits): `cargo run -p gallery`
- Pixel gate (a change to a component's drawing; gallery-only work is checked by eye, D254): `scripts/pixel-gate.sh [--base <rev>] [--allow <id>…]` dumps
  every scene `gallery --list` prints (431 at M11's close; the first 15 are the legacy Painter-API scenes) for `<rev>`
  (default `HEAD`) and for the working tree and compares them
  (`dump-compare`); zero delta unless the look change is intended: name its scenes with `--allow <id>`
  and give the reason in a DECISIONS.md row in the same commit. Exit 0 pass, 1 delta, 2 setup error.
  The working tree dumps headless (no window); a base from before M2 step 4 (`52b11d6` and older)
  opens a hidden window, so such a gate needs a logged-in macOS GUI session.
- Platform smoke (needs a logged-in macOS or Windows GUI session; puts a tray item up for about 2 s; not in the gate; exit 0 held, 1 failed, 2 no session): `cargo run -p gallery -- --smoke-platform`
- Dump the scenes by hand: `cargo run -p gallery -- --dump <dir>` (`-- --list` prints the scene names)
- M1-only proof against Luneterm's 18-scene spike gate: `scripts/pixel-gate-luneterm.sh` (its full run
  cannot build against the M2 API and exits 2; `--parity-only` checked that `Cargo.lock` holds no
  version Luneterm's lock lacks, and fails by design from M10 on: `arboard` and its tree are new, D213)
- Cross-target (Windows clippy, Linux in Docker; skips Linux when Docker is not running): `scripts/check-targets.sh`
- Snapshot export for an app that vendors the toolkit (D239; exit 0 done, 1 a check failed, 2 usage or setup; reads `git archive <rev>`, never the working tree; `docs/consuming.md` says how it is used): `scripts/export-snapshot.sh <dest> [--rev <rev>]`
- Docs (every public function, method, type, trait and module has a doc comment, D242; must build clean): `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --exclude gallery --exclude dump-compare`

Toolchain pin `1.92.0` (rustfmt, clippy), edition 2024, `rust-version = 1.85`, resolver 3: in
`rust-toolchain.toml` and the root manifest.

## Architecture rules

- Swap points: text shaping (glyphon), GPU renderer (wgpu), OS glass (`Material`), windowing (winit).
  Each sits behind a core type in the pure `lune-ui-core` (no dependencies; no wgpu, winit or glyphon
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
- A composite with one tab stop (radio group, star rating, nav list, tab strip, table header, board cards) uses roving focus: `Sense.skip_tab`,
  `Ui::rove_stop`, `rove_keys`, `rove_focus` (D113); text that must fit a width goes through `text::elide` / `Painter::elide`
  (D114). A drag ghost draws one layer group above its source, out of any `record` (`Ui::drag_ghost`, D168).
- A loaded theme must reach every widget: a widget takes every colour from `ui.theme()` (or its options), never a literal or the preset (`c4::m10` draws the controls under a theme of distinct colours and fails on a preset colour; a fixed colour on purpose is listed in `docs/STATE.md`).
- One text editing engine: `TextEdit` edits through `EditText` and `EditBuffer`, implemented for `str`, `String` and the line-indexed
  `CodeDoc` (D180); a new text kind implements them, it does not get a second engine. A text view draws only the lines in view and
  never shapes, hashes or clones the whole text per frame (D179, D195).
- A wheel delta routes per axis: the topmost scroll region live on that axis takes it (D178). A composite with nothing to scroll on
  an axis disables that axis (`Bars::unless_fits`), so the delta reaches the area around it (D176).
- An overlay that opens above an active modal is put on `LayerId::Raised` by `Ui::overlay`, once at open (D137): a widget
  never names the layer. A widget in a `with_enabled(false, ..)` scope needs no rule of its own for its nodes (`Ui::node`
  marks them, D140), but it consumes its keys before it gates on `enabled` (`docs/architecture.md`, "Disabled keys", is the one list, D159); `testing::disabled_violations` checks a subtree.
- Each seam has a fake and a contract test in `lune_ui_core::testing` (`RecordingRenderer`, `FakeText`
  with `text_system_contract`, `FakeImages` with `image_store_contract`, `ScriptedInput`, `NullMaterial` with
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
- No remote is configured yet; nothing is pushed until the user says so.

## Read on demand

- `docs/STATE.md` - current work and next steps
- `docs/DECISIONS.md` - decisions and what was rejected
- `docs/requirements.md` - what the toolkit must provide (requirement ids `R-<area>-<n>`)
- `docs/components/INDEX.md` - generated catalogue index; open a component's file under
  `docs/components/<area>/<slug>.md` for its detail; `docs/components/README.md` explains how to add one
- `docs/architecture.md` - crates, seams, module map, extension points, pixel gates
- `docs/consuming.md` - how an app vendors the toolkit (the snapshot, its duties, release size, the migration map)
