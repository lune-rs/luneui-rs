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

All not yet runnable (no code); the first plan creates the workspace and these scripts.
- Build: `cargo build --workspace` (not yet runnable (no code))
- Test (all): `cargo test --workspace` (not yet runnable (no code))
- Test (one): `cargo test -p <crate> <name>` (not yet runnable (no code))
- Format: `cargo fmt --all` (not yet runnable (no code))
- Lint: `cargo clippy --workspace --all-targets -- -D warnings` (not yet runnable (no code))
- Boundaries: `scripts/check-boundaries.sh` (not yet runnable (no code))
- Gate (before every commit; must be green): `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && scripts/check-boundaries.sh` (not yet runnable (no code)); the first plan adds the catalogue index staleness check to it
- Pixel gate (any drawing change): dump offscreen frames of the reference scenes to a dir before and
  after the change, then compare with the compare tool; zero delta unless the look change is intended
  and re-baselined in the same commit with the reason. Exact commands: set by the first plan
  (not yet runnable (no code)).

Toolchain pin `1.92.0` (rustfmt, clippy), edition 2024, `rust-version = 1.85`, resolver 3: in
`rust-toolchain.toml` and the root manifest (to be created by the first plan).

## Architecture rules

- Swap points: text shaping (glyphon), GPU renderer (wgpu), OS glass (`Material`), windowing (winit).
  Target, decided by the extraction plan and not true today: each sits behind a trait owned by a pure
  core with no wgpu, winit or glyphon types in its signatures, adapters depending on the core only.
- The first extraction is behaviour-preserving: code moves from `lune-ui` as is, pixel gate at zero
  delta. The seams follow as separate, gated steps. Crate names and split: set by the first plan.
- The toolkit has no composition root: each consuming app wires adapters. The toolkit never starts an
  event loop and never reads the clock or environment; the app passes time, sizes, fonts, theme and hit
  values in, so frames are deterministic.
- Extend, don't patch: new OS glass = a `material` submodule; new overlay or widget = a new module plus
  a catalogue file; new look = a `Theme` value. App-specific drawing goes through the `CustomDraw`
  hook, not into the toolkit (D8).
- As each seam lands it gets a contract test and a fake (headless target, null material).
  `docs/architecture.md` (to be created by the first plan) holds the module map.

## Hard rules

1. App-agnostic: no app, terminal or domain words in library source. Enforced by
   `scripts/check-boundaries.sh` and its allow list (to be created by the first plan; `lunegit` is
   allowed only in the theme preset module).
2. No clock, environment or event-loop access in library `src` (same script; tests and examples may).
3. `unsafe` only in platform modules, each block with a `// SAFETY:` comment; `unsafe_op_in_unsafe_fn`
   denied. Platform code lives in `#[cfg(target_os = ...)]` modules inside the crate that owns the trait;
   shared code never branches on OS inline.
4. Third-party crates only for platform layers (windowing, GPU, text shaping, OS glass); a new
   dependency needs a DECISIONS.md row.
5. Size budget: when adding a dependency, build the release profile and note the binary-size change in
   the commit message.
6. Every bundled third-party asset or code (the Avalonia acrylic noise texture and colour formulas,
   fonts, shaders from elsewhere) has its licence text in `licenses/` and a row in
   `THIRD_PARTY_NOTICES.md` (both to be created by the first plan).
7. Every manifest inherits `license = "MIT OR Apache-2.0"` from the workspace; `publish = false` until
   the user decides otherwise.
8. A drawing change passes the pixel gate (see Commands). Look values follow Lunegit; a deliberate
   deviation is a DECISIONS.md row.
9. Library API: `unreachable_pub`, `missing_debug_implementations` warn; `unwrap_used`, `dbg_macro`,
   `todo` warn (workspace lints, to be created by the first plan).
10. Every component has one catalogue file `docs/components/<area>/<id>.md` with front matter; the id
    is an area-prefixed slug (`core.painter`, `lunegit.commit-graph`), no numeric counter. Nobody
    hand-edits `docs/components/INDEX.md`: a script generates it and the gate fails when it is stale
    (script and check to be created by the first plan).

## Git

- Worktree per orchestrated task; commit per change.
- No remote is configured yet; nothing is pushed until the user says so.

## Read on demand

- `docs/STATE.md` - current work and next steps
- `docs/DECISIONS.md` - decisions and what was rejected
- `docs/requirements.md` - what the toolkit must provide (to be created by the first plan)
- `docs/components/INDEX.md` - generated catalogue index; open a component's file under
  `docs/components/<area>/<id>.md` for its detail (to be created by the first plan)
- `docs/architecture.md` - crates, seams, module map (to be created by the first plan)
