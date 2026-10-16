//! The art cache folder (effects: the filesystem): bringing a local cover into it and keeping it
//! to its limits.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use ds_helpers::{NewFileMode, ReadCap, ReadError, WriteError, read_capped, write_atomic};
use sill_model::now_playing::{CacheLimits, CachedCover, evictions};

use super::ART_MAX_BYTES;

/// How many covers the cache keeps: the oldest go first.
const CACHE_FILES_MAX: usize = 64;

/// How many bytes of covers the cache keeps: a cover is usually some tens of kilobytes, so this
/// is only reached by a run of large ones.
const CACHE_BYTES_MAX: u64 = 128 * 1024 * 1024;

/// The cache's limits.
const LIMITS: CacheLimits = CacheLimits {
    files: CACHE_FILES_MAX,
    bytes: CACHE_BYTES_MAX,
};

/// Why a local cover was not copied.
#[derive(Debug, thiserror::Error)]
pub(super) enum CopyError {
    /// Not a regular file, larger than [`ART_MAX_BYTES`], or unreadable.
    #[error("cover not read: {0}")]
    Read(#[from] ReadError),
    /// The cache folder would not take it.
    #[error("cover not kept: {0}")]
    Write(#[from] WriteError),
    #[error("cover not kept: {0}")]
    Folder(#[from] std::io::Error),
}

/// Whether the local file `from` is now at `to`: read whole (a regular file of at most
/// [`ART_MAX_BYTES`]; a device, a pipe, a socket, a folder or a bigger file is refused, and a
/// pipe is never waited on), then written beside `to` and renamed into place, readable by the
/// person only, so a stopped copy never leaves half a cover that looks cached. Blocking.
pub(super) fn copy_cover(from: &Path, to: &Path) -> Result<(), CopyError> {
    let bytes = read_capped(from, ReadCap::bytes(ART_MAX_BYTES))?;
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir)?;
    }
    write_atomic(to, &bytes, NewFileMode::OwnerOnly)?;
    Ok(())
}

/// The cache folder held to its limits ([`CACHE_FILES_MAX`] files, [`CACHE_BYTES_MAX`] bytes):
/// the oldest files go first.
pub(super) async fn prune(dir: PathBuf) {
    let _ = tokio::task::spawn_blocking(move || prune_blocking(&dir, LIMITS)).await;
}

fn prune_blocking(dir: &Path, limits: CacheLimits) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let covers: Vec<CachedCover> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            meta.is_file().then(|| CachedCover {
                path: entry.path(),
                // A file with no readable time is the first to go.
                modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                len: meta.len(),
            })
        })
        .collect();
    for path in evictions(covers, limits) {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;
    use std::time::{Duration, SystemTime};

    use sill_model::now_playing::CacheLimits;
    use sill_testkit::TempDir;

    use super::{ART_MAX_BYTES, CopyError, copy_cover, prune_blocking};

    #[test]
    fn a_regular_cover_is_copied_into_the_cache_and_nothing_else_is() {
        let dir = TempDir::new("art-copy");
        let cache = dir.path().join("cache").join("art");
        let source = dir.path().join("cover.png");
        std::fs::write(&source, b"cover bytes").unwrap_or_else(|e| panic!("{e}"));

        let kept = cache.join("kept.png");
        copy_cover(&source, &kept).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            std::fs::read(&kept).ok().as_deref(),
            Some(&b"cover bytes"[..])
        );

        let big = dir.path().join("big.png");
        std::fs::File::create(&big)
            .and_then(|file| file.set_len(ART_MAX_BYTES + 1))
            .unwrap_or_else(|e| panic!("{e}"));
        let socket = dir.path().join("cover.sock");
        let _listener = UnixListener::bind(&socket).unwrap_or_else(|e| panic!("{e}"));
        let refused = [
            ("a folder", dir.path().to_path_buf()),
            ("a socket", socket),
            ("past the size limit", big),
            ("missing", dir.path().join("missing.png")),
        ];
        for (name, from) in refused {
            let to = cache.join(format!("{}.png", name.replace(' ', "-")));
            let result = copy_cover(&from, &to);
            assert!(
                matches!(result, Err(CopyError::Read(_))),
                "{name}: {result:?}"
            );
            assert!(!to.exists(), "{name}: something was kept");
        }
    }

    #[test]
    fn a_cover_at_the_size_limit_is_copied() {
        let dir = TempDir::new("art-copy-limit");
        let source = dir.path().join("cover.png");
        std::fs::File::create(&source)
            .and_then(|file| file.set_len(ART_MAX_BYTES))
            .unwrap_or_else(|e| panic!("{e}"));
        let kept = dir.path().join("cache").join("kept.png");
        copy_cover(&source, &kept).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            std::fs::metadata(&kept).map(|meta| meta.len()).ok(),
            Some(ART_MAX_BYTES)
        );
    }

    #[test]
    fn pruning_removes_the_oldest_files_and_keeps_the_newest() {
        let dir = TempDir::new("art-prune");
        let total = 7_u64;
        for n in 0..total {
            let path = dir.path().join(format!("{n}.png"));
            std::fs::write(&path, b"x").unwrap_or_else(|e| panic!("{e}"));
            let when = SystemTime::now() - Duration::from_secs((total - n) * 10);
            std::fs::File::options()
                .write(true)
                .open(&path)
                .and_then(|file| file.set_modified(when))
                .unwrap_or_else(|e| panic!("{e}"));
        }
        prune_blocking(
            dir.path(),
            CacheLimits {
                files: 4,
                bytes: 1_000,
            },
        );
        let mut left: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap_or_else(|e| panic!("{e}"))
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left, ["3.png", "4.png", "5.png", "6.png"]);
    }
}
