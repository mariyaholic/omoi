# Connected UI evidence — 2026-11-03

Local Windows GNU optimized build; installed Edge, headless. All data was generated in an isolated temporary catalogue. Browser discovery roots were replaced with empty fixture roots. No personal history was read and no Windows task was installed.

Binary `omoi.exe`: 2,331,136 bytes. SHA-256: `ed58d20d704e324aa4ad5c0e73073fa4cf461a48752237dc9cda0db587b8bc73`.

The executable ran from D:; the temporary catalogue ran from the Windows temporary directory on C:. These measurements ran after compilation completed. Other user applications were not stopped. Results describe this fixture and run, not a universal latency guarantee.

| Measurement | Result |
|---|---:|
| Items after confirmed deletion | 10,000 |
| History page size | 50 items |
| Warm History navigation, median of 10 | 113.3 ms |
| Warm History navigation, maximum of 10 | 128.2 ms |
| History API, median of 20 | 49.6 ms |
| History API, maximum of 20 | 60.6 ms |
| Server working set after the journey | 9,830,400 bytes |
| Server lifetime peak working set | 11,550,720 bytes |
| Server CPU time recorded during 3 idle seconds | 0 ms |

Warm UI timing measures the browser click from Settings to History until 50 rows are present, with cleared search. It includes browser automation overhead. All ten sampled runs were below the 200 ms warm target. Cold browser launch, large-title stress, browser memory and long-duration idle CPU were not measured here. The zero CPU delta is limited by Windows counter resolution and the short sampling interval.

The journey passed: empty state; collecting only a checked synthetic profile; pagination; FTS and literal search; HTML escaping; delete cancellation and confirmed cascade; all four screens and details in all five themes at 1440/375/320; theme persistence; token removal from the visible URL; authentication and Host/Origin rejection; malformed/oversized request rejection; speculative idle connections; port collision; and idle shutdown. No page JavaScript errors or external requests were observed.

Machine-readable receipt: [verification.json](verification.json). Reproduction command and behavior: [README.md](README.md).

Remaining acceptance: personal-profile retained-week check, installed scheduler smoke, independent review, current Linux/hosted validation of this UI change, full accessibility/zoom audit and a manual default-browser launch check. Existing M0 gaps remain open; these results do not mark the entire roadmap complete.
