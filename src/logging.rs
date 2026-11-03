//! Bounded, counter-only JSONL logs. No user-controlled strings enter a record.
use crate::{collect::CollectReport, config::Config};
use std::{io::Write, path::Path};
pub fn record(
    root: &Path,
    config: &Config,
    report: Option<&CollectReport>,
    exit: u8,
) -> std::io::Result<()> {
    std::fs::create_dir_all(root)?;
    let lock_path = root.join("logging.lock");
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    lock.try_lock().map_err(std::io::Error::from)?;
    // Keep this inode and handle for the critical section. The OS releases the
    // lock on process death; unlinking it would let a second inode bypass it.
    let record = serde_json::json!({"at":chrono::Utc::now().timestamp(),"operation":"collect","exit":exit,"profiles_ok":report.map(|r|r.profiles_ok),"profiles_failed":report.map(|r|r.profiles_failed),"rows_read":report.map(|r|r.rows_read),"rows_rejected":report.map(|r|r.rows_rejected),"views_inserted":report.map(|r|r.views_inserted),"warning_count":report.map(|r|r.warnings.len())});
    let mut bytes = serde_json::to_vec(&record)?;
    bytes.push(b'\n');
    if bytes.len() > 4096 || bytes.len() as u64 > config.log_file_bytes {
        return Err(std::io::Error::other("log record too large"));
    }
    let active = root.join(format!("collect-{}.log", std::process::id()));
    let mut files = vec![];
    for e in std::fs::read_dir(root)? {
        let e = e?;
        let n = e.file_name();
        let n = n.to_string_lossy();
        if n.starts_with("collect-") && n.ends_with(".log") && e.file_type()?.is_file() {
            let m = e.metadata()?;
            files.push((e.path(), m.len(), m.modified()?));
        }
    }
    if files
        .iter()
        .any(|(p, len, _)| p == &active && len + bytes.len() as u64 > config.log_file_bytes)
    {
        std::fs::remove_file(&active)?;
        files.retain(|(p, _, _)| p != &active);
    }
    files.sort_by_key(|(_, _, t)| *t);
    while files.len() + usize::from(!files.iter().any(|(p, _, _)| p == &active)) > config.log_files
        || files.iter().map(|(_, s, _)| s).sum::<u64>() + bytes.len() as u64
            > config.log_file_bytes * config.log_files as u64
    {
        if files.is_empty() {
            return Err(std::io::Error::other("log cap unavailable"));
        }
        let (p, _, _) = files.remove(0);
        std::fs::remove_file(p)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(active)?;
    f.write_all(&bytes)?;
    f.flush()
}
