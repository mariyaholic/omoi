CREATE TABLE item_sequence(singleton INTEGER PRIMARY KEY CHECK(singleton=1), last_id INTEGER NOT NULL);
INSERT INTO item_sequence SELECT 1,coalesce(max(id),0) FROM items;
CREATE TRIGGER items_identity AFTER INSERT ON items BEGIN
 UPDATE item_sequence SET last_id=max(last_id,new.id) WHERE singleton=1;
END;
CREATE TABLE deleted_items(platform TEXT NOT NULL, external_id TEXT NOT NULL, through INTEGER NOT NULL, PRIMARY KEY(platform,external_id)) WITHOUT ROWID;
ALTER TABLE playback_live_keys ADD COLUMN last_seq INTEGER NOT NULL DEFAULT -1;
ALTER TABLE playback_live_keys ADD COLUMN last_at_ms INTEGER NOT NULL DEFAULT 0;
ALTER TABLE playback_live_keys ADD COLUMN stopped INTEGER NOT NULL DEFAULT 0;
