//! `sill --write-selectors DIR`: the shell's public style parts as data, so Settings can check a
//! person's stylesheet against them without linking sill (CONVENTIONS.md section 16). The table
//! lists only the parts a stylesheet may rely on; every other `sill-` class is internal.

use std::io;
use std::path::{Path, PathBuf};

use ds::selectors::{AppSelectors, ComponentSelectors};
use ds_settings::PublishedParts;
use sill_settings::APP;

/// The parts of the shell a stylesheet may name.
pub const SILL_SELECTORS: AppSelectors = AppSelectors {
    app: "sill",
    components: &[
        ComponentSelectors {
            component: "Bar",
            root: "sill-bar",
            parts: &["group"],
        },
        ComponentSelectors {
            component: "Dock",
            root: "sill-dock",
            parts: &["hit"],
        },
        ComponentSelectors {
            component: "Switcher",
            root: "sill-switcher",
            parts: &["panel"],
        },
        ComponentSelectors {
            component: "Control Center",
            root: "sill-cc",
            parts: &["media", "detail"],
        },
    ],
    renames: &[],
};

/// The table as data, the file Settings reads.
pub fn parts() -> PublishedParts {
    PublishedParts::new(APP.0, SILL_SELECTORS.classes())
}

/// Write `sill.toml` into `dir` (made if it is missing) and return its path.
pub fn write_to(dir: &Path) -> io::Result<PathBuf> {
    parts().write_to(dir)
}

#[cfg(test)]
mod tests {
    use ds_settings::discover_parts;

    use super::{SILL_SELECTORS, write_to};

    #[test]
    fn every_root_carries_the_shells_prefix() {
        assert_eq!(SILL_SELECTORS.misnamed_roots(), Vec::<&str>::new());
    }

    /// The file lands in the directory it was given (a scratch one) and the reader Settings uses
    /// gives back every class the table publishes.
    #[test]
    fn the_written_file_is_read_back_by_the_parts_reader() {
        let dir = std::env::temp_dir().join(format!("sill-selectors-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let written = write_to(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        assert_eq!(written, dir.join("sill.toml"));
        let found = discover_parts(std::slice::from_ref(&dir));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].app, "sill");
        assert_eq!(found[0].classes, SILL_SELECTORS.classes());
        assert!(
            found[0]
                .classes
                .iter()
                .any(|class| class == "sill-bar-group")
        );
    }
}
