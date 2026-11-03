# Implementation progress

2026-11-03. Branch `feat/core-foundations`, private PR #2. Current work supersedes earlier foundations-only and manual-only notes.

Current follow-through adds in-app export, scheduler controls, metadata/tag editing, opt-in userscript listener, transactional sessions and overlap-aware playback totals. Migration 2 preserves existing catalogues. See [current implementation and acceptance](docs/completion-20261103.md); this supersedes older unavailable-input and CLI-only statements below.

Implemented: browser discovery/configuration, migrations and FTS, private main/WAL/journal snapshots, bounded readers, URL normalization, transactional dedupe/cursors, manual and dry-run collection, CSV/JSON export, bounded logs, weekly optimization, once-only conflict retry, Windows task XML/COM adapter and hidden helper, and four offline recommendation queries. Synthetic generator and Windows lifetime-counter measurements are included. Hosted CI runs formatting, lint, tests and release builds on Ubuntu and Windows GNU.

The embedded UI (`omoi ui --open`) uses the same catalogue and collector. It provides paginated history, FTS/literal search, filters, details, deletion, selected collection and local suggestions. All five themes remain. Edited/captured metadata, tags and playback now feed the local shelves; browser openings remain distinct from watching. RSS and TMDB are not connected.

Current evidence, exact test totals, resource numbers and remaining gaps: [M0 acceptance](docs/m0-acceptance.md). No milestone is silently accepted based on code presence. In particular, retained personal-week import, live scheduler registration/launch, independent review and the unimplemented strict named cases stay open.

Remaining work includes the acceptance matrix's unrun cases, live-player/manager compatibility, full accessibility review, long-duration playback resources and independent review. Capture remains opt-in. Current behavior and reproduction: [ui/README.md](ui/README.md).

Local build environment: direct toolchain executables under `D:\rust\rustup\toolchains\stable-x86_64-pc-windows-gnu\bin`, MinGW under `D:\rust\mingw64\bin`, `CARGO_HOME=D:\rust\.cargo`, `CARGO_TARGET_DIR=D:\mariyaholic-build\omoi\target`. Use `cargo fmt --check`, `cargo clippy --locked --offline --all-targets -- -D warnings`, `cargo test --locked --offline --all-targets`, and `cargo build --locked --offline --release --bins --example synthetic`. Put the direct toolchain ahead of the older Cargo proxy on PATH.
