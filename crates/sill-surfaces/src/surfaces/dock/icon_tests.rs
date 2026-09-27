use super::{IconContext, TileArt, TileSizes, drawn, tile_art};
use crate::app_icons::cache::Generation;
use crate::app_icons::{IconLook, IconResolver};
use crate::surfaces::dock::items::ItemKind;
use ds::icon::{IconStyle, Tint};
use ds::{Icon, IconPx, IconSize, IconSource, Scale};
use sill_launcher::AppId;
use sill_services::{App, AppIcon, DbusActivation, Listing, Terminal};
use sill_settings::{DockSettings, Magnification};
use std::path::PathBuf;

fn app(id: &str, icon: AppIcon) -> App {
    App {
        id: AppId(id.to_owned()),
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

fn file(id: &str, path: &str) -> App {
    app(id, AppIcon::File(PathBuf::from(path)))
}

#[test]
fn each_entry_picks_its_art() {
    let kind = ItemKind::Pinned(AppId("term".to_owned()));
    let cases = [
        (
            Some(file("term", "/i/term.svg")),
            TileArt::Themed("/i/term.svg".into()),
        ),
        (
            Some(file("term", "/i/term-symbolic.svg")),
            TileArt::Symbolic("/i/term-symbolic.svg".into()),
        ),
        (
            Some(file("term", "i/term.svg")),
            TileArt::Glyph(Icon::Window),
        ),
        (
            Some(app("term", AppIcon::None)),
            TileArt::Glyph(Icon::Window),
        ),
        (None, TileArt::Glyph(Icon::Window)),
        (
            // A desktop id that used to take a shipped icon now takes its own file, like any
            // other app.
            Some(file("com.system76.CosmicTerm", "/i/term.svg")),
            TileArt::Themed("/i/term.svg".into()),
        ),
    ];
    for (entry, want) in cases {
        assert_eq!(tile_art(&kind, entry.as_ref()), want, "{want:?}");
    }
    assert_eq!(
        tile_art(&ItemKind::Trash, None),
        TileArt::Glyph(Icon::Trash)
    );
}

fn cx(style: IconStyle, scale: u32) -> IconContext {
    IconContext {
        look: IconLook {
            style,
            tint: Tint {
                hue: 268.0,
                chroma: 0.06,
            },
            scheme: ds::Scheme::Light,
        },
        scale: Scale(scale),
        resolver: IconResolver {
            seen: Generation::default(),
        },
    }
}

fn word(source: &IconSource) -> String {
    match source {
        IconSource::Glyph(icon) => format!("glyph {icon:?}"),
        IconSource::Symbolic(e) => format!("symbolic {} {:?}", e.url.as_str(), e.size),
        IconSource::Image(e) => format!("image {} {:?}", e.url.as_str(), e.size),
        IconSource::Status(state) => format!("status {}", state.words()),
    }
}

/// Every kind of art at a fixed, self-contained path: an exact word.
#[test]
fn theme_symbolic_and_glyph_art_draw_their_word_exactly() {
    let dock = DockSettings::default();
    let rest = TileSizes::new(48.0, &dock);
    let cases = [
        (
            TileArt::Themed("/i/firefox.svg".into()),
            IconStyle::Colour,
            "image file:///i/firefox.svg Px(IconPx(28))",
        ),
        (
            TileArt::Themed("/i/firefox.svg".into()),
            IconStyle::Monochrome,
            "image file:///i/firefox.svg Px(IconPx(28))",
        ),
        (
            TileArt::Symbolic("/i/a-symbolic.svg".into()),
            IconStyle::Monochrome,
            "symbolic file:///i/a-symbolic.svg Px(IconPx(28))",
        ),
        (
            TileArt::Glyph(Icon::Trash),
            IconStyle::Monochrome,
            "glyph Trash",
        ),
    ];
    for (art, style, want) in cases {
        let got = drawn(art.clone(), rest, &cx(style, 120));
        assert_eq!(word(&got), want, "{art:?} {style:?}");
    }
}

#[test]
fn icons_take_their_share_of_the_plate_in_whole_pixels() {
    let dock = DockSettings::default();
    let size = |side: f64| match drawn(
        TileArt::Themed("/i/term.svg".into()),
        TileSizes::new(side, &dock),
        &cx(IconStyle::Colour, 120),
    ) {
        IconSource::Image(e) => e.size,
        other => panic!("{other:?}"),
    };
    // A 48 px tile's plate is 38.64 px: the icon 27.8.
    assert_eq!(size(48.0), IconSize::Px(IconPx(28)));
    // Fully magnified (96 px): 55.6.
    assert_eq!(size(96.0), IconSize::Px(IconPx(56)));
}

/// Without magnification the theme icon is loaded once, at the tile's rest size (no shipped
/// per-size files to pick between any more: the source stays whatever `app.icon` names).
#[test]
fn without_magnification_the_rest_size_still_shows_the_file() {
    let dock = DockSettings {
        magnification: Magnification::Off,
        ..DockSettings::default()
    };
    let sizes = TileSizes::new(48.0, &dock);
    let got = drawn(
        TileArt::Themed("/i/photos.svg".into()),
        sizes,
        &cx(IconStyle::Colour, 120),
    );
    assert_eq!(word(&got), "image file:///i/photos.svg Px(IconPx(28))");
}
