//! The settings provider against this machine's real installed schemas
//! (`ds_settings::schema::data_dirs`'s own directories, reimplemented in
//! `providers::settings::schema_file` — `schema_file.rs`'s module doc explains why this crate
//! cannot just call the quire function).
//!
//! CONVENTIONS.md "run it the way its user would": design/22-SETTINGS.md §9.2 says a developer
//! installs a schema locally with `cargo run -p <app> -- --write-schema <dir>`, and that CLI
//! path is exactly `sill_settings::schema().write_to(dir)` (`crates/sill/src/main.rs`) — so
//! this test calls that same function directly, rather than shelling out to the `sill` binary
//! itself, only because `crates/sill` also links `shell-host` (the Blitz/Wayland stack), which
//! this crate has no other reason to build; the write path this test exercises is identical
//! either way. `sill-settings` is a dev-dependency of this crate for this one test
//! (`Cargo.toml`'s comment); it is not linked into `sill-services` itself
//! (`scripts/check-boundary.sh` only inspects normal and build dependencies).
//!
//! Writes into this machine's real `$XDG_DATA_HOME/quire/settings` (or its XDG fallback).
//! Whatever quire's own schema installer would add there stays untouched; only the one file
//! this test writes is removed afterward, panic or not — the same shape as `tests/icons.rs`'s
//! real-`$HOME` guard.

use std::fs;
use std::path::PathBuf;

use sill_launcher::{Cancel, Provider, Query, VecSink};
use sill_services::providers::SettingsProvider;
use sill_services::providers::settings::schema_file::{data_dirs, parse_schema};

/// Removes the one file it names, on drop, panic or not.
struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
#[ignore = "writes sill's schema into this machine's real XDG settings directory"]
fn the_real_installed_schemas_are_indexed() {
    let dirs = data_dirs();
    eprintln!("schema discovery directories: {dirs:?}");
    let install_dir = dirs
        .first()
        .cloned()
        .expect("data_dirs always yields at least one directory");

    let written = sill_settings::schema()
        .write_to(&install_dir)
        .expect("sill's schema should write to this machine's real XDG data dir");
    let _cleanup = Cleanup(written.clone());

    // Cross-check: how many keys did sill's own schema write, independent of discovery, by
    // re-reading the file this test just wrote through this crate's own parser.
    let written_text = fs::read_to_string(&written).expect("the file it just wrote is readable");
    let written_schema = parse_schema(&written_text).expect("sill's own schema parses");
    let sill_key_count = written_schema.key.len();
    assert!(
        sill_key_count > 0,
        "sill's own schema should have at least one key"
    );

    let provider = SettingsProvider::new();
    eprintln!(
        "cosmic-settings on PATH: {}, {} COSMIC pages loaded",
        !provider.cosmic_pages.is_empty(),
        provider.cosmic_pages.len()
    );

    // A path-exact query for one of sill's own keys should find it: proof the just-written file
    // was actually discovered and indexed, not just present on disk.
    let sample_path = &written_schema.key[0].path;
    let cancel = Cancel::new();
    let query = Query::begin(sample_path.clone(), &cancel);
    let mut sink = VecSink::default();
    provider.query(&query, &mut sink);
    let found = sink
        .0
        .iter()
        .flat_map(|batch| &batch.items)
        .any(|item| item.id.0.ends_with(sample_path.as_str()));
    assert!(found, "expected to find {sample_path} after indexing");

    eprintln!(
        "sill's own schema: {sill_key_count} keys written and discovered at {}",
        written.display()
    );
    eprintln!(
        "note: quire ships no aggregating --write-schema binary of its own yet (only the \
         per-struct derive), so nothing installs its appearance/icons schema automatically on \
         this machine; any count beyond sill's own {sill_key_count} came from a real quire \
         install already present here, if any."
    );
}
