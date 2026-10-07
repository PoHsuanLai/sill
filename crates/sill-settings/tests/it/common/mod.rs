//! Shared by the integration tests: a temp directory (no `tempfile`: not in the pinned block)
//! and the design doc's key tables.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// A fresh directory under the system temp dir, removed on drop.
#[derive(Debug)]
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("sill-settings-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        TempDir(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// quire's `design/22-SETTINGS.md`, a sibling checkout of this repository.
pub fn doc() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../quire/design/22-SETTINGS.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The `Key` column of every data row between `heading` and `next_heading`, in file order. A
/// data row is `| \`dotted.key\` | ... |`; the header and separator rows do not start their
/// first cell with a backtick. (The same parse as ds-settings' `tests/keys.rs`.)
pub fn keys_in_section(doc: &str, heading: &str, next_heading: &str) -> Vec<String> {
    let start = doc
        .find(heading)
        .unwrap_or_else(|| panic!("{heading:?} not found in 22-SETTINGS.md"));
    let after = &doc[start..];
    let end = after
        .find(next_heading)
        .unwrap_or_else(|| panic!("{next_heading:?} not found after {heading:?}"));
    after[..end]
        .lines()
        .filter_map(|line| {
            let mut cells = line.split('|').map(str::trim);
            let first_cell = cells.nth(1)?;
            let key = first_cell.strip_prefix('`')?.strip_suffix('`')?;
            // A retiring key has no field on purpose (its row says why).
            let status = line
                .trim_end_matches('|')
                .rsplit('|')
                .next()
                .unwrap_or("")
                .trim();
            (!status.starts_with("RETIRING")).then(|| key.to_owned())
        })
        .collect()
}
