CREATE TABLE metadata_fields(item_id INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE, field TEXT NOT NULL, source TEXT NOT NULL, PRIMARY KEY(item_id,field)) WITHOUT ROWID;
CREATE TABLE playback_receipts(event_id TEXT PRIMARY KEY NOT NULL, producer TEXT NOT NULL, seq INTEGER NOT NULL, UNIQUE(producer,seq)) WITHOUT ROWID;
CREATE TABLE playback_live_keys(producer TEXT NOT NULL, channel TEXT NOT NULL, external_id TEXT NOT NULL, PRIMARY KEY(producer,channel)) WITHOUT ROWID;
CREATE TABLE playback_state(producer TEXT PRIMARY KEY NOT NULL, seq INTEGER NOT NULL, item_id INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE, session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE, at_ms INTEGER NOT NULL, counter_ms INTEGER NOT NULL, played_ms INTEGER NOT NULL, stopped INTEGER NOT NULL) WITHOUT ROWID;
CREATE TABLE playback_intervals(id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE, start_ms INTEGER NOT NULL, end_ms INTEGER NOT NULL, played_ms INTEGER NOT NULL);
CREATE INDEX playback_intervals_session ON playback_intervals(session_id);
CREATE TABLE playback_stats(key TEXT PRIMARY KEY NOT NULL, value INTEGER NOT NULL) WITHOUT ROWID;
INSERT INTO playback_stats VALUES('dropped',0),('late',0);
