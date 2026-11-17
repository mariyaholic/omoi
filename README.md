# omoi

Local, low-overhead watch history for the PC: reads browser history (Chrome, Edge, Brave,
Firefox) into one SQLite file, with optional hourly collection and explainable local suggestions.

Status: the CLI and UI share one catalogue. History, search, filters, deletion, selected-profile collection, local suggestions, metadata/tag editing, exports and scheduler controls are implemented. Optional userscript capture feeds measured playback into the catalogue. All five themes remain. See [headless acceptance](docs/headless-acceptance-20261103.md), [UI instructions](ui/README.md) and [playback setup](userscripts/README.md).

On Windows, launch `omoi-desktop.exe` for a standalone Omoi window without browser tabs, an address bar or a console. It uses the same catalogue and five themes. Close the window to stop its UI server. The optional browser interface remains available through the CLI.

## Open the app

Next development task: [cloud implementation handoff](CLOUD_HANDOFF.md) and
[installation-friction roadmap](docs/installation-friction.md). These describe
planned setup improvements; the current setup instructions below still apply.

The [runtime state-machine inventory](docs/state-machines.md) describes transition
ownership, retry/recovery rules, deletion semantics and known limits.

```text
omoi-desktop.exe
```

In the portable Windows ZIP, extract all files and double-click **omoi-desktop.exe** or **Run Omoi.cmd**. Starting the app opens the existing catalogue, or creates an empty one. To import, choose **Sources**, select profiles and confirm **Collect selected**. No collection or scheduled task is started automatically.

The desktop uses loopback port 43823 and runs until its window closes. `WebView2Loader.dll` must remain beside the executable. The optional `omoi ui --open` command opens the browser interface on port 43822 with a ten-minute idle timeout; `--port 0` selects a free port for that mode. Use the app's Refresh button to reload catalogue data.

Settings includes **Uika**, **Confluence**, **Panel**, **Mariyaholic-98**, and **Argonaut Oak**. Appearance persists in the app's webview profile. Export is in Settings; Windows scheduling controls are in Sources. CLI commands remain available.

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

The collector exits after each run. Unchanged profiles are skipped; changed profiles are reconciled in bounded batches so backfilled visits, replaced history files and updated observations are seen. This costs a scan of each changed profile. Reloads within 30 minutes of a fixed view anchor merge into that view. `--browser chrome`, `--browser edge`, `--browser brave` or `--browser firefox` restricts a run; repeat the option for multiple browsers. Export requires one format and refuses to overwrite an existing destination. Keep export files outside Omoi's data directory.

Deleting an item removes its catalogue details, views and sessions. Deletion tombstones and a minimal per-producer live Twitch frontier (identity, sequence, timestamp, cumulative counter and stopped state) suppress old history and pending continuous playback across processing batches. Valid suppressed samples advance this control record without restoring viewing rows. These control records remain after deletion; they do not retain titles or view/session rows, and non-live playback does not gain this retained frontier. A later opening can add the item again with a new numeric ID. Schema 4 adds the durable frontier on first writable open. Stop older Omoi processes and back up the catalogue before upgrading; schema-3 binaries cannot reopen schema 4. See [playback state and legacy-queue recovery](docs/state-audit.md).

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

`install` creates/replaces only `\Omoi\Collect`. It collects hourly and 30 seconds after logon, only while the user is logged on. It does not wake the PC, skips/stops on battery, ignores overlapping triggers, and has a ten-minute execution limit. The collector exits between runs. The GUI-subsystem helper avoids a flashing console window. Registration uses the built-in Windows PowerShell Task Scheduler COM adapter; PowerShell exits after installation/inspection/removal and is not part of hourly collection. The build and Rust unit suite do not install tasks.

Status returns the registered task XML and last-run result, or an explicit not-installed message. Removal preserves the catalogue. Linux provides an unsupported-platform stub for these commands. `scripts/verify_schedule.py` explicitly creates, launches and removes a separate synthetic task for acceptance.

## Local suggestions

`omoi recommend --seconds 1200` returns explained JSON shelves. Add `--item ID` for similar items based on stored tags. Rediscover uses history older than 30 days; Fits your time needs known video duration; Continue watching needs actual recorded session position. Browser visits alone supply neither measured playback nor full video duration. Edit tags/duration in item details or enable optional userscript playback capture. Shelves stay empty when the required evidence is absent. No model, paid API or remote recommendation service is required.

## Current limits

A clickable [visual prototype](prototype/index.html) demonstrates History, Suggestions, Sources and Settings with fictional data. Open its `index.html` directly in a browser; see [prototype instructions](prototype/README.md). It is separate from the working CLI and does not collect history or install tasks.

The connected UI is embedded in `omoi.exe`; opening `prototype/index.html` still opens the separate fictional-data study. The UI edits metadata/tags and processes the playback inbox; the optional listener must be started separately. Snapshotting live SQLite files is best-effort. The owner-authorized retained-week snapshot import and isolated installed-task test have passed. Further acceptance work is deferred by the owner for this private development merge and retained locally. The [verification summary](docs/headless-acceptance-20261103.md) records measured results and their process/workload boundaries.

- Design: `docs/watch-history-architecture.md` (§0 holds Maria's decisions)
- Review: `docs/omoi-design-review.md`
- M0 build contract (units U1–U14 with acceptance tests): `docs/omoi-architecture-detail.md`
