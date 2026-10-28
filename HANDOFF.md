# Omoi — handoff (M0 boilerplate)

Prepared by Ava.tower, 2026-10-24 (Asia/Manila). The free models hit their usage limits, so this is a partial M0 handed over for you to continue locally.

> **Update 2026-10-28 (claude.scout):** Omoi lives in its own local repo with no GitHub remote yet. The docs carry Maria's Q3/Q6/Q7/Q8 answers (2026-10-25) and the conflicts between them are resolved (see the Reconciliation note in the architecture draft; two small decisions are recorded there and in detail §8 and §13). Two newly decided points a builder will meet: M3 sessionizing runs inside `omoi collect`, and the M1 UI is delete-only (tags are M4). U4 also gains `profile_excludes` and the test `profile_excludes_applied` (detail doc §3 and §14). Build with the `D:\rust` GNU toolchain (cargo 1.98.1). `cargo check --locked` currently fails only with E0583 for `readers`, as described below.

## Where things are

Everything is in this repo, `C:\Users\<you>\Desktop\mariyaholic\omoi` (local, no remote), on `main`:

- `docs/watch-history-architecture.md`: draft 2, renamed to Omoi, with Maria's decisions in §0. Its "Reconciliation note" says where the detail doc governs.
- `docs/omoi-design-review.md`: the Ava.tower review.
- `docs/omoi-architecture-detail.md`: the M0 build contract by Astra (gpt-6-astra). **This is the document to build from.**
- `ARCHITECTURE.md`: module map, process model and budgets in one place. `FEATURES.md`: 49 features with milestone, state and the tests that verify them.
- The code is **partial and does not build yet** (see below).
- The box's `omoi-box.bundle` audit bundle is not in this repo.

## Done

| Item | By | State |
|---|---|---|
| Docs: rename, Decisions (2026-10-24), sources/milestones/§15 | Ava.tower | done |
| Design review (claims spot-check, listener security, roster) | Ava.tower | done |
| Architecture detail / M0 contract (14 units U1–U14) | Astra, `gpt-6-astra` (Codex) | done |
| U1 crate/types/errors | `opencode/deepseek-v4.1-flash-free` | **partial**: `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/main.rs`, `src/error.rs` are written. `src/readers/mod.rs` and `tests/types.rs` are missing, so `cargo build` fails with E0583 (`readers` module not found). |
| U2–U14 | — | not started |

What happened with the free models (2026-10-24, about 18:18–18:19 Asia/Manila):
- DeepSeek v4.1 flash wrote the U1 files above, then hit "Free usage limit reached".
- GLM 5.3 flash and MiMo v2.6 flash returned the same limit straight away.
- `opencode/minimax-m3.1-flash-free` returned "Model unavailable"; it is no longer in OpenCode's model list.

No paid model was used.

Astra's review of the partial: **APPROVE-AS-PARTIAL**.
- Matches the contract: `Cargo.toml` follows §1 and the `Error` enum and exit-code mapping follow §9.
- Gaps: the missing `readers` module (compile blocker), the missing reader types and `HistoryReader` trait, and the three named U1 tests.

## Remaining units (in order), with acceptance tests

Full detail is in `docs/omoi-architecture-detail.md` §14 and the sections it references. Finish U1 first.

### U1 — crate/types/errors
- Files / deps: `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/main.rs`, `src/error.rs`, `src/readers/mod.rs`, `tests/types.rs`; none. Empty declared modules may be created here.
- Public API: `Error`, `Result`, Browser/SourceId/Cursor/RawVisit/Batch, HistoryReader; main placeholder exits NotImplemented until U13.
- Acceptance tests: `cursor_json_roundtrip`: serialize/deserialize tuple `(100,7)` and all three stamps unchanged. `error_exit_mapping`: Config→2, Busy→4, Unsupported→5, Db→1. `source_id_distinguishes_profiles`: two same-basename absolute profile paths yield different IDs.

### U2 — paths/config
- Files / deps: `src/paths.rs`, `src/config.rs`, `tests/config.rs`; U1.
- Public API: `Paths::resolve(&PathEnvironment)`, `Config::load(path)`, `Config::parse(&str)`. Inject env map, no global-env mutation in tests.
- Acceptance tests: `home_override_precedes_os`: absolute override wins both simulated OS roots; `linux_xdg_fallback`: missing XDG uses HOME/.local/share/omoi; `config_defaults_and_errors`: missing file defaults batch512/+08; reject unknown/duplicate key, table, relative home, timeout0; `config_comments_in_strings`: `#` inside quoted path survives.

### U3 — schema/migrations
- Files / deps: `src/db/mod.rs`, `schema.rs`, `migrations.rs`, `0001.sql`, `tests/db.rs`; U1–U2.
- Public API: `db::open`, `migrate(&mut Connection)`, `info(&Connection)`.
- Acceptance tests: `migration_0001_idempotent`: migrate twice, version1 and seven ordinary tables plus FTS; `migration_rollback`: injected invalid migration leaves version0/no items; `reject_future_schema`: user_version2 fails; `fts_insert_update_delete`: alpha searchable, update beta removes alpha, delete removes beta; `foreign_key_cascade`: deleting item removes view/session/tags; `required_columns`: channel_id and sessions.source exist.

### U4 — profile discovery
- Files / deps: `src/readers/discover.rs`, `tests/discover.rs`; U1–U2.
- Public API: `discover(config,env)->Result<Discovery>`, Profile/Discovery types.
- Acceptance tests: `chromium_all_profiles`: Default/Profile 2/custom included, System/Guest excluded; `firefox_ini_paths`: relative and absolute fixtures resolved/deduplicated; `discovery_missing_and_denied`: missing root harmless, injected permission error counted; `explicit_profile_filter`: firefox override excluded by chrome-only filter.

### U5 — snapshot manager
- Files / deps: `src/readers/snapshot.rs`, `tests/snapshot.rs`; U1–U2,U4.
- Public API: `snapshot`, Fingerprint/Snapshot/SnapshotOutcome; injected filesystem hooks only for deterministic race tests.
- Acceptance tests: `skip_unchanged_triplet`: identical main/wal/journal stamps returns Unchanged; `wal_only_change_copies`: WAL length/mtime change returns Ready; `snapshot_preserves_sidecars`: private names match main and recovery sees committed WAL row; `snapshot_retry_bound`: mutate source on every copy, exactly3 attempts then Source error, cursor unchanged; `snapshot_cleanup`: owned temp dir removed on error.

### U6 — Chromium reader
- Files / deps: `src/readers/chromium.rs`, `tests/chromium.rs`; U1,U5.
- Public API: `ChromiumReader::open`, HistoryReader implementation, checked epoch helper.
- Acceptance tests: `chrome_epoch`: epoch and epoch+1e6 map to0/1 seconds; `chrome_tuple_pagination`: 3 rows same time and limit2 yield all IDs once; `chrome_duration_optional`: absent column→None, 2.5e6→2; `chrome_reject_long_url_advances`: oversized row rejected but final tuple includes it; `reader_batch_bound`: no batch exceeds requested512.

### U7 — Firefox reader
- Files / deps: `src/readers/firefox.rs`, reader factory in `mod.rs`, `tests/firefox.rs`; U1,U5.
- Public API: `FirefoxReader::open`, HistoryReader, `open_reader`.
- Acceptance tests: `firefox_prtime`: 1000001→1s and -1→-1s; `firefox_join_and_cursor`: joined URL/title and tied timestamps page correctly; `firefox_duration_none`: duration always None; `browser_schema_failure_is_source`: missing moz_places yields Source, never migrates source; `cursor_reset_rescans`: max below checkpoint restarts scan.

### U8 — normalizer
- Files / deps: `src/normalize.rs` (split platform helpers if needed), `tests/normalize.rs`, test vector data; U1–U2.
- Public API: `normalize`, ItemKey/Normalized/Platform.
- Acceptance tests: `normalizer_url_vectors`: every §6 table row asserted; `twitch_local_midnight`: 15:59:59Z vs16:00:00Z keys differ at +08; `normalizer_identity_params`: p1/default merge, p2 distinct, duplicate v rejected; `normalizer_no_network`: pure function has no transport argument or side effect; `local_path_case`: drive letter normalized, filename case retained.

### U9 — transactional dedupe
- Files / deps: `src/dedupe.rs`, `tests/dedupe.rs`; U3,U8.
- Public API: `Candidate`, `ingest(tx,&Candidate)`, rank helper, IngestOutcome.
- Acceptance tests: `dedupe_window_vectors`: all §7 rows except orchestration cases; `dedupe_fixed_anchor`: 0/1200/2400 creates2; `dedupe_richness`: firefox then chrome retains anchor and upgrades title/source; `dedupe_rollback`: dropped transaction leaves0 items/views; `dedupe_two_writers`: two independent connections contend, no duplicate after retry; `dedupe_approx_max`:10+20→20.

### U10 — export
- Files / deps: `src/export.rs`, `tests/export.rs`; U3.
- Public API: `export(conn,format,writer)`, `export_to_path(...)->Result<()>`.
- Acceptance tests: `csv_escaping`: comma/quote/newline title round-trips using csv reader; `json_null_and_order`: null preserved, ordered timestamps/IDs; `export_one_row_per_view`:2 views→2 records despite one item; `export_no_clobber`: preexisting output bytes unchanged/error; `export_streaming`:100k records emitted without collecting Vec of rows.

### U11 — bounded logging
- Files / deps: `src/logging.rs`, `tests/logging.rs`; U1–U2.
- Public API: `Logger::record(&LogRecord)`, validated log settings.
- Acceptance tests: `logs_global_cap`: simulated PIDs/rotation never exceed3×1MiB; `logs_redact_and_truncate`: private URL/title/token absent and oversized message still valid JSON ≤4096 bytes; `log_lock_fallback`: occupied lock writes stderr sink without failing collect; `dry_run_no_log_files`: no real log directory created.

### U12 — scheduler/helper
- Files / deps: `src/scheduler.rs`, `src/scheduler/windows.rs`, `src/bin/omoiw.rs`, `tests/scheduler.rs`; U1–U2,U11; helper entry delegates to U13 when integrated.
- Public API: `task_xml(&TaskSpec)`, `task_argv`, `install/remove`, `helper_run`.
- Acceptance tests: `schedule_xml_contract`: literal XML snapshot contains PT1H/logon/InteractiveToken/priority7/battery/hidden and matching principal; `schedule_escapes_paths`: ampersand/quotes round-trip expected XML escapes and argument quoting; `schedule_argv_no_shell`: exact arrays in §10; `schedule_linux_stub`: install/remove/helper return Unsupported, no files/processes created. Windows manual smoke separately required.

### U13 — collect/CLI/session stub
- Files / deps: `src/collect.rs`, `src/cli.rs`, `src/main.rs`, `src/sessionize.rs`, `tests/cli.rs`, `tests/collect.rs`; U1–U12. Private `collect/spool.rs` helper if needed to keep files small.
- Public API: `cli::run(args)->ExitCode`, `collect`, `sessionize::available/run`.
- Acceptance tests: `cli_surface`: exact subcommands; csv+json and invalid browser→2; `collect_week_idempotent`: fixture week imports then0 additions; `collect_partial`: one corrupt/one good profile→3, good committed, bad checkpoint absent; `collect_atomic_checkpoints`: injected write error leaves rows/cursors unchanged; `collect_dry_run`: real DB/checkpoints bytes/log directory unchanged, predicted counts correct; `collect_concurrent_checkpoint`: stale prepared run cannot overwrite newer cursor; `collect_nonvideo_progress`: rejected-only batch advances cursor; `sessionize_m0_stub`:false/NotImplemented.

### U14 — fixture/measurement harness
- Files / deps: `examples/synthetic.rs`, `scripts/measure.ps1`, `tests/synthetic.rs`; U13.
- Public API: example parser/generator (testable helper module); documented harness args in §12.
- Acceptance tests: `synthetic_counts`: 100000 views/40000 items after ingest, second run0; `synthetic_deterministic`: same seed→same ordered logical source rows; `synthetic_spacing`: repeated item anchors >1800s, duplicates ≤60s; `synthetic_no_user_paths`: every generated path below supplied temp root. Linux generator/collector tests required; Windows counters validated on Windows, no fabricated Linux working-set values.

Integration gate: `cargo test --all-targets` must be green on Linux and on Windows GNU. The named FTS, sidecar, concurrent-writer, dry-run and cap tests must not be `#[ignore]`d.

## Build and test on Windows (D:\rust GNU toolchain; MSVC link is broken)

```powershell
cd C:\Users\<you>\Desktop\mariyaholic\omoi
# use the GNU toolchain from D:\rust (MinGW gcc/ar must be on PATH for bundled SQLite)
rustup toolchain list                      # expect stable-x86_64-pc-windows-gnu
rustup override set stable-x86_64-pc-windows-gnu
cargo build
cargo test --all-targets
cargo build --release --bins               # target omoi.exe <= 6,000,000 bytes
```

Notes:
- Use rustc 1.88 or newer. The `url` → `idna` → `icu_*` dependency chain needs it, and 1.85 fails. The box tested with 1.99.1; the local `D:\rust` toolchain is cargo/rustc 1.98.1, which also satisfies it.
- Don't install or use the MSVC toolchain.
- Windows-only code (Task Scheduler, `omoiw.exe`) sits behind `#[cfg(windows)]`, with Linux stubs that return `Unsupported`.

## Continuing with the free models (never paid)

OpenCode free tier only, in this order:
1. `opencode/deepseek-v4.1-flash-free`
2. `opencode/glm-5.3-flash-free`
3. `opencode/minimax-m3.1-flash-free` (currently unavailable, so skip it)
4. `opencode/mimo-v2.6-flash-free`

Run one unit per prompt, for example:

```
opencode run --auto -m opencode/deepseek-v4.1-flash-free "Implement ONLY unit U1 from docs/omoi-architecture-detail.md §14 (files, public API, named acceptance tests). Keep earlier tests passing; cfg(windows) for Windows-only code with Linux stubs; no git; no new deps beyond §1; run cargo test until green."
```

Commit each unit only when `cargo test --all-targets` is green. Then have a different model review it (Astra, `gpt-6-astra`), not the model that wrote it.

## Open questions for Maria

- Q3: S4 yt-dlp with cookies. Still open, and S4 stays off.
- Q6: keep Bilibili at #2 or move it after Twitch?
- Q7: is a 250 MB thumbnail cap OK?
- Q8: should M0 read all browser profiles, or a named list?
