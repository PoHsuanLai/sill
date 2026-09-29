//! `resolve_icon` against a real fixture icon theme directory.
//!
//! `freedesktop_icons`' theme table (`THEMES`) is a process-wide `once_cell::sync::Lazy`,
//! seeded once from `$HOME`/XDG env at first use and cached for the rest of the process; the
//! crate has no API to point a lookup at an arbitrary directory instead of a named, installed
//! theme. The only way to steer it is the environment it reads at that first use, and this
//! workspace denies `unsafe_code` everywhere but shell-host's one allowed module
//! (CONVENTIONS §2), which rules out `std::env::set_var` (`unsafe fn` since edition 2024).
//!
//! So this test writes its fixture theme into a real, already-scanned base path instead:
//! `$HOME/.icons` (`freedesktop_icons::theme::paths::icon_theme_base_paths`'s own
//! unconditional entry, filtered only by `.exists()`), which needs no env mutation at all. It
//! is the one file in this crate that touches the real home directory; the fixture's name is
//! unique enough not to collide with a real theme, and a `Drop` guard removes only that one
//! subdirectory afterward, panic or not.
//!
//! This is a single `#[test]` function on purpose: every other test in this crate's `tests/`
//! directory that reaches `resolve_icon` (`scan.rs`) uses fixtures with no `Icon=` at all, so
//! nothing else in this binary — or any other `tests/*.rs` binary, each its own process —
//! touches `THEMES` before this one seeds it.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use sill_services::AppIcon;
use sill_services::apps::{IconTheme, resolve_icon};

const FIXTURE_THEME: &str = "SillM2ApfixtureTheme";

/// Removes the one directory it names, on drop, panic or not.
struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn home_icons_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|e| panic!("no $HOME: {e}"));
    PathBuf::from(home).join(".icons")
}

/// A theme with a `48x48` and a `256x256` fixed directory and a `scalable` one, wide enough to
/// prove an SVG beats every raster size and the larger of two PNGs wins when there is no SVG.
fn write_fixture_theme() -> Cleanup {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let theme = home_icons_dir().join(format!("{FIXTURE_THEME}-{}-{n}", std::process::id()));
    let cleanup = Cleanup(theme.clone());

    fs::create_dir_all(&theme).unwrap_or_else(|e| panic!("{e}"));
    // `freedesktop_icons` returns the first directory (in the file's own section order, not
    // sorted by size) that has *any* matching file for the requested size: `scalable/apps`
    // must come first so its SVG is found before either `Fixed` directory's PNG, or the
    // preference this fixture exists to prove would depend on section order by accident.
    fs::write(
        theme.join("index.theme"),
        format!(
            "[Icon Theme]\n\
             Name={FIXTURE_THEME}\n\
             Directories=scalable/apps,48x48/apps,256x256/apps\n\
             [scalable/apps]\n\
             Size=256\n\
             MinSize=8\n\
             MaxSize=512\n\
             Type=Scalable\n\
             Context=Applications\n\
             [48x48/apps]\n\
             Size=48\n\
             Type=Fixed\n\
             Context=Applications\n\
             [256x256/apps]\n\
             Size=256\n\
             Type=Fixed\n\
             Context=Applications\n"
        ),
    )
    .unwrap_or_else(|e| panic!("{e}"));

    let small = theme.join("48x48/apps");
    let large = theme.join("256x256/apps");
    let scalable = theme.join("scalable/apps");
    for dir in [&small, &large, &scalable] {
        fs::create_dir_all(dir).unwrap_or_else(|e| panic!("{e}"));
    }
    fs::write(small.join("both.png"), b"png48").unwrap_or_else(|e| panic!("{e}"));
    fs::write(large.join("both.png"), b"png256").unwrap_or_else(|e| panic!("{e}"));
    fs::write(scalable.join("both.svg"), b"<svg/>").unwrap_or_else(|e| panic!("{e}"));
    fs::write(small.join("raster-only.png"), b"png48").unwrap_or_else(|e| panic!("{e}"));
    fs::write(large.join("raster-only.png"), b"png256").unwrap_or_else(|e| panic!("{e}"));

    cleanup
}

#[test]
fn resolve_icon_prefers_svg_then_the_largest_raster() {
    let cleanup = write_fixture_theme();
    let theme_dir = cleanup.0.clone();
    let theme_name = theme_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_else(|| panic!("bad fixture dir name"))
        .to_owned();
    let theme = IconTheme(theme_name);

    assert_eq!(
        resolve_icon("both", &theme),
        AppIcon::File(theme_dir.join("scalable/apps/both.svg")),
        "svg beats every raster size"
    );
    assert_eq!(
        resolve_icon("raster-only", &theme),
        AppIcon::File(theme_dir.join("256x256/apps/raster-only.png")),
        "the larger of two pngs wins when there is no svg"
    );
    assert_eq!(
        resolve_icon("no-such-icon-in-the-fixture", &theme),
        AppIcon::None
    );
}
