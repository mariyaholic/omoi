# Guided setup implementation and acceptance

Implementation branch: `feat/guided-setup`, based on documentation commit
`eb69e67b7adad1b93dfa4e8c7c893edf961b10a8` (not an ancestor of main at checkout).
The existing transfer checkout was left untouched. Actual checkout:
`/workspace/omoi`, verified remote `mariyaholic/omoi`. No AGENTS.md or repo skills
were present on the requested branch. No owner data, paths or private receipts used.

## Package status

| Package | Implemented and locally checked | Remaining acceptance |
|---|---|---|
| IF1 | Versioned bounded setup record, independent state dimensions, stale mutation rejection, consent epochs, source scope; schema 3 unchanged | Windows UI consistency below |
| IF2 | App-owned listener, safe collision, cancellable network reads, startup inbox drain, 256-event transactions, automatic processing, stop/restart | Full Windows process-tree/long-run measurements |
| IF3 | Guided optional capture, resumable step, explicit credential copy, script export/import fallback, authenticated hello, shared 0.2.0 capture core | Real Edge manager permission/foreground-dialog journey |
| IF4 | Generation-owned replies, automatic catalogue refresh, persisted exact scheduled scope, explicit native background opt-in | Physical logon/sleep/battery; no desktop auto-launch feature promised |
| IF5 | Packaged MV3 companion, bounded serialized durable worker queue, Rust native host, exact origin and catalogue/consent binding, protocol negotiation | Actual Edge extension→host journey, permission UI, multi-profile/provider/DRM behavior |
| IF6 | Inno per-user installer, portable package, exact-ID HKCU registration/repair/removal; hosted Windows install/reinstall/uninstall and preservation checks passed | Clean Windows device, true cross-version upgrade and missing-runtime recovery |
| IF7 | Reproducible tests, Linux synthetic measurements, CI gates, setup/protocol/handoff docs | Signing, store identity/publication and physical-device gates are owner release work |

These are source/logic deliverables, not a declaration that every roadmap acceptance
cell is complete. No public release or extension was published. Installer/companion
are development artifacts; store ID and signing remain deliberately unconfigured.

## Validation actually run locally

Linux, Rust 1.99.1, Node and Python. Baseline before implementation passed format,
Clippy, all-target Rust tests, seven userscript tests, two desktop cleanup tests,
release build, smoke journey and playback transport/restart journey.

Implementation validation:

- `cargo fmt --check`
- `cargo clippy --locked --all-targets -- -D warnings`
- `cargo test --locked --all-targets`: 94 passing Rust tests
- `node --test tests/userscript.test.cjs tests/extension.test.cjs`: 16 passing tests
- `python tests/desktop_state.py`: two passing cleanup tests
- `OMOI_TEST_BROWSER=/usr/bin/chromium python tests/ui_state.py`: 13 passing UI
  transition tests, synthetic intercepted requests, Linux Chromium, **not Edge**
- `python scripts/build_capture.py --check`: shipped bundles match local sources
- `cargo build --locked --release --bins --example synthetic`
- `python scripts/smoke.py --bin-dir target/release`
- `python scripts/verify_playback.py --binary target/release/omoi`: original
  authentication/origin/host, storage rejection, restart, dedupe and collision checks
- `python scripts/verify_setup.py --bin-dir target/release`: actual binaries;
  history-only, skip/resume/stop, automatic materialization, native origin/framing,
  version/permission/binding rejection, DB-busy no-false-ack, stale consent rejection,
  progress preserving capture epoch, app-absent rejection, explicit background, EOF
  and truncated input. Three-event HTTP→catalogue journey: 402 ms in the final run.
- JS syntax, Python compilation and `git diff --check`.

The existing state/migration/accounting tests remain enabled. New tests include
startup inbox drain, partial-client shutdown/port release, credential-preserving
collision, worker concurrency/restart, rejection of new foreground samples while the app is absent, stale setup generation and explicit epoch
queue discard. Fake worker/platform tests are labeled; they are not browser proof.

Windows-only CI now additionally compiles the unsigned installer, exercises
registration/path repair in an isolated HKCU test namespace, runs install/reinstall/
uninstall on a disposable runner, verifies the portable extracted executables,
and retains private unsigned review artifacts for seven days. Hosted results are
recorded in the PR; source presence alone is not a pass. Native Windows UI and installer checks are established separately by the hosted run below. Actual Edge companion integration, physical hardware behavior and signing remain unverified.

## Measured Linux costs

Reproduce: `python scripts/measure_capture.py --bin-dir target/release`.
This measures the UI HTTP process with its in-process listener and a separate native
host; no WebView2 or Edge process is included.

| Measurement | Observed |
|---|---:|
| Cold UI server launch, not desktop launch | 4.17 ms |
| Capture off, five-second sample | 0 CPU ms at OS tick resolution; 3,624,960-byte peak RSS |
| Capture on, five-second idle sample | 0 CPU ms at OS tick resolution; 3,817,472-byte peak RSS |
| UI main-thread voluntary context switches in each idle sample | 49 in five seconds |
| 100 synthetic events, UI+listener CPU | 130 ms |
| UI+listener peak RSS over capture | 4,026,368 bytes |
| Receipt→catalogue latency (100 events) | median 100.70 ms; max 203.57 ms |
| Catalogue/WAL/metadata disk growth | 1,044,424 bytes |
| Native-host five-second idle sample | 0 CPU ms at OS tick resolution; 2,904,064-byte peak RSS |

Short samples and tick quantization do not establish zero CPU. Main-thread context
switches are not total process-tree wake frequency. GUI/browser costs, total temp
growth, retained database growth and Windows cold-launch/long-duration distributions
remain pending. UI idle polling was reduced from 100 to 10 polls/second; managed
capture polls at 10/second, with empty processing avoiding a writer transaction.
The existing Windows listener 10 MB fixture gate was not changed or represented as
passed on Linux. `scripts/measure.py` remains the Windows collector measurement tool;
`scripts/verify_playback.py --duration ... --stress-inbox` retains its resource gate.

## Review and release limits

- Approve an actual release extension ID and configure the release companion/host
  identity before store distribution. Current shipped companion uses the distinct
  development host name. No wildcard origin or unauthenticated pairing endpoint.
- Verify a clean Windows/Edge install, site-permission denial/retry, missing WebView2,
  foreground manager recovery, worker/browser sleep/restart, concurrent tabs/profiles,
  and provider-specific ads/DRM. Test with disposable profiles and synthetic media.
- Measure WebView2 + Edge + host separately and together. Capture process costs above
  are not a total desktop/browser budget. Inspect hosted Windows artifacts and tests.
- Arrange signing/store submission separately. Do not automatically downgrade an app
  against a newer schema. Use an explicit compatible backup recovery if needed.
- Legacy manager queues retain the pre-existing cross-tab storage limitation; use
  the dedicated companion for a single durable queue owner. New background mode is
  native companion only; no always-on system service or implicit app autostart.


### First hosted Windows run

[Run 41606286748](https://github.com/mariyaholic/omoi/actions/runs/41606286748)
on `a22364c` passed Rust format/lint/tests, release build, smoke, the hidden native
WebView2 window (five themes and appearance reload), legacy playback and JS tests.
The listener's existing two-event fixture reported 7,069,696 bytes peak working set,
passing its unchanged 10 MB gate; the three-second idle sample was below an OS tick.
The new setup/native journey passed its assertions (203 ms HTTP→catalogue), then
failed fixture cleanup because Python SQLite context managers do not close their
connections on Windows. The fixture now uses explicit `closing(...)`; this is a
test-handle correction, not suppression of cleanup errors. Later installer/registry
steps were skipped in that run and remain pending the corrected run. CI now runs
independent packaging/registry checks despite an earlier unrelated journey failure.


### Completed hosted acceptance

[Run 41607814846](https://github.com/mariyaholic/omoi/actions/runs/41607814846)
passed both Ubuntu and Windows jobs for implementation head `6699b83` (CI tested
its merge with main). Windows passed native desktop/themes/reload, all Rust and JS
checks, actual-binary guided setup/native transport including fixture cleanup,
isolated HKCU registration/path repair/removal, Inno compilation, real per-user
install/reinstall/uninstall preservation, portable extraction/system-only PATH,
scheduler fixtures and 11 Edge UI transition tests. Listener peak: 7,041,024 bytes;
HTTP→catalogue fixture: 203 ms. These remain scoped synthetic results, not physical
device or browser-companion permission proof.

[Private unsigned artifact](https://github.com/mariyaholic/omoi/actions/runs/41607814846/artifacts/13217268855)
contains the installer and portable ZIP; retention ends 2026-11-10. Its source is
recorded in BUILD.json (the CI merge commit). It is not a signed/public release.

Two subsequent focused UI regressions cover deferred refresh after closing Details
and first recording before the initial status poll. The UI tracks the revision
actually rendered, retaining refresh work while a dialog/input owns focus. All 13
UI tests passed locally in Chromium; the PR's current CI rechecks them on Edge.
