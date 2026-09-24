//! A tile's icon (design/08-ICONS.md §4): the app's own themed icon as a `ds::IconSource`
//! (`Symbolic` for a freedesktop `*-symbolic` file, recoloured like a glyph; `Image` for the
//! rest), or quire's `Window` glyph when the app has none (§4.3), `Trash` for the Trash tile.
//! The plate and the 72 % inset are the tile's; the icon is drawn at quire's largest icon
//! size and scaled up to the inset, since `ds::IconSize` stops at 22 px (FINDINGS "M3
//! dock-ui", Q14).

use ds::{ExternalIcon, Icon, IconSize, IconSource, IconUrl};
use sill_services::{App, AppIcon};

use crate::surfaces::dock::items::ItemKind;

/// The size every tile icon is drawn at before scaling.
pub const DRAWN: IconSize = IconSize::Bar;

/// design/08 §2.2: the plate is 80.5 % of the tile (824 / 1024).
pub const PLATE_PERMILLE: i32 = 805;

/// design/08 §4.1: a third-party icon fills 72 % of the plate; §4.3: a fallback glyph 56 %.
pub const ICON_PERMILLE: i32 = 720;
pub const GLYPH_PERMILLE: i32 = 560;

/// What the tile for `kind` shows, `app` being its entry when the index has one.
pub fn tile_icon(kind: &ItemKind, app: Option<&App>) -> IconSource {
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
            let external = ExternalIcon { url, size: DRAWN };
            if symbolic {
                IconSource::Symbolic(external)
            } else {
                IconSource::Image(external)
            }
        }
        Err(_) => IconSource::Glyph(Icon::Window),
    }
}

/// The scale that takes the drawn icon to its share of a plate `plate_px` wide.
pub fn icon_scale(source: &IconSource, plate_px: f64) -> f64 {
    let share = match source {
        IconSource::Glyph(_) => GLYPH_PERMILLE,
        IconSource::Symbolic(_) | IconSource::Image(_) => ICON_PERMILLE,
    };
    plate_px * f64::from(share) / 1000.0 / f64::from(DRAWN.px())
}

#[cfg(test)]
mod tests {
    use super::{icon_scale, tile_icon};
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
            assert_eq!(word(tile_icon(&kind, entry.as_ref())), want, "{want}");
        }
        assert_eq!(
            tile_icon(&ItemKind::Trash, None),
            IconSource::Glyph(Icon::Trash)
        );
    }

    #[test]
    fn a_48_px_tile_shows_its_icon_at_72_percent_of_its_plate() {
        // Plate 38.64 px (80.5 % of 48), icon 27.8 px: 22 px drawn, scaled by 1.264.
        let plate = 48.0 * 0.805;
        let scale = icon_scale(&tile_icon(&ItemKind::Trash, None), plate);
        assert!((scale * 22.0 - plate * 0.56).abs() < 0.01, "{scale}");
    }
}
