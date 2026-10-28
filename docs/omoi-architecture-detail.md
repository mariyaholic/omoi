# Omoi — architecture detail and M0 build contract

Author: Astra. Owner: Maria. Date: 2026-10-24. Status: implementation specification; performance numbers below are acceptance targets, not measurements.

Authority: [architecture draft 2, §0](watch-history-architecture.md) and [Ava.tower review, F1–F8](omoi-design-review.md). M0 is PC-only S1 ingestion, offline, with no resident process. Read all discovered profiles by default, minus `profile_excludes` (Maria Q8, 2026-10-25). Platform order is YouTube, Netflix, Twitch, Bilibili, Prime, Disney+, Crunchyroll (Maria Q6, 2026-10-25). S4 is off by Maria's decision (Q3, 2026-10-25) and not scheduled; S5 is optional. No credentials, browser cookies, enrichment, thumbnail downloads or listener in M0.

## 1. Crate, build and ownership

One Cargo package/workspace member, `omoi`, with a library, console executable and a tiny Windows scheduler executable. No async runtime, ORM, HTTP, logging, regex, TOML or hashing dependency. Standard library implements filesystem operations, rotating logs, the restricted config grammar, XML generation and test fixtures.

```text
Cargo.toml
Cargo.lock                         # commit resolved versions when building
src/lib.rs                         # exports modules, Error/Result
src/main.rs                        # cli::run; maps result to exit code
src/bin/omoiw.rs                    # cfg Windows: windowless collect entrypoint
src/config.rs
src/paths.rs
src/error.rs
src/logging.rs
src/db/mod.rs
src/db/schema.rs                    # include_str! migration registry
src/db/migrations.rs
src/db/0001.sql
src/readers/mod.rs                  # types/traits, factory
src/readers/discover.rs
src/readers/snapshot.rs
src/readers/chromium.rs
src/readers/firefox.rs
src/normalize.rs
src/dedupe.rs
src/sessionize.rs                  # M0 stub, M3 contract
src/export.rs
src/scheduler.rs                   # portable XML/argv builder + cfg dispatch
src/scheduler/windows.rs           # only compiled on Windows
src/cli.rs
src/collect.rs                     # streaming orchestration
examples/synthetic.rs              # deterministic benchmark fixture generator
scripts/measure.ps1                # Windows performance harness
tests/                             # mirror modules; no private history checked in
```

Manifest contract (version ranges are deliberate; lock and test concrete stable-compatible resolutions):

```toml
[workspace]
members = ["."]
resolver = "2"
[package]
name = "omoi"
version = "0.1.0"
edition = "2021"
[dependencies]
rusqlite = { version = "0.37", features = ["bundled", "backup"] }
clap = { version = "4", features = ["derive"] }
thiserror = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
csv = "1"
url = "2"
chrono = { version = "0.4", default-features = false, features = ["std", "clock"] }
[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = ["Win32_Foundation", "Win32_Security", "Win32_System_Threading", "Win32_System_Memory"] }
[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
strip = true
```

No extra dev-dependencies. Tests use generated SQLite fixtures and `std::env::temp_dir()` directories created with `create_dir` + PID + atomic counter, cleaned by RAII. Direct dependencies must stay within the list above; Cargo-managed transitive dependencies are permitted. The `windows` features can be extended for M3 within the same target section.

```sh
# Linux: native stable, system C compiler for bundled SQLite
cargo +stable test --all-targets
cargo +stable build --release --bins
# Maria's Windows terminal: stable GNU, MinGW gcc/ar on PATH
rustup toolchain install stable-x86_64-pc-windows-gnu
cargo +stable-x86_64-pc-windows-gnu test --all-targets
cargo +stable-x86_64-pc-windows-gnu build --release --target x86_64-pc-windows-gnu --bins
```

Do not invoke the broken MSVC linker. Cross-compiling from Linux additionally requires a MinGW C compiler/linker; native Linux tests do not. Check `sqlite_compileoption_used('ENABLE_FTS5') = 1`; refuse migration if unavailable. Primary `omoi.exe` release size target ≤6,000,000 bytes; report helper size separately.

## 2. M0 CLI and output

Clap parser, no implicit/default subcommand. `--help` and `--version` succeed. Options are long-form except Clap's standard help/version aliases. No unimplemented M1–M3 commands appear in M0 help.

| Invocation | Exact behavior |
|---|---|
| `omoi collect [--browser B]... [--dry-run]` | B is `chrome`, `edge`, `brave`, `firefox` or `all`; repeat to select several, reject `all` mixed with others. Absent: config selection. Discover every selected profile. Auto-migrate before ordinary ingest. |
| `omoi export (--csv\|--json) [--out PATH]` | Exactly one format required; stream a consistent read transaction. Default stdout; explicit path must not already exist. No auto-migration or DB creation. |
| `omoi db migrate` | Create directories/DB if absent; migrate to compiled version. Idempotent. |
| `omoi db info` | Read-only JSON: path, user_version, table counts, DB/WAL bytes, SQLite version and FTS5 availability. Missing DB is an error. |
| `omoi schedule install` | Windows: install/replace `\Omoi\Collect` for current user, hourly + logon. |
| `omoi schedule remove` | Windows: remove only `\Omoi\Collect`; absent task succeeds. |
| `omoi paths` | JSON with resolved home/config/db/logs/tmp/thumbs plus browser roots, including nonexistent roots; no creation. |

On Linux, hide `schedule` from help but accept those two forms and return `Unsupported`/exit 5 through a stub. Reject extra arguments. `--daily`, `listen`, `ui`, `thumbs` are not M0 commands.

`collect` writes one JSON summary to stdout; diagnostics go to stderr and bounded logs. Fields: `dry_run`, `profiles_found`, `profiles_unchanged`, `profiles_ok`, `profiles_failed`, `rows_read`, `rows_rejected`, `items_inserted`, `views_inserted`, `views_merged`, `metadata_updated`, `copy_ms`, `read_normalize_ms`, `write_ms`, `total_ms`, `warnings`. Dry-run counts are proposed changes. No discovered profile is a successful no-op with a warning. Explicitly selected but absent browser is also a warned no-op.

Dry-run reads history normally but clones an existing Omoi DB into a temporary DB using SQLite's backup API (read-only source connection), or creates an empty temporary DB. Migrate and ingest that copy. Never advance real checkpoints, create the real DB, register tasks or write real logs; stderr and temporary files are allowed. Remove temporary files on normal exit. A newer real schema still fails.

| Exit | Meaning |
|---|---|
| 0 | Success, including unchanged/no profiles/help |
| 1 | Fatal I/O, DB, export, schema or internal failure; transaction rolled back |
| 2 | CLI or config validation error |
| 3 | Partial collect: at least one discovered profile failed; others committed (also if all discovered profiles failed) |
| 4 | Omoi writer busy after timeout; safe to retry |
| 5 | Unsupported platform operation |

Export is **one row per deduped view**, ordered `(started_at, views.id)`, with columns in this order: `view_id,item_id,platform,external_id,title,channel,channel_id,started_at,source,device,approx_duration_s`. UTC times are integer Unix seconds. CSV is UTF-8, header, RFC4180 quoting, CRLF; null is empty. JSON is a streamed array of objects, null remains null, ending newline. No sessions in M0 export. Titles are raw text (CSV is an interchange format, not spreadsheet formula sanitization). For `--out`, write an adjacent uniquely created temporary file, flush/sync, then publish with a no-clobber operation (`hard_link` then remove temp; failure leaves destination untouched); reject paths inside DB/snapshot/log storage. Stdout broken pipe is a clean exit 0; other write errors exit 1.

## 3. Paths and configuration

Resolution order: nonempty absolute `OMOI_HOME`; Windows `%LOCALAPPDATA%\omoi`; Linux absolute `$XDG_DATA_HOME/omoi`, otherwise `$HOME/.local/share/omoi`. Reject relative `OMOI_HOME`/XDG values; missing required home variable is `Config`. Use `PathBuf`/`OsString`, not lossy path conversion for opening files. Data files: `history.sqlite`, `config.toml`, `logs/`, `tmp/`, `thumbs/`, later `token`. Browser discovery always uses original OS profile directories, never `OMOI_HOME`. No current-directory fallback.

Missing config means defaults; do not generate a file automatically. To honor the dependency limit, M0 supports a documented **subset of TOML**, not arbitrary TOML: flat unique keys, booleans, unsigned decimal integers, double-quoted strings and arrays of double-quoted strings; quoted strings use JSON escapes (a TOML-compatible subset). Strip `#` comments only outside strings. Reject tables, dotted/unknown/duplicate keys, multiline strings and unsupported syntax with line number. File ≤64 KiB. Parse scalar/array values using `serde_json`, integers with checked conversion; no interpolation or shell expansion.

```toml
browsers = ["chrome", "edge", "brave", "firefox"]
profile_paths = []                  # extra absolute History/places.sqlite files
profile_excludes = []               # profiles to skip, e.g. "chrome:Profile 2", "firefox:work"
local_utc_offset = "+08:00"         # Maria: Asia/Manila; stable historical key
busy_timeout_ms = 5000              # allowed 1..60000
journal_size_limit_bytes = 1048576  # allowed 0..16777216
reader_batch_rows = 512             # allowed 1..2048
snapshot_retries = 3                # allowed 1..5
log_file_bytes = 1048576            # allowed 65536..4194304
log_files = 3                      # active + rotated; allowed 1..5
thumb_cache_bytes = 100000000        # reserved M1.5, >= 1 (Maria Q7: 100 MB)
listener_port = 43821              # reserved M3, 1024..65535
```

`profile_paths` entries are prefixed `chrome:`, `edge:`, `brave:` or `firefox:` followed by an absolute OS path, e.g. `firefox:C:\\Profiles\\Maria\\places.sqlite`. Split only at the first colon. Union with discovery; browser filter applies; deduplicate canonical paths. Invalid config is fatal; nonexistent configured files become per-profile failures. Offset accepts `+HH:MM`/`-HH:MM` up to ±14:00. Named IANA zones/DST lookup are not implemented in M0. `+08:00` represents Asia/Manila for Maria; changing it affects only newly normalized live events and warns of potential key splits. Never derive old Twitch dates from today's machine offset.

`profile_excludes` entries are `chrome:`, `edge:`, `brave:` or `firefox:` followed by a profile name: the Chromium profile directory name (`Default`, `Profile 2`, a custom name) or, for Firefox, the `Name=` value from `profiles.ini` or the profile directory basename. Match case-insensitively, after discovery and the `profile_paths` union; an excluded profile is never snapshotted or checkpointed and counts in `profiles_excluded` in the collect summary. An entry that matches nothing is a warning, not a config error, so removing a profile later does not break `collect`.

## 4. Database and migration 0001

All integer timestamps below are UTC Unix seconds. Microseconds remain in reader cursors. Parameterize all SQL values. The following is the complete `src/db/0001.sql` (transaction/user_version managed outside it):

```sql
CREATE TABLE items (
  id INTEGER PRIMARY KEY,
  platform TEXT NOT NULL CHECK(platform IN
    ('youtube','bilibili','netflix','twitch','prime','disney','crunchyroll','local','unknown')),
  external_id TEXT NOT NULL,
  title TEXT, channel TEXT, channel_id TEXT, series TEXT,
  season INTEGER, episode INTEGER,
  duration_s INTEGER CHECK(duration_s IS NULL OR duration_s >= 0),
  published_at INTEGER, kind TEXT, enriched_at INTEGER,
  metadata_rank INTEGER NOT NULL DEFAULT 0,
  UNIQUE(platform, external_id)
);
CREATE TABLE views (
  id INTEGER PRIMARY KEY,
  item_id INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  started_at INTEGER NOT NULL,
  source TEXT NOT NULL,
  device TEXT NOT NULL DEFAULT 'pc',
  approx_duration_s INTEGER CHECK(approx_duration_s IS NULL OR approx_duration_s >= 0),
  UNIQUE(item_id, started_at, source)
);
CREATE INDEX views_item_time ON views(item_id, started_at);
CREATE INDEX views_time ON views(started_at, id);
CREATE TABLE sessions (
  id INTEGER PRIMARY KEY,
  item_id INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  start INTEGER NOT NULL,
  end INTEGER NOT NULL CHECK(end >= start),
  played_s INTEGER NOT NULL CHECK(played_s >= 0 AND played_s <= end-start),
  max_position_s INTEGER CHECK(max_position_s IS NULL OR max_position_s >= 0),
  source TEXT NOT NULL
);
CREATE INDEX sessions_item_start ON sessions(item_id, start);
CREATE TABLE item_tags (
  item_id INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  tag TEXT NOT NULL,
  source TEXT NOT NULL,
  PRIMARY KEY(item_id, tag)
) WITHOUT ROWID;
CREATE TABLE channel_tags (
  platform TEXT NOT NULL,
  channel TEXT NOT NULL,
  tag TEXT NOT NULL,
  PRIMARY KEY(platform, channel, tag)
) WITHOUT ROWID;
CREATE TABLE checkpoints (
  source TEXT PRIMARY KEY NOT NULL,
  cursor TEXT NOT NULL
) WITHOUT ROWID;
CREATE TABLE raw_events (
  id INTEGER PRIMARY KEY,
  at INTEGER NOT NULL,
  source TEXT NOT NULL,
  body TEXT NOT NULL CHECK(length(CAST(body AS BLOB)) <= 16384)
);
CREATE VIRTUAL TABLE items_fts USING fts5(
  title, channel, series, content='items', content_rowid='id'
);
CREATE TRIGGER items_ai AFTER INSERT ON items BEGIN
  INSERT INTO items_fts(rowid,title,channel,series)
  VALUES(new.id,new.title,new.channel,new.series);
END;
CREATE TRIGGER items_ad AFTER DELETE ON items BEGIN
  INSERT INTO items_fts(items_fts,rowid,title,channel,series)
  VALUES('delete',old.id,old.title,old.channel,old.series);
END;
CREATE TRIGGER items_au AFTER UPDATE OF title,channel,series ON items BEGIN
  INSERT INTO items_fts(items_fts,rowid,title,channel,series)
  VALUES('delete',old.id,old.title,old.channel,old.series);
  INSERT INTO items_fts(rowid,title,channel,series)
  VALUES(new.id,new.title,new.channel,new.series);
END;
```

F3: `channel_id` is the stable vendor ID, nullable until enrichment; display `channel` stays independent. F4: `sessions.source` carries the measurement tier. `unknown` is reserved for unmatched M3 SMTC; never emitted by M0 normalizer. `metadata_rank` persists richness across runs; `approx_duration_s` preserves Chromium tab-open duration without claiming measured playback. Tags on channels retain draft semantics (display name); migrate to channel IDs before M4 if needed.

FTS triggers handle search synchronization, including privacy deletes; they do not guarantee forensic disk erasure. Use UPDATE/UPSERT, never `INSERT OR REPLACE` on items. Validate FTS with actual `MATCH` queries and the FTS integrity check, not `SELECT count(*) FROM items_fts`, which can read external content. [SQLite external-content FTS5](https://www.sqlite.org/fts5.html#external_content_tables).

Connection initialization for writable Omoi DBs, outside transactions:

```sql
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;          -- config override, validated integer
PRAGMA journal_mode = WAL;          -- require returned value 'wal'
PRAGMA synchronous = NORMAL;
PRAGMA journal_size_limit = 1048576; -- config override
PRAGMA wal_autocheckpoint = 256;
PRAGMA cache_size = -2048;          -- 2 MiB per connection
PRAGMA mmap_size = 0;
PRAGMA temp_store = FILE;
```

Read-only commands set `query_only=ON`, busy timeout and foreign keys; do not change journal mode. `journal_size_limit` limits retained journal space after checkpoints, **not** active WAL growth. After collection request `wal_checkpoint(PASSIVE)`; a blocked checkpoint warns, does not undo success. Weekly `PRAGMA optimize`, with a reserved checkpoint key `maintenance:optimize` containing last-success UTC; never automatic VACUUM.

Migration algorithm: open; verify FTS5; `BEGIN IMMEDIATE`; read `PRAGMA user_version` **inside the lock**. Version 0 is acceptable only for an empty user schema; reject nonempty unversioned DB. Version >1 fails `SchemaTooNew`. Apply every ordered migration after the stored version, then `PRAGMA user_version=1`, COMMIT. Any SQL error rolls back both schema and version. Re-running at 1 is a no-op. Never reset/delete a damaged DB automatically. Back up existing data before future destructive migrations.

`raw_events` is empty in M0. M3 insertion must enforce both ≤100,000 rows and ≤20,000,000 summed UTF-8 body bytes in the same write transaction: insert, delete oldest IDs until both bounds hold, increment a persistent dropped count (M3 migration). Bound body bytes also at HTTP parsing. These are logical inbox caps; DB/WAL allocation must be measured separately. Delete processed events in the same transaction that commits their sessions.

## 5. Reader contracts, profiles and snapshots

Types live in `readers/mod.rs`; all derive only the traits actually required. `Result<T> = std::result::Result<T, crate::Error>`. No `Vec` containing a whole history.

```rust
pub enum Browser { Chrome, Edge, Brave, Firefox }
pub struct SourceId(pub String); // browser + ':' + reversible profile path encoding
pub struct FileStamp { pub present: bool, pub len: u64, pub mtime_ns: i128 }
pub struct Fingerprint {
    pub main: FileStamp, pub wal: FileStamp, pub journal: FileStamp,
}
pub struct Cursor {
    pub version: u32,              // 1; reject unknown cursor versions
    pub last_native_us: i64,
    pub last_visit_id: i64,        // tuple tie-breaker
    pub fingerprint: Option<Fingerprint>,
}
pub struct RawVisit {
    pub source: SourceId,
    pub browser: Browser,
    pub visit_id: i64,
    pub native_us: i64,
    pub unix_us: i64,
    pub url: String,
    pub title: Option<String>,
    pub approx_duration_s: Option<i64>,
}
pub struct Batch {
    pub visits: Vec<RawVisit>,     // <= reader_batch_rows
    pub next: Cursor,             // last scanned row, including filtered rows
    pub exhausted: bool,
}
pub trait HistoryReader {
    fn id(&self) -> &SourceId;
    fn read_since(&mut self, cursor: &Cursor, limit: usize) -> Result<Batch>;
}
pub fn discover(config: &Config, env: &PathEnvironment) -> Result<Discovery>;
pub fn snapshot(profile: &Profile, prior: &Cursor, config: &Config)
    -> Result<SnapshotOutcome>;   // Unchanged or Ready(owned temp directory)
pub fn open_reader(profile: &Profile, snapshot: Snapshot) -> Result<Box<dyn HistoryReader>>;
```

`Cursor` JSON is stored under `checkpoints.source = SourceId`. Profile identity uses canonical absolute path encoded as lowercase hex of UTF-16LE code units on Windows, raw Unix path bytes on Linux; prefix with browser name and OS (`chrome:win:...`, `firefox:unix:...`). This avoids basename collisions and loss of non-UTF-8 paths. Canonical identity is stable until a profile is moved. Snapshot path never enters identity. Initial tuple `(i64::MIN, i64::MIN)` reads all retained history. Detect a replaced/reset DB when snapshot maximum tuple is below cursor, and rescan; dedupe keeps replay safe. Same-size/mtime replacement and late inserts behind a cursor are known S1 limitations; deleting that source's checkpoint explicitly via a maintenance procedure enables a full rescan. M0 does not modify browser databases to solve them.

| Browser | Windows roots | Linux development roots |
|---|---|---|
| Chrome | `%LOCALAPPDATA%\Google\Chrome\User Data` | `$XDG_CONFIG_HOME/google-chrome` or `~/.config/google-chrome` |
| Edge | `%LOCALAPPDATA%\Microsoft\Edge\User Data` | `$XDG_CONFIG_HOME/microsoft-edge` or `~/.config/microsoft-edge` |
| Brave | `%LOCALAPPDATA%\BraveSoftware\Brave-Browser\User Data` | `$XDG_CONFIG_HOME/BraveSoftware/Brave-Browser` or `~/.config/BraveSoftware/Brave-Browser` |
| Firefox | `%APPDATA%\Mozilla\Firefox\profiles.ini`; fallback `Profiles\*\places.sqlite` | `~/.mozilla/firefox/profiles.ini`; fallback `~/.mozilla/firefox/*/places.sqlite` |

Chromium: enumerate immediate child directories containing `History`, including `Default`, `Profile *` and custom names; exclude `System Profile`, `Guest Profile`, caches and symlinks. Do not rely on display names in `Local State`. Firefox: parse `[ProfileN]` INI sections with `Path` and `IsRelative`; resolve relative paths against directory of `profiles.ini`, absolute paths unchanged. Ignore `[Install...]`; union discovered `Profiles` children; canonical-path deduplicate. Missing roots are normal; permission/read failures on present roots are warnings and mark that source failed. Skip symlink fallback children; explicit profile paths may resolve a symlink once. Firefox INI parser supports literal values, CRLF/comments and rejects malformed profile entries individually.

Snapshot protocol (F6):

1. Stat main, `-wal`, `-journal`; represent absence explicitly. If all three mtime+size pairs equal the last **committed** fingerprint, skip. A WAL-only change must not skip. Ignore `-shm` (regenerate in private copy).
2. Create private `tmp/<pid>-<counter>/` with `create_dir`; Linux mode 0700, Windows current-user data-directory ACL inheritance. Copy main and present sidecars using their exact basename relationships. Never open the live DB through SQLite; never copy only the main file when a sidecar exists.
3. Stat the source triplet again. Changed membership, size or timestamp means discard entire copy and retry, up to `snapshot_retries`, sleeping 50 ms between attempts. A Windows sharing violation is retryable but may require closing the browser; report it rather than bypassing locks.
4. Open the **copy** read/write so SQLite can recover a hot rollback journal or WAL; rebuild SHM there. Force an initial schema read to complete recovery, then set `query_only=ON`, run `PRAGMA quick_check`, require single `ok`, validate necessary schema/columns and prepare reader statements. Never apply Omoi migrations or WAL settings to browser copies.
5. Only accept a successful source after all batches have been read and spooled. Save the pre/post-equal fingerprint with its final cursor only on Omoi COMMIT. Remove snapshot after reading; on failure remove its spool and leave checkpoint unchanged.

This is a best-effort snapshot, not an atomic online backup: same mtime/size and `quick_check` cannot prove transactional consistency during concurrent writes. Do not describe it as guaranteed lossless; if instability persists, skip and retry next hour or after browser exit. SQLite warns about copying live files without a coherent journal; retaining sidecars and rejecting unstable copies reduces, but cannot eliminate, that risk. [SQLite copying hazards](https://www.sqlite.org/howtocorrupt.html#backup_or_restore_while_a_transaction_is_active).

SQL, positional parameters `(last_native_us,last_visit_id,limit)`:

```sql
-- Chromium; replace visit_duration expression with NULL if PRAGMA table_info says absent.
SELECT v.id,v.visit_time,u.url,u.title,v.visit_duration
FROM visits AS v JOIN urls AS u ON u.id=v.url
WHERE (v.visit_time,v.id) > (?1,?2)
ORDER BY v.visit_time,v.id LIMIT ?3;
-- Firefox
SELECT v.id,v.visit_date,p.url,p.title
FROM moz_historyvisits AS v JOIN moz_places AS p ON p.id=v.place_id
WHERE (v.visit_date,v.id) > (?1,?2)
ORDER BY v.visit_date,v.id LIMIT ?3;
```

Snapshot holds stable data while paging; SQLite can use its time index. Cursor advances on every scanned row even if the URL does not normalize. Read oversized URLs/titles with SQL `CASE WHEN length(CAST(... AS BLOB)) <= bound THEN ... ELSE NULL END`; limit URLs to 8,192 bytes (reject row if exceeded), titles to 1,024 bytes (drop oversized title). Invalid UTF-8/invalid timestamps reject a row and count it, without failing the profile. NULL native timestamps are excluded/counted separately once; do not put them in a tuple cursor. A bad column/schema or query failure fails the whole profile.

Conversions, checked in i128 then narrowed to i64:

```text
Chrome: unix_us = chrome_us - 11_644_473_600_000_000
Firefox: unix_us = PRTime_us
unix_s = unix_us.div_euclid(1_000_000)      # floor, including negative epochs
Chrome approximate seconds = max(visit_duration_us, 0) / 1_000_000
Firefox approximate duration = None
local date = UTC(unix_s) + configured fixed offset, formatted YYYY-MM-DD
```

Reject timestamps outside chrono's representable range; do not cast overflowing values. Browser history live keys use visit time as the only available stream-start proxy; M3 uses actual stream/session start. Vectors: Chrome `11644473600000000 → 0s`; `11644473601000000 → 1s`; Firefox `1000001 → 1s`; Firefox `-1 → -1s`; duration `2500000 → 2s`, negative →0.

## 6. Normalization contract and vectors

```rust
pub struct ItemKey { pub platform: Platform, pub external_id: String }
pub struct Normalized { pub key: ItemKey, pub kind: Option<&'static str> }
pub fn normalize(raw: &str, unix_s: i64, offset: chrono::FixedOffset)
    -> Option<Normalized>;
```

Use `url::Url`; accept HTTP/HTTPS or explicitly supported `file:`. Reject userinfo, nondefault ports, encoded path separators/NUL, invalid percent escapes and lookalike hosts. Match lowercase parsed hosts against exact allowlists, not substring/suffix wildcards. `www.` is allowed only where listed below. Preserve case in vendor IDs unless stated. Ignore fragments and unrelated query parameters. Reject duplicate identity parameters (`v`, `p`, `gti` where used), including conflicting values. No fetches, redirects or generic title-page matching. Require exact segment counts except the optional Crunchyroll slug and documented Amazon optional slug.

| Platform | Accepted hosts/path | External ID and validation |
|---|---|---|
| YouTube | `youtube.com`, `www.youtube.com`, `m.youtube.com`, **`music.youtube.com`**: `/watch?v=ID`, `/shorts/ID`, `/live/ID`, `/embed/ID`; `youtu.be`, `www.youtu.be`: `/ID` | ID exactly 11 ASCII `[A-Za-z0-9_-]`. Music video watch URLs count as YouTube; music landing/playlist URLs do not. shorts→kind short, live→live; otherwise video. |
| Bilibili | `bilibili.com`, `www.bilibili.com`, `m.bilibili.com`: `/video/BV…` or `/video/avDIGITS` | BV exactly 12 chars total (`BV` + 10 alphanumeric), case preserved; av digits canonical decimal. Append `:pN`, default N=1, positive decimal ≤10000; `?p=01` equals `:p1`. Parts are distinct items. |
| Netflix | `netflix.com`, `www.netflix.com`: `/watch/DIGITS` | Positive decimal, strip leading zeroes; ignore trackId. `/title/…` is not a view. |
| Twitch | `twitch.tv`, `www.twitch.tv`, `m.twitch.tv`: `/videos/DIGITS`, `/<channel>` | VOD→`vod:DIGITS` canonical positive decimal. Live channel lowercased `[a-z0-9_]{1,25}`→`live:channel:YYYY-MM-DD`. Reject reserved routes `directory,downloads,settings,subscriptions,inventory,wallet,search,login,signup,prime,turbo,jobs,p,products,videos,clip,clips`. Clips unsupported M0. |
| Prime | `primevideo.com`, `www.primevideo.com`: `/detail/ID`, `/detail/SLUG/ID`, `/gp/video/detail/ID`; exact Amazon domains below, optional `www.`: `/gp/video/detail/ID`, `/gp/video/detail/SLUG/ID`, `/dp/ID`, `/gp/aw/d/ID` | ID ASCII alphanumeric, 10..32 chars, uppercase canonical. Last segment in these forms is ID; slug nonempty. Do not infer arbitrary Amazon product `/dp` is video: accept `/dp` and `/gp/aw/d` **only with `video=1`**. Bare product URLs→None. Query `gti` is not an alternate ID in M0. |
| Disney+ | `disneyplus.com`, `www.disneyplus.com`: `/video/ID`, `/play/ID`; optional leading locale `[a-z]{2}-[a-z]{2}` | ID UUID `8-4-4-4-12` hex, or ASCII alphanumeric/hyphen token 8..80 chars; lowercase UUID, preserve other tokens. `/series`, `/movies` excluded. |
| Crunchyroll | `crunchyroll.com`, `www.crunchyroll.com`: `/watch/ID[/slug]`, optionally leading two-letter locale or `xx-xx` | ID 9 uppercase ASCII alphanumeric; lowercase input canonical uppercase. Optional one nonempty slug. |
| Local | `file:///C:/…` or Unix absolute `file:///…`, authority empty or `localhost` only | `file:` + normalized absolute URL path; percent-decode UTF-8, collapse dot segments, slash separators, uppercase Windows drive letter only; remaining case preserved; re-encode using URL path serialization. No content hash in M0. Allow media extensions mp4,mkv,webm,avi,mov,m4v,mp3,flac,m4a,ogg,wav (case insensitive). Query/fragment ignored. UNC and relative paths rejected. |

Amazon domains are exactly `amazon.com`, `amazon.co.uk`, `amazon.de`, `amazon.co.jp`, `amazon.in`, `amazon.fr`, `amazon.it`, `amazon.es`, `amazon.ca`, `amazon.com.au`, `amazon.com.br`, `amazon.com.mx`. This is an explicit support set, not `amazon.*` suffix matching. `/dp` restriction avoids collecting shopping activity as watches. Other regional domains need a tested allowlist extension. Normalize optional trailing slash away once. Normalized IDs ≤512 UTF-8 bytes; larger local paths return None. Local paths reveal filenames in exports, and are path identity, not moved-file/content identity; M2 reuses this rule.

Test fixture clock unless specified: `2026-10-24T16:30:00Z`, offset `+08:00`, so local date `2026-10-25`. Every row is an independent assertion; `—` means None.

| Input URL | Expected `(platform, external_id)` |
|---|---|
| `https://youtube.com/watch?v=dQw4w9WgXcQ` | youtube, dQw4w9WgXcQ |
| `https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=93s&list=PLx#t=1` | youtube, dQw4w9WgXcQ |
| `https://youtu.be/dQw4w9WgXcQ?t=12` | youtube, dQw4w9WgXcQ |
| `https://m.youtube.com/shorts/dQw4w9WgXcQ` | youtube, dQw4w9WgXcQ |
| `https://youtube.com/live/dQw4w9WgXcQ?feature=share` | youtube, dQw4w9WgXcQ |
| `https://youtube.com/embed/dQw4w9WgXcQ?start=30` | youtube, dQw4w9WgXcQ |
| `https://music.youtube.com/watch?v=dQw4w9WgXcQ` | youtube, dQw4w9WgXcQ |
| `https://music.youtube.com/playlist?list=PLx` | — |
| `https://youtube.com/watch?v=short` | — |
| `https://youtube.com/watch?v=dQw4w9WgXcQ&v=aaaaaaaaaaa` | — |
| `https://youtube.com.evil.test/watch?v=dQw4w9WgXcQ` | — |
| `https://youtube.com@evil.test/watch?v=dQw4w9WgXcQ` | — |
| `https://bilibili.com/video/BV1xx411c7mD` | bilibili, BV1xx411c7mD:p1 |
| `https://www.bilibili.com/video/BV1xx411c7mD/?p=2&spm_id_from=333` | bilibili, BV1xx411c7mD:p2 |
| `https://m.bilibili.com/video/BV1xx411c7mD?p=01` | bilibili, BV1xx411c7mD:p1 |
| `https://bilibili.com/video/av00123?p=3` | bilibili, av123:p3 |
| `https://bilibili.com/video/BV1xx411c7mD?p=0` | — |
| `https://bilibili.com/video/BV1xx411c7mD?p=2&p=3` | — |
| `https://netflix.com/watch/80100172?trackId=123` | netflix, 80100172 |
| `https://www.netflix.com/watch/00042` | netflix, 42 |
| `https://netflix.com/title/80100172` | — |
| `https://netflix.com/watch/abc` | — |
| `https://twitch.tv/videos/0012345?t=1h` | twitch, vod:12345 |
| `https://www.twitch.tv/Maria_Channel` | twitch, live:maria_channel:2026-10-25 |
| `https://twitch.tv/directory` | — |
| `https://clips.twitch.tv/SomeClip` | — |
| `https://twitch.tv/maria/videos` | — |
| `https://primevideo.com/detail/0ABCDEF123456789` | prime, 0ABCDEF123456789 |
| `https://www.primevideo.com/detail/a-show/0abcdef123456789` | prime, 0ABCDEF123456789 |
| `https://www.amazon.com/gp/video/detail/B012345678?ref_=atv` | prime, B012345678 |
| `https://amazon.co.jp/gp/video/detail/show/B012345678` | prime, B012345678 |
| `https://amazon.co.uk/dp/B012345678?video=1` | prime, B012345678 |
| `https://amazon.de/gp/aw/d/B012345678?video=1` | prime, B012345678 |
| `https://amazon.com/dp/B012345678` | — |
| `https://amazon.com.evil.test/gp/video/detail/B012345678` | — |
| `https://disneyplus.com/video/12345678-1234-ABCD-ABCD-123456789ABC` | disney, 12345678-1234-abcd-abcd-123456789abc |
| `https://www.disneyplus.com/play/abcd1234?foo=1` | disney, abcd1234 |
| `https://disneyplus.com/en-gb/play/abcd1234` | disney, abcd1234 |
| `https://disneyplus.com/series/abcd1234` | — |
| `https://crunchyroll.com/watch/G6Q4MGP1R/a-title` | crunchyroll, G6Q4MGP1R |
| `https://www.crunchyroll.com/en-gb/watch/g6q4mgp1r/title` | crunchyroll, G6Q4MGP1R |
| `https://crunchyroll.com/watch/G6Q4MGP1R` | crunchyroll, G6Q4MGP1R |
| `https://crunchyroll.com/series/G6Q4MGP1R` | — |
| `file:///c:/Videos/A%20Film.mkv` | local, file:/C:/Videos/A%20Film.mkv |
| `file:///home/maria/a/../film.mp4#t=3` | local, file:/home/maria/film.mp4 |
| `file://localhost/home/maria/music.flac` | local, file:/home/maria/music.flac |
| `file://server/share/film.mp4` | — |
| `file:///home/maria/notes.txt` | — |
| `https://youtube.com:444/watch?v=dQw4w9WgXcQ` | — |
| `https://youtube.com/embed/dQw4w9WgXcQ%2Fextra` | — |
| `not a url` | — |

Boundary vectors: Twitch `maria` at `2026-10-24T15:59:59Z`→`live:maria:2026-10-24`; at `16:00:00Z`→`live:maria:2026-10-25`. For a single M3 playback spanning midnight, retain its initial key until stream/session closes.

## 7. Dedupe and collect transaction

Deduplication is an ingestion policy: one item opening within an **inclusive 1,800 seconds** of an existing view's fixed anchor joins that view. It applies within one source as well as across sources (reloads). It is not transitive clustering. Keep the anchor's timestamp/id unchanged when merging: otherwise chains of reloads could suppress a whole day's views. Results are repeatable for the same ingestion order; late historical imports can encounter different existing anchors. Do not promise a unique global session reconstruction from browser visits.

Richness rank (higher wins): `userscript=60 > smtc=50 > netflix_csv=40 > local_player=35 > takeout=30 > chrome=edge=brave=20 > firefox=10`; future S4 rank must be specified if enabled. On equal rank, source name lexicographic order wins (`brave`, `chrome`, `edge`, `firefox`); same-source ties retain existing metadata. `views.source` stores the short source type, not the profile identifier; checkpoints preserve profile identity. Dedupe never sums durations: retain the maximum available Chromium approximation, separately from future measured sessions.

Metadata policy: trim title; treat empty or exact case-insensitive platform names (`YouTube`, `Netflix`, `Twitch`, `Prime Video`, `Disney+`, `Crunchyroll`, `Bilibili`) as missing. In M0 only title/kind are supplied. Fill missing title at any rank; replace nonempty title only at a higher `metadata_rank`; set rank when a title is accepted. Equal rank retains existing title. Generic titles never overwrite meaningful ones. Preserve all channel/enrichment fields; the M3 extension needs per-field provenance before it can replace independently sourced fields. URL-kind evidence `short`/`live` wins over generic `video`.

```rust
pub struct Candidate {
    pub key: ItemKey, pub started_at: i64, pub source: String,
    pub title: Option<String>, pub kind: Option<String>,
    pub approx_duration_s: Option<i64>,
}
pub fn ingest(tx: &rusqlite::Transaction<'_>, row: &Candidate) -> Result<IngestOutcome>;
pub fn collect(config: &Config, paths: &Paths, request: &CollectRequest)
    -> Result<CollectReport>;
```

Pipeline pseudocode; error boundary is deliberate:

```text
discover profiles; load committed cursors from read connection
for each profile, sequentially:
    if fingerprint unchanged: record skip; continue
    snapshot + reader; read bounded batches
    normalize and write accepted Candidate JSON lines into private spool
    track last scanned tuple, rejected counts, and candidate count
    on any source read error: delete spool, mark source failed, keep old cursor
    on success: retain spool + proposed final cursor/fingerprint
# spool/snapshot disk-full or inability to create private storage is fatal, not a source error

BEGIN IMMEDIATE on Omoi writable connection              # F7, only one ingest transaction
re-read checkpoint for every proposed source
if any differs from cursor captured before reading:
    ROLLBACK, discard prepared spools, restart once from fresh checkpoints
    if conflict repeats: return Busy (4)
# prevents a second collector regressing a checkpoint after waiting for this lock
for source spool in lexical SourceId order:
    for candidate in native time/visit_id order:
        item = lookup UNIQUE(platform, external_id), insert if missing
        update eligible metadata using rank policy
        matches = SELECT id,started_at,source FROM views
                  WHERE item_id=? AND started_at BETWEEN t-1800 AND t+1800
                  ORDER BY abs(started_at-t),started_at,id LIMIT 1
        if no match:
            INSERT view(item,t,source,'pc',approx)
        else:
            keep anchor started_at and id
            replace source only if candidate wins rank/tie rule
            approx = max(existing approximate value, candidate approximate value)
    UPSERT source checkpoint to final cursor JSON, including fingerprint
COMMIT                                                   # rows and checkpoints together
cleanup private spools; passive WAL checkpoint; emit summary
```

Use checked/saturating bounds for `t±1800`; timestamp validity already restricts range. Transaction/query/commit failures roll back **all** prepared sources and their checkpoints. Because all source reading occurs first, a corrupt profile cannot leave half its rows inserted. Per-source spools keep RAM bounded and avoid holding a write lock while copying browsers. Only one snapshot and one batch at a time; spool disk is proportional to accepted rows, removed on success/failure. `std::io::BufReader` decodes one bounded line at a time. Cleanup only directories owned by this run; stale crash directories may be reported for manual removal, never delete another live run's files.

Do not save cursors for failed sources or before COMMIT. On an empty but successful scan, save its fingerprint and unchanged tuple. M0 writes no raw_events or sessions. Export readers may run concurrently under WAL; two collectors serialize or exit 4. Database size budget counts the added provenance/index columns, rather than assuming the draft's estimate still holds.

Dedupe vectors (same item unless stated; seconds relative to an arbitrary valid UTC base; execute left-to-right):

| Input | Assertion |
|---|---|
| chrome@0, chrome@0 | 1 view; no unique error |
| firefox@0 title=Good, chrome@60 title=Better | 1 view anchored 0, source chrome, title Better |
| chrome@0 title=Good, firefox@60 title=Other | 1 view; title Good/source chrome |
| chrome@0, edge@1800 | 1 view (inclusive boundary) |
| chrome@0, edge@1801 | 2 views |
| chrome@0, chrome@1200, chrome@2400 | 2 views anchored 0 and 2400 (no chain extension) |
| chrome@0, chrome@3600, firefox@1800 | 2 views; equidistant candidate joins anchor 0 |
| key A@0, key B@1 | 2 items and 2 views |
| firefox/profile1@0, firefox/profile2@20 | 1 view; separate checkpoints |
| chrome@0 duration=10, chrome@10 duration=20 | 1 view, approx=20, not 30 |
| chrome@0 title=Good, chrome@2 title=YouTube | title Good |
| process same week twice | second run inserts 0 items/views; unchanged profiles not copied |
| fail injected SQL after first candidate | 0 committed candidates and unchanged checkpoints |
| two connections collect same event | one committed view; second retries/stops on checkpoint conflict |

## 8. Sessionization (M3 contract; M0 stub)

M0 `sessionize::available() -> bool` returns false and `sessionize::run() -> Result<()>` returns `Error::NotImplemented("M3 sessionize")`; collect never calls it in M0. Schema existence does not imply playback measurement. **M3 caller (decided 2026-10-28):** `omoi collect` runs `sessionize::run` inside its single `BEGIN IMMEDIATE` transaction, consuming the `raw_events` the listener appended and deleting each processed row in the transaction that commits its sessions; the listener only appends and never writes `sessions`. Revisit at M3.

M3 types: `PlaybackEvent { event_id, producer_id, seq, item_key, source, at_ms, state, position_ms, cumulative_played_ms }`. Use event IDs plus producer/sequence for durable retry idempotence in a new migration. Userscript accumulates **actual playing elapsed time** with a monotonic clock, including playback rate but excluding pause/buffering/seek jumps; it sends on player events and throttled `timeupdate` callbacks, not listener polling. SMTC uses playback-state events plus timestamped timeline samples. Confirmed adjacent sample intervals only; absent telemetry is not assumed playback.

```text
partition by (item, source, producer); order by (at_ms,seq)
discard exact replay IDs; reject invalid times/counters; log out-of-order late events
if adjacent trusted event gap > 300_000 ms:
    close previous session at last confirmed event time; open new session
else:
    keep same session, including pause/resume within the gap
for adjacent events in a session:
    userscript: played += clamp(delta cumulative_played_ms, 0, delta wall_ms)
    SMTC: if previous state Playing and gap <= 300_000:
              played += delta wall_ms (approximation, tier smtc)
    max_position = max(all valid sampled positions)  # seeking does not add played time
on stop: close at stop time
on item change: close old at last confirmed sample; open new item session
persist session.start=floor(first_ms/1000), end=ceil(last_confirmed_ms/1000)
persist played_s=floor(sum_confirmed_ms/1000), max_position_s=floor(max_ms/1000)
upsert stable session identity and processed-event markers atomically; delete inbox rows
```

Paused time can lengthen a session envelope but does not increase played time. Exactly 5 minutes stays in one session; more closes it. A playing interval lacking samples for >5 minutes is conservatively unmeasured; do not count the silence. Open sessions require durable M3 state so restarts do not double count or invent time. SMTC missing stop is closed at last confirmed sample. M3 may add an event-driven expiry timer; no periodic polling loop. Midnight does not split an existing Twitch session/key.

Store each source's sessions with provenance. UI resolves overlapping measured intervals by userscript > SMTC > genuine CSV duration; never add overlapping tiers. Chromium approximation remains separately labelled and does not become a `sessions` row. Daily total uses union of selected intervals, not a sum of all sources. Match SMTC `(normalized title, ±600s)` only when there is one unambiguous S1/S3 item; otherwise retain `unknown` with reversible `source_app + title` key until resolved.

Session vectors (`P` playing, `A` paused, `S` stopped; user counters in seconds):

| Events | Expected |
|---|---|
| P@0/c0, P@30/c30, A@60/c60, P@120/c60, S@180/c120 | one [0,180], played=120 |
| P@0/c0, A@10/c10, P@310/c10, S@320/c20 | one [0,320], played=20; gap exactly 300 |
| P@0/c0, A@10/c10, P@311/c10, S@321/c20 | [0,10] played10 and [311,321] played10 |
| P@0/position0/c0, P@10/position600/c10, S@20/position610/c20 | played20, max_position610; seek adds no 590s |
| replay same producer/seq/events | unchanged session count and played total |
| P@0/c0, P@600/c600, S@610/c610 | first session played0; second played10; missing 600s not credited |
| source userscript [0,60] played60 + SMTC [0,60] played60 | two provenance rows; UI total60 |

## 9. Error handling and bounded logs

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("configuration: {0}")] Config(String),
    #[error("I/O during {context}: {source}")]
    Io { context: &'static str, #[source] source: std::io::Error },
    #[error("database: {0}")] Db(#[from] rusqlite::Error),
    #[error("database version {found} is newer than supported {supported}")]
    SchemaTooNew { found: u32, supported: u32 },
    #[error("unsupported database schema: {0}")] Schema(String),
    #[error("writer busy")] Busy,
    #[error("profile {source_id}: {reason}")]
    Source { source_id: String, reason: String },
    #[error("export: {0}")] Export(String),
    #[error("unsupported: {0}")] Unsupported(&'static str),
    #[error("not implemented: {0}")] NotImplemented(&'static str),
}
```

Source errors are captured into `CollectReport`, not returned to abort other sources. Fatal: invalid config, unsupported schema/cursor version, inaccessible Omoi storage, disk full, damaged Omoi DB, migration/transaction failure, inability to spool safely. Skippable: locked/missing/disappearing browser profile, corrupt/incompatible browser copy, invalid individual Firefox profile entry. Malformed row/URL/title is a counted row rejection, not a source failure. SQLite BUSY/LOCKED on Omoi maps to Busy/4; browser locks map to source failure/3. Logging failure warns to stderr and does not discard successfully collected history.

No `unwrap`, `expect`, unchecked indexing or `panic!` on collect's production path. Use checked arithmetic and `Result`; RAII transactions roll back on drop. Panics are permitted only in test assertions. Allocator/process termination is not recoverable and must not be represented as a successful run.

Log UTF-8 JSONL to `%LOCALAPPDATA%\omoi\logs` (or resolved home), record UTC time, severity, operation, short browser type, opaque profile number, error category and counts. No full URLs, titles, local file paths, tokens or browser row contents. Truncate each record at 4 KiB at a UTF-8 boundary while preserving valid JSON. To avoid competing rotation, use per-process files `collect-<pid>.log`; a short `create_new` logging-lock file serializes rotation and opening/appending. Failure to acquire it immediately falls back to stderr. Enforce directory-wide `log_files` and `log_file_bytes`: before each append remove oldest **closed** log files, rotate active under lock, and decline file logging if caps cannot be maintained. Keep handles open only during append; enforce ≤3 MiB total at defaults. Never auto-remove a possibly live lock; stale lock means stderr until manual removal. Dry-run logs only stderr. Scheduled `omoiw` uses the same log sink because it has no console.

## 10. Windows Task Scheduler (F2)

Choose separate `omoiw.exe`: `#![cfg_attr(windows, windows_subsystem = "windows")]`; its Windows main calls the same library CLI with only `collect` arguments allowed. Console `omoi.exe` retains normal stdout/stderr, pipes and shell exit status without attach-console races. On Linux, `omoiw` returns Unsupported/5; pure XML/argv builders remain testable. Install both adjacent in a stable directory; schedule installation fails if helper is missing. Helper reads config/home normally; scheduler captures resolved `OMOI_HOME` using an internal `--home ABSOLUTE_PATH` helper-only flag so environment overrides survive logon. This flag is not part of `omoi`'s public CLI.

Generate UTF-8 XML (below) in private temp storage; XML-escape all interpolated values. `{SID}` is current token's user SID obtained via Windows token APIs, not an administrator account. `{START}` is next whole local hour in ISO-8601 with numeric offset. `{BIN}`/`{DIR}` are absolute helper path/directory, `{ARGS}` is `--home "<resolved home>" collect` escaped with Windows argument quoting and then XML escaping. Do not execute via a shell. Reject control characters illegal in XML. XML `Command` contains no surrounding quotes. [Microsoft Task Scheduler schema](https://learn.microsoft.com/en-us/windows/win32/taskschd/task-scheduler-schema).

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo><Description>Omoi local browser history collection</Description></RegistrationInfo>
  <Triggers>
    <TimeTrigger>
      <Repetition><Interval>PT1H</Interval><StopAtDurationEnd>false</StopAtDurationEnd></Repetition>
      <StartBoundary>{START}</StartBoundary><Enabled>true</Enabled>
    </TimeTrigger>
    <LogonTrigger><Enabled>true</Enabled><UserId>{SID}</UserId><Delay>PT30S</Delay></LogonTrigger>
  </Triggers>
  <Principals><Principal id="Maria">
    <UserId>{SID}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel>
  </Principal></Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>true</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>true</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <AllowStartOnDemand>true</AllowStartOnDemand><Enabled>true</Enabled><Hidden>true</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle><WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT10M</ExecutionTimeLimit><Priority>7</Priority>
  </Settings>
  <Actions Context="Maria"><Exec>
    <Command>{BIN}</Command><Arguments>{ARGS}</Arguments><WorkingDirectory>{DIR}</WorkingDirectory>
  </Exec></Actions>
</Task>
```

`InteractiveToken` means run only when Maria is logged on; no stored password or elevation. Priority 7 is below-normal. Battery policy skips on battery and stops on transition; SQLite recovers interrupted transactions next run. Hidden is Task Scheduler visibility; the GUI subsystem helper prevents console flashes. Ten-minute cap is a backstop, not a performance allowance.

Exact process argument arrays (call `%SystemRoot%\System32\schtasks.exe`):

```text
install: ["/Create", "/TN", "\\Omoi\\Collect", "/XML", xml_path, "/F"]
remove:  ["/Delete", "/TN", "\\Omoi\\Collect", "/F"]
verify:  ["/Query", "/TN", "\\Omoi\\Collect", "/XML"]
```

Rendered example: `schtasks.exe /Create /TN "\Omoi\Collect" /XML "C:\Users\Maria\AppData\Local\omoi\tmp\task.xml" /F`. Installer ensures folder `\Omoi` exists through Task Scheduler COM before invoking schtasks. For U12, add `windows` features `Win32_System_Com`, `Win32_System_TaskScheduler`, `Win32_System_Ole`; use `ITaskService`, root `GetFolder`, `CreateFolder("Omoi")`, accepting already-exists only. Never alter another folder/task. Check child exit status; do not infer success from stdout. Remove treats task absence as success only after an explicit absence check (COM HRESULT), not every schtasks failure. Query installed XML for both triggers/settings. Installation is idempotent. Windows smoke test: install, query, manually run, inspect log/exit status/no flash, remove; Linux tests validate XML escaping/argv and unsupported dispatch.

## 11. M1.5 thumbnail measurement

Future command: `omoi thumbs --measure --limit 500`; require positive limit ≤500 in this milestone. No automatic scheduling or eviction. Default test data under `thumbs/measure-<UTC>-<counter>/`; report atomically to `thumbs-measure.json`. Retain downloaded files after reporting until Maria inspects them. A rerun must include prior test files in the total test cap or fail with instructions to remove them explicitly; do not silently delete a prior measurement.

Hard limits: **50,000,000 bytes** aggregate test files including in-flight partial bytes; per-object 1,000,000 bytes; sequential downloads; 5s connect/15s total deadlines; ≤2 retries per object. Default future cache cap **100,000,000 bytes** (Maria Q7). MB throughout this spec is decimal; report raw bytes to avoid ambiguity. Reserve room for the next chunk before writing; absent/lying Content-Length cannot bypass cap. Delete only failed/incomplete files from the current operation, never completed samples. Existing production cache is outside test budget, report its size separately.

Algorithm: query latest view per distinct item; take newest ≤500 items globally (not 500 per platform). Use smallest known usable thumbnail; initially only YouTube `https://i.ytimg.com/vi/{id}/mqdefault.jpg` (320×180). Other platforms require documented provider URLs/auth before support; record them as unsupported, never scrape pages or count missing bytes as zero. Fetch only allowlisted HTTPS hosts, reject private-address redirects, cap redirects at 2 and validate each target. Validate HTTP success/content type plus JPEG/PNG/WebP signature; count bytes on disk, not header estimates. Store `<item_id>.<ext>` with no query strings/remote names. These network features start in M1.5 using WinHTTP via target-only `windows`; Linux tests use an injected in-memory transport. If portable runtime HTTP is wanted later, explicitly revise the dependency budget first.

For platform p: `mean_p = successful_bytes_p / successful_samples_p`; `projected_p = ceil(mean_p * eligible_catalog_items_p)`; total projection is sum across platforms. No successes → projection null, not zero. Failed requests/unsupported platforms make whole-catalogue projection incomplete. Newest-500 sampling is biased: show sample coverage and range, label estimate, do not claim a statistical confidence interval. Pass gate only if complete full-catalogue projection ≤100 MB; otherwise keep feature disabled, or separately propose an explicitly bounded LRU cache to Maria. Future cache eviction is by last view, independent of retained test files.

Example output (one-platform catalogue, all arithmetic consistent):

```json
{
  "schema_version": 1,
  "started_at": "2026-10-24T08:00:00Z",
  "requested_limit": 500,
  "attempted": 500,
  "succeeded": 500,
  "failed": 0,
  "unsupported": 0,
  "test_cap_bytes": 50000000,
  "cache_cap_bytes": 100000000,
  "test_bytes_before": 0,
  "downloaded_bytes": 7500000,
  "test_bytes_after": 7500000,
  "stopped_reason": "limit",
  "platforms": [{
    "platform": "youtube", "catalog_items": 40000, "eligible_items": 40000,
    "attempted": 500, "succeeded": 500, "failed": 0,
    "bytes": 7500000, "min_bytes": 10000, "max_bytes": 20000,
    "mean_bytes": 15000, "projected_bytes": 600000000
  }],
  "projection_complete": true,
  "projected_total_bytes": 600000000,
  "within_cache_cap": false,
  "feature_enabled": false,
  "files_retained": true
}
```

Allowed stop reasons: `limit`, `catalog_exhausted`, `byte_cap`, `error`. Test with fake unknown-length chunks at 49,999,999 bytes: next 2-byte chunk is refused; directory never exceeds cap. Report bytes for successes separately from any retained partials (normally zero). Run this experiment before enabling offline thumbnails.

## 12. Resource measurement and milestone exits

Synthetic generator is an **example binary**, not an extra public CLI subcommand. Deterministic, streaming, fixed seed 1, generated fixtures only. Command:

```sh
cargo run --release --example synthetic -- --out /tmp/omoi-fixture --views 100000 --items 40000 --seed 1
OMOI_HOME=/tmp/omoi-fixture/home target/release/omoi collect
OMOI_HOME=/tmp/omoi-fixture/home target/release/omoi collect
/usr/bin/time -v env OMOI_HOME=/tmp/omoi-fixture/home target/release/omoi collect
```

Generator creates two fake Chromium/Firefox profiles, config with explicit prefixed profile paths, and empty home. Distribution: 40k items cycling through platforms (local included), 100k distinct view anchors spaced >1,800s per item, titles 40 ASCII bytes, mean URLs ~120 bytes, no sessions. Add 10% cross-source duplicate rows within 60s and 10% irrelevant URLs; report exact source row totals. Use deterministic valid IDs, fixed +08 offset and schedule dates so Twitch IDs remain stable within a synthetic item. Also generate `--long-titles` (1,024 bytes) as a separate stress run; baseline budget does not promise every worst-case text distribution fits 30 MB.

Windows harness `powershell.exe -NoProfile -File scripts\measure.ps1 -Exe .\target\x86_64-pc-windows-gnu\release\omoi.exe -Fixture D:\tmp\omoi-fixture -Runs 20` sets OMOI_HOME only for child processes. Start child and retain its process handle; use peak working-set OS counter (`GetProcessMemoryInfo.PeakWorkingSetSize`) and `GetProcessTimes` user+kernel time, not a 60s sampling loop that misses a subsecond collector. A 10–20ms sample of working/private bytes may supplement, not replace, lifetime peak. Wrapper measures wall time, captures collect JSON, and queries process I/O counters. Linux `/usr/bin/time -v` supplies maximum RSS and user/system CPU. Report units/platform/toolchain/SQLite version, CPU model and storage type.

Measure separately: first full import; unchanged rerun; changed snapshot with only 10 new rows; metadata-only/WAL-only change; all profiles. Reset fixture to a fresh empty home for each full-import repetition. Warm filesystem cache before warm-run suite; report cold first run separately. Report median/p95/max over 20 runs and **copy wall time independently**. `processing_ms = total_ms - copy_ms`; CPU time includes copying and is not interchangeable with wall time. Do not subtract CPU to hide expensive copies. No claimed pass without saved measurements.

| Metric | Gate/report |
|---|---|
| Collector resident memory | zero between runs; process exits |
| Collector lifetime peak working set / Linux RSS | <30,000,000 bytes, including full import |
| Warm hourly changed/unchanged collect | processing wall <1s and CPU <1s per run; copy wall reported separately (F6) |
| 100k-view import | report wall/CPU/peak; first backfill is not falsely called an hourly 1s gate |
| DB disk | ≤30,000,000 bytes/100k views including items, views, indexes, FTS and checkpoints; baseline ~40k items |
| UI M1 | <200ms page load at 10k items, local warm measurements |
| Listener M3 | ≤10,000,000-byte working set; idle CPU ≈0%, report core-seconds over one hour playback |
| Primary release executable | ≤6,000,000 bytes; helper reported separately |

Disk procedure: stop writers, `wal_checkpoint(TRUNCATE)` from test harness, close connections, sum DB + any WAL/SHM; report before/after checkpoint, page_count×page_size and freelist bytes. Do not VACUUM before acceptance to conceal routine storage growth. Also report temporary snapshot/spool peak disk, logs and binaries separately from catalogue budget. Baseline has empty sessions/raw_events; M3 must remeasure with sessions and full inbox, rather than extrapolating M0 pass. If a gate fails, retain the measurement and optimize before marking milestone done; estimates are not waivers.

M0 final acceptance: Maria's retained week imports; rerun inserts zero; every source's rejected/skipped counts visible; all Linux acceptance tests pass; GNU Windows build/scheduler smoke test pass; resource report recorded. This spec does not claim these have been executed.

## 13. M1–M3 implementation outline

| Milestone | Concrete scope and boundaries |
|---|---|
| M1 | `omoi ui`: on-demand loopback server, single static page, timeline, FTS search, platform/channel filters and paginated SQL (50 rows/page). Ten-minute idle exit, no polling resident service. Use standard library bounded HTTP parsing/IO initially or revisit dependency budget explicitly. Per-launch OS-random token in initial URL, immediately remove from address/history with replaceState; keep only in page memory and require it on each mutation, exact loopback Host and same-origin checks. Read-only otherwise. Send restrictive CSP/referrer policy; no remote assets. Escape titles, parameterize FTS inputs; provide literal search mode. Delete transactions keep FTS synchronized; tag writes arrive with the tagging UI in M4 (decided 2026-10-28). Token is separate from listener token. |
| M1.5 | Perform §11 measurement; retain report/files for Maria. Offline thumbnails remain disabled until measured gate passes. No enrichment implied by thumbnail experiment. |
| M2 S6 | `omoi import netflix --csv PATH` plus configured hourly drop-folder scan. Header-driven CSV adapter, exact format/version fixtures. Standard viewing-activity exports may have **only Title and Date**: do not assume duration or stable ID from the draft's S6 shorthand. Parse duration/device only when actually present and specified units are known. Match title/date only when unique; otherwise pending imported identity (`netflix`, `csv:<escaped-title>:<date>`) with low-confidence date precision in a new migration. Never fabricate a Netflix watch ID or midnight exact watch time; reconcile provisional entries only with unique evidence. File progress/idempotence by recorded file stamp plus stable row key; explicit review on ambiguous matches. |
| M2 S8 | mpv watch_later and VLC recent files are evidence of open/resume, not proof of completion; handle mpv hashed filenames only when associated path metadata is available. Reuse local file normalization, record source/provenance and available progress. Read via snapshots and bounded scans; no watcher. Jellyfin/Plex are opt-in server adapters with credentials in Credential Manager and network permissions explicitly configured, not assumed local-only traffic. New migrations store server item IDs and mapping confidence. |
| M2 optional S5 | Takeout backfill only if Maria requests it; isolated streaming parser, idempotent stable event IDs, no dependency on S4, no cookies. ZIP support requires a separately scoped dependency/platform decision. |
| M3 | `omoi listen` optional, event-driven; WinRT SMTC subscriptions and userscript endpoint; §8 sessions/provenance and interval-aware total. Userscripts in order YouTube, Netflix, Twitch, Bilibili, then Prime, Disney+, Crunchyroll. Listen task opt-in at logon, separate from M0 hourly task. No implicit resident launch from collect. |

M3 listener security acceptance (F1 and review §4):

- Bind **127.0.0.1 only**, fixed configured port 43821; fail loudly if occupied, never fall back to another port/interface. Exact `Host: 127.0.0.1:<port>`; reject duplicate Host or ambiguous framing.
- Only `POST /event` with JSON; `OPTIONS` returns 403. No `Access-Control-Allow-*` headers. Reject all page `http:`/`https:` Origins and `null`; accept absent Origin or an explicitly configured extension origin (`chrome-extension://<manager-id>`/`moz-extension://<manager-id>`), never a platform-site allowlist. Origin is defense-in-depth, not authentication.
- Require `X-Omoi-Token`: 32 OS-random bytes encoded as 64 hex chars. Constant-time fixed-length comparison; reject wrong length before compare. Windows randomness through BCryptGenRandom, token file with protected user-only ACL; Linux dev through `/dev/urandom`, mode 0600. Add target-only windows features in M3. Never log the token. Rotation requires updating installed userscripts.
- Userscripts send via `GM_xmlhttpRequest` from extension context, token only in GM storage, never DOM/page JS/query string. Buffer bounded events in GM storage and retry with backoff; stable event IDs make acknowledgments/retries safe. Do not claim all managers send no Origin; test actual manager/browser behavior before release.
- Body ≤16,384 bytes, headers ≤8,192 bytes, 2s read timeout, reject chunked encoding in initial implementation, reject conflicting/duplicate Content-Length. Bounded connection queue; no unbounded per-client threads. Validate platform IDs, timestamps, counters and event schema before storage. Inbox cap/drop reporting per §4. Reply success only after durable acceptance transaction.
- UI mutation token is separate and per-launch. Listener token grants event append only, never query/export/delete. Test hostile Host/Origin, missing token, oversized/slow request, full inbox, repeated event ID and port collision on Linux through pure request validators and injected transports.

## 14. Boilerplate work units (M0)

U1–U14 are dependency-ordered, small reviewable slices. Keep each production `.rs` file below roughly 250 nonblank lines; tests/data are separate. If a unit exceeds this, split private helpers within that module before review, preserving public API. Tests below are mandatory named `#[test]` cases, runnable via `cargo test <name>` on Linux. No test depends on Maria's profiles, network, a Windows registry or installed browsers. Windows-only calls live behind `cfg(windows)` with an explicit Linux stub.

| Unit | Files touched; dependencies | Public API | Named acceptance tests and concrete assertions |
|---|---|---|---|
| **U1 — crate/types/errors** | `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/main.rs`, `src/error.rs`, `src/readers/mod.rs`, `tests/types.rs`; none. Empty declared modules may be created here. | `Error`, `Result`, Browser/SourceId/Cursor/RawVisit/Batch, HistoryReader; main placeholder exits NotImplemented until U13. | `cursor_json_roundtrip`: serialize/deserialize tuple `(100,7)` and all three stamps unchanged. `error_exit_mapping`: Config→2, Busy→4, Unsupported→5, Db→1. `source_id_distinguishes_profiles`: two same-basename absolute profile paths yield different IDs. |
| **U2 — paths/config** | `src/paths.rs`, `src/config.rs`, `tests/config.rs`; U1. | `Paths::resolve(&PathEnvironment)`, `Config::load(path)`, `Config::parse(&str)`. Inject env map, no global-env mutation in tests. | `home_override_precedes_os`: absolute override wins both simulated OS roots; `linux_xdg_fallback`: missing XDG uses HOME/.local/share/omoi; `config_defaults_and_errors`: missing file defaults batch512/+08; reject unknown/duplicate key, table, relative home, timeout0; `config_comments_in_strings`: `#` inside quoted path survives. |
| **U3 — schema/migrations** | `src/db/mod.rs`, `schema.rs`, `migrations.rs`, `0001.sql`, `tests/db.rs`; U1–U2. | `db::open`, `migrate(&mut Connection)`, `info(&Connection)`. | `migration_0001_idempotent`: migrate twice, version1 and seven ordinary tables plus FTS; `migration_rollback`: injected invalid migration leaves version0/no items; `reject_future_schema`: user_version2 fails; `fts_insert_update_delete`: alpha searchable, update beta removes alpha, delete removes beta; `foreign_key_cascade`: deleting item removes view/session/tags; `required_columns`: channel_id and sessions.source exist. |
| **U4 — profile discovery** | `src/readers/discover.rs`, `tests/discover.rs`; U1–U2. | `discover(config,env)->Result<Discovery>`, Profile/Discovery types. | `chromium_all_profiles`: Default/Profile 2/custom included, System/Guest excluded; `firefox_ini_paths`: relative and absolute fixtures resolved/deduplicated; `discovery_missing_and_denied`: missing root harmless, injected permission error counted; `explicit_profile_filter`: firefox override excluded by chrome-only filter; `profile_excludes_applied`: `chrome:profile 2` skips `Profile 2` case-insensitively, an unmatched entry only warns. |
| **U5 — snapshot manager** | `src/readers/snapshot.rs`, `tests/snapshot.rs`; U1–U2,U4. | `snapshot`, Fingerprint/Snapshot/SnapshotOutcome; injected filesystem hooks only for deterministic race tests. | `skip_unchanged_triplet`: identical main/wal/journal stamps returns Unchanged; `wal_only_change_copies`: WAL length/mtime change returns Ready; `snapshot_preserves_sidecars`: private names match main and recovery sees committed WAL row; `snapshot_retry_bound`: mutate source on every copy, exactly3 attempts then Source error, cursor unchanged; `snapshot_cleanup`: owned temp dir removed on error. |
| **U6 — Chromium reader** | `src/readers/chromium.rs`, `tests/chromium.rs`; U1,U5. | `ChromiumReader::open`, HistoryReader implementation, checked epoch helper. | `chrome_epoch`: epoch and epoch+1e6 map to0/1 seconds; `chrome_tuple_pagination`: 3 rows same time and limit2 yield all IDs once; `chrome_duration_optional`: absent column→None, 2.5e6→2; `chrome_reject_long_url_advances`: oversized row rejected but final tuple includes it; `reader_batch_bound`: no batch exceeds requested512. |
| **U7 — Firefox reader** | `src/readers/firefox.rs`, reader factory in `mod.rs`, `tests/firefox.rs`; U1,U5. | `FirefoxReader::open`, HistoryReader, `open_reader`. | `firefox_prtime`: 1000001→1s and -1→-1s; `firefox_join_and_cursor`: joined URL/title and tied timestamps page correctly; `firefox_duration_none`: duration always None; `browser_schema_failure_is_source`: missing moz_places yields Source, never migrates source; `cursor_reset_rescans`: max below checkpoint restarts scan. |
| **U8 — normalizer** | `src/normalize.rs` (split platform helpers if needed), `tests/normalize.rs`, test vector data; U1–U2. | `normalize`, ItemKey/Normalized/Platform. | `normalizer_url_vectors`: every §6 table row asserted; `twitch_local_midnight`: 15:59:59Z vs16:00:00Z keys differ at +08; `normalizer_identity_params`: p1/default merge, p2 distinct, duplicate v rejected; `normalizer_no_network`: pure function has no transport argument or side effect; `local_path_case`: drive letter normalized, filename case retained. |
| **U9 — transactional dedupe** | `src/dedupe.rs`, `tests/dedupe.rs`; U3,U8. | `Candidate`, `ingest(tx,&Candidate)`, rank helper, IngestOutcome. | `dedupe_window_vectors`: all §7 rows except orchestration cases; `dedupe_fixed_anchor`: 0/1200/2400 creates2; `dedupe_richness`: firefox then chrome retains anchor and upgrades title/source; `dedupe_rollback`: dropped transaction leaves0 items/views; `dedupe_two_writers`: two independent connections contend, no duplicate after retry; `dedupe_approx_max`:10+20→20. |
| **U10 — export** | `src/export.rs`, `tests/export.rs`; U3. | `export(conn,format,writer)`, `export_to_path(...)->Result<()>`. | `csv_escaping`: comma/quote/newline title round-trips using csv reader; `json_null_and_order`: null preserved, ordered timestamps/IDs; `export_one_row_per_view`:2 views→2 records despite one item; `export_no_clobber`: preexisting output bytes unchanged/error; `export_streaming`:100k records emitted without collecting Vec of rows. |
| **U11 — bounded logging** | `src/logging.rs`, `tests/logging.rs`; U1–U2. | `Logger::record(&LogRecord)`, validated log settings. | `logs_global_cap`: simulated PIDs/rotation never exceed3×1MiB; `logs_redact_and_truncate`: private URL/title/token absent and oversized message still valid JSON ≤4096 bytes; `log_lock_fallback`: occupied lock writes stderr sink without failing collect; `dry_run_no_log_files`: no real log directory created. |
| **U12 — scheduler/helper** | `src/scheduler.rs`, `src/scheduler/windows.rs`, `src/bin/omoiw.rs`, `tests/scheduler.rs`; U1–U2,U11; helper entry delegates to U13 when integrated. | `task_xml(&TaskSpec)`, `task_argv`, `install/remove`, `helper_run`. | `schedule_xml_contract`: literal XML snapshot contains PT1H/logon/InteractiveToken/priority7/battery/hidden and matching principal; `schedule_escapes_paths`: ampersand/quotes round-trip expected XML escapes and argument quoting; `schedule_argv_no_shell`: exact arrays in §10; `schedule_linux_stub`: install/remove/helper return Unsupported, no files/processes created. Windows manual smoke separately required. |
| **U13 — collect/CLI/session stub** | `src/collect.rs`, `src/cli.rs`, `src/main.rs`, `src/sessionize.rs`, `tests/cli.rs`, `tests/collect.rs`; U1–U12. Private `collect/spool.rs` helper if needed to keep files small. | `cli::run(args)->ExitCode`, `collect`, `sessionize::available/run`. | `cli_surface`: exact subcommands; csv+json and invalid browser→2; `collect_week_idempotent`: fixture week imports then0 additions; `collect_partial`: one corrupt/one good profile→3, good committed, bad checkpoint absent; `collect_atomic_checkpoints`: injected write error leaves rows/cursors unchanged; `collect_dry_run`: real DB/checkpoints bytes/log directory unchanged, predicted counts correct; `collect_concurrent_checkpoint`: stale prepared run cannot overwrite newer cursor; `collect_nonvideo_progress`: rejected-only batch advances cursor; `sessionize_m0_stub`:false/NotImplemented. |
| **U14 — fixture/measurement harness** | `examples/synthetic.rs`, `scripts/measure.ps1`, `tests/synthetic.rs`; U13. | example parser/generator (testable helper module); documented harness args in §12. | `synthetic_counts`: 100000 views/40000 items after ingest, second run0; `synthetic_deterministic`: same seed→same ordered logical source rows; `synthetic_spacing`: repeated item anchors >1800s, duplicates ≤60s; `synthetic_no_user_paths`: every generated path below supplied temp root. Linux generator/collector tests required; Windows counters validated on Windows, no fabricated Linux working-set values. |

Integration gate: `cargo +stable test --all-targets` on Linux; stable GNU build/tests on Windows; named FTS, sidecar, concurrent-writer, dry-run and cap tests must not be ignored. Keep acceptance tests behavior-based: actual SQLite results, file bounds and exit codes. Unit-test XML construction on Linux; this does not replace registration verification on Maria's Windows PC.

Review trace: F1→§13 listener rules; F2→§10; F3/F4/F5→§4; F6→§5 and §12; F7→§7; F8→§3/§6. Remaining platform adapters and budgets are explicit future work, not hidden M0 dependencies.
