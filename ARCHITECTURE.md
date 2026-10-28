# Omoi architecture (structural record)

Status: design only. M0 U1 partial. Does not build yet: `src/lib.rs` declares `pub mod readers;` and `src/readers/mod.rs` is missing, so `cargo build` fails with E0583. Date: 2026-10-28.

Sources: `docs/watch-history-architecture.md` (draft 2; §0 decisions override the rest), `docs/omoi-architecture-detail.md` (M0 build contract; governs where it differs from draft 2), `docs/omoi-design-review.md` (findings F1-F8), `HANDOFF.md` (build status). File state was checked read-only on 2026-10-28.

## 1. What it is

- Omoi is a local, PC-only watch-history app for Maria. It records what she watches on YouTube, Netflix, Twitch, Bilibili, Prime Video, Disney+ and Crunchyroll, plus local players.
- All data lives in one SQLite file under `%LOCALAPPDATA%\omoi\`. Nothing syncs to the cloud.
- It shows a browsable catalogue with time spent, video types and recommendations.
- One binary, `omoi`, with subcommands `collect`, `listen` and `ui`. M0 also has `export`, `db`, `schedule` and `paths` (detail §2).

## 2. Process model

```
Task Scheduler  \Omoi\Collect   (hourly + logon, InteractiveToken)
  |
  v
omoi collect  --reads-->  COPIES of browser History / places.sqlite   (S1)
  |                       [M2: Netflix CSV drop, local player files]
  |--writes-->  history.sqlite   (WAL; one writer, BEGIN IMMEDIATE)
  v
exits

omoi listen   (optional, M3, resident from logon)
  SMTC events (S2) ----------------------------+
  userscripts POST /event (S3) ----------------+--> 127.0.0.1:43821, token required
                                               |
                                               +--appends--> history.sqlite (raw_events)

omoi ui       (M1, on demand)
  browser --> 127.0.0.1 page --reads (query_only)--> history.sqlite
                             --delete/tag writes (per-launch token)-->
  exits after 10 min idle
```

| Process | Lifetime | Memory budget | CPU budget | DB access |
|---|---|---|---|---|
| `omoi collect` (also `omoiw.exe`, windowless) | One run, then exits. Task Scheduler hourly and at logon, "run only when user is logged on" | 0 between runs. Lifetime peak under 30,000,000 bytes, including a full import | Processing under 1 s per warm run (wall and CPU). Copy time reported separately (F6) | Writes `items`, `views`, `checkpoints` and FTS in one `BEGIN IMMEDIATE` transaction. A busy writer exits 4. Reads browser copies only, never the live file |
| `omoi listen` (optional, M3) | Resident from logon. Off means no time tracking | 10,000,000 bytes working set or less | About 0 % when idle. Event-driven, no polling. Core-seconds reported over one hour of playback | Appends to `raw_events` (inbox). Which process turns inbox rows into `sessions` is unspecified in docs |
| `omoi ui` (M1) | On demand. Opens the browser. Exits after 10 min idle | Unspecified in docs | Unspecified in docs. Page load target under 200 ms at 10k items | Read-only connection (`query_only`). Writes only delete and tag transactions, each with a per-launch token |

Userscripts run inside the browser, not as a process. They send events to the listener through `GM_xmlhttpRequest`.

## 3. Module map

Layout from detail §1. Only `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/main.rs` and `src/error.rs` exist. Nothing is marked built, because the crate does not compile.

| File | Owns | Milestone | State |
|---|---|---|---|
| `Cargo.toml` | Package and dependency manifest. Matches detail §1 | M0 U1 | partial (exists, matches §1) |
| `Cargo.lock` | Resolved versions | M0 U1 | partial (exists; no build) |
| `src/lib.rs` | Crate root. Declares modules (`readers` is missing). Re-exports `Error`, `Result` | M0 U1 | partial (build blocker) |
| `src/main.rs` | Console entry. Placeholder returns `NotImplemented` | M0 U1, replaced in U13 | partial (placeholder) |
| `src/error.rs` | `Error` enum and exit-code mapping (detail §9) | M0 U1 | partial (82 lines) |
| `src/bin/omoiw.rs` | Windowless collect entry (Windows subsystem) | M0 U12 | not started |
| `src/config.rs` | Restricted TOML subset parser, defaults, validation | M0 U2 | not started |
| `src/paths.rs` | Home and data path resolution from an injected environment | M0 U2 | not started |
| `src/logging.rs` | Bounded JSONL logs, lock file, rotation caps | M0 U11 | not started |
| `src/db/mod.rs` | `open`, writable and read-only connection PRAGMAs | M0 U3 | not started |
| `src/db/schema.rs` | `include_str!` migration registry | M0 U3 | not started |
| `src/db/migrations.rs` | `migrate()`: `BEGIN IMMEDIATE`, `user_version` | M0 U3 | not started |
| `src/db/0001.sql` | Schema 0001: tables, indexes, FTS, triggers | M0 U3 | not started |
| `src/readers/mod.rs` | Reader types, `HistoryReader` trait, `open_reader` factory | M0 U1 | not started (missing; build blocker) |
| `src/readers/discover.rs` | Chromium and Firefox profile discovery, `profile_excludes` | M0 U4 | not started |
| `src/readers/snapshot.rs` | Fingerprint, sidecar copy, retry, cleanup | M0 U5 | not started |
| `src/readers/chromium.rs` | Chromium visits reader, 1601 epoch | M0 U6 | not started |
| `src/readers/firefox.rs` | Firefox visits reader, PRTime | M0 U7 | not started |
| `src/normalize.rs` | URL to `(platform, external_id)` for 7 platforms | M0 U8 | not started |
| `src/dedupe.rs` | `Candidate`, `ingest()`, 1,800 s window, richness rank | M0 U9 | not started |
| `src/sessionize.rs` | M0 stub (`NotImplemented`); M3 contract | M0 U13 (stub); M3 | not started |
| `src/export.rs` | CSV and JSON streaming export, no-clobber publish | M0 U10 | not started |
| `src/scheduler.rs` | Portable XML and argv builders | M0 U12 | not started |
| `src/scheduler/windows.rs` | Task folder via COM, `schtasks` calls (`cfg(windows)`) | M0 U12 | not started |
| `src/cli.rs` | clap surface and exit codes | M0 U13 | not started |
| `src/collect.rs` | Orchestration: spool, one transaction, summary | M0 U13 | not started |
| `examples/synthetic.rs` | Deterministic fixture generator | M0 U14 | not started |
| `scripts/measure.ps1` | Windows performance harness | M0 U14 | not started |
| `tests/` | Named acceptance tests per unit | M0 U1-U14 | not started (no test files yet) |

Not in the §1 layout, so their files are unspecified in docs: the M1 UI page, the M1.5 thumbnail code, the M2 importers and the M3 listener and userscript source. Userscript location is also unspecified.

## 4. Data flow

1. Source: Chromium `History` and Firefox `places.sqlite`, one per profile found by `readers/discover.rs`. The live file is never opened.
2. Snapshot: `readers/snapshot.rs` copies the main file with `-wal` and `-journal` into a private `tmp/` directory. It skips a profile whose fingerprint is unchanged, and retries up to `snapshot_retries`.
3. Reader: `readers/chromium.rs` or `readers/firefox.rs` reads bounded batches of at most `reader_batch_rows`, using the `(time, id)` tuple cursor. Chromium time is µs since 1601. Firefox time is µs since 1970.
4. Normalize: `normalize.rs` maps a URL to `(platform, external_id)` or `None`. It is pure and makes no network calls.
5. Dedupe: `dedupe.rs` `ingest()`, called by `collect.rs` inside one `BEGIN IMMEDIATE` transaction. Fixed-anchor 1,800 s window. Richest metadata kept. Approximate duration takes the max and is never summed.
6. Sessionize: `sessionize.rs`. In M0 it is a stub that returns `NotImplemented`, and collect never calls it. The M3 contract (300 s gap rule, provenance) is in detail §8.
7. Enrich: module unspecified in docs. M4 (YouTube Data API, TMDB, Twitch Helix, oEmbed). No enrichment in M0.
8. Tag: module unspecified in docs. M4 layered rules, API results and channel tags (draft 2 §7). No tagging in M0.

`collect.rs` saves checkpoints only in the same COMMIT as the rows. A failed source keeps its old cursor.

## 5. Data model

| Table | Holds | Key constraints |
|---|---|---|
| `items` | One row per distinct video, episode or stream | `UNIQUE(platform, external_id)`. `platform` CHECK over `youtube, bilibili, netflix, twitch, prime, disney, crunchyroll, local, unknown` (codes, not display names). `channel_id` nullable until enrichment (F3). `metadata_rank`. `duration_s >= 0` |
| `views` | One deduped "I watched this" event | `UNIQUE(item_id, started_at, source)`. `device` defaults to `pc`. `approx_duration_s` is from Chromium only and is not measured playback. Indexes on `(item_id, started_at)` and `(started_at, id)` |
| `sessions` | Measured playing time (M3; none written in M0) | `end >= start`. `0 <= played_s <= end - start`. `source` names the measurement tier (F4) |
| `item_tags` | Tags per item | PK `(item_id, tag)`. `source` is rule, api or user. `WITHOUT ROWID` |
| `channel_tags` | Tags per channel | PK `(platform, channel, tag)`. Keyed by display name; migrate to channel IDs before M4 if needed |
| `checkpoints` | Per-source cursor JSON with fingerprint. Also a reserved `maintenance:optimize` key | PK `source`. Written only in the COMMIT that also writes rows. Cursor `version` 1; unknown versions rejected |
| `raw_events` | Listener inbox (M3; empty in M0) | `body` at most 16,384 bytes. Logical caps 100,000 rows and 20,000,000 body bytes. Processed rows are deleted in the transaction that commits their sessions |
| `items_fts` | FTS5 over `title, channel, series` | External content. AFTER INSERT, UPDATE and DELETE triggers. Verify with `MATCH` and the integrity check, not `count(*)` |

Schema version is `PRAGMA user_version`. Migration 0001 sets it to 1. A newer version fails with `SchemaTooNew`. A nonempty unversioned database is rejected.

## 6. Boundaries and invariants

- Never open a live browser database. Copy the main file first, with its `-wal` and `-journal` sidecars, and open only the copy.
- Copy sidecars together. Never copy the main file alone when a sidecar exists. Ignore `-shm`.
- No async runtime. M0 has no ORM, HTTP, logging, regex, TOML or hashing dependency beyond the list in detail §1.
- No network in M0. No credentials, browser cookies, enrichment, thumbnail downloads or listener in M0.
- Listener binds `127.0.0.1` only, on fixed port 43821. It requires the exact `Host: 127.0.0.1:<port>` header and an `X-Omoi-Token` header (32 random bytes). It rejects page Origins and sends no CORS headers.
- The `ui` server has its own per-launch token for mutations. It is separate from the listener token.
- API keys go in Windows Credential Manager, never in files. No API keys are used in M0.
- Logs hold no full URLs, titles, local file paths, tokens or browser row contents. Each record is capped at 4 KiB.
- Single writer: `BEGIN IMMEDIATE`. A busy writer exits 4 and is safe to retry. Checkpoints move only on COMMIT.
- No `unwrap`, `expect`, unchecked indexing or `panic!` on the collect path.
- Browser discovery always uses the original OS profile directories, never `OMOI_HOME`.
- A damaged database is never reset or deleted automatically. No automatic VACUUM.
- The scheduled task runs only when the user is logged on. It stores no password and does not elevate.
- S4 (yt-dlp with cookies) is off. Nothing in the core path may depend on it.
- Phone and TV sources are out of scope.

## 7. Decisions that override older text

| Date | Item | Ruling | Effect on the build |
|---|---|---|---|
| 2026-10-25 | Q3: S4 yt-dlp with Firefox cookies | Off | Not scheduled in any milestone |
| 2026-10-25 | Q6: Bilibili position | Moved after Twitch. Order: YouTube, Netflix, Twitch, Bilibili, Prime Video, Disney+, Crunchyroll | Sets M3 userscript order and M4 enrichment order. Overrides review §3 and §6 |
| 2026-10-25 | Q7: thumbnail cache cap | 100 MB (`thumb_cache_bytes = 100000000`), LRU by last view | Overrides the 250 MB proposal in review §3 and §6. The 250000000 value in detail §11 example JSON is stale |
| 2026-10-25 | Q8: browser profiles | All discovered profiles, minus `profile_excludes` | M0 config key. An unmatched exclude entry warns only |
| 2026-10-24 (applied in draft 2 §11) | F1: listener Origin rule | Platform Origin allowlist withdrawn. Reject any web-page Origin (http, https, null). Accept no Origin or an explicit extension origin. Userscripts use `GM_xmlhttpRequest` | Detail §13 is the M3 contract. Origin is defense in depth, not authentication |
| 2026-10-24 | Q1, Q2, Q4, Q5 | Top platforms by users; PC-only; thumbnails only after the M1.5 measurement; name Omoi | Scope and naming: `omoi.exe`, `%LOCALAPPDATA%\omoi\`, task folder `\Omoi\` |
| 2026-10-24 | Build contract | Detail doc governs where draft 2 differs: schema (detail §4, not draft 2 §4), dedupe (fixed anchor, not "within 30 min" chaining, detail §7), crate list (no tiny_http or ureq in M0, detail §1) | Build from detail §1, §4, §7 |

## 8. Budgets

| Resource | Budget | Source | Measured? |
|---|---|---|---|
| Collector RAM | 0 between runs. Under 30,000,000 bytes lifetime peak, including a full import | Draft 2 §1; detail §12 | Target only |
| Listener RAM | Under 10,000,000 bytes working set | Draft 2 §1; detail §12 | Target only |
| Collector CPU | Under 1 s per warm run. Copy time reported separately | Draft 2 §1; detail §12 (F6) | Target only |
| Listener CPU | About 0 % idle. Core-seconds reported over one hour of playback | Draft 2 §1; detail §12 | Target only |
| Database disk | Under 30,000,000 bytes per 100k views, including items, views, indexes, FTS and checkpoints. Baseline about 40k items | Draft 2 §1; detail §12 | Target only |
| UI page load | Under 200 ms at 10k items (warm, local) | Draft 2 §14 M1; detail §12 | Target only |
| `omoi.exe` size | Up to 6,000,000 bytes. Helper reported separately | Detail §1, §12 | Target only |
| Thumbnail cache cap | 100,000,000 bytes default, LRU by last view | Draft 2 §0 Q7; detail §11 | Not started |
| Thumbnail test | Hard stop 50,000,000 bytes aggregate; 1,000,000 bytes per object; 2 redirects; 5 s connect, 15 s total | Detail §11 | Not started |
| Thumbnail enable gate | Full-catalogue projection at or under 100,000,000 bytes | Detail §11 | Not started |
| Network | Only the enrichment APIs and feeds in draft 2 §2. None in M0 | Draft 2 §1 | n/a |
| Enrichment per run | Capped per run, "e.g. 500 items" | Draft 2 §10 | Not started |
| YouTube quota | 10,000 units/day; 1 unit per 50 IDs | Draft 2 §7; review §2 | External |
| Listener request | Body at most 16,384 bytes; headers at most 8,192 bytes; 2 s read timeout | Detail §13 | Not started |
| Listener inbox | At most 100,000 rows and 20,000,000 body bytes | Detail §4 | Not started |
| Logs | 3 files of 1,048,576 bytes by default (at most 3 MiB total). Record at most 4 KiB | Detail §3, §9 | Not started |
| URL and title limits | URL 8,192 bytes (row rejected). Title 1,024 bytes (dropped) | Detail §5 | Not started |
| Config | File at most 65,536 bytes. `busy_timeout_ms` 1-60000. `reader_batch_rows` 1-2048, default 512 | Detail §3 | Not started |
| Dedupe window | 1,800 s, inclusive | Detail §7 | Not started |
| Session gap | 300,000 ms (exactly 5 min stays in one session) | Detail §8 | M3 |
| Snapshot retries | Default 3 (1-5), 50 ms apart | Detail §3, §5 | Not started |
| Scheduled task | Every PT1H. Execution cap PT10M. Priority 7 | Detail §10 | Not started |

## 9. Where to look

- `docs/watch-history-architecture.md`: §0 decisions (the overrides above); §1 budgets; §2 sources (S1-S8); §3 components; §9 scheduling; §11 privacy and listener; §12 URL patterns (superseded in detail §6); §14 milestones. §15 has no open questions.
- `docs/omoi-architecture-detail.md`: §1 crate layout and manifest; §2 CLI and exit codes; §3 paths and config subset; §4 schema 0001 and migration; §5 reader contracts and snapshot protocol; §6 normalizer and vectors; §7 dedupe and collect transaction; §8 sessionization (M3); §9 errors and logs; §10 Task Scheduler; §11 M1.5 thumbnail measurement; §12 resource gates; §13 M1-M3 outline and listener security; §14 units U1-U14 with acceptance tests.
- `docs/omoi-design-review.md`: §1 findings F1-F8; §2 claim spot-check; §3 source matrix and budgets; §4.1 listener gaps; §6 questions Q6-Q8 and answers.
- `HANDOFF.md`: "Done" table (U1 partial); "Remaining units" (U2-U14, same acceptance tests as detail §14); "Build and test on Windows"; free-model continuation rules. Its build path (`C:\Users\iuniq\Desktop\omoi`) is stale. The repo is at `C:\Users\iuniq\Desktop\mariyaholic\omoi`.
