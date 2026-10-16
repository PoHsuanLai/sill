//! Which covers the art cache lets go of. Pure; the service lists the folder and removes what
//! this names.

use std::path::PathBuf;
use std::time::SystemTime;

/// One file in the art cache folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedCover {
    pub path: PathBuf,
    pub modified: SystemTime,
    pub len: u64,
}

/// How much the art cache may hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheLimits {
    /// The most files kept.
    pub files: usize,
    /// The most bytes kept, all files together.
    pub bytes: u64,
}

/// The files to remove so the rest fit `limits`: the newest are kept, then each older one while
/// it still fits. A file larger than the byte limit alone never fits. Newest first, a tie broken
/// by the path, so the same folder always gives the same answer.
pub fn evictions(mut covers: Vec<CachedCover>, limits: CacheLimits) -> Vec<PathBuf> {
    covers.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.path.cmp(&b.path))
    });
    let mut kept_bytes = 0_u64;
    let mut kept_files = 0_usize;
    let mut evicted = Vec::new();
    for cover in covers {
        let total = kept_bytes.saturating_add(cover.len);
        if kept_files < limits.files && total <= limits.bytes {
            kept_files += 1;
            kept_bytes = total;
        } else {
            evicted.push(cover.path);
        }
    }
    evicted
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::{CacheLimits, CachedCover, evictions};

    /// A cover named `name`, `age` seconds old, `len` bytes long.
    fn cover(name: &str, age: u64, len: u64) -> CachedCover {
        CachedCover {
            path: PathBuf::from(name),
            modified: UNIX_EPOCH + Duration::from_secs(1_000_000) - Duration::from_secs(age),
            len,
        }
    }

    #[test]
    fn the_oldest_covers_go_first_until_the_files_and_the_bytes_fit() {
        let limits = CacheLimits {
            files: 3,
            bytes: 100,
        };
        let cases: [(&str, Vec<CachedCover>, Vec<&str>); 6] = [
            ("empty", vec![], vec![]),
            (
                "within both limits",
                vec![cover("a", 3, 10), cover("b", 2, 10), cover("c", 1, 10)],
                vec![],
            ),
            (
                "one file too many: the oldest",
                vec![
                    cover("a", 4, 10),
                    cover("b", 3, 10),
                    cover("c", 2, 10),
                    cover("d", 1, 10),
                ],
                vec!["a"],
            ),
            (
                "too many bytes: the oldest until the rest fit",
                vec![cover("a", 3, 60), cover("b", 2, 60), cover("c", 1, 30)],
                vec!["a"],
            ),
            (
                "a file that alone is past the byte limit never stays",
                vec![cover("big", 1, 101), cover("a", 2, 10)],
                vec!["big"],
            ),
            (
                "the same age: by path, so the answer does not depend on listing order",
                vec![
                    cover("d", 1, 10),
                    cover("b", 1, 10),
                    cover("c", 1, 10),
                    cover("a", 1, 10),
                ],
                vec!["d"],
            ),
        ];
        for (name, covers, want) in cases {
            let mut got = evictions(covers, limits);
            got.sort();
            let want: Vec<PathBuf> = want.into_iter().map(PathBuf::from).collect();
            assert_eq!(got, want, "{name}");
        }
    }

    #[test]
    fn a_file_with_no_time_is_the_first_to_go() {
        let epoch = CachedCover {
            path: PathBuf::from("no-time"),
            modified: SystemTime::UNIX_EPOCH,
            len: 1,
        };
        let limits = CacheLimits {
            files: 1,
            bytes: 100,
        };
        assert_eq!(
            evictions(vec![epoch, cover("fresh", 1, 1)], limits),
            vec![PathBuf::from("no-time")]
        );
    }
}
