//! A launcher row's tile in the icon style: the row's resolved icon file, re-tinted under
//! Muted and Monochrome; a symbolic file and a glyph as before. Every row takes this one path.
//!
//! The rows draw their icons at 48 px (`IconSize::Tile48`), so the raster is loaded at 48
//! times the output scale. An app row's entry is its id; a window row's is the app whose
//! resolved icon it carries (the windows provider copies it from the matched entry).

use std::path::Path;

use ds::prelude::*;
use ds::style::icon::url::IconUrl;
use sill_launcher::{IconRef, Item};

use crate::launcher::entries::glyph;
use sill_ui_kit::app_icons::{IconContext, IconFile, LogicalPx, pick, theme_art};

/// The side the rows draw an icon at.
const ROW_ICON: LogicalPx = LogicalPx(48);

/// The icon file `item`'s tile draws; `None` shows the row's glyph (its own, else its
/// provider's).
pub fn row_art(item: &Item) -> Option<IconFile> {
    match &item.icon {
        IconRef::File(path) => IconFile::of(path),
        IconRef::Glyph(_) | IconRef::None => None,
    }
}

/// `item`'s tile for `art` in `cx`.
pub fn row_tile(item: &Item, art: Option<IconFile>, cx: &IconContext) -> RowLeading {
    let fallback = || RowLeading::Icon(glyph(item));
    let px = pick(ROW_ICON, cx.scale);
    match art {
        None => fallback(),
        Some(IconFile::Symbolic(path)) => {
            external(&path).map_or_else(fallback, |e| RowLeading::Source(IconSource::Symbolic(e)))
        }
        Some(IconFile::Themed(path)) => {
            let shown = cx.resolver.path(&theme_art(&path, px, cx.look));
            external(&shown).map_or_else(fallback, |e| RowLeading::Source(IconSource::Image(e)))
        }
    }
}

fn external(path: &Path) -> Option<ExternalIcon> {
    Some(ExternalIcon {
        url: IconUrl::file(path).ok()?,
        size: IconSize::Tile48,
    })
}

#[cfg(test)]
mod tests {
    use super::{row_art, row_tile};
    use ds::prelude::*;
    use ds::style::icon::retint::IconStyle;
    use ds::style::icon::retint::Tint;
    use sill_launcher::{IconRef, Item, ItemId, ProviderKind, Score};
    use sill_ui_kit::app_icons::cache::Generation;
    use sill_ui_kit::app_icons::{IconContext, IconFile, IconLook, IconResolver};
    use std::path::PathBuf;

    fn row(provider: ProviderKind, id: &str, icon: IconRef) -> Item {
        Item {
            id: ItemId(id.to_owned()),
            provider,
            title: id.to_owned(),
            subtitle: String::new(),
            icon,
            score: Score(0),
            actions: Vec::new(),
            subject: sill_launcher::Subject::None,
        }
    }

    fn file(path: &str) -> IconRef {
        IconRef::File(PathBuf::from(path))
    }

    #[test]
    fn rows_pick_their_art() {
        let term = "foot";
        let cases = [
            (
                row(ProviderKind::Apps, term, file("/i/term.svg")),
                Some(IconFile::Themed("/i/term.svg".into())),
            ),
            (
                row(ProviderKind::Apps, "firefox", file("/i/firefox.svg")),
                Some(IconFile::Themed("/i/firefox.svg".into())),
            ),
            (
                row(ProviderKind::Settings, "x", file("/i/x-symbolic.svg")),
                Some(IconFile::Symbolic("/i/x-symbolic.svg".into())),
            ),
            (
                row(
                    ProviderKind::System,
                    "lock",
                    IconRef::Glyph("lock".to_owned()),
                ),
                None,
            ),
            (row(ProviderKind::Apps, "firefox", file("rel.svg")), None),
        ];
        for (item, want) in cases {
            assert_eq!(row_art(&item), want, "{}", item.id.0);
        }
    }

    /// Each style's file for a themed row at the 48 px raster, the scale folded in.
    #[test]
    fn row_tiles_load_the_48_px_raster_for_the_scale() {
        let cx = |style, scale| IconContext {
            look: IconLook {
                style,
                tint: Tint::NEUTRAL,
                scheme: Scheme::Light,
            },
            scale: Scale(scale),
            resolver: IconResolver {
                seen: Generation::default(),
            },
        };
        let item = row(ProviderKind::Apps, "x", IconRef::None);
        let url = |tile: RowLeading| match tile {
            RowLeading::Source(IconSource::Image(e)) => e.url.as_str().to_owned(),
            other => panic!("{other:?}"),
        };
        let themed = row_tile(
            &item,
            Some(IconFile::Themed("/i/f.svg".into())),
            &cx(IconStyle::Muted, 120),
        );
        assert_eq!(url(themed), "file:///i/f.svg");
        assert_eq!(
            row_tile(&item, None, &cx(IconStyle::Muted, 120)),
            RowLeading::Icon(Icon::Grid)
        );
    }
}
