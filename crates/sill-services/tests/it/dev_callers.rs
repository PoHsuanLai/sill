//! `sill run --dev-callers FILE` (feature `dev-callers`): a person's own rows file is taken only
//! when it is a plain file that this user or root owns and nobody else can write, and it must
//! parse; any other state adds nothing. Scratch folders only; nothing reads the desktop's table.
#![cfg(feature = "dev-callers")]

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use sill_services::callers::DesktopCallers;
use sill_testkit::TempDir;

const ROWS: &str = "[[peer]]\napp = \"org.quire.Accounts\"\nrole = \"accounts\"\nexe = [\"/opt/checkout/standin\"]\n";

fn write(dir: &Path, name: &str, text: &str, mode: u32) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, text).unwrap_or_else(|e| panic!("{e}")); // test-only
    fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap_or_else(|e| panic!("{e}")); // test-only
    path
}

#[test]
fn only_the_persons_own_plain_unshared_file_that_parses_adds_rows() {
    let dir = TempDir::new("dev-callers");
    let own = write(dir.path(), "own.toml", ROWS, 0o600);
    let link = dir.path().join("link.toml");
    symlink(&own, &link).unwrap_or_else(|e| panic!("{e}")); // test-only
    let cases = [
        ("a file of the person's, mode 600", own.clone(), true),
        (
            "mode 644 (others read, nobody else writes)",
            write(dir.path(), "readable.toml", ROWS, 0o644),
            true,
        ),
        (
            "writable by the group",
            write(dir.path(), "group.toml", ROWS, 0o660),
            false,
        ),
        (
            "writable by everyone",
            write(dir.path(), "world.toml", ROWS, 0o666),
            false,
        ),
        ("a link to a good file", link, false),
        (
            "text that is not a table",
            write(dir.path(), "junk.toml", "not = [a table", 0o600),
            false,
        ),
        (
            "a row naming no image",
            write(
                dir.path(),
                "noexe.toml",
                "[[peer]]\napp = \"org.quire.Accounts\"\nrole = \"accounts\"\n",
                0o600,
            ),
            false,
        ),
        (
            "a file that is not there",
            dir.path().join("none.toml"),
            false,
        ),
        ("a folder", dir.path().to_path_buf(), false),
    ];
    for (name, path, taken) in cases {
        let got = DesktopCallers::new(peephole_bus::peephole_core::Table::empty())
            .with_dev_rows(&path)
            .is_ok();
        assert_eq!(got, taken, "{name}");
    }
}
