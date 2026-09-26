//! A launcher row's tile in the icon style: quire's shipped icon for an app it stands for
//! (`dock.shipped_icons`, which both the dock and the launcher follow), else the row's resolved
//! icon file, re-tinted under Muted and Monochrome; a symbolic file and a glyph as before.
//!
//! The rows draw their icons at 48 px (`IconSize::Tile48`), so the raster is loaded at 48
//! times the output scale. An app row's entry is its id; a window row's is the app whose
//! resolved icon it carries (the windows provider copies it from the matched entry).

use std::path::{Path, PathBuf};

use ds::{ExternalIcon, IconSize, IconSource, IconUrl, Tile};
use sill_launcher::{IconRef, Item, ProviderKind};
use sill_services::providers::row_app_id;
use sill_services::{App, AppIcon, AppIndex};
use sill_settings::ShippedIcons;

use crate::app_icons::{
    DEFAULT_TABLE, IconContext, LogicalPx, ShippedApp, pick, shipped_app, shipped_art, theme_art,
};
use crate::surfaces::launcher::entries::glyph;

/// The side the rows draw an icon at.
const ROW_ICON: LogicalPx = LogicalPx(48);

/// What a row's tile shows, before sizes and styles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowArt {
    /// The row's glyph (its own, else its provider's).
    Glyph,
    /// A symbolic icon file, in the text colour.
    Symbolic(PathBuf),
    /// A theme (or the entry's own) icon file.
    Themed(PathBuf),
    /// quire's own icon.
    Shipped(ShippedApp),
}

/// What `item`'s tile shows, `index` naming its app.
pub fn row_art(item: &Item, index: &AppIndex, shipped: ShippedIcons) -> RowArt {
    let ours = row_app(item, index).and_then(|app| shipped_app(&app.id, shipped, DEFAULT_TABLE));
    if let Some(ours) = ours {
        return RowArt::Shipped(ours);
    }
    match &item.icon {
        IconRef::File(path) if !path.is_absolute() => RowArt::Glyph,
        IconRef::File(path) if symbolic(path) => RowArt::Symbolic(path.clone()),
        IconRef::File(path) => RowArt::Themed(path.clone()),
        IconRef::Glyph(_) | IconRef::None => RowArt::Glyph,
    }
}

/// The entry a row stands for: an app row's by id, a window row's by the icon it carries.
fn row_app<'a>(item: &Item, index: &'a AppIndex) -> Option<&'a App> {
    match (item.provider, &item.icon) {
        (ProviderKind::Apps, _) => index.get(&row_app_id(&item.id)),
        (ProviderKind::Windows, IconRef::File(path)) => index
            .apps
            .iter()
            .find(|app| matches!(&app.icon, AppIcon::File(own) if own == path)),
        _ => None,
    }
}

fn symbolic(path: &Path) -> bool {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem.ends_with("-symbolic"))
}

/// `item`'s tile for `art` in `cx`.
pub fn row_tile(item: &Item, art: RowArt, cx: &IconContext) -> Tile {
    let px = pick(ROW_ICON, cx.scale);
    let file = match art {
        RowArt::Glyph => None,
        RowArt::Symbolic(path) => {
            return external(&path).map_or(Tile::Icon(glyph(item)), |e| {
                Tile::Source(IconSource::Symbolic(e))
            });
        }
        RowArt::Themed(path) => Some(cx.resolver.path(&theme_art(&path, px, cx.look))),
        RowArt::Shipped(app) => cx
            .resolver
            .shipped_path(app, px, cx.look.style)
            .map(|source| cx.resolver.path(&shipped_art(source, app, px, cx.look))),
    };
    file.as_deref()
        .and_then(external)
        .map_or(Tile::Icon(glyph(item)), |e| {
            Tile::Source(IconSource::Image(e))
        })
}

fn external(path: &Path) -> Option<ExternalIcon> {
    Some(ExternalIcon {
        url: IconUrl::file(path).ok()?,
        size: IconSize::Tile48,
    })
}

#[cfg(test)]
mod tests {
    use super::{RowArt, row_art, row_tile};
    use crate::app_icons::cache::Generation;
    use crate::app_icons::{IconContext, IconLook, IconResolver, ShippedApp};
    use ds::icon::{IconStyle, Tint};
    use ds::{Icon, IconSource, Scale, Tile};
    use sill_launcher::{AppId, IconRef, Item, ItemId, ProviderKind, Score};
    use sill_services::{
        App, AppIcon, AppIndex, AppsEvent, DbusActivation, Listing, Terminal, reduce_apps,
    };
    use sill_settings::ShippedIcons;
    use std::path::PathBuf;

    fn entry(id: &str, icon: &str) -> App {
        App {
            id: AppId(id.to_owned()),
            name: id.to_owned(),
            generic_name: String::new(),
            comment: String::new(),
            exec: id.to_owned(),
            icon: AppIcon::File(PathBuf::from(icon)),
            startup_wm_class: None,
            flatpak: None,
            terminal: Terminal::No,
            activation: DbusActivation::Exec,
            actions: Vec::new(),
            mime_types: Vec::new(),
            keywords: Vec::new(),
            listing: Listing::Shown,
            path: PathBuf::from(format!("/usr/share/applications/{id}.desktop")),
        }
    }

    fn index() -> AppIndex {
        reduce_apps(
            AppIndex::default(),
            AppsEvent::Scanned(vec![
                entry("com.system76.CosmicTerm", "/i/term.svg"),
                entry("firefox", "/i/firefox.svg"),
            ]),
        )
    }

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
        let term = "com.system76.CosmicTerm";
        let cases = [
            (
                row(ProviderKind::Apps, term, file("/i/term.svg")),
                ShippedIcons::On,
                RowArt::Shipped(ShippedApp::Terminal),
            ),
            (
                row(
                    ProviderKind::Apps,
                    &format!("{term}\u{1}new-window"),
                    file("/i/term.svg"),
                ),
                ShippedIcons::On,
                RowArt::Shipped(ShippedApp::Terminal),
            ),
            (
                row(ProviderKind::Windows, "7", file("/i/term.svg")),
                ShippedIcons::On,
                RowArt::Shipped(ShippedApp::Terminal),
            ),
            (
                row(ProviderKind::Apps, term, file("/i/term.svg")),
                ShippedIcons::Off,
                RowArt::Themed("/i/term.svg".into()),
            ),
            (
                row(ProviderKind::Apps, "firefox", file("/i/firefox.svg")),
                ShippedIcons::On,
                RowArt::Themed("/i/firefox.svg".into()),
            ),
            (
                row(ProviderKind::Settings, "x", file("/i/x-symbolic.svg")),
                ShippedIcons::On,
                RowArt::Symbolic("/i/x-symbolic.svg".into()),
            ),
            (
                row(
                    ProviderKind::System,
                    "lock",
                    IconRef::Glyph("lock".to_owned()),
                ),
                ShippedIcons::On,
                RowArt::Glyph,
            ),
            (
                row(ProviderKind::Apps, "firefox", file("rel.svg")),
                ShippedIcons::On,
                RowArt::Glyph,
            ),
        ];
        for (item, shipped, want) in cases {
            assert_eq!(row_art(&item, &index(), shipped), want, "{}", item.id.0);
        }
    }

    /// Under quire's real apps directory (the sibling checkout this workspace builds against,
    /// `ds_settings`'s development fallback; sill FINDINGS Q71): each style's file for Notes at
    /// the 48 px row raster, the scale folded in.
    #[test]
    fn row_tiles_load_the_48_px_raster_for_the_scale() {
        let cx = |style, scale| IconContext {
            look: IconLook {
                style,
                tint: Tint::NEUTRAL,
            },
            scale: Scale(scale),
            resolver: IconResolver {
                seen: Generation::default(),
            },
        };
        let item = row(ProviderKind::Apps, "x", IconRef::None);
        let url = |tile: Tile| match tile {
            Tile::Source(IconSource::Image(e)) => e.url.as_str().to_owned(),
            other => panic!("{other:?}"),
        };
        let shipped =
            |style, scale| row_tile(&item, RowArt::Shipped(ShippedApp::Notes), &cx(style, scale));
        assert!(url(shipped(IconStyle::Colour, 120)).ends_with("notes/48.png"));
        assert!(url(shipped(IconStyle::Muted, 180)).ends_with("notes/muted/72.png"));
        assert!(
            url(shipped(IconStyle::Monochrome, 240)).ends_with("notes/monochrome/96.png"),
            "the untinted source until the cache has it"
        );
        let themed = row_tile(
            &item,
            RowArt::Themed("/i/f.svg".into()),
            &cx(IconStyle::Muted, 120),
        );
        assert_eq!(url(themed), "file:///i/f.svg");
        assert_eq!(
            row_tile(&item, RowArt::Glyph, &cx(IconStyle::Muted, 120)),
            Tile::Icon(Icon::Grid)
        );
    }
}
