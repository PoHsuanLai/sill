//! The daemon's writes into `settings.toml` change only their own keys: an edit another program
//! (Settings, a person's editor) made since the daemon last read the file survives, with its
//! comments, its spacing, the keys nobody reads and the values a key refused. Scratch config
//! directory only.

use ds_settings::{ConfigRoot, Store};
use prov::UnixSeconds;
use sill::voice::write_consent;
use sill_model::voice::VoiceConsent;
use sill_settings::{APP, Percent, ShellFile};
use sill_testkit::TempDir;
use tokio::sync::watch;

/// What Settings and a person left in the file: a comment, a key the daemon never reads, and a
/// value its key refuses.
const EDITED: &str = "\
# my bar
[bar]
battery_low_percent = 31   # earlier, on purpose
mystery = \"kept\"

[dock]
tile_size_px = 200
";

fn file_in(dir: &TempDir) -> std::path::PathBuf {
    dir.path().join("sill/settings.toml")
}

fn store_in(dir: &TempDir) -> Store {
    Store::new(ConfigRoot::Scratch(dir.path().to_path_buf()), APP)
}

/// A file the daemon never read: written after its feed was filled with the defaults.
fn edited_behind_its_back(dir: &TempDir) {
    let file = file_in(dir);
    std::fs::create_dir_all(file.parent().unwrap_or_else(|| panic!("a parent")))
        .unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(file, EDITED).unwrap_or_else(|e| panic!("{e}"));
}

fn text(dir: &TempDir) -> String {
    std::fs::read_to_string(file_in(dir)).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn an_edit_of_another_key_survives_the_consent_answer() {
    let dir = TempDir::new("patch-consent");
    let (tx, rx) = watch::channel(ShellFile::default());
    edited_behind_its_back(&dir);

    write_consent(&store_in(&dir), &tx, VoiceConsent::Declined).unwrap_or_else(|e| panic!("{e}"));

    let after = text(&dir);
    for kept in [
        "# my bar",
        "battery_low_percent = 31   # earlier, on purpose",
        "mystery = \"kept\"",
        "tile_size_px = 200",
    ] {
        assert!(after.contains(kept), "{kept:?} lost:\n{after}");
    }
    assert!(after.contains("declined"), "{after}");
    assert_eq!(rx.borrow().voice.consent, VoiceConsent::Declined);
    assert_eq!(
        rx.borrow().bar.battery_low_percent,
        Percent(31),
        "what is published is what the file says now"
    );
}

#[test]
fn a_second_write_replaces_the_key_in_place() {
    let dir = TempDir::new("patch-twice");
    let (tx, _rx) = watch::channel(ShellFile::default());
    edited_behind_its_back(&dir);
    let store = store_in(&dir);

    write_consent(&store, &tx, VoiceConsent::Given(UnixSeconds(1_760_000_000)))
        .unwrap_or_else(|e| panic!("{e}"));
    write_consent(&store, &tx, VoiceConsent::Declined).unwrap_or_else(|e| panic!("{e}"));

    let after = text(&dir);
    assert!(after.contains("# my bar"), "{after}");
    assert!(
        after.contains("battery_low_percent = 31   # earlier, on purpose"),
        "{after}"
    );
    assert!(after.contains("tile_size_px = 200"), "{after}");
    assert_eq!(after.matches("consent").count(), 1, "{after}");
    assert!(after.contains("declined"), "{after}");
}

#[test]
fn a_consent_given_is_stored_as_voiced_reads_it_and_a_missing_file_is_made() {
    let dir = TempDir::new("patch-new-file");
    let (tx, _rx) = watch::channel(ShellFile::default());

    let given = VoiceConsent::Given(UnixSeconds(1_760_000_000));
    write_consent(&store_in(&dir), &tx, given).unwrap_or_else(|e| panic!("{e}"));

    let table: toml::Table = text(&dir).parse().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        table["voice"]["consent"]["given"].as_integer(),
        Some(1_760_000_000)
    );
}

#[test]
fn a_file_that_is_not_toml_is_left_alone() {
    let dir = TempDir::new("patch-garbled");
    let (tx, _rx) = watch::channel(ShellFile::default());
    let file = file_in(&dir);
    std::fs::create_dir_all(file.parent().unwrap_or_else(|| panic!("a parent")))
        .unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(&file, "[bar\nheight_px = ").unwrap_or_else(|e| panic!("{e}"));

    assert!(write_consent(&store_in(&dir), &tx, VoiceConsent::Declined).is_err());
    assert_eq!(text(&dir), "[bar\nheight_px = ");
}
