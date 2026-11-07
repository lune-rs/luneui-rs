# luneui-rs

App-agnostic native Rust glass UI toolkit: immediate-mode painter on wgpu, widgets, OS glass
materials, window helpers and theme tokens. No CSS or web engine; styles are Rust tokens. The look
follows Lunegit. Extracted from Luneterm's `lune-ui` so several Rust apps can share it.

Status: in development. The toolkit is five crates: `lune-ui-core` (pure: painter, display list,
widgets, theme, seam types), the adapters `lune-ui-wgpu`, `lune-ui-winit` and `lune-ui-platform`, and
the facade `lune-ui`. `apps/gallery` renders the reference scenes; see `docs/STATE.md` and
`docs/architecture.md`. `docs/requirements.md` says what the toolkit must provide and
`docs/components/INDEX.md` lists every component (existing, planned, or a rough Lunegit stub).

```bash
cargo run -p gallery               # the showcase: every page live, with knobs and settings
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
scripts/check-boundaries.sh
scripts/gen-component-index.sh          # regenerate docs/components/INDEX.md
```

See `CLAUDE.md` for the rules and `docs/` for state and decisions. An app that takes the toolkit in
before a remote exists vendors a snapshot (`scripts/export-snapshot.sh`); `docs/consuming.md` says how.

## Licence

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
