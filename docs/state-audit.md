# Playback state review

This follow-up is scoped to the existing guided-setup PR, based on
`eb9113cdc64224ff6c35ce02abd319fb4cd651a8`. It preserves the application's existing
themes and excludes unrelated, unpushed installer/release-hardening work.
Only public-repository code, tests and synthetic validation notes are included.

## Corrections

- The companion's one-shot retry alarm previously disappeared when native hello
  returned a temporary rejection instead of throwing. Flush now leaves a future
  alarm whenever an enabled durable queue remains, including that early return.
  No new page sample is needed to restart delivery. Existing permission, consent,
  catalogue-binding, queue limits and new-sample checks remain unchanged.
- Twitch enqueue previously advanced its live identity before processing could
  reject a regressing timestamp/counter. A discarded stop could therefore split
  one session at midnight. Identity continuity now derives from the last accepted
  playback state and matching channel, after monotonic validation. The legacy
  live-key table is updated only for accepted processing and is not trusted as
  an input frontier. This also avoids trusting old enqueue-derived rows after
  upgrade. Switching away from a channel breaks same-producer adjacency, as the
  sessionizer already does; it does not preserve an unseen broadcast across an
  intervening different item.

## Evidence

The actual PR worker regression failed on the pinned code: a rejected hello left
one queued event and no retry. The replacement passes all 10 companion tests,
including three rejected alarms followed by recovery without a new tab message.

An existing earlier Linux application binary reproduced the Twitch defect via
actual HTTP ingestion and separate collector processes: the synthetic regressing
stop produced two items with 20 and 0 seconds, instead of one 40-second session.
That binary is not a verified build of the PR head and is only baseline evidence.
The real-listener verification script now asserts immediate and batched processing
across midnight; Rust regressions also cover a poisoned legacy enqueue frontier.

Rust formatting and Python syntax pass locally. Full local rebuilding is blocked
by exhausted storage; exact-head Windows/Linux CI must run the Rust suite and
real-listener journey. Native Edge/extension/host permission, physical sleep,
long-duration device tests, signing and store publication remain separate gates.
No Wine run, merge, signed installer or release is claimed. Existing private
artifact labels in historical receipts do not make a public repository private.
