# Implementation progress

2026-11-03, codex.impl, model: Astra. Branch: `feat/core-foundations`.

This file supersedes the historical missing-module status in the architecture and feature inventory. The roadmap is still in progress; no milestone is accepted as complete.

Implemented:
- U1 reader contracts, OS-native source ID encoding and versioned cursor serialization. Three named acceptance tests.
- U2 injectable path environment and bounded restricted-config parser, numeric ranges, offsets and profile-selector validation. Four named acceptance tests.
- U3 schema 0001, FTS triggers, foreign keys, transactional migration, future/unversioned-schema rejection, read/write connection initialization and database info. Six named migration tests plus a count-reporting test.
- U4 profile discovery for Windows/Linux layouts, Firefox INI entries, explicit paths, canonical deduplication and exclusions. Five named tests currently cover generated fixtures. The denied-access test currently covers a missing explicit file, not an injected permission error; do not claim that case verified.

Verification environment: native Windows GNU. Direct toolchain executables under `D:\rust\rustup\toolchains\stable-x86_64-pc-windows-gnu\bin`; MinGW under `D:\rust\mingw64\bin`; `CARGO_HOME=D:\rust\.cargo`; `CARGO_TARGET_DIR=D:\mariyaholic-build\omoi\target`. The direct Cargo invocation resolved the earlier baseline uncertainty; original source failed with E0583 before implementation.

Latest local checks: `cargo test --locked --offline --all-targets` passed 20 tests (none ignored); `cargo clippy --locked --offline --all-targets -- -D warnings` passed; direct `rustfmt --check --edition 2021` on library/main and all four integration-test files passed. These are Windows GNU results, not Linux or hosted evidence.

Continue without waiting for routine approval:
1. Complete U3 connection-level file tests (read-only no creation/mutation, WAL and busy handling). Check consistent snapshots for multi-query database info before connecting it to the CLI.
2. Harden U4 INI grammar and bound reads, add injected permission failures and symlink fixtures, and split helpers if needed to retain the module size target.
3. Implement U5 snapshots, U6/U7 readers and U8 normalization with all contract vectors; then continue through U9–U14 per `IMPLEMENTATION.md`.

The console entry remains `NotImplemented`; there is no collector, scheduler registration or UI yet. No real browser history was read. Linux execution, hosted CI, independent review, performance budgets and owner-run smoke checks remain unverified.
