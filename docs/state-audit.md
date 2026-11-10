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
  Explicit native `retry:false` keeps the queue without minute-by-minute wakeups;
  user reconnection retries after fixing permission/consent/protocol problems.
- Twitch enqueue previously advanced its live identity before processing could
  reject a regressing timestamp/counter. A discarded stop could therefore split
  one session at midnight. Identity continuity now derives from the last accepted
  playback state and matching channel, after monotonic validation. The legacy
  live-key table is updated only for accepted processing and is not trusted as
  an input frontier. This also avoids trusting old enqueue-derived rows after
  upgrade. Switching away from a channel breaks same-producer adjacency, as the
  sessionizer already does; it does not preserve an unseen broadcast across an
  intervening different item.
- New queued bodies record their original timezone. Live identity is resolved
  during processing, never pinned at enqueue before a queued stop or channel
  transition can be validated. Explicit legacy migration alone retains the old
  assigned deletion alias, conservatively honoring that identity's cutoff.
- Schema 4 adds a minimal live Twitch frontier independent of item foreign keys.
  It is seeded only from accepted live Twitch playback state, never enqueue-derived
  keys. A valid suppressed live sample advances this control record without
  recreating any item, session or metadata. Continuous deleted backlogs therefore
  remain suppressed across midnight, multiple batches and restart. Regressing
  frames cannot advance it; genuine stops and item/channel transitions end
  continuity. The existing five-minute adjacency rule still defines continuity.
  Only live Twitch identity/producer/sequence/time/counter/stopped control data
  survives deletion. Non-live playback does not gain retained viewing detail;
  its ordinary playback state still cascades with the item. These live control
  records, like deletion tombstones, remain until catalogue removal.

## Schema compatibility

Back up the whole catalogue, stop all Omoi processes and capture writers before
running the new build. Startup's normal migration upgrades schema 3 to schema 4
transactionally. Existing schema-3 binaries cannot reopen the upgraded database;
restore the pre-upgrade backup to use one. Do not run an already-open old process
concurrently with the upgrade. This is an intentional compatibility change for
deletion safety, not the earlier feature's schema-3 compatibility promise.
Migration conflict/rollback and accepted-only frontier seeding are tested.

## Legacy queue compatibility

Old queued Twitch bodies do not record their original timezone and can already
contain an incorrectly pinned key. Accepted same-channel continuity remains
usable, but an ambiguous new identity is refused visibly; processing rolls back
without guessing the current timezone or modifying queued bytes/receipts.

Stop capture/writers and back up the catalogue before recovery. The owner must
choose the historical offset explicitly, for example:

`omoi db migrate-playback --utc-offset +08:00`

This is a read-only preview of at most 256 raw rows, showing event IDs, timestamps,
old keys and proposed keys. Replace the example offset with the historical one;
page with `--after-id` using `next_after_id` until it stops advancing. Mixed DST or
travel offsets must be reviewed and partitioned, not assigned one assumed value.
Apply only an explicitly selected set of IDs that shared the reviewed offset:

`omoi db migrate-playback --utc-offset +08:00 --event-id producer-17 --apply`

Repeat `--event-id` for at most 256 distinct IDs. There is no apply-all default.
Selected-ID migration is transactional and idempotent for the same offset; missing
or duplicated IDs, a different previously assigned offset, invalid bodies/channel
or size violations roll back. The old deletion identity and receipts remain.
Resume collection after all required groups are resolved. If the original offset
cannot be established, retain the backup and blocked data rather than guess.
There is no automatic timezone inference or data repair by this
PR. Concurrent older application versions must not process the new raw format:
they ignore the extra context and do not enforce these protections. Existing
inbox bounds still apply, so stop new capture rather than let a blocked queue
grow until its normal eviction policy applies.

## Evidence

The actual PR worker regression failed on the pinned code: a rejected hello left
one queued event and no retry. The replacement passes all 11 companion tests,
including three rejected alarms followed by recovery without a new tab message.

An existing earlier Linux application binary reproduced the Twitch defect via
actual HTTP ingestion and separate collector processes: the synthetic regressing
stop produced two items with 20 and 0 seconds, instead of one 40-second session.
That binary is not a verified build of the PR head and is only baseline evidence.
The real-listener verification script now asserts immediate and batched processing
across midnight, deletion after enqueue and explicit migration through the CLI.
Rust regressions cover a poisoned legacy frontier, exact old serialized bodies,
three historical offsets, mixed-offset selection/preview, receipt preservation,
rollback, migration deletion, enqueue-all/batch/delete/reopen transitions,
long suppressed backlogs, rejected stops, valid transitions and non-live retention.

Rust formatting and Python syntax pass locally. Full local rebuilding is blocked
by exhausted storage; exact-head Windows/Linux CI must run the Rust suite and
real-listener journey. Native Edge/extension/host permission, physical sleep,
long-duration device tests, signing and store publication remain separate gates.
No Wine run, merge, signed installer or release is claimed. Existing private
artifact labels in historical receipts do not make a public repository private.
