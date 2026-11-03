# Headless verification — November 3, 2026

The owner has deferred further acceptance work for this private development merge. Detailed checklists and raw receipts are retained locally and omitted from current shared logs. This is a scoped development verification record, not a claim of complete M0–M5 acceptance.

The implemented catalogue, five-theme UI, optional playback transport and isolated Windows scheduler passed the completed headless journeys. All 69 Windows Rust tests, formatting, Clippy, Ruff and userscript semantics passed. [Hosted Windows GNU and Ubuntu validation](https://github.com/mariyaholic/omoi/actions/runs/41577077434) passed code/tooling revision `89afbc8`.

## Measured behavior

| Workload | Observed result |
|---|---|
| Listener after full-inbox stress | 3,600.027 seconds, 720 events, 9,781,248-byte lifetime peak, 1.390625 listener CPU-seconds |
| 100,201-row JSON export | 431 ms median, 551 ms maximum across three runs |
| 100,201-row CSV export | 372 ms median, 548 ms maximum across three runs |
| 10,000-item UI | 121 ms median / 138 ms maximum warm navigation across ten samples |
| UI server | 11,599,872-byte lifetime peak; 46.875 ms CPU during a three-second idle sample |
| Isolated Edge private bytes | 213,983,232 before loading; 280,141,824 with catalogue before auditing; 556,122,112 after the full journey including audit tooling |

These measurements describe specific workloads on the owner's Windows PC. The listener figure excludes browser/decoder and collector processes. The browser figures cover the whole isolated test browser, not the incremental cost of an existing personal tab. Other work overlapped part of the hour; system load and cold-cache state were not controlled. Three export runs do not establish a p95.

## Fixes included

- Default discovery skips an unused Firefox profile without a history database with a warning. Explicit missing paths still fail.
- The listener uses a 256 KiB SQLite cache to keep the full-inbox peak below its 10 MB target.
- JSON export streams through a 64 KiB buffer, reducing the observed median from 62.54 seconds to 0.431 seconds without changing exported row counts or catalogue contents.

A processed inbox retains SQLite freelist allocation; the shorter final recheck left a 21,905,408-byte catalogue. Storage depends on workload and retained data, not a universal 30 MB ceiling.

## Windows build

| Executable | Bytes | SHA-256 |
|---|---:|---|
| `omoi.exe` | 2,427,904 | `87f39d4d12cced5273de1d0ff9f7234010c8b2af3d4a73ee3bf544d03ed01f45` |
| `omoiw.exe` | 2,430,976 | `1674b978845e9dbe4c92dfcad25fe10fd41c5cb3b5e9cc4df3285e00ab30f8bc` |

The hour used the same listener implementation before the export-only fix, executable SHA-256 `a271bbc5c0766642cdc2c33b57be41733091c60d6bdd4259e62b769d5dbbc109`. Final export and UI measurements used the binaries above. Portable archives include file hashes in `BUILD.json`.

[UI instructions](../ui/README.md), [playback setup](../userscripts/README.md) and the verification scripts' `--help` describe use and reproduction. Python, Playwright and axe are development tools; optional playback requires a separately installed userscript manager.
