use super::{FileStamp, Fingerprint};
use crate::{Error, Result};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::UNIX_EPOCH,
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub fn io(context: &'static str, source: std::io::Error) -> Error {
    Error::Io { context, source }
}
pub struct TempDir(pub PathBuf);
impl TempDir {
    pub fn create(root: &Path) -> Result<Self> {
        std::fs::create_dir_all(root).map_err(|e| io("temporary root", e))?;
        for _ in 0..100 {
            let p = root.join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut b = std::fs::DirBuilder::new();
            b.recursive(false);
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                b.mode(0o700);
            }
            match b.create(&p) {
                Ok(()) => return Ok(Self(p)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(io("temporary directory", e)),
            }
        }
        Err(Error::Config("temporary directory collision".into()))
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
pub fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(suffix);
    s.into()
}
fn stamp(path: &Path) -> Result<FileStamp> {
    match std::fs::metadata(path) {
        Ok(m) => {
            let t = m.modified().map_err(|e| io("file time", e))?;
            let ns = match t.duration_since(UNIX_EPOCH) {
                Ok(d) => d.as_nanos() as i128,
                Err(e) => -(e.duration().as_nanos() as i128),
            };
            Ok(FileStamp {
                present: true,
                len: m.len(),
                mtime_ns: ns,
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(FileStamp {
            present: false,
            len: 0,
            mtime_ns: 0,
        }),
        Err(e) => Err(io("file stamp", e)),
    }
}
pub fn fingerprint(path: &Path) -> Result<Fingerprint> {
    Ok(Fingerprint {
        main: stamp(path)?,
        wal: stamp(&sidecar(path, "-wal"))?,
        journal: stamp(&sidecar(path, "-journal"))?,
    })
}
pub struct Snapshot {
    pub dir: TempDir,
    pub path: PathBuf,
    pub fingerprint: Fingerprint,
}
pub fn copy(path: &Path, root: &Path, retries: usize) -> Result<Snapshot> {
    copy_observed(path, root, retries, || {})
}
/// Filesystem race injection seam; the observer runs after each copy, before restat.
#[doc(hidden)]
pub fn copy_observed(
    path: &Path,
    root: &Path,
    retries: usize,
    mut observer: impl FnMut(),
) -> Result<Snapshot> {
    for attempt in 0..retries {
        let before = fingerprint(path)?;
        let dir = TempDir::create(root)?;
        let target = dir.0.join("History");
        let copied = (|| -> Result<()> {
            std::fs::copy(path, &target).map_err(|e| io("history copy", e))?;
            for (suffix, present) in [
                ("-wal", before.wal.present),
                ("-journal", before.journal.present),
            ] {
                if present {
                    std::fs::copy(sidecar(path, suffix), sidecar(&target, suffix))
                        .map_err(|e| io("sidecar copy", e))?;
                }
            }
            Ok(())
        })();
        let copied = match copied {
            Err(Error::Io { context, source })
                if matches!(source.raw_os_error(), Some(28 | 112)) =>
            {
                return Err(Error::Io { context, source })
            }
            other => other,
        };
        observer();
        if copied.is_ok() && fingerprint(path)? == before {
            return Ok(Snapshot {
                dir,
                path: target,
                fingerprint: before,
            });
        }
        if attempt + 1 < retries {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
    Err(Error::Source {
        source_id: "browser".into(),
        reason: "snapshot unavailable or unstable".into(),
    })
}
