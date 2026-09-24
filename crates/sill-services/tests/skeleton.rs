//! Behaviour the freeze leaves to its waves, as tests that fail until the stub is filled. Each
//! is ignored with the wave that owns it; the filler removes the `ignore` and makes it pass.

use std::path::{Path, PathBuf};

use sill_launcher::{AppId, DesktopAction};
use sill_services::apps::{IconTheme, parse_entry};
use sill_services::launch::{LaunchRequest, launch_argv, scope_unit};
use sill_services::pins::{load as load_pins, save as save_pins};
use sill_services::wallpaper::source_hash;
use sill_services::{AppIcon, DockPins, Terminal};

fn temp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sill-services-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_desktop_entry_parses_into_an_app() {
    let text = "[Desktop Entry]\nType=Application\nName=Foot\nExec=foot %F\nIcon=/usr/share/icons/foot.svg\n\
                Terminal=false\nStartupWMClass=foot\nActions=server;\n\n[Desktop Action server]\n\
                Name=Server\nExec=foot --server\n";
    let theme = IconTheme("hicolor".to_owned());
    let app = parse_entry(
        Path::new("/usr/share/applications/foot.desktop"),
        "foot",
        text,
        &theme,
        &[],
    )
    .unwrap_or_else(|| panic!("an Application entry parses"));
    assert_eq!(app.id, AppId("foot".to_owned()));
    assert_eq!(app.name, "Foot");
    assert_eq!(app.exec, "foot %F");
    assert_eq!(app.terminal, Terminal::No);
    assert_eq!(
        app.icon,
        AppIcon::File(PathBuf::from("/usr/share/icons/foot.svg"))
    );
    assert_eq!(app.startup_wm_class.as_deref(), Some("foot"));
    assert_eq!(app.actions.len(), 1);
    let link = "[Desktop Entry]\nType=Link\nName=Site\nURL=https://example.org\n";
    assert!(parse_entry(Path::new("/x/site.desktop"), "site", link, &theme, &[]).is_none());
}

#[test]
#[ignore = "M2 launch fill"]
fn a_scope_unit_is_named_and_escaped_like_systemd() {
    assert_eq!(
        scope_unit(&AppId("foot".to_owned()), 7).0,
        "app-sill-foot-7.scope"
    );
    assert_eq!(
        scope_unit(&AppId("org.mozilla.firefox".to_owned()), 1).0,
        "app-sill-org.mozilla.firefox-1.scope"
    );
    assert_eq!(
        scope_unit(&AppId("my-app".to_owned()), 2).0,
        "app-sill-my\\x2dapp-2.scope"
    );
}

#[test]
#[ignore = "M2 launch fill"]
fn a_launch_runs_in_a_transient_scope() {
    let theme = IconTheme("hicolor".to_owned());
    let text = "[Desktop Entry]\nType=Application\nName=Htop\nExec=htop\nTerminal=true\n";
    let app = parse_entry(Path::new("/x/htop.desktop"), "htop", text, &theme, &[])
        .unwrap_or_else(|| panic!("parses"));
    let request = LaunchRequest {
        app: app.id.clone(),
        action: DesktopAction::Default,
        files: Vec::new(),
        uris: Vec::new(),
        token: None,
    };
    let unit = scope_unit(&app.id, 3);
    let argv = launch_argv(&app, &request, &unit, &["foot".to_owned(), "-e".to_owned()])
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        argv,
        [
            "systemd-run",
            "--user",
            "--scope",
            "--collect",
            "--unit=app-sill-htop-3.scope",
            "--",
            "foot",
            "-e",
            "htop"
        ]
    );
}

#[test]
#[ignore = "M2 launch (dock_config) fill"]
fn pins_round_trip_through_dock_json() {
    let dir = temp("pins");
    assert_eq!(load_pins(&dir), DockPins::default(), "missing is no pins");
    let pins = DockPins {
        version: 1,
        pinned: vec![AppId("foot".to_owned())],
    };
    save_pins(&dir, &pins).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(load_pins(&dir), pins);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "M2 wallpaper fill"]
fn a_source_hash_is_stable_and_sees_edits() {
    let a = source_hash(Path::new("/w/a.png"), 10, 1);
    assert_eq!(
        a,
        source_hash(Path::new("/w/a.png"), 10, 1),
        "stable across calls"
    );
    assert_ne!(
        a,
        source_hash(Path::new("/w/a.png"), 10, 2),
        "an edit is a new entry"
    );
    assert_ne!(a, source_hash(Path::new("/w/b.png"), 10, 1));
}
