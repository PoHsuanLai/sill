//! The crate's integration tests: one executable, one module per former test file.
//! A separate test target needs a stated reason (CONVENTIONS.md, Tests).

mod launcher;
mod launcher_actions;
mod launcher_card;
mod launcher_catcher;
mod launcher_companion;
mod launcher_cues;
mod launcher_details;
mod launcher_details_lint;
mod launcher_harness;
mod launcher_keys;
mod launcher_lint;
mod launcher_pane_shots;
mod launcher_pdf_rasters;
mod launcher_preview;
mod launcher_video_frames;

/// A file in `tests/it` that `main.rs` does not list would compile to nothing.
#[test]
fn every_test_file_is_included() {
    sill_testkit::assert_all_modules_included(env!("CARGO_MANIFEST_DIR"));
}
