//! Deterministic synthetic workload; never discovers installed browsers.
use clap::Parser;
use rusqlite::{params, Connection};
use std::path::PathBuf;
#[derive(Parser)]
struct Args {
    #[arg(long)]
    out: PathBuf,
    #[arg(long, default_value_t = 100000)]
    views: u64,
    #[arg(long, default_value_t = 40000)]
    items: u64,
    #[arg(long, default_value_t = 1)]
    seed: u64,
    #[arg(long)]
    long_titles: bool,
}
fn url(i: u64) -> String {
    match i % 8 {
        0 => format!("https://youtube.com/watch?v={i:011}"),
        1 => format!("https://netflix.com/watch/{}", i + 1),
        2 => format!("https://twitch.tv/videos/{}", i + 1),
        3 => format!("https://bilibili.com/video/BV{i:010}"),
        4 => format!("https://primevideo.com/detail/B{i:09}"),
        5 => format!("https://disneyplus.com/video/{i:012}"),
        6 => format!("https://crunchyroll.com/watch/{i:09}"),
        _ => format!("file:///C:/Synthetic/video-{i}.mp4"),
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a = Args::parse();
    if a.items == 0 || a.views < a.items || a.items > 100000000 || a.views > 10000000 {
        return Err("invalid fixture sizes".into());
    }
    std::fs::create_dir(&a.out)?;
    let root = a.out.canonicalize()?;
    let home = root.join("home");
    std::fs::create_dir(&home)?;
    let chrome = root.join("History");
    let firefox = root.join("places.sqlite");
    let mut c = Connection::open(&chrome)?;
    let mut f = Connection::open(&firefox)?;
    c.execute_batch("CREATE TABLE urls(id INTEGER PRIMARY KEY,url TEXT,title TEXT);CREATE TABLE visits(id INTEGER PRIMARY KEY,url INTEGER,visit_time INTEGER,visit_duration INTEGER);CREATE INDEX visit_time_idx ON visits(visit_time,id);")?;
    f.execute_batch("CREATE TABLE moz_places(id INTEGER PRIMARY KEY,url TEXT,title TEXT);CREATE TABLE moz_historyvisits(id INTEGER PRIMARY KEY,place_id INTEGER,visit_date INTEGER);CREATE INDEX visit_date_idx ON moz_historyvisits(visit_date,id);")?;
    let tc = c.transaction()?;
    let tf = f.transaction()?;
    for i in 0..a.items {
        let title = format!(
            "Synthetic item {i:010} {}",
            "x".repeat(if a.long_titles { 998 } else { 14 })
        );
        let u = format!(
            "{}{}fixture={}",
            url(i),
            if i % 8 == 0 { '&' } else { '?' },
            "x".repeat(50)
        );
        tc.execute(
            "INSERT INTO urls VALUES(?1,?2,?3)",
            params![i + 1, u, title],
        )?;
        tf.execute(
            "INSERT INTO moz_places VALUES(?1,?2,?3)",
            params![i + 1, u, title],
        )?;
    }
    for i in 0..a.views {
        let item = i % a.items + 1;
        let time = 1700000000000000i64 + (i as i64) * 3600 * 1000000 + (a.seed % 1000) as i64;
        tc.execute(
            "INSERT INTO visits VALUES(?1,?2,?3,10000000)",
            params![i + 1, item, time + 11644473600000000],
        )?;
        if i % 10 == 0 {
            tf.execute(
                "INSERT INTO moz_historyvisits VALUES(?1,?2,?3)",
                params![i + 1, item, time + 60000000],
            )?;
        }
    }
    tf.execute(
        "INSERT INTO moz_places VALUES(?1,'https://example.com/','Irrelevant')",
        [a.items + 1],
    )?;
    for i in 0..a.views / 10 {
        tf.execute(
            "INSERT INTO moz_historyvisits VALUES(?1,?2,?3)",
            params![
                a.views + i + 1,
                a.items + 1,
                1700000000000000i64 + (i as i64) * 3600000000
            ],
        )?;
    }
    tc.commit()?;
    tf.commit()?;
    let paths = serde_json::to_string(&vec![
        format!("chrome:{}", chrome.display()),
        format!("firefox:{}", firefox.display()),
    ])?;
    std::fs::write(
        home.join("config.toml"),
        format!("browsers=[\"chrome\",\"firefox\"]\nprofile_paths={paths}\n"),
    )?;
    println!(
        "{}",
        serde_json::json!({"home":home,"views":a.views,"items":a.items,"seed":a.seed,"source_rows":a.views+a.views.div_ceil(10)+a.views/10,"long_titles":a.long_titles})
    );
    Ok(())
}
