# Setup and transport contract (version 1)

Setup is one bounded JSON record in `checkpoints`, reserved key `omoi:setup:v1`.
Catalogue schema is 4 after the playback-state follow-up. Browser checkpoint keys use their existing source IDs. Schema 4 retains accepted producer continuity across item deletion; stop older processes and back up before upgrade. Ambiguous pre-upgrade Twitch queue dates require explicit reviewed recovery, not a guessed timezone. See [state audit and recovery](state-audit.md).
A future setup version fails closed. Choices/progress/profile scope persist;
process ownership and client liveness never come from stored booleans alone.
An OS-held per-home app lock proves foreground lifetime and releases on crash.

A setup mutation supplies the last generation. The immediate transaction rejects
stale changes and increments the setup generation. A separate capture epoch changes
only when capture choice/background consent changes; saving a setup step or source
scope never discards queued playback. Native events carry both the random,
non-secret catalogue binding and capture epoch (the wire field is named `generation`). Admission checks them in the same
transaction that inserts receipt+inbox, so a committed Stop rejects subsequent old
requests. Resuming consent also sets an event-time admission cutoff. Older queued HTTP samples are rejected, and HTTP producer counters receive a server-side consent epoch so an unmodified 0.1.1 client cannot retroactively credit stopped time. Already durable inbox entries are retained. Binding is not a network credential; it prevents accidental queue mixing.

HTTP remains loopback, exact Host, absent Origin, exact persistent token, POST JSON
only. `/event` remains compatible with 0.1.1. `/hello` takes
`{protocol:1,version:"0.2.0",permission:true}` with identical authentication. A hello
never creates a fake receipt/session. A single last hello overwrites prior status;
contact expires after 45 seconds. Current client compatibility is 0.2.x, minimum 0.2.0;
the native hello negotiates a protocol range including 1. App version, client version
and protocol are separate. There is no public credential endpoint or CORS exception.
The private UI's explicit POST copy action returns a credential only to its own
session; no URL, source artifact or status response contains it.

Native messaging uses little-endian uint32 lengths, UTF-8 JSON, and a 32 KiB maximum
frame. Zero, oversize, malformed and truncated frames end the process; stdout has
only frames. Host `omoi-host.json` beside the executable contains an absolute home
and at most two exact approved extension origins. The manifest and process arguments
both require the exact `chrome-extension://<32 a-p letters>/` origin. Windows' optional
`--parent-window=<digits>` is accepted separately. Stdio is a browser-owned pipe;
no network endpoint is added. The host has a 60-second idle deadline and bounded
one-frame channel; EOF ends it. Same-user local program/registry tampering is outside
this boundary: local OS file permissions and trusted install sources remain necessary.

Requests:

- `hello`: `protocol_min`, `protocol_max`, `version`, `permission`, nullable `binding`.
  Returns `ok`, protocol, app_version, binding and generation. A stored different
  binding requires explicit disconnect before reconnect. Consent and app lifetime
  (or separately opted-in background mode) must permit capture.
- `event`: binding, generation, event. A successful negotiated hello is required.
  `ok:true` acknowledges the durable receipt/inbox transaction, including duplicate
  delivery. Busy/full storage returns retryable failure without a false ack.
- `status`: bounded pending count and receipt/processing timestamps, no catalogue
  export or arbitrary commands. No request can enable consent.

The same enqueue and sessionize code serves HTTP and native ingress. Automatic
processing commits at most 256 events per transaction, retries at one-second
intervals, and leaves the inbox intact on failure. A managed writer uses a 250 ms
busy timeout and 256 KiB SQLite cache. Inbox 100,000-row/20 MB caps and existing
receipt/session/deletion semantics remain. Status generation counts committed
materialization for UI refresh. The UI polls every two seconds while visible;
changing pages or setup generations invalidates pending replies.

The packaged core is generated into userscript/extension bundles by
`scripts/build_capture.py`; `--check` catches drift. The MV3 worker serializes storage
writes from all tabs, caps admission at 128 pending handlers and its durable queue
at 1,024 events/4 MiB encoded UTF-8 JSON. Content adapters cap their awaiting-ack queues
at 64. Only matching top-level supported origins can submit capture. There is no
external/page-world message handler. Only the popup can connect/disconnect.
New samples require a current native status check; foreground-only samples made while the app is absent are not put into the durable queue. Previously accepted queue IDs survive retries and compatible restarts. Consent generation changes drop
old queued events with a visible counter and reset producer epochs. Native poison
playback events can be discarded explicitly; transport errors retain the head.
Retries use a one-minute browser alarm after failed transport, and an unused native
port closes after 30 seconds. Browser storage is restricted to trusted extension
contexts. Missing/denied permissions have no silent approval loop.

Registration uses HKCU Edge NativeMessagingHosts, distinct development/release names,
exact IDs and ownership markers. A moved installation repairs only a readable owned
same-home registration. Uninstall never deletes a different registered path or data.
No force-install policy, browser-profile edits or token copying is used by native
messaging. Portable registration is an explicit operation.

Primary platform references:
[Edge native messaging](https://learn.microsoft.com/en-us/microsoft-edge/extensions/developer-guide/native-messaging),
[packaged content scripts](https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts),
[MV3 worker lifetime](https://developer.chrome.com/docs/extensions/develop/concepts/service-workers/lifecycle).
These document the platform APIs, not proof that our Windows acceptance has passed.
