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
- Acceptance bench (milestone acceptance only; runnable on a busy machine, D85: each run judged on its own p99 <= 16.6 ms and worst frame <= 33 ms, a page OVER only in at least half its runs, the self-test spread and load printed beside every verdict; exit 0): `cargo run --release -p gallery -- --bench` (`--runs 5` is the default; `--size 3456x2234` is report-only)
- Gate (before every commit; must be green): `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && scripts/check-boundaries.sh && scripts/gen-component-index.sh --check`
- Pixel gate (any drawing change): `scripts/pixel-gate.sh [--base <rev>] [--allow <id>…]` dumps the
  gallery's 15 reference scenes for `<rev>` (default `HEAD`) and for the working tree and compares them
  (`dump-compare`); zero delta unless the look change is intended: name its scenes with `--allow <id>`
  and give the reason in a DECISIONS.md row in the same commit. Exit 0 pass, 1 delta, 2 setup error.
  The working tree dumps headless (no window); a base from before M2 step 4 (`52b11d6` and older)
  opens a hidden window, so such a gate needs a logged-in macOS GUI session.
- Dump the scenes by hand: `cargo run -p gallery -- --dump <dir>` (`-- --list` prints the scene names)
- M1-only proof against Luneterm's 18-scene spike gate: `scripts/pixel-gate-luneterm.sh` (its full run
  cannot build against the M2 API and exits 2; `--parity-only` still checks that `Cargo.lock` holds no
  version Luneterm's lock lacks)
- Cross-target (Windows clippy, Linux in Docker; skips Linux when Docker is not running): `scripts/check-targets.sh`

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
8. A drawing change passes the pixel gate (see Commands). Look values follow Lunegit; a deliberate
   deviation is a DECISIONS.md row.
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
