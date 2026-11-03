<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-03 (init commit)

## Now

Repository initialized with docs only; no code. Nothing in progress.

## Next

1. Bootstrap task: `docs/requirements.md`; the component catalogue (`docs/components/<area>/<id>.md`,
   one file per component with front matter, plus the script that generates `docs/components/INDEX.md`
   and its staleness check in the gate); the Cargo workspace skeleton (`rust-toolchain.toml` 1.92.0,
   `rustfmt.toml`, workspace lints, resolver 3, edition 2024, rust-version 1.85, licence inheritance);
   `scripts/check-boundaries.sh` with its allow list (`lunegit` allowed only in the theme preset module).
2. Extract `lune-ui` from Luneterm behaviour-preserving (pixel gate: zero delta against Luneterm's
   frames); decide the pure-core / adapter split and the seams as separate gated steps.
3. Add `licenses/` and `THIRD_PARTY_NOTICES.md` for the Avalonia acrylic noise texture and formulas
   and anything else bundled.
4. Add CI (fmt, clippy `-D warnings`, tests on macOS, Windows and Linux) once the gate exists; no
   workflow before then.
5. Decide the pixel-gate baseline form (committed PNGs, a hash manifest, or re-captured per change).

## Blockers / open questions

- How Luneterm consumes luneui-rs before a remote exists (a path dependency breaks a clean Luneterm
  CI checkout; a git dependency needs a remote). Must be settled before Luneterm M9 uses the toolkit.
