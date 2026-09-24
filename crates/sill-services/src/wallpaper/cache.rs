//! The cache's file naming: pure, so a size change or a new source is a new name and a stale
//! file is never reused.

#![allow(unused_variables, dead_code)] // Freeze stubs

use std::path::Path;

/// The cache directory's name inside `$XDG_CACHE_HOME/sill`.
pub const CACHE_DIR: &str = "wallpaper";

/// `<hash>-<w>x<h>.png`, the hash as 16 lowercase hex digits.
pub fn cache_file_name(hash: u64, width: u32, height: u32) -> String {
    format!("{hash:016x}-{width}x{height}.png")
}

/// A stable hash of the source file's path, size and modification time, so an edited image
/// at the same path is a new cache entry.
pub fn source_hash(path: &Path, len: u64, modified_unix_nanos: u128) -> u64 {
    todo!("M2 wallpaper: FNV-1a over the three (stable across runs, unlike DefaultHasher)")
}

#[cfg(test)]
mod tests {
    use super::cache_file_name;

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
}
