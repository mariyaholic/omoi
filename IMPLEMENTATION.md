# Omoi implementation handoff

Historical foundation roadmap, prepared 2026-11-03 for Astra and local implementers.
Omoi now runs; the foundation starting point below is historical. For the current
installation-friction assignment, start with [CLOUD_HANDOFF.md](CLOUD_HANDOFF.md)
and its linked plan. Do not restart U1 or treat the old missing-reader error as current.

Current implementation evidence and remaining work are in [PROGRESS.md](PROGRESS.md); its status supersedes the historical starting point below.

## Authority and horizon

Maria authorizes Astra to work autonomously across the Omoi roadmap, beginning with M0 U1 and continuing through U2–U14 and, where feasible, M1–M5. Do not stop at a unit boundary merely to ask whether to continue. Plan, implement, test, debug, review, document and prepare reviewable branches or draft PRs within the existing design and the boundaries below. Use the existing code and contract as starting points, not as a reason to stop when a routine implementation detail is unspecified. Make reasonable reversible choices, record them, and keep going.

Treat milestone acceptance checks as evidence gates, not automatic stopping points. If a check needs an unavailable platform, human interaction, credentials, a separate reviewer or a long-running resource measurement, record it as pending and continue independent work that does not depend on its result. Do not call that milestone complete or treat dependent behavior as proven. If an independent reviewer is available, use one; if not, self-review the diff, report that review is pending, and continue useful work. Address defects found by review without waiting for a new instruction.

Preserve the owner's decisions and privacy scope. Automated tests use synthetic fixtures; real browser profiles, cookies and history are not used or transmitted during development. Do not install a scheduler task, perform real-profile smoke tests, enable cookie-based ingestion, publish a public release, change repository visibility, or use paid services or credentials without explicit owner authorization. Use separate feature branches and reviewable PRs; do not push or rewrite `main`. If a task requires an owner decision or unavailable access, make the issue concrete, document the smallest next action, and continue other independent authorized work.

## Starting point

- M0 U1 is partial: the manifest, lockfile, error model and placeholder entry point exist. `src/lib.rs` references the missing `src/readers/mod.rs`; `tests/types.rs` does not exist. U2–U14 have not been implemented.
- The most recent completed compiler result in the prior handoff was E0583 for `readers`. An offline check attempted on 2026-11-03 was interrupted after it produced no output, so do not treat it as fresh compiler proof. Inspect the current environment and rerun the baseline.
- Design authority: [M0 build contract](docs/omoi-architecture-detail.md), especially §14 and the referenced sections; [architecture decisions](docs/watch-history-architecture.md) §0 and reconciliation note; [feature inventory](FEATURES.md); [original handoff](HANDOFF.md). If specifications conflict, record the conflict and use the reconciliation hierarchy rather than silently inventing behavior.
- The existing `WORKSTACK.md` is local and excluded from Git. Use this tracked file and the contract as the portable handoff; update the local WORKSTACK when available.

## Work method

1. Confirm the checkout, branch, clean/dirty state, toolchain and current `cargo check --locked` result. Do not overwrite existing edits. The old `D:\rust\.cargo\bin\cargo.exe` path is stale on this machine; `D:\rust\cargo\bin\cargo.exe` was discoverable on 2026-11-03. Verify `rustup`, `RUSTUP_HOME`, `CARGO_HOME`, the toolchain and MinGW before relying on them. Keep build artifacts on D: (`CARGO_TARGET_DIR`), as available. Avoid modifying a persistent rustup override merely to perform a check.
2. Work on feature branches. Start with U1 and proceed through the roadmap; use §14 of the contract for exact public types, file paths, dependencies and named acceptance tests. Write fixture-based tests first, implement the minimum, and run the relevant tests. Keep the lockfile and dependency constraints consistent with the contract.
3. For each subsequent unit, implement its named acceptance tests and dependencies, run the applicable tests and `cargo fmt --check`, then run `cargo test --all-targets` once the crate builds. Review the diff and record results and remaining gaps before the next unit. Independent review should check the actual diff and test output, not just an agent summary.
4. Use synthetic browser databases and isolated temporary data directories in automated tests. Do not collect, print, export, upload or transmit the user's real browser history or cookies during development. Scheduler installation and real-profile smoke tests are separate, explicit user-run validation steps.
5. Report failures, unavailable environments and unmeasured budgets accurately. Do not mark a unit complete based on a stub or ignored test. Update the feature inventory and handoff as behavior lands; distinguish implementation from local, Windows, Linux and hosted evidence.

## M0 sequence and gates

The full test names and contracts are in detail §14; these checkpoints do not replace them.

| Sequence | Deliverable | Essential proof |
|---|---|---|
| U1 | Reader contracts, types and error mapping | Three named U1 tests, compilation, serializable cursors and distinct source IDs |
| U2–U3 | Paths/config and SQLite migration | Config validation, transactional migration, future-version rejection, FTS and foreign keys |
| U4–U5 | Browser profile discovery and consistent snapshots | All profiles by default, `profile_excludes`, Firefox/Chromium fixtures, sidecar handling, bounded retry and cleanup |
| U6–U8 | Chromium and Firefox readers; URL normalizer | Epoch conversion, pagination and cursor progress, schema failures, URL vectors, no network |
| U9–U11 | Dedupe, exports and bounded logs | Transaction/concurrency cases, streaming/no-clobber exports, redaction and size caps |
| U12–U14 | Scheduler, collect/CLI, synthetic measurement | Scheduler XML/argument tests; atomic checkpoints, dry-run and idempotence; deterministic fixtures and resource measurement |

M0 integration requires `cargo test --all-targets` on Windows GNU and Linux, with the named critical tests enabled, plus the contract's real-data and Windows scheduler smoke checks when explicitly run. Report each environment separately. Record actual memory, CPU, disk and executable-size measurements before asserting the budget targets. `cargo check` alone is not a test or a performance measurement.

## Later milestones

Once the relevant M0 foundations exist, M1 adds a local read-oriented timeline/search UI and the approved delete flow; tags arrive in M4. If an M0 evidence gate remains pending, later work may proceed only where it does not depend on the missing evidence, and the gate remains open. M1.5 measures thumbnail size with the specified 50 MB test stop and proposed 100 MB cache cap; the cache does not ship until the measurement gate passes or the owner decides otherwise. M2 covers Netflix CSV and local-player imports; Takeout remains optional. M3 adds the optional listener, userscripts and session processing in `omoi collect`. M4 adds metadata and tagging; M5 adds recommendation shelves. See architecture §14 and detail §§8, 11–13 before designing these stages; do not present targets as measured behavior.

Preserve Maria's decisions: yt-dlp/cookie collection remains off; platform priority is YouTube, Netflix, Twitch, Bilibili, then Prime, Disney+ and Crunchyroll; discover supported profiles by default with exclusions. Phone/TV account ingestion is optional and outside the core PC milestones.

## Ready-to-use long-horizon assignment

> Read `IMPLEMENTATION.md`, `HANDOFF.md` and the relevant sections of `docs/omoi-architecture-detail.md`. Inspect the working tree and verify the toolchain and baseline. Finish M0 U1, including the reader types, trait and three named acceptance tests; then continue autonomously through U2–U14, and the later milestones as feasible. Use isolated fixtures, preserve existing edits, implement and test each increment, review diffs, and keep a durable ledger of exact results, pending checks and next actions. Use a feature branch and prepare reviewable PRs. Do not pause for routine choices or at unit boundaries. Respect the owner and privacy boundaries in `IMPLEMENTATION.md`; when a gate cannot be run, record it accurately and continue independent work.

The per-unit tests and milestone gates remain in force throughout. The owner has authorized continued implementation, not a claim that unrun checks have passed or permission to deploy or publish.
