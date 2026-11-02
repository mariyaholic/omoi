# omoi

Local, low-overhead watch history for the PC: reads browser history (Chrome, Edge, Brave,
Firefox) into one SQLite file, with time spent, cataloguing and recommendations planned.

Status: a working command-line baseline is implemented and fixture-tested on Windows GNU. It collects video visits into SQLite and exports CSV/JSON. Full M0 acceptance, automatic scheduling and the UI remain unfinished. See `PROGRESS.md` for exact evidence and remaining work.

## Use the command-line baseline

The local optimized binary is built at `D:\mariyaholic-build\omoi\target\release\omoi.exe`. It is not a published release. From PowerShell, replace `omoi` below with that full path if it is not on PATH.

```text
omoi --help
omoi paths
omoi collect --dry-run
omoi collect
omoi db info
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

Use an absolute `OMOI_HOME` to select a separate catalogue. This does not change browser discovery roots. Exit codes: 0 success; 1 fatal failure; 2 invalid arguments/config; 3 some profiles failed; 4 database/checkpoint contention. Collection prints a JSON summary; diagnostics contain no watched titles or URLs. No cookie access, enrichment requests or cloud sync are implemented.

## Current limits

There is no graphical UI, automatic task installation, playback-time listener or recommendation engine yet. Snapshotting live SQLite files is best-effort. The CLI and synthetic Chrome/Firefox journeys are verified locally; real-profile tests, Linux execution, independent review and resource budgets are still pending. Some strict M0 contract acceptance cases remain open in `PROGRESS.md`.

- Design: `docs/watch-history-architecture.md` (§0 holds Maria's decisions)
- Review: `docs/omoi-design-review.md`
- M0 build contract (units U1–U14 with acceptance tests): `docs/omoi-architecture-detail.md`
