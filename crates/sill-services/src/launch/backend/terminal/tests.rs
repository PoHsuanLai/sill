//! The choice of terminal over scratch folders: lists, entries and programs are all files in a
//! temporary directory.

use std::path::{Path, PathBuf};

use sill_testkit::TempDir;

use super::{Terminal, TerminalSearch, choose, wrapper};

/// A scratch computer: one config folder, one data folder, one program folder.
struct Scratch {
    dir: TempDir,
}

impl Scratch {
    fn new(name: &str) -> Scratch {
        let scratch = Scratch {
            dir: TempDir::new(name),
        };
        for sub in ["config", "data/applications", "bin"] {
            std::fs::create_dir_all(scratch.dir.path().join(sub)).unwrap_or_else(|e| panic!("{e}"));
        }
        scratch
    }

    fn search(&self, desktops: &[&str]) -> TerminalSearch {
        let root = self.dir.path();
        TerminalSearch {
            desktops: desktops.iter().map(|d| (*d).to_owned()).collect(),
            config: vec![root.join("config")],
            data: vec![root.join("data")],
            programs: vec![root.join("bin")],
        }
    }

    fn write(&self, relative: &str, text: &str) {
        std::fs::write(self.dir.path().join(relative), text).unwrap_or_else(|e| panic!("{e}"));
    }

    /// A program on the path.
    fn program(&self, name: &str) -> PathBuf {
        let path: PathBuf = self.dir.path().join("bin").join(name);
        std::fs::write(&path, b"#!/bin/sh\n").unwrap_or_else(|e| panic!("{e}"));
        make_executable(&path);
        path
    }

    /// A terminal's entry in the data folder.
    fn entry(&self, id: &str, exec: &str, extra: &str) {
        self.write(
            &format!("data/applications/{id}"),
            &format!("[Desktop Entry]\nType=Application\nName={id}\nExec={exec}\n{extra}\n"),
        );
    }
}

fn make_executable(path: &Path) {
    let mut perms = std::fs::metadata(path)
        .unwrap_or_else(|e| panic!("{e}"))
        .permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(path, perms).unwrap_or_else(|e| panic!("{e}"));
}

fn words(terminal: Option<Terminal>) -> Vec<String> {
    terminal.map(|t| t.wrapper()).unwrap_or_default()
}

#[test]
fn the_desktops_own_list_beats_the_general_list_and_temor() {
    let scratch = Scratch::new("terminal-lists");
    for bin in ["temor", "kitty", "foot"] {
        scratch.program(bin);
    }
    scratch.entry("org.quire.Temor.desktop", "temor", "");
    scratch.entry("kitty.desktop", "kitty", "");
    scratch.entry("foot.desktop", "foot", "");
    scratch.write("config/xdg-terminals.list", "# mine\nfoot.desktop\n");
    scratch.write("config/quire-xdg-terminals.list", "\nkitty.desktop\n");
    assert_eq!(
        words(choose(&scratch.search(&["quire"]))),
        ["kitty", "-e"],
        "the desktop's list is read first"
    );
    assert_eq!(
        words(choose(&scratch.search(&["other"]))),
        ["foot", "-e"],
        "another desktop reads the general list"
    );
}

#[test]
fn a_listed_terminal_that_is_not_installed_is_skipped() {
    let scratch = Scratch::new("terminal-skip");
    scratch.program("foot");
    scratch.entry("foot.desktop", "foot", "");
    scratch.entry("gone.desktop", "gone", "");
    scratch.write(
        "config/xdg-terminals.list",
        "missing.desktop\ngone.desktop\nfoot.desktop:new-window\nfoot.desktop\n",
    );
    assert_eq!(words(choose(&scratch.search(&[]))), ["foot", "-e"]);
}

#[test]
fn with_no_list_temor_comes_first_then_the_terminals_found_by_name() {
    let scratch = Scratch::new("terminal-temor");
    scratch.program("foot");
    scratch.program("ghostty");
    assert_eq!(
        words(choose(&scratch.search(&[]))),
        ["ghostty", "-e"],
        "ghostty is the first found by name"
    );
    scratch.program("temor");
    assert_eq!(
        words(choose(&scratch.search(&[]))),
        ["temor", "-e"],
        "temor by name beats the rest"
    );
    scratch.entry("org.quire.Temor.desktop", "temor --new", "X-TerminalArgExec=--run");
    assert_eq!(
        words(choose(&scratch.search(&[]))),
        ["temor", "--new", "--run"],
        "temor's entry says how it takes a command"
    );
}

#[test]
fn a_terminal_nobody_listed_is_found_by_its_category() {
    let scratch = Scratch::new("terminal-category");
    scratch.program("kitty");
    scratch.program("editor");
    scratch.entry("editor.desktop", "editor", "Categories=TextEditor;");
    scratch.entry("kitty.desktop", "kitty --single", "Categories=System;TerminalEmulator;");
    assert_eq!(words(choose(&scratch.search(&[]))), ["kitty", "--single", "-e"]);
}

#[test]
fn a_computer_with_no_terminal_has_no_wrapper() {
    let scratch = Scratch::new("terminal-none");
    assert_eq!(choose(&scratch.search(&["quire"])), None);
    assert_eq!(wrapper(&scratch.search(&["quire"])), Vec::<String>::new());
}
