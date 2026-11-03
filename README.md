# luneui-rs

App-agnostic native Rust glass UI toolkit: immediate-mode painter on wgpu, widgets, OS glass
materials, window helpers and theme tokens. No CSS or web engine; styles are Rust tokens. The look
follows Lunegit. Extracted from Luneterm's `lune-ui` so several Rust apps can share it.

Status: scaffolding. The workspace builds with an empty placeholder `lune-ui` crate; no toolkit code
has moved in yet. `docs/requirements.md` says what the toolkit must provide and `docs/components/INDEX.md`
lists every component (existing in Luneterm, planned, or a rough Lunegit stub).

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
scripts/check-boundaries.sh
scripts/gen-component-index.sh          # regenerate docs/components/INDEX.md
```

See `CLAUDE.md` for the rules and `docs/` for state and decisions.

## Licence

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
