# Installation and playback setup roadmap

Status: implementation branch, 2026-11-03. Source and synthetic coverage now exist for IF1–IF7. Hosted Ubuntu and Windows desktop/installer/registration checks passed; actual Edge-companion and physical-device acceptance remains pending. See [acceptance and remaining gates](guided-setup-acceptance.md), [setup](SETUP.md) and [transport contract](capture-protocol.md). Original roadmap requirements follow. Entry point:
[CLOUD_HANDOFF.md](../CLOUD_HANDOFF.md). Baseline: merged PR #4, schema 3,
desktop wrapper and userscript 0.1.1. This document supersedes historical
foundation work orders for the installation task; it does not redefine catalogue
dedupe or playback accounting rules.

## Observed journey and problem

The Windows app and listener worked after an assisted Edge setup, but too much
knowledge lived in the operator's instructions. A copied script is not an installed
script; an installed extension is not an enabled script; a paired producer is not
proof of a materialized playback session.

| Observed friction | Required product response |
|---|---|
| No manager installed in the user's normal Edge profile; earlier manager testing was disposable | Never infer installed state from a shipped file or a test receipt. Offer history use immediately; guide optional capture separately. |
| New app package included script 0.1.1, but did not update a browser-installed script | Distinguish app, protocol and capture-client versions; provide a visible update action and compatible upgrade path. |
| Opening the local `.user.js` URL showed raw code rather than an installer | Provide a verified direct-import fallback; do not report installation merely because a URL was opened or fetched. |
| An additional Allow user scripts toggle was needed | Explain the missing permission with one precise action; readiness must come from the client, not assumptions about browser version. |
| Tampermonkey's background `window.confirm()` was suppressed; browser restart and a foreground retry resolved the observed flow | Open one user-triggered setup destination at a time. Retain progress; provide foreground/retry instructions. Restart is a recovery option, not a universal cure. |
| Pairing required finding a token file, copying it and entering a port | Put connection setup in Omoi; default companion path should not expose tokens or port entry. |
| Listener was a second manually launched program | App owns the opted-in capture lifecycle; background mode is separately chosen and visible. |
| Received events remained in an inbox until Process playback was clicked | Materialize automatically while the chosen capture mode is active, then refresh affected catalogue views. |
| Browser-control tooling could not initialize | This was an assistance-environment failure, not an Omoi runtime defect. End users must be able to complete setup without an agent or remote-control tool. |

The pilot established one Edge capture path, not reliable installation on every
device. Do not commit its private URLs, playback data, tokens, paths or raw logs.

## Target journeys

**History only:** install → open → choose browser/profile scope → collect → view
results. Capture setup can be skipped and resumed later. Automatic history collection
is a separate explicit choice; its persisted profile scope must match what the
user chose, rather than silently collecting every discovered profile.

**Playback:** Enable playback capture → install/approve the Edge companion → grant
supported-site access → verify connection → play a video → recorded session appears.
Target: no shell, script editor, token file, manual port entry, second launcher or
Process playback button on the default companion path. Browser approvals are not
counted as failures or bypassed. The interim userscript path may still require
manager import and permission approval, and must say so.

**Returning user:** launch Omoi → restore opted-in settings → resume transport and
drain pending events → show current connection and last received/processed times.
Reopening setup must not duplicate capture, tasks, host registrations or scripts.

**Second device:** independently install and consent locally. Generate local
credentials; do not copy pairing secrets or assume extension sync also installed
the native host. Existing database import/backup is a separate concern.

## Architecture decisions for implementation

### App-owned capture and processing

Keep Rust/SQLite and the existing capture/session semantics. Factor reusable listener
and ingest lifecycle components; the desktop should not spawn an untracked console
or scrape a launcher's text to infer health. Default capture lasts while Omoi is
open. Closing it stops resources it owns after the current bounded transaction;
background capture/autostart is a separately persisted opt-in, not implicit tray
behavior. Preserve `omoi listen` as an advanced/manual path.

Handle an already-running listener explicitly: verify it belongs to this catalogue
and supports the expected protocol before reusing it. An occupied port is not proof
of ownership. Never terminate an unknown process or rotate an existing token to
resolve a collision. A component stops only resources it created; expose a repair
action when ownership cannot be established.

Signal processing after a durable enqueue, coalesce bursts and consume bounded
batches through `sessionize`. Initial target: a received event is materialized
within five seconds under the standard synthetic workload while capture is active.
Use bounded busy backoff and visible backlog status. No full-history browser scan
per heartbeat, per-frame database writes, indefinite shutdown join, or continuously
busy polling. Startup drains a prior inbox. Disconnect/retry never changes the
event ID; receipt+inbox commit still precedes acknowledgement.

### Interim userscript flow

Provide in-app guided steps, a versioned script download/export and direct-import
fallback. Only open a browser destination after a click; keep a return-to-Omoi
action and persist the step. If loopback serving is used, serve an explicit asset
allowlist, without directory listing, tokens or a publicly reachable bind; expire
the temporary endpoint. Do not depend on navigating to internal extension URLs
or inspect private browser storage to determine completion.

Add a versioned, authenticated client hello/status contract. A hello proves a
reachable client, not successful recording; separately show last event and last
materialized session. Existing 0.1.1 clients lack this contract, so preserve their
ingestion and show version unknown until upgraded. Do not overload receipt/event
tables with fake playback to implement readiness.

If copying a pairing token remains necessary during this bridge phase, do it only
through an explicit local action, never in a URL, source bundle, screenshot, log or
cloud artifact. A short-lived one-use pairing mechanism requires an explicit threat
model and tests before replacing the current authentication. No unauthenticated
endpoint may hand arbitrary webpages the persistent listener credential.

### Default Edge companion

Build a dedicated Manifest V3 extension using packaged content scripts for the
supported video sites. Reuse capture logic/tests through a shared core and thin
manager/extension adapters. Do not rely on Tampermonkey GM APIs in an extension,
or turn Omoi into a general remote-script loader. Packaged content scripts and
user-provided scripts are distinct APIs; the latter introduces an extra userScripts
permission flow. See [content scripts](https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts)
and [userScripts permissions](https://developer.chrome.com/docs/extensions/reference/api/userScripts).

Preferred companion transport: a small Rust native-messaging host registered per
user by the Windows installer. Edge native messaging supports a separate host
manifest with explicit allowed extension origins; the application installer owns
host installation/registration. Validate exact Edge/Windows behavior against
[Microsoft's native messaging documentation](https://learn.microsoft.com/en-us/microsoft-edge/extensions/developer-guide/native-messaging).
This is the proposed Omoi design, not implemented behavior.

Use a versioned, size-bounded stdio protocol. Keep protocol stdout free of logs,
handle truncated/malformed frames, and reuse the same durable enqueue/processing
service. Prefer a connection with a defined idle lifetime over launching a process
per sample. Test service-worker suspension, reconnect, EOF, duplicate delivery and
concurrent tabs; do not depend on a permanently running MV3 worker. Consolidate
client queue ownership rather than inheriting the userscript's cross-tab storage
race. Give every capture epoch a fresh producer identity after a counter reset.

Allowlist only intended extension IDs, with distinct development/release registration.
Restrict the host protocol to opt-in capture, status and version negotiation; it
must not expose unrestricted catalogue export/delete or arbitrary commands. Persist
and enforce capture consent across UI/host processes so a reconnect cannot override
Stop. Background mode must work only when explicitly enabled. Define an explicit
catalogue binding so two homes cannot mix queues or credentials.

The existing HTTP listener rejects requests carrying an Origin. A plain extension
fetch must not be made to work by accepting all origins, permissive CORS or disabling
authentication. Native messaging avoids needing the default companion to copy the
userscript's loopback credential. Retain the authenticated listener for legacy capture.
If native messaging proves unsuitable, document concrete evidence and a reviewed
replacement contract before changing that boundary; continue IF1–IF4 independently.

### Installation, repair and updates

Deliver a per-user Windows installer alongside the portable ZIP. Detect missing
WebView2/runtime dependencies and offer an actionable supported recovery. Register
only Omoi's shortcuts/host components; do not use browser force-install policy,
edit browser profiles or silently grant permissions. Portable mode must disclose
that host registration is separate and supply an explicit install/remove action.

Persist consent, theme/density and catalogue location across app updates. Make host
registration idempotent and repair stale executable paths when installing to a new
location. Remove only owned registrations on uninstall; preserve catalogue data by
default. Keep the current monotonic-ID, deletion-cutoff, FTS and transactional
migration guarantees. An older binary must never be launched against a newer schema
as an automatic rollback: retain a compatible app or restore an explicit pre-upgrade
backup through a separately confirmed recovery flow.

Extension updates and app updates are independent. Negotiate a protocol range and
minimum capture-client version, reject incompatibility with a useful action, and
preserve queued events/IDs across compatible upgrades. Do not promise automatic
desktop updates without an implemented trust/verification mechanism. Build unsigned
development artifacts in cloud; signing, store publication and public distribution
remain separate owner release actions. No account or hosting service is needed for
ordinary local capture.

## State and readiness contract

Represent independent dimensions rather than a single optimistic Connected flag:

| Dimension | States/evidence | Recovery |
|---|---|---|
| User choice | not chosen / skipped / enabled / stopped | Resume setup; stopping revokes active capture without deleting history. |
| Capture client | unknown / permission needed / hello observed / incompatible | Explain the exact missing action; installed files alone prove nothing. |
| Transport | stopped / starting / ready / reconnecting / collision / failed | Bounded retries, ownership-safe stop, one repair action. |
| Data path | no event yet / receiving / queued / processing / recorded / busy / failed | Show distinct timestamps and backlog; do not tell the user to reinstall for ordinary DB contention. |
| Update | compatible / newer available / upgrade required / failed | Preserve settings/queue; report which component requires action. |

Persist choices and setup progress; derive liveness from current handshakes/process
ownership. Expired last-seen timestamps must not look connected. Retry callbacks
must carry a request/setup generation and cannot update an abandoned screen.
Use inline status and confirmations in Omoi; avoid background `window.confirm()`.
Never mark ready merely after opening the store, downloading a script or dismissing
a dialog. Stop/disable and re-enable must be idempotent.

## Ordered work packages

| Package | Deliverable and source surfaces | Acceptance before calling it complete |
|---|---|---|
| IF1: status and fixtures | Define versioned setup/status types and persisted choices; `src/config.rs`, `src/ui.rs`, proposed setup module, `tests/` | Fresh/skipped/resumed/stopped setup, absent/old client, stale reply and unknown version have honest observable states. Existing catalogue opens unchanged. |
| IF2: lifecycle and automatic ingestion | Reusable `src/listener.rs` lifecycle, desktop ownership, `src/sessionize.rs` work scheduling | Opt-in starts once; a second window/CLI collision is safe; idle/off is cheap; inbox becomes a session without collect/process clicks; stop and restart preserve durable data and release owned ports. |
| IF3: guided bridge | `ui/app.js`, existing five-theme UI/styles, `userscripts/omoi.user.js`, `userscripts/README.md`, status hello | Fresh and existing manager paths, denied permission, raw-file fallback and background-dialog recovery; first received event is distinct from first recorded session; legacy script still works. |
| IF4: resumable controls | UI capture status, automatic refresh, explicit background/autostart and profile-scope settings; scheduler integration where relevant | No stale UI writes; restart restores choices; persisted collection scope matches selection; Stop works after navigation/reconnect; no hidden task installation. |
| IF5: companion and native host | Proposed `extension/` plus Rust host entry point, shared capture core/adapters, protocol fixtures | Framing/size validation, origin allowlist, permission denial, cross-tab queue ownership, worker restart, app absence, catalogue binding, version mismatch and lost acknowledgements tested. Synthetic Edge journey records without token entry. |
| IF6: installer and upgrades | `packaging/`, `scripts/package_windows.py`, proposed installer definition/registration support | Clean per-user install, missing runtime, upgrade with old client, path repair, idempotent reinstall and uninstall preserve catalogue/settings. Store/signing work explicitly separate. |
| IF7: acceptance and handback | CI, setup documentation and measurements | Full matrix below, packaged journey and measured costs by process role; precise remaining real-device/store actions. No “complete” based solely on mocked UI. |

IF1 → IF2 → IF3 is the first usable slice; IF4 builds on it. IF5 can proceed once
IF1/IF2 contracts stabilize; IF6 needs the host identity/protocol contract. IF7 closes
the integrated flow. Cloud agents may make routine implementation choices and
continue through these packages without repeatedly asking to proceed. The original documentation commit implemented no production feature. The current implementation branch adds the source/tests described in the acceptance record.

## Acceptance matrix

| Scenario | Required observable outcome | Where it can be proved |
|---|---|---|
| Fresh app, no extension | History works; capture optional and clearly incomplete | Synthetic tests plus clean Windows profile |
| Denied capture/site permission | No capture, no silent retry prompt loop; resumable action | Mock contracts; actual Edge permission UI separately |
| Wrong/background tab during setup | No lost progress or false success; foreground retry works | Browser fixture; real manager regression where available |
| App/listener absent or port occupied | Bounded queue/retry; exact status; no unrelated process killed | Native/HTTP fixture and Windows integration |
| Duplicate setup, two windows/tabs | One owned service, no duplicate credit or lost queue writes | Concurrency tests and disposable browser |
| First playback | Durable event acknowledged; session visible automatically within target latency | Isolated connected journey, then owner device |
| Pause, ads, seek, rate/URL change | Preserve capture regressions and strict epoch rules | Existing Node fixtures, shared-core extension fixtures |
| Sleep/restart/worker suspension | Resume without counter reuse; visible disconnect; bounded retries | Simulated lifecycle plus Windows sleep/device check |
| Stop and re-enable | Capture stops across transports; queued-data policy explicit; new epoch on resume | App/host/client integration |
| DB busy/full or crash between enqueue and ack | Honest failure/backlog, no false ack, retries idempotent | Fault-injected synthetic storage |
| Old app/client or malformed host message | Useful version/rejection response; no unsafe schema rollback | Protocol and migration tests |
| Upgrade/reinstall/uninstall | Preserve chosen data/settings; repair/remove only owned registration | Disposable Windows user/profile/registry sandbox |
| Another device/profile | New local binding; no copied token or assumption of shared native host | Separate disposable homes/profiles |

Preserve existing inbox/log caps. Add bounded retention for setup status/heartbeats;
do not append a diagnostic record on every poll. Reuse `scripts/measure.py` and
`scripts/verify_playback.py`, and measure the desktop/browser/native-host process
tree separately. The baseline listener's 10 MB fixture gate remains; if architecture
requires a changed budget, provide measured justification rather than weakening the
check silently. Record cold launch, idle CPU, capture CPU, peak memory, disk/temp
growth, wake frequency and event-to-visible latency. Do not claim collector-only
memory as total GUI memory or a short idle sample as a zero-CPU guarantee.

Cloud-only proof can cover Rust/JS logic, synthetic HTTP/stdio, schema behavior and
packaging source. A Linux desktop stub cannot prove WebView2, Edge permission UX,
Windows host registration or real sleep/logon behavior. Use Windows CI/disposable
profiles for available integration and report remaining device checks explicitly.

## Existing implementation map

- Capture and retry: `userscripts/omoi.user.js`, `tests/userscript.test.cjs`.
- Listener/authentication: `src/listener.rs`, `src/listener/protect.ps1`, `tests/playback.rs`.
- Durable ingestion/accounting: `src/sessionize.rs`, `src/db/`, `src/dedupe.rs`, `tests/state_transitions.rs`.
- Desktop/HTTP ownership: `src/bin/omoi-desktop.rs`, `src/ui.rs`, `tests/ui.rs`.
- Screens and stale-response guards: `ui/app.js`, `ui/forms.css`, `tests/ui_state.py`.
- Scheduling: `src/scheduler.rs`, `src/scheduler/windows.ps1`, `tests/scheduler_state.py`.
- Packaging and integration: `packaging/`, `scripts/package_windows.py`, `scripts/verify_desktop.py`, `scripts/verify_ui.py`, `scripts/verify_manager.py`, `.github/workflows/ci.yml`.

Keep detailed personal pilot receipts local. Commit implementation contracts,
synthetic fixtures, instructions, reproducible measurements and concise validation.
When finished, update this plan's package status and the portable handoff so the
next agent can distinguish shipped behavior from proposed work.
