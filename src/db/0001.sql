CREATE TABLE items (
 id INTEGER PRIMARY KEY,
 platform TEXT NOT NULL CHECK(platform IN ('youtube','bilibili','netflix','twitch','prime','disney','crunchyroll','local','unknown')),
 external_id TEXT NOT NULL, title TEXT, channel TEXT, channel_id TEXT, series TEXT,
 season INTEGER, episode INTEGER, duration_s INTEGER CHECK(duration_s IS NULL OR duration_s >= 0),
 published_at INTEGER, kind TEXT, enriched_at INTEGER, metadata_rank INTEGER NOT NULL DEFAULT 0,
 UNIQUE(platform,external_id)
);
CREATE TABLE views (
 id INTEGER PRIMARY KEY, item_id INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE,
 started_at INTEGER NOT NULL, source TEXT NOT NULL, device TEXT NOT NULL DEFAULT 'pc',
 approx_duration_s INTEGER CHECK(approx_duration_s IS NULL OR approx_duration_s >= 0),
 UNIQUE(item_id,started_at,source)
);
CREATE INDEX views_item_time ON views(item_id,started_at);
CREATE INDEX views_time ON views(started_at,id);
CREATE TABLE sessions (
 id INTEGER PRIMARY KEY, item_id INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE,
 start INTEGER NOT NULL, end INTEGER NOT NULL CHECK(end >= start),
 played_s INTEGER NOT NULL CHECK(played_s >= 0 AND played_s <= end-start),
 max_position_s INTEGER CHECK(max_position_s IS NULL OR max_position_s >= 0), source TEXT NOT NULL
);
CREATE INDEX sessions_item_start ON sessions(item_id,start);
CREATE TABLE item_tags (
 item_id INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE, tag TEXT NOT NULL, source TEXT NOT NULL,
 PRIMARY KEY(item_id,tag)
) WITHOUT ROWID;
CREATE TABLE channel_tags (platform TEXT NOT NULL, channel TEXT NOT NULL, tag TEXT NOT NULL, PRIMARY KEY(platform,channel,tag)) WITHOUT ROWID;
CREATE TABLE checkpoints (source TEXT PRIMARY KEY NOT NULL, cursor TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE raw_events (id INTEGER PRIMARY KEY, at INTEGER NOT NULL, source TEXT NOT NULL, body TEXT NOT NULL CHECK(length(CAST(body AS BLOB)) <= 16384));
CREATE VIRTUAL TABLE items_fts USING fts5(title,channel,series,content='items',content_rowid='id');
CREATE TRIGGER items_ai AFTER INSERT ON items BEGIN
 INSERT INTO items_fts(rowid,title,channel,series) VALUES(new.id,new.title,new.channel,new.series);
END;
CREATE TRIGGER items_ad AFTER DELETE ON items BEGIN
 INSERT INTO items_fts(items_fts,rowid,title,channel,series) VALUES('delete',old.id,old.title,old.channel,old.series);
END;
CREATE TRIGGER items_au AFTER UPDATE OF title,channel,series ON items BEGIN
 INSERT INTO items_fts(items_fts,rowid,title,channel,series) VALUES('delete',old.id,old.title,old.channel,old.series);
 INSERT INTO items_fts(rowid,title,channel,series) VALUES(new.id,new.title,new.channel,new.series);
END;
