# Omoi UI direction

Audience: one person finding something they opened and deciding what to watch next.
Concept: a personal viewing ledger with a small, explained suggestion shelf.
Three words: nocturnal, indexed, quiet.
References: library catalogue indexes and the dense list hierarchy of early desktop media libraries.
Density: compact editorial rows; a calm reading surface rather than a wall of thumbnail cards.
Canon: inherit the local `design/uika-canonical.css` estate tokens: ground `#0b0913`, panel `#14101c`, platinum text `#e9e4da`, gold `#bb9955`. Do not edit the supplied canon. Accessibility contrast and focus states still need rendered verification.
Forbidden: giant metric tiles, autoplay previews, unsupported "watched" claims, recommendation filler, or controls that install tasks without explaining the action.

## Recommended first screen

Default to **History**, not recommendations. Keep search at the top, a compact source/date filter line underneath, and a chronological list grouped by day. Each row contains title, platform, visit time and a small evidence label: **Opened** for browser history, **Playing time measured** only when session telemetry exists. A duration approximated from a browser tab is explicitly **Tab time**, never viewing time.

Use one short sidebar: History, Suggestions, Sources, Settings. Put "Last collected" and a manual Refresh action at the bottom. Show source failures near the refresh state; the rest of a successful import remains usable. No startup chart grid.

Select a row to open a right-side detail pane with canonical identity, source evidence, visits, tags when implemented, and Open original. Delete requires a clear confirmation and explains whether the item and all visits are removed. URL/title rendering is escaped; file URLs must not execute content automatically.

## Suggestions

Present a maximum of a few compact lists, each with a reason attached:
- **Continue watching:** only real session position and duration; absent in current browser-only data.
- **Rediscover:** previously opened, no visit in 30 days. Say "opened before", not "you loved this".
- **Similar by tags:** explain which metadata supports the match; available only with a selected item and stored tags.
- **Fits your time:** only known video durations; unknown duration is not a zero-length video.

These four local shelves have a CLI implementation. New-from-channels RSS and TMDB recommendations remain future adapters. Display one truthful setup/empty state rather than suggesting unrelated old entries as unseen recommendations. Do not make a full shelf look mandatory.

## Sources and automation

Show discovered profiles as a checklist with readable browser/profile names and an exclusion toggle. Keep paths in an expandable technical detail area. Use a two-step first collection: confirm selected sources, then collect. A dry run shows proposed counts and explains that it still reads history.

Automation has a clear state: Not installed / Installed / Needs attention. Explain hourly plus logon, logged-on user only, skipped on battery, and no resident collector. Install, inspect and remove are deliberate actions. Show the resolved catalogue location and executable directory so moving binaries has a visible consequence.

## Build order and cost

1. Read-only timeline, literal search and platform/date filtering, 50 rows per page. Use the existing SQLite FTS; no embedding model.
2. Source status and manual collect with an explicit busy state.
3. Deletion and preferences with per-launch mutation token and origin/Host checks.
4. Local suggestion shelves and their reasons.
5. Measured thumbnails only after the existing disk-use gate; retain a text-only mode.

Use the planned on-demand loopback UI, static local assets and ten-minute idle exit. Keep the browser as the first UI host; reconsider a native wrapper only after measuring the UI server/browser cost. No Electron runtime, network font/CDN, remote analytics, or background recommendation service.

Measure the UI server and the browser tab separately. The collector's measured footprint is not a budget claim for a browser-hosted interface. Start with plain HTML/CSS and small JavaScript modules, ordinary paginated queries and local system fonts. Update the view after explicit actions instead of polling the database continuously. Give the search field a visible label, keep filter state when opening details, and restore keyboard focus when the detail pane closes.

## What still needs visual validation

An interactive, fictional-data prototype is now available at [prototype/index.html](../prototype/index.html), with [browser verification notes](../prototype/README.md). History, Suggestions, Sources, Settings and details were checked at 1440, 375 and 320 pixels in all five appearances: Uika, Mariyaholic Confluence, Mariyaholic Panel, Mariyaholic-98 and Argonaut Oak. Empty/partial states are also available. The [Open Design reference review](../prototype/REFERENCE_REVIEW.md) records the local systems behind those appearances. It is not connected to the Rust CLI, catalogue or scheduler. The production checks below still apply; 18 sample entries do not establish the 10k-item performance target.

The production UI remains unbuilt. Before implementation is accepted, repeat desktop/mobile checks against the connected application: empty history, mixed sources, long titles, partial import failure, missing metadata, keyboard-only focus and delete confirmation. Validate contrast, 200% text enlargement, 320 CSS-pixel reflow (including 400% zoom from 1280px), persistent visible keyboard focus and the 10k-item load-time target. Keep later time-spent charts separate until their provenance and overlap handling are implemented. These accessibility checks follow [W3C reflow guidance](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html) and [visible focus guidance](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html); the prototype checks do not establish production accessibility conformance.
