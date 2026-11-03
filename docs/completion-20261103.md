# Catalogue controls and playback

Historical implementation record. [Headless acceptance](headless-acceptance-20261103.md) supersedes the remaining-gate list and measurements below.

This revision adds in-app export, scheduler controls, metadata/tag editing and optional playback capture. All five themes remain: Uika, Confluence, Panel, Mariyaholic-98 and Argonaut Oak.

## Use

- Details → **Edit metadata and tags** edits title, channel, type, duration and tags. Tags are normalized to lowercase and deduplicated. Edits and FTS updates commit together; invalid input leaves the item unchanged. Manual metadata survives browser imports.
- Settings → **Export JSON / Export CSV** exports all openings. Supporting browsers stream to a chosen file; others download a bounded Blob (64 MB maximum, then use CLI). The server streams through a temporary file, cleaned after the request. These are history interchange exports, not full database backups containing tags/sessions.
- Sources → **Automatic collection** checks status or explicitly enables/disables the hourly/logon task with confirmation. It uses config.toml, not checkbox selection. Keep both executables in a stable folder. Linux reports unsupported scheduling.
- Settings → **Playback capture** inspects/processes the event inbox. Normal collection processes it transactionally too; dry runs use a temporary database.
- **Run Playback.cmd** starts the separate opt-in listener. Pair the included script using [Playback setup](../userscripts/README.md). Neither UI nor collector starts the listener implicitly.

## Storage and boundaries

Migration 2 preserves existing catalogues and adds field provenance, replay receipts, stream identities, session state, interval measurements and drop counters. Listener acknowledgements follow a FULL-synchronous transaction. Requests require the separate protected token, exact Host, absent Origin, JSON framing and bounded bodies. The listener exposes no read/export/delete/scheduling API.

Sessions survive processing restarts. Pauses and seeks do not add time; telemetry gaps over five minutes are unmeasured. Retry IDs and sequences prevent duplicate time. Twitch live producers retain their initial date identity across midnight. Overlapping capture for an item uses maximum interval density instead of adding tabs. SQLite sorts endpoints with its bounded cache and disk-backed temporary storage; Rust holds active intervals. Time remains sample-derived.

The inbox caps at 100,000 rows and 20 MB of body bytes. The manager caps at 32 queues of 256 events with retry backoff to 60 seconds. Receipts, sessions and intervals are retained; long-term disk growth remains an acceptance item. Export temporary disk can equal exported history size.

## Verification and remaining acceptance

Local final verification: **61 Rust tests**, formatting, Clippy with warnings denied, Ruff, JS syntax and capture unit test pass. Release CLI, playback transport/restart, isolated Windows task and five-theme browser journeys pass. Receipts are under `ui/`. The executable SHA-256 is `49e193e906e8598191f4c95188248870c95bf32cd46143348fded72d2618d918`.

Tests cover version-1 migration, metadata/FTS atomicity, manual precedence, pause/retry/restart, long gaps, overlap, midnight identity, inbox caps, request authorization and injected catalogue-write failure with checkpoint rollback. A JS capture test checks pause/seek exclusion and retry identity. Process harnesses exercise listener restart and isolated Windows task install/run/remove.

Browser checks cover five themes and four screens at 1440/375/320 widths, editing/export, confirmation cancellation and keyboard containment/focus restoration. Focus indicators, reduced motion and forced-colour treatment are included. This is not full screen-reader/WCAG conformance proof; actual browser zoom and default-browser launch need manual review.

The generic HTML-video adapter still needs acceptance against authenticated live players. The later headless record covers an installed manager with a synthetic HTML video and the owner's existing retained-week history. Native SMTC, remote metadata APIs, RSS/TMDB recommendations and M2 imports remain separate roadmap work. No cookies or paid services are used. Independent review remains open. The entire M0–M5 roadmap is not declared complete.
