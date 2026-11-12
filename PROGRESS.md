# Implementation progress

2026-11-03. PRs #2 (core), #3 (desktop/icon) and #4 (state transitions) are merged.
The current code baseline is `cc530f0`, including audit commit `3bb38cc`; PR #4's
Windows and Ubuntu CI both passed. Schema 3, standalone desktop, five themes and
userscript 0.1.1 are implemented. See [runtime state machines](docs/state-machines.md).

Next work: [installation-friction roadmap](docs/installation-friction.md), with
[cloud handoff](CLOUD_HANDOFF.md). An assisted owner Edge setup demonstrated actual
capture, but the package still requires a separate manager/script install, pairing,
listener start and inbox processing. Guided setup, managed capture and a dedicated
companion are planned, not shipped. The remaining text records the earlier core
implementation; its migration-2 references are historical.

Current follow-through adds in-app export, scheduler controls, metadata/tag editing, opt-in userscript listener, transactional sessions and overlap-aware playback totals. Migration 2 preserves existing catalogues. See [current implementation and acceptance](docs/completion-20261103.md); this supersedes older unavailable-input and CLI-only statements below.

Implemented: browser discovery/configuration, migrations and FTS, private main/WAL/journal snapshots, bounded readers, URL normalization, transactional dedupe/cursors, manual and dry-run collection, CSV/JSON export, bounded logs, weekly optimization, once-only conflict retry, Windows task XML/COM adapter and hidden helper, and four offline recommendation queries. Synthetic generator and Windows lifetime-counter measurements are included. Hosted CI runs formatting, lint, tests and release builds on Ubuntu and Windows GNU.

The embedded UI (`omoi ui --open`) uses the same catalogue and collector. It provides paginated history, FTS/literal search, filters, details, deletion, selected collection and local suggestions. All five themes remain. Edited/captured metadata, tags and playback now feed the local shelves; browser openings remain distinct from watching. RSS and TMDB are not connected.

Current measured results and completed verification are in the [verification summary](docs/headless-acceptance-20261103.md). The owner has deferred further acceptance for this private development merge. Detailed checklists and raw logs are retained locally; deferred work is not recorded as passed.

Capture remains opt-in. Current behavior and reproduction: [ui/README.md](ui/README.md).

Local build environment: direct toolchain executables under `D:\rust\rustup\toolchains\stable-x86_64-pc-windows-gnu\bin`, MinGW under `D:\rust\mingw64\bin`, `CARGO_HOME=D:\rust\.cargo`, `CARGO_TARGET_DIR=D:\mariyaholic-build\omoi\target`. Use `cargo fmt --check`, `cargo clippy --locked --offline --all-targets -- -D warnings`, `cargo test --locked --offline --all-targets`, and `cargo build --locked --offline --release --bins --example synthetic`. Put the direct toolchain ahead of the older Cargo proxy on PATH.
