# Implementation progress

2026-11-03. Branch `feat/core-foundations`, private PR #2. Current work supersedes earlier foundations-only and manual-only notes.

Implemented: browser discovery/configuration, migrations and FTS, private main/WAL/journal snapshots, bounded readers, URL normalization, transactional dedupe/cursors, manual and dry-run collection, CSV/JSON export, bounded logs, weekly optimization, once-only conflict retry, Windows task XML/COM adapter and hidden helper, and four offline recommendation queries. Synthetic generator and Windows lifetime-counter measurements are included. Hosted CI runs formatting, lint, tests and release builds on Ubuntu and Windows GNU.

The base program is a CLI, not a graphical app. Browser openings are not proof of watching. Recommendation inputs beyond visits remain unavailable until the later metadata/tag/session features are implemented. RSS and TMDB are not connected.

Current evidence, exact test totals, resource numbers and remaining gaps: [M0 acceptance](docs/m0-acceptance.md). No milestone is silently accepted based on code presence. In particular, retained personal-week import, live scheduler registration/launch, independent review and the unimplemented strict named cases stay open.

Next independent implementation work: close remaining contract cases in the acceptance matrix, then build the on-demand read-only timeline/search described in [UI direction](docs/ui-recommendations.md). Session capture remains a separate opt-in milestone. Preserve low-overhead operation and source provenance.

Local build environment: direct toolchain executables under `D:\rust\rustup\toolchains\stable-x86_64-pc-windows-gnu\bin`, MinGW under `D:\rust\mingw64\bin`, `CARGO_HOME=D:\rust\.cargo`, `CARGO_TARGET_DIR=D:\mariyaholic-build\omoi\target`. Use `cargo fmt --check`, `cargo clippy --locked --offline --all-targets -- -D warnings`, `cargo test --locked --offline --all-targets`, and `cargo build --locked --offline --release --bins --example synthetic`. Put the direct toolchain ahead of the older Cargo proxy on PATH.
