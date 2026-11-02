use crate::{Error, Result};
use rusqlite::Connection;
use serde::Serialize;
use std::io::Write;
#[derive(Serialize)]
struct Row {
    view_id: i64,
    item_id: i64,
    platform: String,
    external_id: String,
    title: Option<String>,
    channel: Option<String>,
    channel_id: Option<String>,
    started_at: i64,
    source: String,
    device: String,
    approx_duration_s: Option<i64>,
}
pub fn export(conn: &Connection, json: bool, writer: &mut dyn Write) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    let mut stmt=tx.prepare("SELECT v.id,i.id,i.platform,i.external_id,i.title,i.channel,i.channel_id,v.started_at,v.source,v.device,v.approx_duration_s FROM views v JOIN items i ON i.id=v.item_id ORDER BY v.started_at,v.id")?;
    let rows = stmt.query_map([], |r| {
        Ok(Row {
            view_id: r.get(0)?,
            item_id: r.get(1)?,
            platform: r.get(2)?,
            external_id: r.get(3)?,
            title: r.get(4)?,
            channel: r.get(5)?,
            channel_id: r.get(6)?,
            started_at: r.get(7)?,
            source: r.get(8)?,
            device: r.get(9)?,
            approx_duration_s: r.get(10)?,
        })
    })?;
    if json {
        writer.write_all(b"[").map_err(err)?;
        let mut first = true;
        for row in rows {
            if !first {
                writer.write_all(b",").map_err(err)?;
            }
            first = false;
            serde_json::to_writer(&mut *writer, &row?).map_err(|e| {
                if e.io_error_kind() == Some(std::io::ErrorKind::BrokenPipe) {
                    err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
                } else {
                    Error::Export(e.to_string())
                }
            })?;
        }
        writer.write_all(b"]\n").map_err(err)?;
    } else {
        let mut csv = csv::WriterBuilder::new()
            .terminator(csv::Terminator::CRLF)
            .has_headers(false)
            .from_writer(writer);
        csv.write_record([
            "view_id",
            "item_id",
            "platform",
            "external_id",
            "title",
            "channel",
            "channel_id",
            "started_at",
            "source",
            "device",
            "approx_duration_s",
        ])
        .map_err(|e| Error::Export(e.to_string()))?;
        for row in rows {
            csv.serialize(row?)
                .map_err(|e| Error::Export(e.to_string()))?;
        }
        csv.flush().map_err(err)?;
    }
    drop(stmt);
    tx.commit()?;
    Ok(())
}
fn err(source: std::io::Error) -> Error {
    Error::Io {
        context: "export",
        source,
    }
}
