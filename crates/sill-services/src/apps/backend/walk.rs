//! Finding the `.desktop` files under the applications directories.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The extension a desktop entry has.
const EXTENSION: &str = "desktop";

/// Every `.desktop` file under `roots`, in order: a directory's own files by name, then its
/// subdirectories by name, each root before the next. A root or folder that cannot be read is
/// skipped. A folder is entered once however many links lead to it, so a link back up the tree
/// ends the walk instead of repeating it.
pub fn desktop_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut walk = Walk::default();
    for root in roots {
        walk.enter(root);
    }
    walk.found
}

/// The walk so far: the folders entered (by their real path) and the files met.
#[derive(Default)]
struct Walk {
    entered: BTreeSet<PathBuf>,
    found: Vec<PathBuf>,
}

impl Walk {
    fn enter(&mut self, dir: &Path) {
        let real = dir.canonicalize().unwrap_or_else(|_| dir.to_owned());
        if !self.entered.insert(real.clone()) {
            return;
        }
        let Ok(read) = std::fs::read_dir(&real) else {
            return;
        };
        let mut children: Vec<PathBuf> = read.filter_map(Result::ok).map(|e| e.path()).collect();
        children.sort_unstable();
        let (dirs, files): (Vec<PathBuf>, Vec<PathBuf>) =
            children.into_iter().partition(|child| child.is_dir());
        self.found
            .extend(files.into_iter().filter(|file| is_entry(file)));
        for sub in dirs {
            self.enter(&sub);
        }
    }
}

/// A file (a link to one counts) named `*.desktop`.
fn is_entry(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == EXTENSION) && path.is_file()
}

#[cfg(test)]
mod tests {
    use super::desktop_files;
    use std::path::Path;

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| panic!("{e}"));
        }
        std::fs::write(path, "").unwrap_or_else(|e| panic!("{e}"));
    }

    #[test]
    fn files_come_before_subfolders_and_a_link_back_up_ends_the_walk() {
        let scratch = sill_testkit::TempDir::new("apps-walk");
        let root = scratch.path().join("applications");
        touch(&root.join("b.desktop"));
        touch(&root.join("a.desktop"));
        touch(&root.join("notes.txt"));
        touch(&root.join("kde/c.desktop"));
        std::os::unix::fs::symlink(&root, root.join("kde/loop")).unwrap_or_else(|e| panic!("{e}"));

        let real = root.canonicalize().unwrap_or_else(|e| panic!("{e}"));
        let got = desktop_files(&[root, scratch.path().join("missing")]);
        let want = ["a.desktop", "b.desktop", "kde/c.desktop"].map(|name| real.join(name));
        assert_eq!(got, want);
    }
}
