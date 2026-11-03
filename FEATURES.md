# Omoi: feature catalogue

*Historical design inventory, dated 2026-10-28. The state and test columns below describe that planning snapshot, not current implementation. For current CLI and connected UI behavior, evidence and remaining gates, see [PROGRESS.md](PROGRESS.md), [M0 acceptance](docs/m0-acceptance.md) and [UI instructions](ui/README.md).*

Omoi is a PC-only, local watch-history app. The hourly collector exits after each run. The listener is optional.

## The budget

The numbers come from the docs. The detail doc says its performance figures are acceptance targets, not measurements. No measurements exist yet.

| Resource | Budget | Source |
|---|---|---|
| RAM, collector | 0 between runs. Under 30,000,000 bytes peak working set during a run, including a full import | arch §1, detail §12 |
| RAM, listener | 10,000,000 bytes working set at most | arch §1, detail §12 |
| RAM, ui server | unspecified in docs | n/a |
| CPU, collector | Warm run: processing under 1 s wall and under 1 s CPU. Copy time is reported separately. Arch §1 says "under 1 s per hour" | arch §1, detail §12 |
| CPU, listener | About 0% while idle. Event-driven, no polling | arch §1, detail §12 |
| Disk, catalogue | 30,000,000 bytes per 100,000 views, including items, views, indexes, FTS and checkpoints | arch §1, detail §12 |
| Disk, thumbnails | Off until the M1.5 test passes. Cache cap 100,000,000 bytes, LRU by last view | arch §0 Q4 and Q7, detail §11 |
| Disk, thumbnail test | 50,000,000 bytes hard stop, including in-flight bytes | detail §11 |
| Disk, listener inbox | raw_events: 100,000 rows and 20,000,000 body bytes (M3) | detail §4 |
| Disk, logs | 3 files of 1,048,576 bytes at defaults (3 MiB total) | detail §3, §9 |
| Disk, spool and snapshots | Proportional to accepted rows, removed after each run. No cap in docs; peak is measured and reported | detail §7, §12 |
| Network | Only the enrichment APIs and feeds named in arch §2. YouTube Data API quota is 10,000 units per day | arch §1, §7 |
| Binary | `omoi.exe` at most 6,000,000 bytes. Helper executable reported separately | detail §1 |
| UI latency | Page load under 200 ms at 10,000 items, warm, local | arch §14, detail §12 |
| Listener requests | Body at most 16,384 bytes. Headers at most 8,192 bytes. Read timeout 2 s | detail §13 |
| Reader limits | URL at most 8,192 bytes (row rejected). Title at most 1,024 bytes (title dropped) | detail §5 |

## Feature catalogue

State values: **specified** (contract written), **partial** (some code exists), **not started**, **off by decision**, **optional**. The "Verified by" column names the tests planned in detail §14 (U-unit in brackets) or the gate the docs name. No test has run.

| ID | Feature | Milestone | Source(s) | State | Budget cost | Verified by |
|---|---|---|---|---|---|---|
| F01 | Crate skeleton, error model, exit codes 0 to 5 | M0 | detail §1, §2, §9 | partial (U1: Cargo.toml, lib.rs, main.rs, error.rs written; readers/mod.rs missing, cargo build fails with E0583) | none | `error_exit_mapping` (U1) |
| F02 | Config file: restricted TOML subset, defaults, validation | M0 | detail §3 | specified | file at most 64 KiB | `config_defaults_and_errors`, `config_comments_in_strings` (U2) |
| F03 | Data paths: `OMOI_HOME`, `%LOCALAPPDATA%\omoi`, `omoi paths` | M0 | detail §2, §3 | specified | none | `home_override_precedes_os`, `linux_xdg_fallback` (U2) |
| F04 | Chromium-family history reader: Chrome, Edge, Brave | M0 | arch §2 S1; detail §5 | specified | copy time reported separately (F6); batches of 512 rows | `chrome_epoch`, `chrome_tuple_pagination`, `chrome_duration_optional`, `chrome_reject_long_url_advances`, `reader_batch_bound` (U6) |
| F05 | Firefox history reader: `places.sqlite` | M0 | arch §2 S1; detail §5 | specified | as F04; duration is always None | `firefox_prtime`, `firefox_join_and_cursor`, `firefox_duration_none`, `browser_schema_failure_is_source`, `cursor_reset_rescans` (U7) |
| F06 | Profile discovery: every Chrome, Edge, Brave and Firefox profile found | M0 | arch §0 Q8; detail §5 | specified | none | `chromium_all_profiles`, `firefox_ini_paths`, `discovery_missing_and_denied` (U4) |
| F07 | `profile_excludes`: skip named profiles; unmatched entry only warns | M0 | arch §0 Q8; detail §3, §5 | specified | none | `profile_excludes_applied`, `explicit_profile_filter` (U4) |
| F08 | Snapshot of live DBs: copy main file and sidecars, retry, `quick_check` on the copy | M0 | detail §5 (F6) | specified | the copies are the main IO cost (review F6); skipped when fingerprint is unchanged | `skip_unchanged_triplet`, `wal_only_change_copies`, `snapshot_preserves_sidecars`, `snapshot_retry_bound`, `snapshot_cleanup` (U5) |
| F09 | Incremental checkpoints: one cursor per profile, committed with the rows | M0 | arch §5; detail §5, §7 | specified | one small write per profile | `collect_atomic_checkpoints`, `collect_concurrent_checkpoint`, `collect_week_idempotent` (U13) |
| F10 | URL normalizer for 7 platforms, plus local files | M0 | arch §12; detail §6 | specified | pure function; no network | `normalizer_url_vectors` (every §6 row), `twitch_local_midnight`, `normalizer_identity_params`, `normalizer_no_network`, `local_path_case` (U8) |
| F11 | Dedupe: 1,800 s fixed-anchor window across sources, richness rank | M0 | arch §5 step 3; detail §7 | specified | all writes in one `BEGIN IMMEDIATE` | `dedupe_window_vectors`, `dedupe_fixed_anchor`, `dedupe_richness`, `dedupe_approx_max`, `dedupe_two_writers`, `dedupe_rollback` (U9) |
| F12 | Atomic collect transaction and JSON summary; exit 3 partial, exit 4 busy | M0 | detail §2, §7 | specified | none beyond F09 | `collect_partial`, `cli_surface` (U13) |
| F13 | Dry run: `omoi collect --dry-run` | M0 | detail §2 | specified | clones DB to a temp file; browser copies as F08 | `collect_dry_run` (U13) |
| F14 | SQLite schema 0001, migrations, FTS5 check, `omoi db info` | M0 | arch §4; detail §4 | specified | items about 200 B, views about 40 B, sessions about 40 B, FTS about 100 B per item | `migration_0001_idempotent`, `migration_rollback`, `reject_future_schema`, `required_columns`, `foreign_key_cascade` (U3) |
| F15 | CSV export, one row per deduped view | M0 | arch §14 M0; detail §2 | specified | streamed, no Vec of rows | `csv_escaping`, `export_one_row_per_view`, `export_no_clobber`, `export_streaming` (U10) |
| F16 | JSON export, streamed array | M0 | detail §2 | specified | streamed | `json_null_and_order` (U10) |
| F17 | Bounded JSONL logs: no URLs, titles or tokens in records | M0 | detail §9 | specified | at most 3 MiB on disk; 4 KiB per record | `logs_global_cap`, `logs_redact_and_truncate`, `log_lock_fallback`, `dry_run_no_log_files` (U11) |
| F18 | Hourly `collect` in Task Scheduler (`\Omoi\Collect`): hourly and at logon, logged-on users only, hidden, below-normal priority | M0 | arch §9; detail §10 (F2) | specified | 10-minute cap; skipped on battery; no resident process | `schedule_xml_contract`, `schedule_escapes_paths`, `schedule_argv_no_shell`, `schedule_linux_stub` (U12); Windows smoke test by hand |
| F19 | Idempotent re-run: a second run adds 0 items and 0 views | M0 | arch §14 M0; detail §7, §14 | specified | none | `collect_week_idempotent`, `synthetic_counts` (U13, U14); Maria's real week at M0 exit |
| F20 | Timeline UI: `omoi ui` on demand, 127.0.0.1 only, 50 rows per page, exits after 10 min idle | M1 | arch §3, §14; detail §13 | specified | RAM unspecified; no resident process | M1 exit: page load under 200 ms at 10k items. No named test |
| F21 | Full-text search: FTS5, literal mode, parameterized inputs | M1 | arch §4, §14; detail §13 | specified | FTS inside the 30 MB catalogue budget | `fts_insert_update_delete` (U3); real MATCH queries per detail §4 |
| F22 | Per-platform and per-channel views and filters | M1 | arch §14; detail §13 | specified | paginated SQL | M1 exit (page load). No named test |
| F23 | Delete controls (the only write in the M1 UI): per item, per platform, or everything; FTS kept in sync | M1 | arch §11; detail §13 | specified | none | `foreign_key_cascade`, `fts_insert_update_delete` (U3) |
| F24 | Thumbnail disk-use test: `omoi thumbs --measure --limit 500`, YouTube `mqdefault.jpg` | M1.5 | arch §0 Q4, §14; detail §11 | specified | 50,000,000 bytes hard stop; sequential; 5 s connect and 15 s total deadlines | Report `thumbs-measure.json`; fake-chunk test at 49,999,999 bytes (detail §11). No U-unit |
| F25 | Thumbnail cache: capped, LRU by last view | M1.5, only if the gate passes | arch §0 Q7, §14; detail §11 | specified, conditional on F24 | cap 100,000,000 bytes by default, configurable | Gate in detail §11. No named test |
| F26 | Netflix CSV import (S6), with an hourly drop-folder scan | M2 | arch §2 S6; detail §13 | specified | none | Header-driven fixtures (detail §13). M2 exit: Netflix items visible, no duplicates against S1 |
| F27 | Local player history: mpv `watch_later`, VLC recent list (S8) | M2 | arch §2 S8; detail §13 | specified | snapshot reads; no watcher | M2 exit. No named test |
| F28 | Jellyfin and Plex adapters | M2, opt-in | arch §2 S8; detail §13 | specified | network only when configured; credentials in Credential Manager | M2 exit. No named test |
| F29 | Google Takeout backfill (S5), import of zip from a Drive folder | M2, optional | arch §2 S5, §9; detail §13 | optional | ZIP support needs a separate dependency decision (detail §13) | Idempotent by file hash (arch §9). No named test |
| F30 | Time spent from userscripts (S3): played seconds, buffered in GM storage | M3 | arch §2 S3, §6; detail §8 | specified | throttled `timeupdate` only; no polling | Session vectors in detail §8 (7 rows) |
| F31 | Time spent from Windows media session (S2, SMTC) | M3 | arch §2 S2, §5; detail §8 | specified | event subscription only | Session vectors in detail §8 (SMTC rules). M3 exit: listener CPU about 0% over 1 h |
| F32 | Sessions table and sessionizer: gap over 300 s closes a session | M3 | arch §5, §8; detail §8 | specified | durable open-session state; run by `omoi collect` in its own transaction (decided 2026-10-28); M0 stub returns NotImplemented | `sessionize_m0_stub` (U13); session vectors in detail §8 |
| F33 | Localhost listener: `POST /event` on 127.0.0.1:43821 with a token | M3 | arch §11; detail §3, §13 | specified | at most 10,000,000 bytes working set; body 16,384 bytes | Hostile Host and Origin, missing token, oversized or slow request, full inbox, repeated event ID, port collision. Pure validators (detail §13) |
| F34 | `omoi listen` at logon, opt-in, separate from the hourly task | M3 | arch §9; detail §13 | specified | the only resident process | M3 exit. No named test |
| F35 | Interval-aware time totals with provenance; the UI never adds overlapping tiers | M3 | detail §8 | specified | none | Session vector: userscript [0,60] and SMTC [0,60] give two rows, UI total 60 |
| F36 | Time-spent views: totals, per day and week heatmap, completed vs sampled | milestone unspecified in docs (after M3) | arch §6 | specified | SQL only | unspecified in docs |
| F37 | Video-type rules: short and live from the URL; length buckets; episode or movie | M0 for URL kind (detail §6); buckets and episode or movie unspecified | arch §7 step 1; detail §6 | specified | free and offline | `normalizer_url_vectors` checks the kind column; no test for buckets |
| F38 | YouTube Data API enrichment: category, tags, duration, channel ID | M4 | arch §7; review F3 | specified | 1 quota unit per 50 videos; 10,000 per day; per-run cap "e.g. 500" (arch §10) | M4 exit: at least 90% of YouTube items have a category |
| F39 | TMDB enrichment: genres (recommendations in F47) | M4 | arch §7 | specified | paginated, 20 per page; about 40 to 50 requests per second; back off on 429; attribution in the UI (review §2) | None named |
| F40 | Twitch Helix enrichment: VOD and game category | M4 | arch §7; review §2 | specified | Client-Id plus app token; client secret in Credential Manager; VODs expire, so enrich promptly | None named |
| F41 | YouTube oEmbed title fallback | M4 | arch §7 | specified | no key; one request per item | None named |
| F42 | Item tags: rule, api or user, with source per tag | M4 | arch §4, §7 | specified | `item_tags` rows | None named. Tag writes come with the M4 tagging UI; the M1 UI is delete-only |
| F43 | Channel tags: tag a channel once, items inherit | M4 | arch §7; detail §4 | specified | `channel_tags` rows | M4 exit (category coverage). No direct test |
| F44 | Shelf: Continue watching (max position 10 to 90% of duration) | M5 | arch §8 | specified | SQL only; needs F32 | M5 exit: all six shelves populated from real data |
| F45 | Shelf: New from your channels (YouTube RSS, top 30 channels, unseen items) | M5 | arch §8; review F3 | specified | one RSS request per channel per day; 15 newest per feed; needs `channel_id` from F38 | M5 exit. Daily job milestone unspecified |
| F46 | Shelf: Drifted away (high 90-day time, nothing in last 30 days) | M5 | arch §8 | specified | SQL only; needs F30 to F32 | M5 exit |
| F47 | Shelf: More like this, shows and films | M5 | arch §8 | specified | TMDB recommendations, 20 per page; title-to-TMDB mapping unspecified | M5 exit |
| F48 | Shelf: More like this, videos | M5 | arch §8 | specified | FTS5 bm25 over the last 50 completed items; needs F32 and F42 | M5 exit |
| F49 | Shelf: Rewatch (at least 2 views, not seen in 6 months) | M5 | arch §8 | specified | SQL only; needs views from M0 and completion from F32 | M5 exit |

## Platform order

Maria's order (arch §0 Q6, 2026-10-25). The user counts use different metrics, so the order is approximate (arch §0). The review's earlier order, with Bilibili second, is superseded.

| Order | Platform | Reported users (metric, as of) | Reason for position | URL pattern (normalizer, detail §6) | Userscript group (M3) |
|---|---|---|---|---|---|
| 1 | YouTube | About 2.7 B, monthly active users, 2026 | Largest by global user count | `watch?v=`, `youtu.be/ID`, `/shorts/`, `/live/`, `/embed/`; `music.youtube.com` watch URLs | First group, priority 1 |
| 2 | Netflix | 325 M+ paid memberships, end 2025 | Second by global user count | `netflix.com/watch/DIGITS`; title comes only from S3 or S6 | First group, priority 2 |
| 3 | Twitch | About 240 M MAU, third-party estimate (Twitch does not publish MAU) | Third by count | `/videos/ID` as VOD; `/<channel>` live as `live:channel:date` | First group, priority 3 |
| 4 | Bilibili | 371 M MAU, Q2 2026 (company report) | Ranked second by count, but mostly a mainland China audience. Maria moved it after Twitch (Q6) | `/video/BV...` with `?p=`; `/video/avDIGITS` | First group, priority 4 |
| 5 | Prime Video | 200 M+ members, last disclosed 2021 | Fifth by count | `primevideo.com/detail/ID`; 12 listed Amazon domains; `/dp` and `/gp/aw/d` only with `video=1` | Second group |
| 6 | Disney+ | About 128 to 132 M paid subscribers, FY2025 | Sixth by count | `disneyplus.com/video/ID` or `/play/ID` (UUID or token) | Second group |
| 7 | Crunchyroll | 21 M paid subscribers, May 2026 | Seventh by count | `crunchyroll.com/watch/ID[/slug]`, 9 uppercase alphanumeric | Second group |

## Recommendation shelves

| Shelf | Data it needs | Method | Cost |
|---|---|---|---|
| Continue watching | Sessions (M3) with max position and duration | Sessions at 10 to 90% of duration, newest first | SQL only; no network |
| New from your channels | YouTube `channel_id` (F38); top 30 channels by time spent | Channel RSS feed, fetched once a day; unseen items only | At most 30 RSS requests per day; each returns 15 newest |
| Drifted away | Time per channel over 90 days; last upload date | Channels with high 90-day time and nothing in the last 30 days | SQL only; no network |
| More like this (shows and films) | Title to TMDB ID mapping (unspecified in docs) | TMDB `/movie/{id}/recommendations` and `/tv/{id}/recommendations` | Paginated, 20 per page; back off on 429 |
| More like this (videos) | Titles and tags of the last 50 completed items | FTS5 bm25 query | SQL only; no network |
| Rewatch | Views (M0); completion (M3) | Completed items with at least 2 views, not seen in 6 months | SQL only; no network |

All shelves score items by `time_spent_on_channel × recency_decay` and cap each shelf at 20 items (arch §8). The `recency_decay` function is unspecified in docs.

## Time spent

Precedence for one item (arch §6, detail §8):

1. Userscript played seconds (S3). Measured. Stored with source `userscript`.
2. Windows media session playing spans (S2). Measured approximation. Stored with source `smtc`. Counts playing time when the gap between samples is 300 s or less.
3. Netflix CSV duration (S6). Used only when the export carries a duration. Detail §13 says a standard export may have only Title and Date.
4. Chromium `visit_duration` (S1). Approximate tab-open time, labelled as approximate. Stored in `approx_duration_s`, never as a `sessions` row.
5. None. The UI shows no time.

Rules:
- The UI resolves overlaps as userscript, then SMTC, then CSV. It never adds overlapping tiers (detail §8).
- Daily totals use the union of intervals, not a sum of sources (detail §8).
- Dedupe keeps the maximum approximation and never sums durations (detail §7).
- Firefox never provides a duration (detail §5).
- M0 produces tier 4 only and writes no sessions. No measured time exists before M3.

## Off or optional by decision

| Item | Decision | Reason (from docs) | Date |
|---|---|---|---|
| S4: yt-dlp with Firefox cookies (YouTube account history, including phone and TV) | Off by decision, not in M0 to M3 | Uses Maria's login cookies. Terms-of-service grey zone. Nothing in the core path may depend on it. Chrome cookie encryption makes Firefox the only option | 2026-10-25 (arch §0 Q3, §2 and §15) |
| S5: Google Takeout export (F29) | Optional, M2 only if Maria asks | Mainly a one-time backfill for a PC-only app | Scope set 2026-10-24 (arch §0 Q2) |
| Phone and TV sources | Out of scope | Omoi is a PC app. The `views.device` column stays, but is almost always `pc` | 2026-10-24 (arch §0 Q2) |
| Jellyfin and Plex (F28) | Opt-in adapters | Network traffic must be configured, not assumed local | detail §13 (no date) |
| S7: manual data requests (Amazon, Twitch, Disney+) | Manual. No milestone | Arch §2 lists it as manual backfill | unspecified in docs |

## Explicitly not doing

- No async runtime, ORM, HTTP, logging, regex, TOML or hashing dependency in M0 (detail §1). Arch §3 names `tiny_http` and `ureq`, which the detail doc does not allow in M0.
- No cloud sync and no telemetry (arch §11).
- No local ML classifier in v1 (arch §7).
- No browser cookies in M0, and no cookie scraping of history pages (detail header, arch §2). S4 stays off.
- No writes to browser databases. No Omoi migrations or WAL settings on browser copies (detail §5).
- No fetches, redirects or title-page matching in the normalizer (detail §6).
- No page-origin allowlist used as authentication. The listener rejects page origins and uses a token (detail §13).
- No file watcher (arch §9). No polling loops anywhere (arch §10).
- No resident process except `omoi listen` (review §4).
- No automatic VACUUM, and no automatic reset or delete of a damaged DB (detail §4).
- No scraping of pages for thumbnails. A missing thumbnail byte count is never counted as zero (detail §11).
- No API keys or secrets in files. Keys go in Windows Credential Manager (arch §11).
- No named IANA time zone or DST lookup in M0. The offset is fixed at `+08:00` (detail §3).
- No sessions in the M0 export (detail §2).
- No content hash for local files in M0. Local identity is the path (detail §6).
- No CORS headers and no fallback to another port (detail §13).
- No Linux product. Linux is the test platform only (detail §1).

## Milestone map

| Milestone | Features delivered | Exit check (from docs) |
|---|---|---|
| M0 | F01 to F19; F37 for URL kind | A week of real history imported. Idempotent re-run adds 0 rows. Warm run under 1 s (copy measured separately). Rejected and skipped counts visible. Linux acceptance tests pass. GNU Windows build and scheduler smoke test pass. Resource report recorded (detail §12) |
| M1 | F20 to F23 | Page load under 200 ms at 10,000 items (arch §14) |
| M1.5 | F24, F25 | Report written to `thumbs-measure.json`. Cache ships only if the projection is at most 100,000,000 bytes (arch §14, detail §11) |
| M2 | F26 to F29 (F29 optional) | Netflix and local items visible. No duplicates against S1 (arch §14) |
| M3 | F30 to F35 | Listener at most 10,000,000 bytes working set, about 0% CPU over 1 h of playback (arch §14). Remeasure with sessions and a full inbox (detail §12) |
| M4 | F38 to F43 | At least 90% of YouTube items have a category (arch §14) |
| M5 | F44 to F49 | All six shelves populated from real data (arch §14) |

## Open items

As of 2026-10-25, no owner questions are open (arch §15). The items below are gaps the docs leave.

Unspecified in docs:
- `recency_decay` function and the shelf score constants (arch §8).
- Title-to-TMDB ID mapping for Netflix, Prime, Disney+ and Crunchyroll titles, which the TMDB shelves need.
- Exact per-run enrichment cap. Arch §10 says "e.g. 500".
- Daily job: arch §9 runs `collect --daily` for RSS and enrichment, but detail §2 says `--daily` is not an M0 command and no milestone assigns it.
- Milestone for time-spent views (F36) and the duration source used before M4 for completion.
- Length-bucket and episode or movie rules (F37): milestone and duration source.
- Thumbnail URLs for platforms other than YouTube. Arch §0 requires documented provider URLs first.
- RAM budget for `omoi ui`.
- Twitch credentials contract: review §2 adds a client secret and token refresh, but no M4 contract covers them.
- Netflix CSV columns and format versions beyond Title and Date (detail §13). Arch §2 S6 now treats duration and device as present only if the export carries them.
- Userscript manager behaviour for `Origin` headers. Detail §13 says to test the real manager before release. No result is recorded.
- Chromium `-journal` copy claim is unverified (review §2).
- Measurements: no numbers exist for any budget. Windows smoke tests have not run (detail §12).

Resolved on 2026-10-28 (the docs were edited to agree; build contract wins):
- Thumbnail cap is 100 MB everywhere, including the detail §11 example JSON. The M1.5 gate is the detail wording: ship only at or under 100 MB, otherwise stay disabled until Maria decides on a bounded LRU cache.
- One `omoi.exe` plus the windowless `omoiw.exe` helper (detail §10). No HTTP dependency in M0 (detail §1); the listener/UI server and enrichment client are chosen at M1/M3.
- Export is CSV and JSON in M0 (detail §2). Prime uses the 12 named domains and `video=1` (detail §6). Dedupe is the fixed 1,800 s anchor (detail §7).
- Takeout ZIP needs its own dependency decision (detail §13); until then the owner unzips the JSON into the drop folder. The network budget names channel RSS (arch §8) and none in M0. The CPU budget is processing time per warm run, copy time separate.
- M3 sessionizing runs inside `omoi collect`; tags are written from M4, not M1.

Still open: Netflix CSV columns beyond Title and Date, and the items under Unspecified in docs above.
