# luneui-rs

App-agnostic native Rust glass UI toolkit: immediate-mode painter on wgpu, widgets, OS glass
materials, window helpers and theme tokens. No CSS or web engine; styles are Rust tokens. The look
follows Lunegit. Extracted from Luneterm's `lune-ui` so several Rust apps can share it.

Status: scaffolding; nothing usable yet (no code in the repository).

Planned commands (not yet runnable; the first plan creates the workspace):

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

See `CLAUDE.md` for the rules and `docs/` for state and decisions.

## Licence

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
