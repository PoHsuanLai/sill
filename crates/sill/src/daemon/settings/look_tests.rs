//! The look the keeper puts in the cascade: the Extension `appearance.look` names, found in the
//! data directories it is handed (a scratch directory here, never the person's), and nothing when
//! the choice is `standard`, turned off in `extensions.toml`, not installed or damaged.

use std::fs;
use std::path::{Path, PathBuf};

use ds_extensions::{ExtensionId, ExtensionsFile, ExtensionsSettings, Power};
use ds_settings::schema::Provided;
use ds_settings::{AppearanceFile, Store};

use super::{ChosenLook, LookSearch, chosen_look};
use crate::daemon::settings::store::desktop_store;

const PAPER: &str = "org.example.Paper";
const SHEET: &str = ".sill-bar { background: ivory; }\n";

/// A scratch directory with a `config` (the quire folder) and a `data` root.
pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new(tag: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("sill-look-test-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        Scratch(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn search(&self) -> LookSearch {
        LookSearch::new(self.0.join("data"), Vec::new())
    }

    /// `PAPER` as a look Extension in the scratch data root; returns its folder.
    pub fn install_paper(&self) -> PathBuf {
        let folder = self
            .0
            .join("data")
            .join("quire")
            .join("extensions")
            .join(PAPER);
        fs::create_dir_all(&folder).unwrap_or_else(|e| panic!("{}: {e}", folder.display()));
        let manifest = format!(
            "id = \"{PAPER}\"\nname = \"Paper\"\nprotocol = 1\nversion = \"1.0.0\"\n\
             licence = \"MIT\"\nsummary = \"A test look\"\n\n[[provides]]\ncapability = \"look\"\n\
             stylesheet = \"look.css\"\nstyles_for = 1\n"
        );
        fs::write(folder.join("extension.toml"), manifest).unwrap_or_else(|e| panic!("{e}"));
        fs::write(folder.join("look.css"), SHEET).unwrap_or_else(|e| panic!("{e}"));
        folder
    }

    /// `appearance.look = choice` in the scratch quire folder.
    pub fn choose(&self, choice: &str) {
        let mut file = AppearanceFile::default();
        file.appearance.look = Provided::new(choice);
        desktop_store(&self.0)
            .save(&file)
            .unwrap_or_else(|e| panic!("{e}"));
    }

    /// `id` turned off in the scratch `quire/extensions.toml`.
    pub fn turn_off(&self, id: &str) {
        let file = ExtensionsFile {
            extensions: ExtensionsSettings::default().with(&ExtensionId(id.to_owned()), Power::Off),
            ..ExtensionsFile::default()
        };
        desktop_store(&self.0)
            .save(&file)
            .unwrap_or_else(|e| panic!("{e}"));
    }

    pub fn store(&self) -> Store {
        desktop_store(&self.0)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn look_of(scratch: &Scratch) -> ChosenLook {
    let store = scratch.store();
    chosen_look(&store, &store, &scratch.search())
}

#[test]
fn an_installed_look_the_appearance_names_gives_its_sheet_and_folder() {
    let scratch = Scratch::new("named");
    let folder = scratch.install_paper();
    scratch.choose(PAPER);
    let look = look_of(&scratch);
    assert_eq!(look.style.0, SHEET);
    assert_eq!(look.home, Some(folder));
}

#[test]
fn standard_and_a_look_nobody_installed_give_no_sheet() {
    let scratch = Scratch::new("standard");
    scratch.install_paper();
    scratch.choose("standard");
    assert_eq!(look_of(&scratch), ChosenLook::default());
    scratch.choose("org.example.Removed");
    assert_eq!(look_of(&scratch), ChosenLook::default());
}

#[test]
fn a_look_turned_off_in_extensions_toml_is_not_used() {
    let scratch = Scratch::new("off");
    scratch.install_paper();
    scratch.choose(PAPER);
    scratch.turn_off(PAPER);
    assert_eq!(look_of(&scratch), ChosenLook::default());
}

#[test]
fn a_damaged_look_is_set_aside_and_never_half_used() {
    let scratch = Scratch::new("damaged");
    let folder = scratch.install_paper();
    scratch.choose(PAPER);
    fs::remove_file(folder.join("look.css")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(look_of(&scratch), ChosenLook::default());
}
