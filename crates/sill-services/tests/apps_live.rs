//! The real scan against this machine's own applications directories and icon theme
//! (CONVENTIONS "Run it the way its user would": no fixture, no override, the same
//! `applications_dirs`/`icon_theme`/`scan` the daemon calls from `apps::spawn`).

use sill_services::apps::{applications_dirs, icon_theme, scan};

#[test]
#[ignore = "reads this machine's applications dirs"]
fn the_real_scan_finds_real_applications() {
    let dirs = applications_dirs();
    let theme = icon_theme();
    let locales = freedesktop_desktop_entry::get_languages_from_env();
    let found = scan(&dirs, &theme, &locales);

    eprintln!("dirs searched: {dirs:?}");
    eprintln!("icon theme: {theme:?}");
    eprintln!("apps found: {}", found.len());
    for app in found.iter().take(5) {
        eprintln!(
            "  {} | {:?} | exec={:?} | icon={:?} | listing={:?}",
            app.id.0, app.name, app.exec, app.icon, app.listing
        );
    }

    assert!(!found.is_empty(), "expected at least one real application");
}
