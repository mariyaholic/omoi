# Omoi — watch-history architecture (draft 2)

Name: **Omoi** (思い, "thoughts / memories"); binary and subcommands `omoi`, data in `%LOCALAPPDATA%\omoi\`.
(Draft 1 used the working name *ashiato*; renamed by Maria on 2026-10-24.)
Status: design only, nothing built. Date: 2026-10-24; conflicts with the build contract resolved 2026-10-28 (see the Reconciliation note after §0). Author: claude.scout (opus), for Maria. Draft 2 edits: Ava.tower, recording Maria's §15 answers (see §0) and the review in `docs/omoi-design-review.md`.

```
From→To: claude.scout → Ava.tower
Done: full architecture for a local, low-overhead, multi-platform watch-history app (this file)
Next: Ava reviews the design and proposes a build plan; Maria answers §15 open questions; first build step is M0 (browser-history collector)
Files: docs/watch-history-architecture.md (branch docs/watch-history-architecture, mariyaholic/mariyaholic)
Verify: read this file; every source/API claim in §2 and §13 is checkable against the vendor docs named there
Blockers: none for review; build waits on Maria's answers to §15
```

---

## 0. Decisions (2026-10-24)

Maria answered §15 Q1–Q5 on 2026-10-24 and Q3/Q6–Q8 on 2026-10-25. These override anything below that disagrees.

| # | Question | Maria's answer | Effect on this design |
|---|---|---|---|
| 1 | Which platforms matter most? | The top platforms by user count (YouTube first, etc.) | Platform work (userscripts, normalizer depth, enrichment) is ordered by global user count; see the table below. |
| 2 | PC vs phone/TV split? | Doesn't matter: Omoi is a PC app | PC-only scope. Phone/TV-oriented sources S4 (yt-dlp account history) and S5 (Takeout) are demoted to **optional**, outside the core milestones. The `views.device` column stays but is almost always `pc`. |
| 3 | yt-dlp with Firefox cookies (S4)? | **Keep it off** (2026-10-25) | S4 is off and not scheduled in any milestone. Revisit only if Maria asks. Nothing in the core path may depend on it. |
| 4 | Thumbnails offline? | Yes, but first as a test that measures disk use | New milestone **M1.5 — thumbnail disk-use test** (§14) with a hard cap; the cache only becomes a feature if the measured numbers fit the budget. |
| 5 | Keep the name *ashiato*? | Rename to **Omoi** | Binary `omoi.exe`; subcommands `omoi collect`, `omoi listen`, `omoi ui`, `omoi export`; data dir `%LOCALAPPDATA%\omoi\`; Task Scheduler folder `\Omoi\`. |
| 6 | Bilibili at #2 or after Twitch? | **Move it after Twitch** (2026-10-25) | Work order is now YouTube, Netflix, Twitch, Bilibili, Prime, Disney+, Crunchyroll. |
| 7 | Thumbnail cache cap? | **100 MB** (2026-10-25) | Default `thumb_cache_bytes = 100000000`, LRU by last view; M1.5 ships the cache only if the projection fits 100 MB or the cache is useful at that cap. |
| 8 | Which browser profiles does M0 read? | **All found, with an exclude list** (2026-10-25) | Discovery reads every Chrome/Edge/Brave/Firefox profile; new config key `profile_excludes` skips named ones. |

**Platform order** (used for M3 userscripts and M4 enrichment order): global user count, except Bilibili, which Maria moved after Twitch (Q6). The platforms report different metrics, so this is an ordering, not a like-for-like comparison:

| Rank | Platform | Reported users | Metric, as of | Source |
|---|---|---|---|---|
| 1 | YouTube | ~2.7 B | monthly active users, 2026 | [Vimond 2026 landscape](https://www.vimond.com/news/video-strategic-overview-of-the-landscape-in-2026) |
| 4 | Bilibili | 371 M | MAU, Q2 2026 (company report) | [Bilibili Q2 2026 results (SEC EX-99.1)](https://www.sec.gov/Archives/edgar/data/1723690/000119312526371261/d434015dex991.htm) |
| 2 | Netflix | 325 M+ | paid memberships, end 2025 | [OTTweek ranking](https://ottweek.com/compare) |
| 3 | Twitch | ~240 M | MAU, third-party estimate (Twitch does not publish MAU) | [Twitch statistics (third-party)](https://www.shanethegamer.com/research/twitch-statistics/) |
| 5 | Prime Video | 200 M+ | Prime members (video bundled), last disclosed 2021 | [OTTweek ranking](https://ottweek.com/compare) |
| 6 | Disney+ | ~128–132 M | paid subscribers, FY2025 | [OTTweek](https://ottweek.com/compare), [FlixPatrol](https://flixpatrol.com/streaming-services/subscribers/) |
| 7 | Crunchyroll | 21 M | paid subscribers, May 2026 | [Anime News Network](https://www.animenewsnetwork.com/press-release/2026-05-08/crunchyroll-reaches-21-million-subscribers/.237189) |

Bilibili ranks second on global users but is mostly a mainland-China audience, so Maria moved it after Twitch (Q6, 2026-10-25). Rows above keep their user numbers; only the rank changed.

## Reconciliation note (2026-10-28)

Detail doc governs where it differs from this draft: schema (§4), dedupe (§5 step 3), dependencies (§3), URL patterns (§12), export formats and the M1.5 gate (§14). Two gaps the docs left are now decided, to be revisited at M3 and M1:

- **Who turns listener events into sessions (M3):** `omoi collect` does, inside its single `BEGIN IMMEDIATE` transaction, reading `raw_events` the listener appended and deleting processed rows when it commits their `sessions` (detail §4 and §8). The listener only appends and never writes `sessions`.
- **Tags in M1:** the M1 UI offers delete only. Tag writes arrive with the tagging UI in M4 (§14).

## 1. Goal and budgets

Collect what Maria watches **on her PC** on YouTube, Bilibili, Netflix, Twitch, Prime Video,
Disney+, Crunchyroll (global user-count order, §0) and local players, store it **locally** in one SQLite file, and show it as a
browsable catalogue with time spent, types and recommendations.

Hard budgets (measured, not hoped):

| Resource | Budget |
|---|---|
| RAM, collector | 0 between runs (it exits); < 30 MB peak during a run |
| RAM, listener (optional resident) | ≤ 10 MB working set |
| CPU | collector: processing < 1 s per warm run (copy time reported separately), once an hour; listener ≈ 0 % (event-driven, no polling loops) |
| Disk | ≤ 30 MB per 100k views incl. FTS index; thumbnails off until the M1.5 disk-use test passes, then a capped cache (default cap 100 MB) |
| Network | only enrichment APIs and feeds named in §2 and §8 (channel RSS); none at all in M0; nothing leaves the PC otherwise |

## 2. Data sources

No mainstream platform offers a "my watch history" API (YouTube removed it in 2016). So the
app joins several partial sources on a shared key `(platform, external_id)`.

| # | Source | Automatic? | Gives | Frequency |
|---|---|---|---|---|
| S1 | Browser history DBs: Chrome/Edge/Brave `…\User Data\<Profile>\History`, Firefox `places.sqlite`; every profile found, minus `profile_excludes` (Q8) | yes | URL, page title, visit time, Chrome `visit_duration` | hourly |
| S2 | Windows media session (SMTC, `Windows.Media.Control`) | yes (listener) | now-playing title/artist, play/pause, position, source app | live events |
| S3 | Userscripts (Violentmonkey/Tampermonkey) per site | yes (listener) | exact video id, played seconds, duration, progress, clean titles | live events |
| S4 | *(OFF by Maria's decision, 2026-10-25)* `yt-dlp --cookies-from-browser firefox --flat-playlist -J ":ythistory"` | yes, if enabled | YouTube account history incl. phone/TV | daily |
| S5 | *(optional)* Google Takeout scheduled export → Google Drive for desktop folder | yes after setup | full YouTube history (`watch-history.json`), mainly a one-time backfill for a PC-only app | every 2 months |
| S6 | Netflix `ViewingActivity` CSV (userscript auto-saves on visiting the page) | semi | title and date always; duration and device **only if the export carries them** (detail §13: a standard export may have only Title and Date) | when Maria opens the page (~monthly) |
| S7 | Amazon / Twitch / Disney+ data requests | manual | backfill | ~6-monthly |
| S8 | Local players: mpv `watch_later`, VLC `vlc-qt-interface.ini` recent list, Jellyfin/Plex API | yes | local files watched | hourly |

Notes:
- Extensions/userscripts cost nothing for personal use: Chrome/Edge/Brave "Load unpacked" in
  developer mode is free (the $5 is only for Web Store publishing); userscripts need no
  packaging. Recent Chrome needs "Allow user scripts" on the userscript manager.
- S4 is **off and outside M0–M3** (Maria, 2026-10-25). If enabled later, it uses Maria's login cookies. It is maintained upstream (not a home-made scraper) but is a
  terms-of-service grey zone; it stays opt-in. Chrome's app-bound cookie encryption breaks
  `--cookies-from-browser chrome` on Windows, hence Firefox.

## 3. Components

```
┌──────────── scheduled (Task Scheduler, hourly; daily for RSS) ────────────┐
│ omoi collect                                                              │
│   readers:  S1 browsers · S6 CSV drop · S8 players                        │
│             (optional: S5 Takeout; S4 off)                                │
│   pipeline: normalize → dedupe → sessionize → enrich (rate-limited) → tag │
│   exits when done                                                         │
└─────────────────────────────────────┬─────────────────────────────────────┘
                                      │ writes
┌──────── resident, optional ────────┐▼
│ omoi listen  (~5–10 MB)            │ history.sqlite  (WAL, one writer at a
│   SMTC events (S2)                 │                  time via busy_timeout)
│   127.0.0.1:<port>/event ← S3      │ ──────────────────┐
└────────────────────────────────────┘                   │ reads
                               ┌─────────────────────────▼─────────────────┐
                               │ omoi ui  (on demand)                      │
                               │   serves one local page on 127.0.0.1,     │
                               │   opens the browser, exits on idle 10 min │
                               └───────────────────────────────────────────┘
```

**One binary, three subcommands** (plus a tiny windowless `omoiw.exe` helper for the scheduled collect, detail §10). Rust, matching the estate's other crates (raw Win32 /
`windows` crate style as in mh-ticker, latch, shijima): `rusqlite` with `bundled` (FTS5),
`windows` crate (`Media_Control`, `Foundation`) for SMTC. M0 has no HTTP dependency at all
(detail §1); the listener/UI server and enrichment client (`tiny_http`, `ureq` were the draft-1 idea)
are decided at M1/M3 against the size budget, with standard-library bounded HTTP first (detail §13). Build on the D:\rust GNU toolchain (MSVC link is broken
on this machine). Target binary ≤ 6 MB.

Why a separate listener rather than a tray app: everything except live time-tracking works
with **no resident process at all**. The listener is the only thing that must stay up, so it
stays tiny and does nothing but append events.

## 4. Data model

*Illustrative. The schema that is built is `src/db/0001.sql` in detail §4, which adds `channel_id`, `approx_duration_s`, `metadata_rank`, `sessions.source`, the `platform` CHECK, the `raw_events` size CHECK and the FTS triggers.*

```sql
CREATE TABLE items(              -- one row per distinct video/episode/stream
  id INTEGER PRIMARY KEY,
  platform TEXT NOT NULL,        -- youtube|netflix|twitch|crunchyroll|bilibili|prime|disney|local
  external_id TEXT NOT NULL,     -- video id, Netflix title id, file hash, …
  title TEXT, channel TEXT, series TEXT, season INT, episode INT,
  duration_s INT, published_at INT, kind TEXT,  -- video|short|live|vod|episode|movie
  enriched_at INT,
  UNIQUE(platform, external_id));

CREATE TABLE views(              -- one "I opened/watched this" event after dedupe
  id INTEGER PRIMARY KEY,
  item_id INT NOT NULL REFERENCES items,
  started_at INT NOT NULL,       -- unix seconds UTC
  source TEXT NOT NULL,          -- chrome|edge|brave|firefox|smtc|userscript|ytdlp|takeout|netflix_csv|…
  device TEXT,                   -- pc|phone|tv|unknown
  UNIQUE(item_id, started_at, source));

CREATE TABLE sessions(           -- measured playing time (S2/S3/S6)
  id INTEGER PRIMARY KEY,
  item_id INT NOT NULL REFERENCES items,
  start INT NOT NULL, end INT NOT NULL,
  played_s INT NOT NULL, max_position_s INT);

CREATE TABLE item_tags(item_id INT, tag TEXT, source TEXT,   -- rule|api|user
  PRIMARY KEY(item_id, tag));
CREATE TABLE channel_tags(platform TEXT, channel TEXT, tag TEXT,
  PRIMARY KEY(platform, channel, tag));
CREATE TABLE checkpoints(source TEXT PRIMARY KEY, cursor TEXT);  -- last visit_time / file hash / etc.
CREATE TABLE raw_events(id INTEGER PRIMARY KEY, at INT, source TEXT, body TEXT); -- listener inbox, pruned after ingest

CREATE VIRTUAL TABLE items_fts USING fts5(title, channel, series, content='items', content_rowid='id');
```

Rough size: items ~200 B, views ~40 B, sessions ~40 B, FTS ~100 B per item.

## 5. Ingest pipeline

1. **Read incrementally.** Each reader keeps a cursor in `checkpoints` and reads only newer
   rows/files.
   - Chrome-family: copy `History` (+ `-journal`) to a temp file (the live DB is locked), then
     `SELECT u.url, u.title, v.visit_time, v.visit_duration FROM visits v JOIN urls u ON v.url=u.id WHERE v.visit_time > ?`.
     Time = µs since **1601-01-01**.
   - Firefox: copy `places.sqlite` (+ `-wal`), query `moz_historyvisits` ⨝ `moz_places`.
     Time = µs since 1970.
2. **Normalize** by URL pattern (§12) to `(platform, external_id)`; drop non-matches early
   (most browser history is not video).
3. **Dedupe** into `views`: a fixed 1,800 s window anchored at the first view (no chaining) across
   sources = one view; keep the richest source's metadata (detail §7).
4. **Sessionize** listener events into `sessions`: play→pause/stop spans, summed; gaps
   > 5 min close a session.
5. **Join SMTC to items**: SMTC has no URL, so match `(title, time)` against S1/S3 views
   within ±10 min; unmatched SMTC sessions become `platform=unknown` items keyed by
   `source_app + title` until a later match.
6. **Enrich** (§7), rate-limited, batch, only items with `enriched_at IS NULL`.
7. **Tag** (§7), then refresh `items_fts`.

## 6. Time spent

Best available wins, per item: S3 userscript played seconds → S2 SMTC playing spans →
S6 Netflix CSV duration → Chrome `visit_duration` (tab open time, marked *approximate*) →
none. The UI shows which tier a number came from.

Views offered: total and per platform/channel/tag; per day/week heatmap; "completed"
(max position ≥ 90 % of duration) vs "sampled" (< 10 %).

## 7. Cataloguing (video types)

Layered, cheapest first; every tag stores its `source` so the user always overrides:

1. **Rules (free, offline):** `/shorts/` or duration < 60 s → `short`; live flag → `live`;
   length buckets `<5m`, `5–20m`, `20–60m`, `>60m`; Netflix/Prime with season/episode →
   `episode`, else `movie`.
2. **APIs (free keys, stored in Windows Credential Manager, never in files):**
   - YouTube Data API v3 `videos.list?part=snippet,contentDetails&id=<50 ids>` — category,
     tags, duration; 1 quota unit per 50 videos against 10,000/day.
   - TMDB search + `/genre` — genres for Netflix/Prime/Disney+/Crunchyroll titles.
   - Twitch Helix `videos` / `games` — category.
   - YouTube oEmbed (no key) as a title-only fallback.
3. **Channel tags:** tag a channel once ("cooking", "VTuber", "lecture"); all its items
   inherit. Expected to be the most useful layer in practice.

No local ML classifier in v1 — not worth the RAM/disk.

## 8. Recommendations

All cheap SQL or free feeds; no model server.

| Shelf | How |
|---|---|
| Continue watching | sessions with max position 10–90 % of duration, newest first |
| New from your channels | YouTube channel RSS `youtube.com/feeds/videos.xml?channel_id=…` for the top 30 channels by time spent, fetched by the collector once a day; unseen items only |
| Drifted away | channels with high 90-day time but nothing in the last 30 days |
| More like this (shows/films) | TMDB `/movie/{id}/recommendations`, `/tv/{id}/recommendations` |
| More like this (videos) | FTS5 `bm25` over titles + tags of the last 50 completed items |
| Rewatch | completed items with ≥ 2 views, not seen in 6 months |

Score shelf items by `time_spent_on_channel × recency_decay`; cap each shelf at 20.

## 9. Scheduling

- `omoi collect` — Task Scheduler, hourly, **"run only when user is logged on"** (changed in
  draft 2: browser profiles and Credential Manager are per-user, and nothing new is browsed
  while logged off), low priority, battery-safe, hidden (binary built with the `windows`
  subsystem so no console window flashes). Also on logon.
- `omoi collect --daily` — RSS feeds, enrichment batch (S4 yt-dlp only if Maria enables it).
- Takeout (optional): Maria sets up once in takeout.google.com → YouTube → history only, JSON format,
  "export every 2 months for 1 year", delivery to Drive; the hourly collector checks the synced
  Drive folder and imports any new export (idempotent by file hash). No file watcher. ZIP support needs a separate dependency decision (detail §13), so until then Maria unzips `watch-history.json` into the folder.
- `omoi listen` — Task Scheduler at logon (optional; off = no time tracking).

## 10. Resource discipline

- No polling loops anywhere: SMTC is event subscription; the HTTP endpoint blocks on accept.
- SQLite: WAL, `PRAGMA journal_size_limit`, weekly `PRAGMA optimize`; `raw_events` pruned
  after ingest; vacuum only on explicit command.
- Enrichment backs off and caps per run (e.g. 500 items) so first-run backfill spreads
  over days instead of spiking.
- Measure with the existing `.probe_tmp/measure.ps1` pattern from mh-ticker (working set +
  CPU over 60 s) and record the numbers in the README.

## 11. Privacy and security

- Everything lives in `%LOCALAPPDATA%\omoi\`. No telemetry, no cloud sync.
- Listener binds 127.0.0.1 only and requires a per-install random token in a header;
  userscripts get it at install time and send events with `GM_xmlhttpRequest` (extension
  context, so page scripts never see the token). Draft 2, from the review: the old
  "platform `Origin` allowlist" is replaced. Requests carrying a web-page `Origin` are rejected
  (they can only come from page context), the `Host` header must be `127.0.0.1:<port>`
  (DNS-rebinding guard), no CORS headers are ever sent, bodies are capped at 16 KB, and
  `raw_events` is capped in size. See `docs/omoi-design-review.md` §4.
- `omoi ui` uses its own per-launch token in the URL for every state-changing request
  (delete/tag), so other local web pages cannot CSRF it.
- API keys in Windows Credential Manager. yt-dlp cookie use (S4) is off; if Maria enables it, it is opt-in and documented.
- Export: `omoi export --csv|--json`; delete: per item, per platform, or everything.

## 12. URL patterns (normalizer)

| Platform | Pattern → external_id |
|---|---|
| YouTube | `youtube.com/watch?v=ID`, `youtu.be/ID`, `youtube.com/shorts/ID`, `youtube.com/live/ID`, `m.youtube.com/…` |
| Netflix | `netflix.com/watch/ID` (title only from S3/S6; history title is just "Netflix") |
| Twitch | `twitch.tv/videos/ID` (VOD); `twitch.tv/<channel>` live → `live:<channel>:<date>` |
| Crunchyroll | `crunchyroll.com/watch/ID/…` |
| Bilibili | `bilibili.com/video/BV…` (+ `?p=` part) |
| Prime Video | `primevideo.com/detail/ID`; Amazon domains are the 12 named in detail §6, and `/dp`, `/gp/aw/d` only with `video=1` (detail §6 governs all URL patterns) |
| Disney+ | `disneyplus.com/(video|play)/ID` |

## 13. Known traps

- Chrome holds `History` in exclusive locking mode; always copy it (plus any `-journal`/`-wal`)
  first. Chrome expires visits after 90 days, so hourly collection is also the archive.
- YouTube single-page navigation often records the title as plain "YouTube"; Netflix always
  does. Titles must come from enrichment, S3 or exports.
- Reloads/seeks create multiple visits → the 30-min dedupe window.
- Takeout `watch-history.json` timestamps are ISO-8601 UTC; removed videos have no title.
- Netflix CSV has no stable title id, only display title → match to items by title + date.
- SMTC metadata quality depends on the site setting Media Session metadata; YouTube does,
  some players don't.

## 14. Milestones

Reordered on 2026-10-24 for a PC-only app (§0). S4 is not in any core milestone; S5 is optional.

| M | Scope | Done when |
|---|---|---|
| M0 | `omoi collect` with S1 (Chrome/Edge/Brave/Firefox), schema + migrations, normalizer (all 7 platforms' URL patterns), dedupe, `omoi export` (CSV and JSON) | a week of real history imported; idempotent re-run adds 0 rows; run < 1 s on a warm cache (copy time measured separately) |
| M1 | `omoi ui` — timeline, search (FTS), per-platform/channel views | page loads < 200 ms on 10k items |
| M1.5 | **Thumbnail disk-use test**: `omoi thumbs --measure --limit 500` downloads the smallest usable thumbnail (YouTube `mqdefault.jpg`, 320×180) for the newest 500 items into `%LOCALAPPDATA%\omoi\thumbs\`, records bytes per platform, extrapolates to the full catalogue, then deletes nothing until Maria reads the numbers | report written to `thumbs-measure.json`; hard stop at 50 MB during the test; feature ships only if projected full-catalogue size ≤ 100 MB (cap configurable); otherwise the cache stays disabled until Maria decides on a bounded LRU cache (detail §11) |
| M2 | S6 Netflix CSV + S8 local players; *(optional)* S5 Takeout backfill | Netflix and local items visible; no duplicates vs S1 |
| M3 | `omoi listen` — SMTC + userscripts in platform order (YouTube, Netflix, Twitch, Bilibili first; Prime, Disney+, Crunchyroll next), sessions, time spent | listener ≤ 10 MB, ≈ 0 % CPU over 1 h of playback |
| M4 | enrichment + tags + channel tagging UI (same platform order) | ≥ 90 % of YouTube items have category |
| M5 | recommendation shelves | all six shelves populated from real data |
| — | S4 yt-dlp | off (Maria, §15 Q3, 2026-10-25); not scheduled |

## 15. Open questions for Maria

Answered 2026-10-24 (recorded in §0):
1. ~~Which platforms matter most?~~ Top platforms by user count.
2. ~~PC vs phone/TV split?~~ Doesn't matter; PC app.
3. ~~yt-dlp with Firefox cookies (S4)?~~ Keep it off (2026-10-25).
4. ~~Thumbnails offline?~~ Yes, after the M1.5 disk-use test.
5. ~~Keep the name?~~ Renamed to Omoi.

From the design review (`docs/omoi-design-review.md` §6), answered 2026-10-25:
6. ~~Bilibili at #2 or after Twitch?~~ Move it after Twitch.
7. ~~Thumbnail cap?~~ 100 MB.
8. ~~Which profiles?~~ All found, with an exclude list (`profile_excludes`).

No questions are open.

## 16. Review asks for Ava

Answered in `docs/omoi-design-review.md` (Ava.tower, 2026-10-24).

- Sanity-check the source matrix and budgets; flag anything that needs a resident process
  beyond `listen`.
- Propose who builds M0 (roster impl) and who reviews (not the implementation model).
- Nothing here needs a push to main, a deploy, or a protected-file edit.
