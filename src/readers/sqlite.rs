use super::{snapshot::Snapshot, Batch, Browser, Cursor, HistoryReader, RawVisit, SourceId};
use crate::{Error, Result};
use rusqlite::{Connection, OpenFlags};
pub struct SqliteReader {
    conn: Connection,
    id: SourceId,
    browser: Browser,
    sql: String,
    _snapshot: Snapshot,
}
impl SqliteReader {
    pub fn null_timestamp_rows(&self) -> Result<u64> {
        let (table, time) = if self.browser == Browser::Firefox {
            ("moz_historyvisits", "visit_date")
        } else {
            ("visits", "visit_time")
        };
        Ok(self.conn.query_row(
            &format!("SELECT count(*) FROM {table} WHERE {time} IS NULL"),
            [],
            |r| r.get(0),
        )?)
    }
    pub fn open(id: SourceId, browser: Browser, snapshot: Snapshot) -> Result<Self> {
        let conn = Connection::open_with_flags(&snapshot.path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
            r.get::<_, i64>(0)
        })?;
        conn.pragma_update(None, "query_only", true)?;
        let check: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if check != "ok" {
            return Err(Error::Schema("invalid browser snapshot".into()));
        }
        let bound = |col: &str, max: usize| {
            format!("CASE WHEN length(CAST({col} AS BLOB))<={max} THEN {col} ELSE NULL END")
        };
        let sql = if browser == Browser::Firefox {
            format!("SELECT v.id,v.visit_date,{}, {}, NULL FROM moz_historyvisits v JOIN moz_places u ON u.id=v.place_id WHERE (v.visit_date,v.id)>(?1,?2) ORDER BY v.visit_date,v.id LIMIT ?3",bound("u.url",8192),bound("u.title",1024))
        } else {
            let mut stmt = conn.prepare("PRAGMA table_info(visits)")?;
            let cols = stmt
                .query_map([], |r| r.get::<_, String>(1))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let duration = if cols.iter().any(|s| s == "visit_duration") {
                "v.visit_duration"
            } else {
                "NULL"
            };
            format!("SELECT v.id,v.visit_time,{}, {}, {duration} FROM visits v JOIN urls u ON u.id=v.url WHERE (v.visit_time,v.id)>(?1,?2) ORDER BY v.visit_time,v.id LIMIT ?3",bound("u.url",8192),bound("u.title",1024))
        };
        conn.prepare(&sql)?;
        Ok(Self {
            conn,
            id,
            browser,
            sql,
            _snapshot: snapshot,
        })
    }
    pub fn reset_if_replaced(&self, cursor: &mut Cursor) -> Result<()> {
        let (table, time) = if self.browser == Browser::Firefox {
            ("moz_historyvisits", "visit_date")
        } else {
            ("visits", "visit_time")
        };
        let max=self.conn.query_row(&format!("SELECT {time},id FROM {table} WHERE {time} IS NOT NULL ORDER BY {time} DESC,id DESC LIMIT 1"),[],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?)));
        match max {
            Ok(tuple) if tuple < (cursor.last_native_us, cursor.last_visit_id) => {
                cursor.last_native_us = i64::MIN;
                cursor.last_visit_id = i64::MIN;
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                cursor.last_native_us = i64::MIN;
                cursor.last_visit_id = i64::MIN;
            }
            Err(e) => return Err(e.into()),
            _ => (),
        }
        Ok(())
    }
}
impl HistoryReader for SqliteReader {
    fn id(&self) -> &SourceId {
        &self.id
    }
    fn read_since(&mut self, cursor: &Cursor, limit: usize) -> Result<Batch> {
        if !(1..=2048).contains(&limit) {
            return Err(Error::Config("invalid batch limit".into()));
        }
        let mut stmt = self.conn.prepare(&self.sql)?;
        let mut rows = stmt.query(rusqlite::params![
            cursor.last_native_us,
            cursor.last_visit_id,
            limit as i64
        ])?;
        let (mut visits, mut next, mut scanned) = (vec![], cursor.clone(), 0);
        while let Some(row) = rows.next()? {
            scanned += 1;
            let id: i64 = row.get(0)?;
            let native: i64 = row.get(1)?;
            next.last_native_us = native;
            next.last_visit_id = id;
            let Some(url) = row.get::<_, Option<String>>(2).ok().flatten() else {
                continue;
            };
            let unix = if self.browser == Browser::Firefox {
                Some(native)
            } else {
                native.checked_sub(11644473600000000)
            };
            let Some(unix_us) = unix
                .filter(|t| chrono::DateTime::from_timestamp(t.div_euclid(1000000), 0).is_some())
            else {
                continue;
            };
            visits.push(RawVisit {
                source: self.id.clone(),
                browser: self.browser,
                visit_id: id,
                native_us: native,
                unix_us,
                url,
                title: row.get(3).ok().flatten(),
                approx_duration_s: row
                    .get::<_, Option<i64>>(4)
                    .ok()
                    .flatten()
                    .map(|v| v.max(0) / 1000000),
            });
        }
        Ok(Batch {
            scanned,
            visits,
            next,
            exhausted: scanned < limit,
        })
    }
}
