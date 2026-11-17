-- Validated live Twitch continuity survives item deletion so queued samples
-- still reach the original deletion tombstone. Never seed from enqueue keys.
CREATE TABLE playback_frontier(
 producer TEXT PRIMARY KEY NOT NULL,
 seq INTEGER NOT NULL,
 platform TEXT NOT NULL,
 external_id TEXT NOT NULL,
 at_ms INTEGER NOT NULL,
 counter_ms INTEGER NOT NULL,
 stopped INTEGER NOT NULL
) WITHOUT ROWID;
INSERT INTO playback_frontier
 SELECT p.producer,p.seq,i.platform,i.external_id,p.at_ms,p.counter_ms,p.stopped
 FROM playback_state p JOIN items i ON i.id=p.item_id
 WHERE i.platform='twitch' AND i.external_id LIKE 'live:%';
