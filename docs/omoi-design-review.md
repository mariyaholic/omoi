# Omoi — design review (Ava.tower)

Reviewer: Ava.tower, for Maria. Date: 2026-10-24.
Reviewed: `docs/watch-history-architecture.md` draft 1 (claude.scout / opus, commit `22e97cd`) and the
draft 2 edits that record Maria's §15 answers (§0 of that doc).

**Verdict:** the design holds up. The sources are the right ones, the "collector exits,
listener is optional" shape is right, and the budgets are plausible. Every claim I could check is correct
except one: the listener's **`Origin` allowlist doesn't work as written** (§4.1). It is already fixed in draft 2 §11.
The other findings are schema gaps and one scheduling setting. None of them changes the architecture.

---

## 1. Top findings

| # | Severity | Finding | Fix (applied in draft 2?) |
|---|---|---|---|
| F1 | **High** | The "reject any `Origin` not in the platform allowlist" rule fails both ways. Userscripts that use `GM_xmlhttpRequest` send from the extension context, so the platform origin is not what arrives. Scripts that use page-context `fetch` would expose the token to the site's own scripts. Since Chrome 142 they also trigger the Local Network Access prompt. | Use `GM_xmlhttpRequest` only. Reject requests that carry a web-page `Origin`, check `Host`, send no CORS headers. **Applied** (§11). |
| F2 | Medium | The hourly task used "run whether user is logged on". Browser profiles and Credential Manager are per-user, and nothing new is browsed while logged off. That setting also needs a stored password. | "Run only when user is logged on", hidden, `windows` subsystem (no console flash). **Applied** (§9). |
| F3 | Medium | The "new from your channels" shelf needs the YouTube **channel id** (`UC…`) for the RSS URL, but `items` only stores a channel *name*. | Add `items.channel_id` (from `videos.list` `snippet.channelId`). For Astra's detail spec. |
| F4 | Medium | §6 says "the UI shows which tier a number came from", but `sessions` has no `source` column. | Add `sessions.source`. For Astra's detail spec. |
| F5 | Medium | `items_fts` is an external-content FTS5 table, so it only stays in sync with triggers or explicit `rebuild`/`delete` commands. Without them, deletes leave stale search hits, which matters for the privacy delete in §11. | Add AFTER INSERT/UPDATE/DELETE triggers. For the detail spec. |
| F6 | Low | Budget risk: "collector run < 1 s per hour" while copying Chrome `History` (commonly tens of MB, sometimes 100 MB+) for each browser and profile. | Copy only when the file's mtime or size changed since the last checkpoint. Report copy time separately from the run (M0 "done when" updated). |
| F7 | Low | `UNIQUE(item_id, started_at, source)` together with a 30-minute dedupe window means two sources can still race. Dedupe has to run as one write transaction. | Detail spec: dedupe inside a single `BEGIN IMMEDIATE`. |
| F8 | Low | Twitch live key `live:<channel>:<date>` doesn't say which time zone the date uses. | Use the local date at stream start (Asia/Manila for Maria). Detail spec. |

## 2. Claim spot-check

| Claim in the design | Status | Evidence |
|---|---|---|
| Chrome `History`: `visits.visit_time` joins `urls`, and `visit_duration` exists | **Confirmed** | [history_types.h](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/components/history/core/browser/history_types.h), [schema walkthrough](https://en.wikiversity.org/wiki/Chromium_browsing_history_database) |
| `visit_duration` = tab-open time, approximate | **Confirmed**. It counts active and inactive time and is only written when the visit ends (navigate away or tab close), so open tabs show 0 until then | same as above |
| Chrome time = µs since 1601-01-01 UTC | **Confirmed**. `unix_s = visit_time/1e6 − 11644473600` | same as above |
| Chrome locks `History`, so copy it first | **Confirmed**. Chrome switches the DB to exclusive locking mode after init | [history_database.cc](https://chromium.googlesource.com/chromium/src/+/673a5aee77ad12e118c8edac73359acdeb7f491c/components/history/core/browser/history_database.cc), [SO 53657704](https://stackoverflow.com/questions/53657704/chrome-history-db-access-is-locked) |
| Chrome keeps ~90 days of history | **Confirmed**. `kExpireDaysThreshold = 90` | [history_backend.h](https://chromium.googlesource.com/chromium/src/+/673a5aee77ad12e118c8edac73359acdeb7f491c/components/history/core/browser/history_backend.h) |
| Firefox `moz_historyvisits` ⨝ `moz_places`, µs since 1970 | **Confirmed**. `visit_date` is PRTime and joins on `place_id` | [nsPlacesTables.h](https://searchfox.org/firefox-main/source/toolkit/components/places/nsPlacesTables.h), [Forensics Wiki](https://forensics.wiki/mozilla_firefox_3_history_file_format/) |
| SMTC via `Windows.Media.Control`: now-playing, play/pause, position, source app, event-driven | **Confirmed**. `GlobalSystemMediaTransportControlsSessionManager.RequestAsync`, `CurrentSessionChanged`/`SessionsChanged`, per-session `MediaPropertiesChanged`, `PlaybackInfoChanged`, `TimelinePropertiesChanged`, `SourceAppUserModelId`. There's no URL, so the §5 step-5 join is needed | [SessionManager](https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssessionmanager), [Session](https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssession) |
| YouTube Data API `videos.list`: 1 unit per 50 videos, 10,000/day | **Confirmed**. A call costs 1 unit whatever the part, and `id` takes up to 50 ids, so batched it's 1 unit per 50 | [videos.list](https://developers.google.com/youtube/v3/docs/videos/list), [quota calculator](https://developers.google.com/youtube/v3/determine_quota_cost) |
| YouTube removed watch-history API access in 2016 | **Confirmed**. Since 2016-09-12, `HL`/`WL` playlists return nothing | [Data API revision history](https://developers.google.com/youtube/v3/revision_history), [SO 46987690](https://stackoverflow.com/questions/46987690/tracking-youtube-watch-history) |
| YouTube channel RSS `feeds/videos.xml?channel_id=…`, no key | **Confirmed** (still works in Oct 2026). It returns the 15 newest uploads and needs a `UC…` id, not a handle (see F3) | [cors.dev guide](https://cors.dev/guides/youtube-rss-feed), [Feeder KB](https://feeder.co/knowledge-base/rss-feed-creation/youtube-rss/) |
| TMDB `/movie/{id}/recommendations`, `/tv/{id}/recommendations`, `/genre` | **Confirmed**. Paginated at 20 per page. Rate limit is about 40–50 req/s per IP, so back off on 429. TMDB terms require attribution in the UI | [movie recommendations](https://developer.themoviedb.org/reference/movie-recommendations), [TV recommendations](https://developer.themoviedb.org/reference/tv-series-recommendations), [rate limiting](https://developer.themoviedb.org/docs/rate-limiting) |
| Twitch Helix `videos` / `games` for category | **Confirmed**, with a caveat: every Helix call needs `Client-Id` plus an OAuth token. An **app access token** (client-credentials) is enough, but the design must store a **client secret** in Credential Manager and refresh the token. VODs expire on Twitch's side, so enrich them promptly | [Get Videos](https://dev.twitch.tv/docs/api/reference/#get-videos), [Get Games](https://dev.twitch.tv/docs/api/reference/#get-games), [app access tokens](https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/) |
| YouTube oEmbed as a no-key title fallback | **Confirmed** (`youtube.com/oembed?url=…&format=json`, no key) | [TrueScrape overview](https://truescrape.com/blog/youtube-without-an-api-key) |
| Chrome needs "Allow user scripts" for userscript managers | **Confirmed**. It's a per-extension toggle from Chrome 138; before 138 it needed Developer mode | [chrome.userScripts](https://developer.chrome.com/docs/extensions/reference/api/userScripts), [Chrome blog](https://developer.chrome.com/blog/chrome-userscript), [Tampermonkey FAQ Q209](https://www.tampermonkey.net/faq.php?q=Q209) |
| App-bound cookie encryption breaks `--cookies-from-browser chrome` on Windows, so use Firefox | **Confirmed**. Introduced in Chrome 127 on Windows; Firefox import still works | [Chrome security blog](https://chrome.security/2024/07/30/improving-security-of-chrome-cookies-on.html), [yt-dlp #15401](https://github.com/yt-dlp/yt-dlp/issues/15401), [yt-dlp #7271](https://github.com/yt-dlp/yt-dlp/issues/7271) (S4 is off anyway) |
| Listener "rejects any `Origin` not in the platform allowlist" | **Wrong as a design** (F1). `GM_xmlhttpRequest` requests don't carry the page origin; Tampermonkey only lets scripts set `Origin` when the user enables header modification | [Tampermonkey #2747](https://github.com/tampermonkey/tampermonkey/issues/2747), [SO 18178934](https://stackoverflow.com/questions/18178934/what-http-origin-does-gm-xmlhttprequest-send), [Chrome LNA prompt](https://developer.chrome.com/blog/local-network-access) |
| Chromium `History` copy includes `-journal` | **Unverified**. Copy `History-journal` and `History-wal` if present. Harmless either way | — |

## 3. Source matrix and budgets

- **Sources.** S1 + S3 + S2 cover everything on a PC, which is all Maria wants (§0 Q2). Demoting S4/S5 to
  optional costs only phone/TV history and a pre-install YouTube backfill. S5 is still the
  only legal way to backfill YouTube history from before Omoi existed, so it stays as an optional
  one-time import. S6 (Netflix CSV) is the only source of Netflix durations besides S3/S2. S7 can stay manual.
- **Platform order.** By global user count (sources in the doc §0): YouTube, Bilibili, Netflix, Twitch,
  Prime Video, Disney+, Crunchyroll. The metrics are mixed (MAU vs paid subs vs Prime members),
  so the ordering is approximate below #1. Bilibili's #2 rests on a mostly China-only audience,
  so that's question Q6 for Maria.
- **RAM.** Collector < 30 MB peak is realistic for rusqlite + ureq if the readers stream rows instead
  of collecting into Vecs. Listener ≤ 10 MB is realistic for `tiny_http` + WinRT events, with no async runtime.
- **Disk.** Per 100k views: views ~40 B + the UNIQUE index ~35 B ≈ 7.5 MB. Assuming ~40k distinct items: ~200 B
  + ~100 B FTS + unique index ≈ 13 MB. Total ≈ 21 MB, under 30 MB with little margin, so measure in M0.
- **Thumbnails** (Maria Q4). M1.5 is now a measuring step. YouTube `mqdefault.jpg` (320×180) is usually
  ~10–20 KB, so 40k items ≈ 0.4–0.8 GB. That's why the default cap is 250 MB, with LRU eviction by last view. The
  test downloads 500, stops at a 50 MB hard cap, and extrapolates per platform.
- **CPU.** See F6. Hourly copies of large `History` files are the only real cost. Skip unchanged files.

## 4. Resident processes and the localhost listener

**Anything resident besides `listen`? No**, as long as:
- the Takeout/CSV folders are checked by the hourly `collect` (no file watcher). This is applied in draft 2 §9;
- `ui` keeps its idle-exit (10 min) and is started on demand;
- userscripts **buffer events in GM storage and retry**, so the listener can be off or restarting
  without losing data. The listener's only must-be-resident job is SMTC time tracking.

### 4.1 Listener security gaps (draft 1 §11)

1. **Origin allowlist** (F1). Fixed in draft 2: reject any request with a web-page `Origin`, because only
   page-context code sends those, and page-context code could leak the token to the site.
2. **DNS rebinding.** Require `Host: 127.0.0.1:<port>` exactly, and reject `localhost` names resolved via rebinding.
3. **CORS.** Never send `Access-Control-Allow-*` headers, and answer `OPTIONS` with 403. Then no web page can read
   responses, and preflighted writes fail.
4. **Token handling.** Generate 32 random bytes at install, compare in constant time, and store the token in
   `%LOCALAPPDATA%\omoi\token` with a user-only ACL. Rotating it means reinstalling the userscripts.
5. **Limits.** Cap the body at 16 KB, use a short read timeout so a slow client can't hold the single thread, and cap
   `raw_events` (e.g. 100k rows or 20 MB, dropping the oldest unprocessed rows and counting the drops).
6. **The `ui` server is a second listener.** Its delete and tag endpoints need their own per-launch token
   (in the opened URL) to stop CSRF from other pages. It's read-only otherwise. Added to draft 2 §11.
7. **Port.** Use a fixed configurable port so the userscripts know it. Fail loudly if the port is taken, and
   never silently pick another one.

## 5. M0 roster

| Seat | Who | Model | Why |
|---|---|---|---|
| Spec detail | **Astra** | Codex `gpt-6-astra` | Expands this design into a buildable M0 spec (Maria's steer, t144u) |
| Builder (boilerplate) | **Kurisu** + free fallbacks | OpenCode free tier only: `deepseek-v4.1-flash-free` → `glm-5.3-flash-free` → `minimax-m3.1-flash-free` → `mimo-v2.6-flash-free` | M0 is small, well-specified Rust units with acceptance tests, which suits flash models. **No paid models** |
| Reviewer | **Astra** | Codex `gpt-6-astra` | A different model from the builder. **Not opus**, because opus wrote the design |

I didn't find anything in the review that changes the roster. Astra writes the spec and then reviews code written by
other models, so writer and reviewer stay on separate models.

**M0 scope (final):** `omoi collect` reading S1 (Chrome, Edge, Brave, Firefox; all profiles found), the
schema plus migrations including F3–F5, a normalizer for all 7 platforms' URL patterns, dedupe, and
`omoi export --csv`. Windows-only pieces (`%LOCALAPPDATA%` discovery, Task Scheduler registration) sit
behind `cfg(windows)` with Linux stubs so `cargo test` runs anywhere. M0 has no listener, no SMTC and no network.

## 6. New questions for Maria

- **Q6.** Keep Bilibili at #2 (ordered by global users) or move it after Twitch?
- **Q7.** Is a 250 MB cap for the thumbnail cache acceptable, pending the M1.5 measurement?
- **Q8.** Should M0 read every Chromium-family and Firefox profile it finds, or a named list?
- **Q3, still open.** S4 yt-dlp with Firefox cookies. It stays off until you decide.

**Answers (Maria, 2026-10-25):** Q3 keep S4 off · Q6 move Bilibili after Twitch · Q7 100 MB cap · Q8 all profiles found, with an exclude list. Recorded in the architecture §0 and the detail doc.
