# Command-line baseline evidence

This change replaces the placeholder entry point with a usable manual collection/export loop. It does not complete every M0 contract gate.

Local verification, 2026-11-03: Windows GNU `cargo test --locked --offline --all-targets` passed 27 tests; Clippy with `-D warnings` and rustfmt checks passed. `cargo build --locked --offline --release --bins` succeeded; all six pipeline tests also passed under `--release`. The optimized executable at `D:\mariyaholic-build\omoi\target\release\omoi.exe` is 2,038,784 bytes, SHA-256 `37802e408db3d2044fd8c87f6c116013d3298f91edbf1aaebf532d31a84892fb`. Its help command ran successfully. Size is measured; memory/CPU/catalogue-size targets are not yet measured.

Implemented pipeline: profile discovery → sidecar-aware private snapshot → bounded Chromium/Firefox pagination → strict platform normalization → per-profile JSONL spool → one SQLite transaction with checkpoint conflict detection → CSV/JSON export.

Verified with generated fixtures:
- Chromium collection and unchanged rerun, one deduped view from repeated opens.
- New rows after the first run and fixed-anchor 30-minute merging.
- Dry run before database creation and after an existing import.
- Corrupt profile alongside a valid one: exit 3, valid rows committed, no failed checkpoint.
- Firefox WAL-only updates while the source connection remains open.
- CSV destination no-clobber behavior and JSON row/title/duration output.
- All input URL rows in the architecture contract's normalization table.
- Existing configuration, schema/migration, FTS, identity and profile-discovery tests.

Remaining before declaring full M0 complete:
- Finish all named unit-level acceptance cases, including injected snapshot races, permission failures, concurrent collectors, malformed rows and file-size caps.
- Count NULL-native-timestamp rows separately; source readers currently exclude them. Malformed/oversized non-null rows advance checkpoints and count as rejected.
- Add the specified once-only restart on a concurrent checkpoint conflict. Current behavior returns Busy/4 without committing, so a subsequent run safely retries.
- Complete bounded persistent logs, weekly optimize maintenance, scheduler/helper installation and measurement harness. Current diagnostics go to stderr and the collection summary.
- Harden Firefox INI parsing/size bounds, local URL UTF-8/percent canonicalization, export broken-pipe classification and consistent multi-query db-info snapshots.
- Run Linux and hosted Windows checks, independent review, resource measurements and owner-run real-profile/scheduler smoke tests.

No private history was used in these tests. Keep the PR draft until the outstanding M0 evidence has been reviewed; this document supports trying the manual CLI, not a release-readiness claim.
