# Connected UI evidence — 2026-11-03

Historical measurements of source `ac16495`. See [later headless acceptance](../docs/headless-acceptance-20261103.md) for the retained-week test, full-inbox fix, hour run, real manager transport, browser process measurements, axe audits and actual browser zoom.

Local Windows GNU optimized build; installed Edge, headless. All data was generated in isolated temporary catalogues. Browser discovery roots were replaced with fixture roots. No personal history was read. A uniquely named temporary scheduler task was installed, launched and removed in a separate synthetic check; normal Omoi scheduling was not enabled.

Binary `omoi.exe`: 2,425,344 bytes. SHA-256: `49e193e906e8598191f4c95188248870c95bf32cd46143348fded72d2618d918`.

The executable ran from D:; the temporary catalogue ran from the Windows temporary directory on C:. These measurements ran after compilation completed. Other user applications were not stopped. Results describe this fixture and run, not a universal latency guarantee.

| Measurement | Result |
|---|---:|
| Items after confirmed deletion | 10,000 |
| History page size | 50 items |
| Warm History navigation, median of 10 | 111.4 ms |
| Warm History navigation, maximum of 10 | 137.4 ms |
| History API, median of 20 | 58.5 ms |
| History API, maximum of 20 | 180.1 ms |
| Server working set after the journey | 10,072,064 bytes |
| Server lifetime peak working set | 11,644,928 bytes |
| Server CPU time recorded during 3 idle seconds | 0 ms |

Warm UI timing measures the browser click from Settings to History until 50 rows are present, with cleared search. It includes browser automation overhead. All ten sampled runs were below the 200 ms warm target. Cold browser launch, large-title stress, browser memory and long-duration idle CPU were not measured here. The zero CPU delta is limited by Windows counter resolution and the short sampling interval.

The journey passed: empty state; collecting only a checked synthetic profile; pagination; FTS and literal search; HTML escaping; delete cancellation and confirmed cascade; all four screens and details in all five themes at 1440/375/320; theme persistence; token removal from the visible URL; authentication and Host/Origin rejection; malformed/oversized request rejection; speculative idle connections; port collision; and idle shutdown. No page JavaScript errors or external requests were observed.

The expanded journey also passed metadata/tag editing, JSON download, scheduler-confirmation cancellation, playback inbox processing and dialog keyboard containment/focus restoration.

The restarted listener's two-event fixture measured 9,342,976 bytes working set/peak and 0 ms recorded CPU over three idle seconds. This excludes the browser and short-lived ACL helper; it is not a sustained-playback or full-inbox resource gate.

Receipts: [UI](verification.json), [playback](playback-verification.json), [isolated installed scheduler](schedule-verification.json). Reproduction and behavior: [README.md](README.md).

Remaining acceptance: personal retained-week use, actual logon/battery scheduling, independent review, full accessibility/zoom audit, manual default-browser launch, installed-manager/live-player capture and long-duration playback resources. Current hosted results are reported separately in the PR; older CLI measurements do not establish performance of this revision. Existing M0 gaps remain open.
