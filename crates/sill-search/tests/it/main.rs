//! The crate's integration tests: one executable, one module per former test file.
//! A separate test target needs a stated reason (CONVENTIONS.md, Tests).

mod files_tracker_live;
mod files_walk_live;
mod providers_apps;
mod providers_windows;
mod ranking_table;
mod settings_fixture;

/// A file in `tests/it` that `main.rs` does not list would compile to nothing.
#[test]
fn every_test_file_is_included() {
    sill_testkit::assert_all_modules_included(env!("CARGO_MANIFEST_DIR"));
}
