//! The crate's integration tests: one executable, one module per former test file.
//! A separate test target needs a stated reason (CONVENTIONS.md, Tests).

mod accounts_bus;
mod accounts_sheet_bus;
mod app_menu_bus;
mod auth_live;
mod bluetooth_bus;
mod clipboard_live;
mod companion_bus;
mod companion_cards_bus;
mod companion_routing_bus;
mod companion_session_bus;
mod companion_support;
mod cosmic_wl_nested;
mod events_live;
mod guest_asks_bus;
mod icons;
mod idle_bus;
mod intents_bus;
mod keycap_socket;
mod keycap_systemd_bus;
mod launch_viewer_bus;
mod notifications_bus;
mod polkit_bus;
mod scale_live;
mod scan;
mod screenshots_live;
mod shell_manifest;
mod spawn_sites;
mod sync_hold_bus;
mod tray_bus;
mod voice_bus;
mod voice_support;
mod work_areas_bus;
mod workspace_looks_bus;

/// A file in `tests/it` that `main.rs` does not list would compile to nothing.
#[test]
fn every_test_file_is_included() {
    sill_testkit::assert_all_modules_included(env!("CARGO_MANIFEST_DIR"));
}
