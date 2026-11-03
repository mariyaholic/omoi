# Omoi interactive visual prototype

Open `index.html` in Edge, Chrome or another modern browser. There is no build step, installation, network asset or backend dependency. The prototype also works when served from a local static HTTP server.

## Explore

- **History:** search titles/creators/topics (or press `/`), filter platform and date, select an opening for its detail sheet, save, remove, cancel and undo.
- **Suggestions:** rediscover older entries, select a time budget, change the seed for shared topics. Continue Watching stays empty because the sample contains no measured playback.
- **Sources:** select example profiles and preview enabling/disabling automatic collection. These controls never access browsers or Windows Task Scheduler.
- **Settings:** compare all five themes: Uika, Confluence, Panel, Mariyaholic-98 and Argonaut Oak. Adjust list spacing, export the fictional library as JSON, or reset the preview.
- Use **Preview state** in the top bar to inspect populated, empty and partial-collection states. Refresh only simulates a collection.

All 18 openings are sample data. Preferences and edits live only in memory and reset on reload. The existing Rust CLI, database and scheduler are not connected. Metadata shown here demonstrates future interface behavior, not automatic enrichment already available in the CLI.

## Design

The direction is a chronological viewing ledger with five distinct appearances. Georgia gives Night/Paper headings a reading-journal character; local Segoe UI handles controls and Panel mode. PC-98 adds indigo/coral, monospaced type, hard shadows and square window frames. Oak adds pine-black/brass, dense rows and cyan timestamps. The Night colors inherit the Uika canon. The other four themes use scoped studies informed by the user's local Open Design packages; see [reference review](REFERENCE_REVIEW.md). `tokens.css` is an attributed portable copy; the source estate token sheet and Open Design packages are unchanged. Named source fonts use installed copies or system fallbacks. No fonts, thumbnails, scripts or analytics are fetched remotely.

Review pass:

1. The time column and visible opening evidence preserve the ledger concept; the detail sheet distinguishes approximate tab time from measured playback.
2. Desktop and mobile use the same content; visit times remain visible when the narrow layout drops the separate time column.
3. A 320px navigation overflow was traced to intrinsic button widths and fixed by hiding redundant icons at that width.
4. Paper mode originally inherited already-resolved root color aliases. Re-declaring the aliases on the themed body fixes every surface without changing the canon.
5. Screen navigation restores the top of the page, modal Escape restores focus, and mobile has its own Refresh control. All five appearances include a responsive theme gallery with preserved keyboard focus when selecting a theme.

## Verification

`scripts/verify_prototype.py` in the repository uses Python Playwright and installed Edge. Run a local static server for this directory, then:

```text
python scripts/verify_prototype.py --url http://127.0.0.1:43829 --out D:\mariyaholic-build\omoi\prototype-review
```

Verification covers desktop 1440px and mobile 375/320px, no horizontal overflow on all four screens, long-title details, search/filter interactions, literal hostile query text, save/remove/cancel/undo, keyboard shortcut/focus return, sample profiles and automation, empty/partial states, themes/density, sample export, reset, reduced-motion preference and direct `file:` loading. No JavaScript/console errors or external requests were observed. It is not a full accessibility audit, a performance test with 10k items, or a production UI acceptance claim.

Evidence: `verification.json`; five-theme screenshots are retained locally in `C:\Users\iuniq\.codex\visualizations\2026\10\27\01a17f2f-e6fe-7ac3-ba10-e43e7f5af853\omoi-five-themes`. The application uses small plain HTML/CSS/JS assets and local font stacks. Asset size is not browser memory usage.
