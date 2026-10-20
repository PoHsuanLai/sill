//! Emptying the home Trash on a scratch data folder, per the freedesktop Trash layout.

use sill_services::env::Dirs;
use sill_services::trash::{empty, is_empty, root};
use sill_testkit::TempDir;

fn fill(dirs: &Dirs) {
    let trash = root(dirs);
    std::fs::create_dir_all(trash.join("files/folder/inner")).expect("files");
    std::fs::create_dir_all(trash.join("info")).expect("info");
    std::fs::write(trash.join("files/a.txt"), "a").expect("a");
    std::fs::write(trash.join("files/folder/inner/b.txt"), "b").expect("b");
    std::fs::write(trash.join("info/a.txt.trashinfo"), "[Trash Info]\n").expect("info a");
    std::fs::write(trash.join("info/folder.trashinfo"), "[Trash Info]\n").expect("info folder");
    std::fs::write(trash.join("directorysizes"), "1 2 folder\n").expect("sizes");
}

#[test]
fn a_missing_trash_is_empty_and_emptying_it_is_fine() {
    let root = TempDir::new("trash-missing");
    let dirs = Dirs::under(root.path());
    assert!(is_empty(&dirs));
    empty(&dirs).expect("nothing to do");
}

#[test]
fn emptying_removes_files_and_infos_and_keeps_the_folders() {
    let scratch = TempDir::new("trash-fill");
    let dirs = Dirs::under(scratch.path());
    fill(&dirs);
    assert!(!is_empty(&dirs));
    empty(&dirs).expect("emptied");
    assert!(is_empty(&dirs));
    let trash = root(&dirs);
    let left = |folder: &str| std::fs::read_dir(trash.join(folder)).expect(folder).count();
    assert_eq!((left("files"), left("info")), (0, 0));
    assert!(!trash.join("directorysizes").exists());
}

#[test]
fn a_link_in_the_trash_is_removed_and_its_target_is_not_followed() {
    let scratch = TempDir::new("trash-link");
    let dirs = Dirs::under(scratch.path());
    let outside = scratch.path().join("outside");
    std::fs::create_dir_all(&outside).expect("outside");
    std::fs::write(outside.join("keep.txt"), "k").expect("keep");
    let files = root(&dirs).join("files");
    std::fs::create_dir_all(&files).expect("files");
    std::os::unix::fs::symlink(&outside, files.join("link")).expect("link");
    empty(&dirs).expect("emptied");
    assert!(is_empty(&dirs));
    assert!(outside.join("keep.txt").exists());
}
