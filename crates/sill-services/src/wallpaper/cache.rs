//! The cache's file naming: pure, so a size change or a new source is a new name and a stale
//! file is never reused.

use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// The cache directory's name inside `$XDG_CACHE_HOME/sill`.
pub const CACHE_DIR: &str = "wallpaper";

/// `<hash>-<w>x<h>.png`, the hash as 16 lowercase hex digits.
pub fn cache_file_name(hash: u64, width: u32, height: u32) -> String {
    format!("{hash:016x}-{width}x{height}.png")
}

/// FNV-1a's offset basis and prime (64-bit): a fixed, deterministic hash chosen over
/// `DefaultHasher` precisely because it is *not* randomised per process — a cache name has to
/// be the same after every restart for the cache to do anything.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// One FNV-1a step per byte.
fn fnv1a(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    })
}

/// A stable hash of the source file's path, size and modification time, so an edited image
/// at the same path is a new cache entry.
pub fn source_hash(path: &Path, len: u64, modified_unix_nanos: u128) -> u64 {
    let hash = fnv1a(FNV_OFFSET_BASIS, path.as_os_str().as_bytes());
    let hash = fnv1a(hash, &len.to_le_bytes());
    fnv1a(hash, &modified_unix_nanos.to_le_bytes())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{cache_file_name, source_hash};

    #[test]
    fn names_carry_hash_and_size() {
        const CASES: &[(u64, u32, u32, &str)] = &[
            (0, 1920, 1080, "0000000000000000-1920x1080.png"),
            (0xdead_beef, 3840, 2160, "00000000deadbeef-3840x2160.png"),
            (u64::MAX, 1, 1, "ffffffffffffffff-1x1.png"),
        ];
        for &(hash, w, h, want) in CASES {
            assert_eq!(cache_file_name(hash, w, h), want);
        }
    }

    #[test]
    fn the_same_input_always_hashes_the_same() {
        let a = source_hash(Path::new("/usr/share/backgrounds/default.png"), 12_345, 999);
        let b = source_hash(Path::new("/usr/share/backgrounds/default.png"), 12_345, 999);
        assert_eq!(a, b);
    }

    #[test]
    fn each_input_changes_the_hash_on_its_own() {
        let base = source_hash(Path::new("/a/b.png"), 100, 1);
        let cases: &[(&str, u64)] = &[
            (
                "a different path",
                source_hash(Path::new("/a/c.png"), 100, 1),
            ),
            (
                "a different length",
                source_hash(Path::new("/a/b.png"), 101, 1),
            ),
            (
                "a different modification time",
                source_hash(Path::new("/a/b.png"), 100, 2),
            ),
        ];
        for (name, other) in cases {
            assert_ne!(base, *other, "{name}");
        }
    }

    #[test]
    fn a_known_vector_matches_hand_computed_fnv1a() {
        // FNV-1a over b"sill" alone (len 0, mtime 0 contribute nothing distinctive here, so
        // this pins the byte-feeding order against the published FNV-1a algorithm): computed
        // independently of this module, from the FNV-1a definition (offset basis
        // 0xcbf29ce484222325, prime 0x100000001b3), one XOR-then-multiply per byte of "sill".
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in b"sill" {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        // `source_hash` also feeds an 8-byte length and a 16-byte modification time; with both
        // zero, every fed byte is `0`, each step `hash = (hash ^ 0).wrapping_mul(PRIME)`.
        let mut want = hash;
        for _ in 0..(8 + 16) {
            want = want.wrapping_mul(0x0000_0100_0000_01b3);
        }
        assert_eq!(source_hash(Path::new("sill"), 0, 0), want);
    }
}
