//! Open at Login on a scratch config folder: the entry is a marked copy, turning off removes only
//! a file sill wrote, and a file someone else put there is shown as on and kept.

use sill_launcher::AppId;
use sill_services::autostart::{Login, file_of, login_of, turn_off, turn_on};
use sill_services::env::Dirs;
use sill_testkit::TempDir;

const ENTRY: &str = "[Desktop Entry]\nName=Example\nExec=example %U\n\n[Desktop Action new]\nName=New\n";

fn app() -> AppId {
    AppId("example".to_owned())
}

/// A scratch home with the app's own entry written, and the entry's path.
fn setup(name: &str) -> (TempDir, Dirs, std::path::PathBuf) {
    let root = TempDir::new(name);
    let dirs = Dirs::under(root.path());
    let entry = root.path().join("example.desktop");
    std::fs::write(&entry, ENTRY).expect("entry");
    (root, dirs, entry)
}

#[test]
fn turning_on_copies_the_entry_with_one_line_added_and_off_removes_it() {
    let (_root, dirs, entry) = setup("autostart-on");
    assert_eq!(login_of(&dirs, &app()), Login::Off);
    turn_on(&dirs, &app(), &entry).expect("on");
    assert_eq!(login_of(&dirs, &app()), Login::OurEntry);
    let written = std::fs::read_to_string(file_of(&dirs, &app())).expect("written");
    assert_eq!(
        written,
        ENTRY.replacen(
            "[Desktop Entry]\n",
            "[Desktop Entry]\nX-Quire-Autostart=true\n",
            1
        )
    );
    turn_off(&dirs, &app()).expect("off");
    assert_eq!(login_of(&dirs, &app()), Login::Off);
}

#[test]
fn a_file_someone_else_wrote_shows_on_and_is_never_removed_or_replaced() {
    let (_root, dirs, entry) = setup("autostart-foreign");
    let theirs = "[Desktop Entry]\nName=Mine\nExec=example --quiet\n";
    let target = file_of(&dirs, &app());
    std::fs::create_dir_all(target.parent().expect("folder")).expect("folder");
    std::fs::write(&target, theirs).expect("theirs");
    assert_eq!(login_of(&dirs, &app()), Login::OthersEntry);
    turn_on(&dirs, &app(), &entry).expect("on");
    turn_off(&dirs, &app()).expect("off");
    assert_eq!(std::fs::read_to_string(&target).expect("kept"), theirs);
}

#[test]
fn an_entry_without_its_group_is_refused_and_nothing_is_written() {
    let (_root, dirs, entry) = setup("autostart-bad");
    std::fs::write(&entry, "Name=Example\n").expect("entry");
    assert!(turn_on(&dirs, &app(), &entry).is_err());
    assert_eq!(login_of(&dirs, &app()), Login::Off);
}
