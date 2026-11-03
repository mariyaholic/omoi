# M0 acceptance record — 2026-11-03

Historical CLI measurements and strict-contract map. [Headless acceptance on the owner's PC](headless-acceptance-20261103.md) is the current acceptance record and supersedes the earlier untested retained-week, installed-scheduler and UI/manager statements below.

Follow-through update: [current UI/playback notes](completion-20261103.md) and [new measurements](../ui/MEASUREMENTS.md) supersede earlier unavailable-feature statements. The isolated Windows install/run/remove smoke now passes and exposed/fixed a COM XML encoding defect; real logon/battery and retained-week acceptance remain open. An injected view-write failure now verifies item/view/checkpoint rollback and a successful retry. Earlier CLI resource measurements below remain tied to their original binary.

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

Final production source `4a74949`, with the release journey added at `98709be`, passes **48 Windows tests and 49 Linux tests** (the extra test checks the unsupported Linux scheduler/helper). Local formatting, Clippy with warnings denied, all-target tests and optimized build pass. Ruff and Python compilation checks pass; the release smoke runs locally and in [hosted run 41543519585](https://github.com/mariyaholic/omoi/actions/runs/41543519585), whose Ubuntu and Windows GNU jobs both succeeded. The release smoke checks deterministic fixtures/spacing, 1,200 source rows → 1,000 views/400 items, zero additions on rerun, export and honest empty recommendation shelves. Documentation/report-only commits after `98709be` do not change the measured binaries.

## Recorded Windows resources

Raw [baseline report](measurements/windows-baseline-20261103.json) and [long-title report](measurements/windows-long-titles-20261103.json) retain every measurement and counter. Baseline: 100,000 views / 40,000 items, 120,000 source rows including 10,000 cross-source duplicates and 10,000 irrelevant URLs, 40-byte titles. Twenty fresh full imports, twenty unchanged reruns, twenty ten-row updates, one metadata-only change and one WAL-only change. Final catalogue has 100,201 views after incremental checks.

Environment: Windows 11 build 26200, Intel Core i5-10300H, D: WD My Passport 2626 USB HDD, Rust 1.98.1 Windows GNU, bundled collector SQLite 3.50.2. Harness: Python 3.13.3, measurement/checkpoint SQLite 3.49.1. No local compilation ran during this final pass. Cold cache and unrelated OS load were not controlled.

| Case | Runs | Wall median / p95 / maximum | CPU median / maximum | Maximum lifetime peak |
|---|---:|---:|---:|---:|
| Full import | 20 | 16.752 / 23.367 / 32.598 s | 16.164 / 22.938 s | 10,829,824 bytes |
| Unchanged | 20 | 175 / 232 / 304 ms | 78 / 109 ms | 7,258,112 bytes |
| Ten new rows | 20 | 236 / 402 / 916 ms | 156 / 219 ms | 9,834,496 bytes |
| Metadata-only source change | 1 | 247 ms | 234 ms | 9,768,960 bytes |
| WAL-only source change | 1 | 864 ms | 203 ms | 9,867,264 bytes |

Maximum internal processing wall time excluding snapshot copying: unchanged **14 ms**, ten-new-rows **228 ms**, metadata-only **112 ms**, WAL-only **216 ms**. Maximum copy wall respectively: 0, 688, 12 and 530 ms. Whole-process wall time above includes launch/shutdown and harness observation; it is not interchangeable with the collector's internal timer. Metadata-only rescans advance the fingerprint but do not retroactively update titles for already-consumed visits.

Catalogue DB+WAL+SHM: **15,323,136 bytes** before and after harness checkpoint/connection close; page bytes equal that amount, freelist bytes 0. No VACUUM. Largest sampled temporary snapshot/spool usage: **30,140,206 bytes**, a 50 ms sampled lower bound rather than an exact maximum. It is additional to catalogue storage and removed on successful completion. Retained source fixtures are measurement artifacts, not app catalogue overhead. Final bounded logs: 464 bytes.

Applicable baseline gates pass in these runs: collector peak <30,000,000 bytes; routine processing and CPU each <1 second; catalogue ≤30,000,000 bytes; primary executable ≤6,000,000 bytes. There is no collector process between runs. No UI or listener memory/CPU gate is claimed, and no Linux resource measurements are fabricated.

Executable evidence:

- `omoi.exe`: **2,168,832 bytes**, SHA-256 `bf49a477a9b7e84575479aa319f31ecdcaf61b5e9d1f339ec3711e9267a3a8f6`.
- `omoiw.exe`: **2,172,928 bytes**, SHA-256 `e55deb58c91e4880f71c872da6ef5391f35618d725b9d1184579c74d7aeec26e`.

The separate 1,024-byte-title stress workload (one full import, three warm/changed repetitions) produced **66,498,560 bytes** of catalogue and **11,169,792 bytes** maximum full-import working set. Full import took 24.049 seconds wall / 20.109 seconds CPU, with **184,990,828 bytes** sampled temporary snapshot/spool usage. This exceeds the 30 MB baseline storage target by design and demonstrates that it is a workload budget, not a hard catalogue size cap. Do not promise a fixed 30 MB ceiling for arbitrary history. Three repetitions are descriptive, not a robust p95 claim.

## Acceptance map

| Area | Evidence available | Remaining strict acceptance gaps |
|---|---|---|
| U1–U3 types/config/database | Cursor/config tests, migration rollback/idempotence, FTS insert/update/delete, foreign keys, future-schema rejection, read-only no-create/no-write, WAL mode; November headless pass adds busy exit code 4 and successful retry with rollback | Independent review of complete named contract coverage |
| U4 discovery | Multi-profile Chromium, Firefox INI, explicit paths, exclusions, bounded INI reads | Denied-access test covers missing file, not injected permission error; symlink/strict malformed INI variants |
| U5 snapshot | Deterministic three-attempt race, sidecar copy and cleanup, WAL-only fingerprint; pipeline observes committed Firefox WAL | Not every named contract case has an independent test or fault injection |
| U6/U7 readers | Epochs, negative Firefox times, tied tuple pagination, optional durations, 512-row bounds, reset after source replacement, oversized/NULL rows; missing-table partial failure leaves that source's cursor unchanged | Orphan-join accounting; underlying reader errors currently classified by collector |
| U8 normalization | Contract table vectors, UTF-8/percent local identities, filename case, encoded separator and UNC rejection; midnight and duplicate identity-parameter regressions | Independent review of complete named contract coverage |
| U9 dedupe | Fixed anchor, source/title richness, max duration, transaction/FTS rollback, concurrent collectors; inclusive ±1800-second and deterministic tie vectors | Deterministic stale-prepared-checkpoint race |
| U10 export | CSV quote/newline round-trip, JSON null/order, multiple views per item, no-clobber, broken-pipe regression; 100,201-row export measurement and JSON write batching in the current headless record | Independent review of complete named contract coverage |
| U11 logs | Global cap/redaction and stale-lock tests; dry-run avoids persistent collection logs; logging failure preserves an existing lock | Per-source structured diagnostic counts |
| U12 scheduler | XML options/escaping/argv quoting, Linux unsupported stub; actual uniquely named Windows task install/run/remove with zero last result and absence verified | Actual hidden-window observation, battery/logon transition; native adapter difference below |
| U13 orchestration | End-to-end collect/rerun, new rows, partial corruption, dry run, concurrent writers; retained-week snapshot import/rerun and integrity; write contention rollback/retry | Manual known-video comparison; distinct commit-failure injection; per-source rejected counts (current report is aggregate) |
| U14 resources | Determinism/spacing/counts/isolated-path release journey on Windows and Linux; Windows lifetime OS counters, 20-run distributions and separate long-title stress | Linux resource counters; exhaustive named generator suite beyond the release journey; controlled cold cache and exact transient disk peak |

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
