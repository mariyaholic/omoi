# Headless acceptance on the owner's Windows PC

This record supersedes the open-item lists in the earlier November 3 completion and UI measurement notes. Testing used isolated catalogues, hidden child processes and headless browsers. The normal browser profile, catalogue and Omoi scheduled task were not configured or replaced. This is acceptance evidence for implemented behavior, not completion of the entire M0–M5 roadmap.

## Results

| Area | Result and boundary |
|---|---|
| Retained-week history | Passed the automated import, zero-addition rerun and SQLite integrity checks using read-only snapshots of the existing last seven days. Per-profile counts and warnings are in a local aggregate receipt outside Git. Temporary personal snapshots and the test catalogue were deleted before writing that receipt. No cookies, account login or history upload. This is not seven newly elapsed days of observation or a manual check of known videos. |
| Five-theme UI | History, search, filters, collection selection, pagination, edit/tags, deletion, export, suggestions, settings, focus containment and persistence pass on generated data in headless Edge. All five names remain, with Mariyaholic only on Mariyaholic-98. |
| Browser zoom | Actual `chrome.tabs.setZoom` at 200% and 400% passes across five themes, four screens and editing. The 1280-CSS-pixel client viewport becomes 640/320 CSS pixels; horizontal overflow is checked. Direct CDP surface captures avoid Playwright's incorrect zoom clipping. This is browser zoom, not CSS scaling. |
| Automated accessibility | 25 axe-core 4.14.0 audits (four screens and an edit dialog per theme), zero reported WCAG 2 A/AA or 2.1 AA violations. Incomplete ARIA/contrast checks remain in the raw receipt; screen-reader and visual acceptance are not inferred. |
| Windows scheduler | Real registration, helper execution, last result zero, removal and absence pass in a uniquely named synthetic task folder. Registered hourly/logon triggers, battery restrictions, IgnoreNew, Hidden and the COM WakeToRun=false setting are checked. No logoff or AC/battery transition was induced. |
| Userscript manager | Unmodified shipped script installed through Tampermonkey 5.5.1 in disposable headless Chromium 151.0.7922.34; paired using its real menu and GM storage. Five accepted events produced ten measured playback seconds from a real HTML video fed by a generated canvas stream. Strict listener Host/Origin/token checks remain intact. This tests the manager transport and generic video adapter, not authenticated platform players or DRM. |
| Public YouTube player | One public YouTube video (`jNQXAC9IVRw`), anonymous fresh headless profile, no saved credentials: three accepted events and eight processed playback seconds. Only loopback and YouTube/media hosts were permitted. The temporary browser profile was deleted. This is a short single-player smoke, not ads/DRM/authenticated or cross-platform acceptance. |
| Listener authentication/restart | Invalid token/Host/Origin/framing requests rejected; replay idempotence, restart, collection, metadata and port collision pass. |
| Full inbox / sustained resources | Passed a 19,994,000-byte raw inbox followed by 3,600.027 seconds of real-time synthetic telemetry: 720 accepted events, 1.390625 listener CPU-seconds, 9,781,248 bytes lifetime peak working set. The 10,000,000-byte listener budget passes. Browser/decoder and collector costs are separate. |
| Large CSV/JSON export | Passed all six optimized runs: 100,201 generated views each, unchanged catalogue hash and deleted temporary exports. JSON median 431 ms (previously 62,540 ms), maximum 551 ms; CSV median 372 ms, maximum 548 ms. Maximum export process peak 10,653,696 bytes, below the 30 MB collector budget. |
| Rust and hosted validation | All 69 Windows Rust tests, formatting, Clippy with warnings denied, Ruff and JS capture semantics pass locally. [Hosted run 41575138586](https://github.com/mariyaholic/omoi/actions/runs/41575138586) passes both Windows GNU and Ubuntu for code revision `558ebed`, including optimized builds, synthetic release journeys, playback transport/restart and userscript semantics. |

## Defects found and addressed

1. Firefox's profile index can contain an unused profile with no `places.sqlite`. Default discovery now reports that profile as skipped, rather than making the whole run partially fail. An explicitly configured missing path still fails. A regression failed before the change and passed afterward.
2. A full 20 MB playback inbox filled the listener's inherited 2 MiB SQLite cache, producing 11,907,072 bytes peak working set. The append-only listener now uses a 256 KiB cache; the fixed pilot peaked at 9,777,152 bytes. The collector's cache is unchanged. The [old failed-budget measurement](../ui/acceptance/inbox-before.json) is retained: its `result: pass` meant functional checks only, before the harness enforced the 10 MB resource gate.
3. JSON serialization wrote each small fragment directly to disk. The regression observed 83,167 underlying writes for 1,002 rows. Export now streams through a 64 KiB buffer and explicitly flushes, retaining broken-pipe and no-overwrite behavior. CSV already buffered its writes.

The [original export receipt](../ui/acceptance/export-before.json) and [fixed export receipt](../ui/acceptance/export-fixed.json) each contain three runs per format on the same 100,201-row generated catalogue. Final JSON CPU was 359–500 ms and its file size was 24,791,647 bytes; CSV was 10,162,401 bytes. These are descriptive observations on this PC, not p95 estimates or a cold-cache benchmark.

Additional receipts: [final scheduler/helper pair](../ui/acceptance/scheduler-final.json), [real manager with generated video](../ui/acceptance/manager.json), and [public YouTube player](../ui/acceptance/public-youtube.json). Earlier UI/zoom/scheduler receipts remain under `ui/acceptance/` for comparison; the final UI receipt contains the combined 25 audits and 50 zoom checks on the release binary.

## Measurement interpretation

Final Windows GNU release binaries, used for the corrected export and final UI checks:

| Executable | Bytes | SHA-256 |
|---|---:|---|
| `omoi.exe` | 2,427,904 | `87f39d4d12cced5273de1d0ff9f7234010c8b2af3d4a73ee3bf544d03ed01f45` |
| `omoiw.exe` | 2,430,976 | `1674b978845e9dbe4c92dfcad25fe10fd41c5cb3b5e9cc4df3285e00ab30f8bc` |

The primary executable remains below the 6,000,000-byte target. The portable archive's `BUILD.json` records its exact source commit and all included file hashes. Python, Playwright and axe are acceptance tools, not application dependencies. Optional playback capture requires a userscript manager, installed separately.

The listener's one-hour run uses actual elapsed time with synthetic HTTP telemetry every five seconds. Windows process CPU counters measure core-seconds for the listener alone; browser decoding, the harness, ACL helper and collector are separate processes. Other acceptance checks and debug compilation overlapped part of the run. System load and cold-cache state are not controlled. Its executable hash is recorded separately from the later export-only fix; listener implementation is identical.

The [hour receipt](../ui/acceptance/playback-hour.json) used executable SHA-256 `a271bbc5c0766642cdc2c33b57be41733091c60d6bdd4259e62b769d5dbbc109`. The full inbox was drained before the timed load. Its post-processing DB+WAL+SHM reading was 21,970,944 bytes with the listener still open. Later harness changes additionally assert processed playback time and report closed-connection page/freelist sizes; those assertions are not retroactively attributed to this hour run.

Browser measurements include the browser's own processes. Working sets share pages and must not be added as unique physical memory. The receipt sums process private bytes and gives process coverage. Separate empty-browser, freshly loaded catalogue and post-journey snapshots prevent axe and export activity from being described as normal initial UI cost. These are isolated test-browser readings, not the incremental cost of an existing personal browser tab.

The [final UI receipt](../ui/acceptance/ui-final.json) covers 10,000 items: warm click-to-rows median **121 ms**, maximum **138 ms** across ten samples, within the 200 ms target. API median/max were 80/107 ms. UI-server lifetime peak was **11,599,872 bytes**; its short three-second idle sample recorded **46.875 ms CPU**, so this run does not support a zero-idle-CPU claim. Isolated Edge private bytes were **213,983,232** before the app, **280,141,824** with the catalogue before auditing, and **556,122,112** after the complete journey including axe. These snapshots cover all CDP-reported processes at each point (10/9/10). The browser dominates active UI memory; the listener's 10 MB target does not describe the whole browser session.

The final executable also passes the [15-second playback recheck](../ui/acceptance/playback-final.json): full-inbox peak 9,764,864 bytes, 15,000 ms processed playback, and closed-connection catalogue size 21,905,408 bytes, including 21,786,624 freelist bytes. The full-size file remains allocated after processing. The hour receipt and this shorter, stronger accounting check are distinct observations.

The 30 MB catalogue target describes the documented 100k-view/40k-item baseline. A 20 MB raw-event inbox, retained receipts, sessions, long titles and freelist space are additional workload-dependent costs. Processing the inbox does not shrink the SQLite file automatically; no VACUUM is used to hide allocated space.

## Remaining gates

- Authenticated YouTube and Netflix, Twitch, Bilibili, Prime Video, Disney+ and Crunchyroll players; DRM/ad behavior and actual manager installation in the owner's usual browser. The isolated manager and short public YouTube proof do not establish these.
- Actual logon and battery transitions, manual default-browser launch, screen-reader behavior, unresolved accessibility checks and owner visual acceptance.
- Strict M0 cases still listed in `m0-acceptance.md`: injected discovery permission denial, orphan-join accounting, deterministic stale-prepared-checkpoint race and production per-profile diagnostics. Current collection JSON/logs have aggregate rejection counts; the retained-week test's per-profile receipt does not change that production contract difference.
- Independent review of the documented PowerShell COM adapter difference, snapshot consistency limits and remaining strict contract coverage. Native SMTC, M2 imports and remote recommendation/enrichment adapters remain roadmap features.

## Reproduce

Use the release binaries from a stable directory. All scripts have `--help`. Python/Playwright/axe are development tools, not application dependencies.

```text
python scripts/verify_ui.py --binary PATH/omoi.exe --out NEW_OUTPUT_DIR --axe-file PATH/axe.min.js --zoom
python scripts/verify_playback.py --binary PATH/omoi.exe --stress-inbox --duration 3600 --out playback-hour.json
python scripts/verify_schedule.py --binary PATH/omoi.exe --out scheduler.json
python scripts/verify_manager.py --binary PATH/omoi.exe --extension-dir PATH/official-tampermonkey-extracted --out manager.json
python scripts/verify_manager.py --binary PATH/omoi.exe --extension-dir PATH/official-tampermonkey-extracted --out public-youtube.json --public-youtube
python scripts/verify_export.py --binary PATH/omoi.exe --synthetic-home PATH/generated-fixture/home --out exports.json
cargo run --release --example retained_week -- --consent-local-history --out LOCAL_ONLY_COUNTS.json
```

The retained-week command is intentionally separate and requires explicit consent; do not run it in CI. Keep its receipt local. The scheduler script creates and removes only its uniquely named synthetic task. Manager and zoom tests require Playwright's bundled Chromium; ordinary UI tests use installed Edge. The downloaded manager CRX SHA-256 was `b5167937817e636adeba462ed9bf9b32ff01c2888f78a9afc6aa4f8e655fe872`; its package is neither bundled with Omoi nor added to the normal browser.

References: [Playwright extension testing](https://playwright.dev/docs/chrome-extensions), [Chrome tab zoom API](https://developer.chrome.com/docs/extensions/reference/api/tabs#method-setZoom), [Chrome userscript permission](https://developer.chrome.com/docs/extensions/reference/api/userScripts), [Playwright accessibility testing and limitations](https://playwright.dev/docs/accessibility-testing).
