# Connected catalogue UI

Run `omoi ui --open`. Without `--open`, copy the printed launch URL into a browser.
The executable embeds the interface and all five themes; no Node, Python or asset installation is needed.

The server binds only `127.0.0.1:43822`. Use `--port 0` for an available port if that port is occupied. It exits after ten minutes without authenticated API requests, or when the console is closed. `--idle-seconds` overrides the timeout for testing (1–3600 seconds).

## Workflow

1. History reads the catalogue selected by `OMOI_HOME`, or the normal OS default. Starting the UI creates/migrates an empty catalogue if needed; it does not collect browser history.
2. Sources lists profiles allowed by `config.toml`. Select the profiles to collect, choose **Collect selected**, and confirm. This selection is for the current manual run and does not change scheduled collection.
3. History shows one row per item, ordered by its most recent opening, 50 rows per page. Search uses a quoted FTS phrase by default; Literal text uses a substring search. Platform, exact channel and date filters apply to the entire catalogue. Date filtering uses the browser's local midnight; displayed dates use the browser's timezone.
4. Details shows the latest 50 openings, available duration/tags and recorded playback totals. Tab time remains distinct from playback. Open original links use normalized platform identities; local files are not executed. Twitch live identities reopen the channel, not an archived stream.
5. Delete asks for confirmation and removes the item plus related openings, sessions and tags in one transaction. It cannot be undone here. Browser files and import checkpoints remain unchanged; a later new opening may recreate the item.
6. Suggestions uses the existing local recommendation queries. Missing sessions/tags/durations yield empty shelves. Select an item in Details to find shared tags.
7. Settings provides Uika, Confluence, Panel, Mariyaholic-98 and Argonaut Oak. Appearance and spacing persist in browser local storage, scoped to the catalogue and HTTP origin; changing the port creates a different browser storage origin.

Export and Windows task installation/status/removal remain CLI operations, with commands shown in the UI. Metadata enrichment, tag editing and playback capture are not implemented by this change. No fake save controls or simulated collection are present in the connected UI. The separate `prototype/` remains a fictional-data design study.

## Session and request boundaries

Every launch generates 32 bytes from the OS random source. The launch URL carries the hex token in its fragment; JavaScript immediately removes it from the visible URL and keeps it only in memory. Every API call requires the token header, including reads. Reloading the tab loses the token: reopen the printed launch URL. Do not share that URL.

Exact loopback Host is required. Foreign or null Origins are rejected; mutations require the exact same origin and JSON content type. No CORS permissions are sent. Assets are an explicit embedded allowlist. CSP forbids external resources and framing; responses are no-store with no-referrer. Titles and metadata are escaped in the UI and queries use bound parameters. The UI token is separate from any future listener token.

The standard-library server bounds headers to 16 KiB and request bodies to 64 KiB. It checks a five-second read deadline between reads, with two-second socket timeouts; a read already in progress may extend beyond that deadline. Writes have a five-second timeout. Up to 16 speculative sockets are parked until readable; idle sockets expire after two seconds. Requests execute sequentially; a collection can delay other API requests. No resident collector, database polling or remote service is added.

## Verification

```text
cargo test --locked --test ui
python scripts/verify_ui.py --binary path/to/omoi.exe --out path/to/evidence
```

The Python journey isolates all browser roots and creates synthetic browser databases. It exercises authenticated HTTP, selected-profile collection, a 10,000-item catalogue, search, pagination, safe rendering, confirmed deletion, five themes at 1440/375/320, appearance persistence, port collision and idle shutdown. It records warm UI timings, API latency and Windows server working set/idle CPU. Browser memory is not measured by this script.

Current measured results and limitations: [MEASUREMENTS.md](MEASUREMENTS.md).
