# omoi

Local, low-overhead watch history for the PC: reads browser history (Chrome, Edge, Brave,
Firefox) into one SQLite file, with optional hourly collection and explainable local suggestions.

Status: a working command-line baseline collects video visits into SQLite and exports CSV/JSON. Windows scheduling and local recommendation queries are implemented. A graphical UI and measured playback capture remain future work. See `docs/m0-acceptance.md` for evidence and remaining acceptance gates.

## Use the command-line baseline

The local optimized binary is built at `D:\mariyaholic-build\omoi\target\release\omoi.exe`. It is not a published release. From PowerShell, replace `omoi` below with that full path if it is not on PATH.

```text
omoi --help
omoi paths
omoi collect --dry-run
omoi collect
omoi db info
omoi recommend --seconds 1200
omoi export --csv --out watched.csv
omoi export --json --out watched.json
```

`collect` reads supported browser history files through temporary snapshots. It stores normalized video identities, titles, visit times and available tab-duration approximations under `%LOCALAPPDATA%\omoi\history.sqlite`. This records openings, not proof of watching or completion. `--dry-run` reads the same sources but applies changes only to a temporary database. Review the discovered browser/profile scope before running either command on personal history. Close a browser and retry if Windows prevents copying its history.

The collector exits after each run. Run it again to collect new visits; unchanged profiles are skipped. Reloads within 30 minutes of a fixed view anchor merge into that view. `--browser chrome`, `--browser edge`, `--browser brave` or `--browser firefox` restricts a run; repeat the option for multiple browsers. Export requires one format and refuses to overwrite an existing destination. Keep export files outside Omoi's data directory.

Optional `%LOCALAPPDATA%\omoi\config.toml` uses the restricted grammar in the architecture contract:

```toml
browsers = ["edge", "firefox"]
profile_excludes = ["edge:Profile 2", "firefox:work"]
local_utc_offset = "+08:00"
```

Use an absolute `OMOI_HOME` to select a separate catalogue. This does not change browser discovery roots. Exit codes: 0 success; 1 fatal failure; 2 invalid arguments/config; 3 some profiles failed; 4 database/checkpoint contention; 5 unsupported platform. Collection prints a JSON summary and writes bounded counter-only logs. No cookie access, enrichment requests or cloud sync are implemented.

## Optional Windows automation

Place **both** `omoi.exe` and `omoiw.exe` in a stable directory before installing. The task captures that directory and the resolved catalogue location. Moving either binary requires reinstalling the task. Installation is per-user and needs no administrator account.

```text
omoi schedule status
omoi schedule install
omoi schedule remove
```

`install` creates/replaces only `\Omoi\Collect`. It collects hourly and 30 seconds after logon, only while the user is logged on. It does not wake the PC, skips/stops on battery, ignores overlapping triggers, and has a ten-minute execution limit. The collector exits between runs. The GUI-subsystem helper avoids a flashing console window. Registration uses the built-in Windows PowerShell Task Scheduler COM adapter; PowerShell exits after installation/inspection/removal and is not part of hourly collection. No task is installed automatically by the build or tests.

Status currently returns the task XML or an explicit not-installed message, not last-run health. Removal preserves the catalogue. Linux provides an unsupported-platform stub for these commands.

## Local suggestions

`omoi recommend --seconds 1200` returns explained JSON shelves. Add `--item ID` for similar items based on stored tags. Rediscover uses history older than 30 days; Fits your time needs known video duration; Continue watching needs actual recorded session position. Browser visits alone supply neither measured playback nor full video duration. Tags, durations and sessions have storage/query support, but this baseline does not yet provide their capture/editing flows. Empty shelves are expected. No model, paid API, remote recommendation service or continuous process is required.

## Current limits

There is no graphical UI or playback-time listener yet. Snapshotting live SQLite files is best-effort. Fixture-based CLI journeys and hosted Windows/Linux verification support trying the baseline; they do not establish acceptance on personal browser profiles. Real retained-week import, installed scheduler smoke, independent review and remaining strict contract cases stay open. Resources are measured on synthetic data; see `docs/m0-acceptance.md`. UI direction is in `docs/ui-recommendations.md`.

- Design: `docs/watch-history-architecture.md` (§0 holds Maria's decisions)
- Review: `docs/omoi-design-review.md`
- M0 build contract (units U1–U14 with acceptance tests): `docs/omoi-architecture-detail.md`
