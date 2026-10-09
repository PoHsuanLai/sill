//! The include guard of a crate's one integration test executable (`tests/it/main.rs`): a file
//! nobody listed there compiles to nothing and its tests silently never run.

use std::path::{Path, PathBuf};

/// Fails when a `*.rs` file in the crate's `tests/it` is not declared by `mod <name>;` in
/// `main.rs`, or a directory with a `mod.rs` is declared by no file there. Pass
/// `env!("CARGO_MANIFEST_DIR")`.
pub fn assert_all_modules_included(manifest_dir: &str) {
    let dir = Path::new(manifest_dir).join("tests").join("it");
    let main = read(&dir.join("main.rs"));
    let everywhere = sources(&dir)
        .iter()
        .map(|path| read(path))
        .collect::<Vec<_>>()
        .join("\n");
    let missing: Vec<String> = entries(&dir)
        .into_iter()
        .filter_map(|path| module_of(&path))
        .filter(|(name, is_dir)| {
            let declared = if *is_dir { &everywhere } else { &main };
            !declared.contains(&format!("mod {name};"))
        })
        .map(|(name, _)| name)
        .collect();
    assert!(
        missing.is_empty(),
        "tests/it holds modules that nothing includes: {}",
        missing.join(", ")
    );
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

fn entries(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("listing {}: {e}", dir.display()))
        .flatten()
        .map(|entry| entry.path())
        .collect()
}

/// Every `.rs` file under `dir`.
fn sources(dir: &Path) -> Vec<PathBuf> {
    entries(dir)
        .into_iter()
        .flat_map(|path| match path.is_dir() {
            true => sources(&path),
            false => vec![path],
        })
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect()
}

/// The module a top-level entry stands for: a `.rs` file other than `main.rs`, or a directory
/// with a `mod.rs` (a directory without one holds `#[path]` files, which name themselves).
fn module_of(path: &Path) -> Option<(String, bool)> {
    let stem = path.file_stem()?.to_str()?.to_owned();
    match path.is_dir() {
        true => path.join("mod.rs").is_file().then_some((stem, true)),
        false => (path.extension()? == "rs" && stem != "main").then_some((stem, false)),
    }
}
