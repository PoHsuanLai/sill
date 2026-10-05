//! `run` against a scratch config home; nothing here reads or writes the real `~/.config`.

use std::fs;
use std::path::Path;

use super::{Mode, Outcome, TARGET, run};

const KCMINPUTRC: &str = "[Libinput][5][6][Acme Wheel Mouse]\nNaturalScroll=true\n";

fn scratch(name: &str, kcminputrc: Option<&str>) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("sill-import-input-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch dir");
    if let Some(text) = kcminputrc {
        fs::write(dir.join("kcminputrc"), text).expect("kcminputrc");
    }
    dir
}

fn target(home: &Path) -> std::path::PathBuf {
    home.join(TARGET)
}

#[test]
fn a_missing_file_is_written_with_the_imported_devices() {
    let home = scratch("writes", Some(KCMINPUTRC));
    let outcome = run(&home, Mode::Write);
    assert!(matches!(outcome, Outcome::Wrote { .. }), "{outcome:?}");
    let text = fs::read_to_string(target(&home)).expect("written");
    assert!(text.contains("\"Acme Wheel Mouse\""), "{text}");
    assert!(outcome.report().starts_with("wrote "));
    fs::remove_dir_all(home).expect("cleanup");
}

#[test]
fn dry_run_says_what_it_would_write_and_writes_nothing() {
    let home = scratch("dry", Some(KCMINPUTRC));
    let outcome = run(&home, Mode::DryRun);
    assert!(matches!(outcome, Outcome::WouldWrite { .. }), "{outcome:?}");
    assert!(!target(&home).exists());
    assert!(outcome.report().contains("Acme Wheel Mouse"));
    fs::remove_dir_all(home).expect("cleanup");
}

#[test]
fn an_existing_file_is_left_alone_and_exits_2() {
    for mode in [Mode::Write, Mode::DryRun] {
        let home = scratch("exists", Some(KCMINPUTRC));
        fs::create_dir_all(target(&home).parent().expect("dir")).expect("dir");
        fs::write(target(&home), "{}").expect("existing");
        let outcome = run(&home, mode);
        assert!(
            matches!(outcome, Outcome::Exists { .. }),
            "{mode:?}: {outcome:?}"
        );
        assert_eq!(fs::read_to_string(target(&home)).expect("read"), "{}");
        assert_eq!(outcome.exit_code(), std::process::ExitCode::from(2));
        fs::remove_dir_all(home).expect("cleanup");
    }
}

#[test]
fn no_kcminputrc_and_nothing_to_import_are_told_apart() {
    let none = scratch("none", None);
    let outcome = run(&none, Mode::Write);
    assert!(matches!(outcome, Outcome::NoSource { .. }), "{outcome:?}");
    assert!(outcome.failed());
    let bare = scratch("bare", Some("[Keyboard]\nRepeatRate=10\n"));
    let outcome = run(&bare, Mode::Write);
    assert!(
        matches!(outcome, Outcome::NothingToImport { .. }),
        "{outcome:?}"
    );
    assert!(!target(&bare).exists());
    for home in [none, bare] {
        fs::remove_dir_all(home).expect("cleanup");
    }
}
