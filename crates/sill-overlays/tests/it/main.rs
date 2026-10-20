//! The crate's integration tests: one executable, one module per former test file.
//! A separate test target needs a stated reason (CONVENTIONS.md, Tests).

mod accounts_sheet;
mod confirm_limits;
mod confirm_sheet;
mod display_trial;
mod empty_trash;
mod eyedropper;
mod force_quit;
mod hot_corners;
mod idle_dim;
mod layers;
mod lock;
mod osd;
mod permission_sheet;
mod polkit_prompt;
mod power_menu;
mod spaces_bar;
mod split_picker;
mod switcher;
mod switcher_wlr;
mod user_style;
mod voice_consent;
mod wallpaper;

/// A file in `tests/it` that `main.rs` does not list would compile to nothing.
#[test]
fn every_test_file_is_included() {
    sill_testkit::assert_all_modules_included(env!("CARGO_MANIFEST_DIR"));
}
