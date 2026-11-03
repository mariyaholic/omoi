# UI measurements — November 3, 2026

The final optimized Windows build completed the five-theme journey with 10,000 generated items. Warm navigation measured 121 ms median and 138 ms maximum across ten samples. UI-server lifetime peak was 11,599,872 bytes.

Isolated Edge private bytes were 213,983,232 before the app, 280,141,824 with the catalogue before auditing and 556,122,112 after the full journey including audit tooling. These figures include the browser processes and do not measure the incremental cost of a tab in an existing browser.

The server recorded 46.875 ms CPU over a short three-second idle sample. Other applications and cold-cache state were uncontrolled. The results are observations, not universal latency or zero-idle-CPU guarantees.

Detailed receipts and acceptance follow-up are retained locally at the owner's request. See the [verification summary](../docs/headless-acceptance-20261103.md) for binary hashes and measurement scope, and [UI instructions](README.md) for use.
