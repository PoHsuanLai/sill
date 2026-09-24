//! A tile's icon (design/08-ICONS.md §4): the app's own themed icon as a `ds::IconSource`
//! (`Symbolic` for a freedesktop `*-symbolic` file, recoloured like a glyph; `Image` for the
//! rest), or quire's `Window` glyph when the app has none (§4.3), `Trash` for the Trash tile.
//! The plate is quire's (`IconView { plate }`), which draws a glyph at its 56 % and an image at
//! its 72 % (`--plate-glyph`, `--plate-inset`); an icon file is still resolved at its 72 % share
//! in whole pixels through `IconSize::Px`, at every magnified size.

use ds::{ExternalIcon, Icon, IconPx, IconSize, IconSource, IconUrl};
use sill_services::{App, AppIcon};

use crate::surfaces::dock::items::ItemKind;

/// design/08 §2.2: the plate is 80.5 % of the tile (824 / 1024).
pub const PLATE_PERMILLE: i32 = 805;

/// design/08 §4.1: a third-party icon fills 72 % of the plate.
pub const ICON_PERMILLE: i32 = 720;

/// What the tile for `kind` shows on a plate `plate_px` wide, `app` being its entry when the
/// index has one.
pub fn tile_icon(kind: &ItemKind, app: Option<&App>, plate_px: f64) -> IconSource {
    if *kind == ItemKind::Trash {
        return IconSource::Glyph(Icon::Trash);
    }
    let file = app.and_then(|app| match &app.icon {
        AppIcon::File(path) => Some(path.clone()),
        AppIcon::None => None,
    });
    let Some(path) = file else {
        return IconSource::Glyph(Icon::Window);
    };
    let symbolic = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem.ends_with("-symbolic"));
    match IconUrl::file(&path) {
        Ok(url) => {
            let external = ExternalIcon {
                url,
                size: share_of(ICON_PERMILLE, plate_px),
            };
            if symbolic {
                IconSource::Symbolic(external)
            } else {
                IconSource::Image(external)
            }
        }
        Err(_) => IconSource::Glyph(Icon::Window),
    }
}

/// `permille` of `plate_px`, in whole pixels (at least one).
fn share_of(permille: i32, plate_px: f64) -> IconSize {
    let side = (plate_px * f64::from(permille) / 1000.0)
        .round()
        .clamp(1.0, 255.0);
    // Rounded and clamped to u8's range just above, so the cast is exact.
    IconSize::Px(IconPx(side as u8))
}

#[cfg(test)]
mod tests {
    use super::tile_icon;
    use crate::surfaces::dock::items::ItemKind;
    use ds::{Icon, IconSource};
    use sill_launcher::AppId;
    use sill_services::{App, AppIcon, DbusActivation, Listing, Terminal};
    use std::path::PathBuf;

    fn app(icon: AppIcon) -> App {
        App {
            id: AppId("term".to_owned()),
            name: "Term".to_owned(),
            generic_name: String::new(),
            comment: String::new(),
            exec: "term".to_owned(),
            icon,
            startup_wm_class: None,
            flatpak: None,
            terminal: Terminal::No,
            activation: DbusActivation::Exec,
            actions: Vec::new(),
            mime_types: Vec::new(),
            keywords: Vec::new(),
            listing: Listing::Shown,
            path: PathBuf::from("/usr/share/applications/term.desktop"),
        }
    }

    #[test]
    fn each_icon_kind_picks_its_source() {
        let kind = ItemKind::Pinned(AppId("term".to_owned()));
        let file = |p: &str| app(AppIcon::File(PathBuf::from(p)));
        let word = |source: IconSource| match source {
            IconSource::Glyph(icon) => format!("glyph {icon:?}"),
            IconSource::Symbolic(e) => format!("symbolic {}", e.url.as_str()),
            IconSource::Image(e) => format!("image {}", e.url.as_str()),
        };
        let cases = [
            (Some(file("/i/term.svg")), "image file:///i/term.svg"),
            (
                Some(file("/i/term-symbolic.svg")),
                "symbolic file:///i/term-symbolic.svg",
            ),
            (Some(app(AppIcon::None)), "glyph Window"),
            (None, "glyph Window"),
        ];
        for (entry, want) in cases {
            assert_eq!(
                word(tile_icon(&kind, entry.as_ref(), 38.64)),
                want,
                "{want}"
            );
        }
        assert_eq!(
            tile_icon(&ItemKind::Trash, None, 38.64),
            IconSource::Glyph(Icon::Trash)
        );
    }

    #[test]
    fn icons_take_their_share_of_the_plate_in_whole_pixels() {
        use ds::{IconPx, IconSize};
        let file = app(AppIcon::File(PathBuf::from("/i/term.svg")));
        let kind = ItemKind::Pinned(AppId("term".to_owned()));
        let size = |plate: f64| match tile_icon(&kind, Some(&file), plate) {
            IconSource::Image(e) => e.size,
            other => panic!("{other:?}"),
        };
        // A 48 px tile's plate is 38.64 px: the icon 27.8.
        assert_eq!(size(48.0 * 0.805), IconSize::Px(IconPx(28)));
        // Fully magnified (96 px): 55.6.
        assert_eq!(size(96.0 * 0.805), IconSize::Px(IconPx(56)));
    }
}
