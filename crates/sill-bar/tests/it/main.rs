//! The crate's integration tests: one executable, one module per former test file.
//! A separate test target needs a stated reason (CONVENTIONS.md, Tests).

mod bar_render;
mod cc_brightness;
mod cc_details_modules;
mod cc_details_rows;
mod cc_night_shift;
mod cc_support;
mod cc_terminal;
mod control_center_bluetooth_ask;
mod control_center_dark;
mod control_center_keys;
mod control_center_lint;
mod control_center_render;
mod dark_contrast;
mod status_glyphs;

/// A file in `tests/it` that `main.rs` does not list would compile to nothing.
#[test]
fn every_test_file_is_included() {
    sill_testkit::assert_all_modules_included(env!("CARGO_MANIFEST_DIR"));
}
