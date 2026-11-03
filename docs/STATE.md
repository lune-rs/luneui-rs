<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-04 (Wane component entries)

## Now

Skeleton, requirements and catalogue are done: Cargo workspace with an empty placeholder `lune-ui`
crate, `scripts/check-boundaries.sh`, `docs/requirements.md`, and `docs/components/` with a generated
`INDEX.md` (26 existing Luneterm components, 3 planned Luneterm needs, 1 external, 29 rough Lunegit
stubs). The gate runs green. No toolkit code has moved in yet.

Wane is a third consumer (2026-11-04): 20 new `planned` entries from its UI spec (tray icon, sidebar, page,
panel, radio group, status dot, nav item, badge, form field, row, callout, steps, verdict row, flyout, stat,
table, key-value list, progress, bar chart, code block), `wane` added to 15 existing entries, and new
requirements R-platform-6, R-widget-4 and R-data-2. Source: Wane `docs/ui/components.md` (P0/P1/P2 order
in its section 7).

## Next

1. A planner deep-analyses the `lunegit.*` stubs (and the stubs in library areas) by reading Lunegit's
   code, then replaces each "Rough stub" body with real needs.
2. Extraction plan: move `lune-ui` from Luneterm behaviour-preserving (pixel gate at zero delta against
   Luneterm's 18 scenes), decide crate names and the pure-core / adapter split and the seams as
   separate gated steps. Includes `licenses/`, `THIRD_PARTY_NOTICES.md`, `docs/architecture.md` and the
   pixel-gate baseline form.
3. CI (fmt, clippy `-D warnings`, tests on macOS, Windows and Linux) once there is code to run.
4. Luneterm consumption decision (blocker for Luneterm M9).

## Blockers / open questions

- How Luneterm consumes luneui-rs before a remote exists (a path dependency breaks a clean Luneterm
  CI checkout; a git dependency needs a remote). Must be settled before Luneterm M9 uses the toolkit.
