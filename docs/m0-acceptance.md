# M0 acceptance record — 2026-11-03

Decision: **usable CLI baseline; full M0 acceptance remains open**. This is a measured implementation record, not a release certificate. All history used for automated checks and resource runs is generated locally. No personal history or cookies were read, and no scheduled task was installed.

## Implemented behavior

- Bounded browser readers collect Chrome/Edge/Brave and Firefox history through private snapshots. Fingerprints cover main/WAL/journal. Unchanged sources skip scanning; malformed rows advance cursors where possible. NULL timestamps count as rejected on each changed-source scan.
- Dedupe/checkpoints commit together, with one retry on contention. Dry runs use temporary catalogues. Partial profile failures preserve good sources and return exit 3.
- CSV/JSON exports stream and refuse destination overwrite. Read-only database inspection uses a consistent transaction. Percent-encoded local paths canonicalize UTF-8 and preserve filename case.
- Counter-only JSONL logs have per-file and total caps; stale logging locks are preserved. Weekly `PRAGMA optimize` is separate from ingestion; no automatic VACUUM.
- Windows optional hourly/logon task, hidden helper, install/status/remove. The task captures home and binary location. It skips/stops on battery, ignores overlaps, and does not wake the PC. Read-only `schedule status` on this PC returned **Omoi task is not installed**.
- Local recommendation shelves: rediscover, measured continue-watching, shared tags and known-duration timeboxes. Missing metadata yields empty shelves; no external adapter or model call is made. Browser ingestion itself does not populate playback sessions, tags or full video durations.

## Verification

The earlier scheduling/recommendation revision `1d7d719` passed formatting, Clippy, all 38 tests and optimized builds locally. [Hosted run 41541772350](https://github.com/mariyaholic/omoi/actions/runs/41541772350) passed Ubuntu and Windows GNU formatting, lint, tests and release builds.

Final revision verification and resource figures are appended below after the expanded run. A hosted check for one revision is not evidence for later edits until their own run succeeds.

## Acceptance map

| Area | Evidence available | Remaining strict acceptance gaps |
|---|---|---|
| U1–U3 types/config/database | Cursor/config tests, migration rollback/idempotence, FTS insert/update/delete, foreign keys, future-schema rejection, read-only no-create/no-write, WAL mode | Additional injected connection contention coverage |
| U4 discovery | Multi-profile Chromium, Firefox INI, explicit paths, exclusions, bounded INI reads | Denied-access test covers missing file, not injected permission error; symlink/strict malformed INI variants |
| U5 snapshot | Deterministic three-attempt race, sidecar copy and cleanup, WAL-only fingerprint; pipeline observes committed Firefox WAL | Not every named contract case has an independent test or fault injection |
| U6/U7 readers | Epochs, negative Firefox times, tied tuple pagination, optional durations, 512-row bounds, reset after source replacement, oversized/NULL rows | Explicit missing-table error classification and orphan-join accounting; underlying reader errors currently classified by collector |
| U8 normalization | Contract table vectors, UTF-8/percent local identities, filename case, encoded separator and UNC rejection | Separate midnight and duplicate-parameter named tests still to expand |
| U9 dedupe | Fixed anchor, source/title richness, max duration, transaction/FTS rollback, concurrent collectors | Deterministic stale-prepared-checkpoint race and full boundary/tie vector table |
| U10 export | CSV quote/newline round-trip, JSON null/order, multiple views per item, no-clobber, broken-pipe regression | Dedicated 100k-row export memory measurement |
| U11 logs | Global cap/redaction and stale-lock tests; dry-run avoids persistent collection logs | Per-source structured diagnostic counts and explicit logging-failure sink assertions |
| U12 scheduler | XML options/escaping/argv quoting, Linux unsupported stub, Windows COM read-only absence query | Actual install/launch/remove and hidden-window/battery/logon smoke; native adapter difference below |
| U13 orchestration | End-to-end collect/rerun, new rows, partial corruption, dry run, concurrent writers, session capture stub | Owner retained-week import; injected write/commit failure with cursor rollback; per-source rejected counts (current report is aggregate) |
| U14 resources | Deterministic generator; Windows lifetime OS counters and saved reports | Linux resource counters; exhaustive generator determinism/spacing named suite; controlled cold cache and exact transient disk peak |

The table maps behavior, not a claim that all mandatory test names in the original contract exist. Some combined integration tests cover multiple assertions. The outstanding rows remain work; they are not waived by green CI.

## Implementation differences requiring review

The original scheduler contract specified native Rust Windows COM plus `schtasks`. This implementation uses an embedded, static Windows PowerShell script calling the same Task Scheduler COM service. Paths/modes are separate process arguments; paths are never interpolated into executable script text. PowerShell runs only during explicit install/status/remove, not hourly collection. This avoids expanding native COM bindings, but has not satisfied the original native-argv acceptance case. Treat it as a documented design difference pending review, not identical implementation.

The resource harness is Python stdlib with Windows OS counters, rather than the specified PowerShell wrapper. It retains each child process handle and reads lifetime peak working set and CPU after exit. Sampled temporary disk usage is only a lower bound. Logs contain summary counts rather than full per-profile structured records. `schedule status` reports installed configuration, not last-run health.

## Owner acceptance procedure

1. Put both executables in a stable local directory. Review `omoi paths` and the configured browsers/profile exclusions. `collect --dry-run` still reads selected history, so this is an intentional owner action.
2. Import the retained week and inspect the JSON rejected/failed counts and exports. Rerun with no new visits: expect zero inserted views and unchanged source skips. Check several known videos manually across the selected sources.
3. Only when automation is wanted, run `schedule install`, inspect `schedule status`, and inspect `\Omoi\Collect` in Windows Task Scheduler. Trigger once, verify no console flash and a new bounded log, then verify logon/hourly behavior and battery settings. `schedule remove` must leave history intact and remove only that task.
4. Obtain independent review of the adapter difference and remaining fault-injection cases before declaring full M0 accepted. Continue UI work independently; do not silently label the open gates complete.

## Reproduce synthetic measurements

Build with `cargo build --locked --release --bins --example synthetic`. On Windows, run the following in the repository; output directories must not already exist:

```text
python scripts/measure.py --exe D:\mariyaholic-build\omoi\target\release\omoi.exe --generator D:\mariyaholic-build\omoi\target\release\examples\synthetic.exe --out D:\mariyaholic-build\omoi\measure-reproduce --runs 20 --full-runs 20
```

The harness isolates browser discovery roots, generates explicit fixture profiles, and deletes only known generated catalogue files between full imports. It retains fixtures and raw reports under the selected output directory. `--long-titles` adds a separate 1,024-byte-title stress workload; its storage size is not covered by the 40-byte-title baseline budget. The harness does not install a task.
