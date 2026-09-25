use super::{Drawn, IconContext, TileArt, TileSizes, drawn, tile_art};
use crate::app_icons::cache::Generation;
use crate::app_icons::{AssetDir, IconLook, IconResolver, ShippedApp};
use crate::surfaces::dock::items::ItemKind;
use ds::icon::{IconStyle, Tint};
use ds::{Icon, IconPx, IconSize, IconSource, Scale};
use sill_launcher::AppId;
use sill_services::{App, AppIcon, DbusActivation, Listing, Terminal};
use sill_settings::{DockSettings, Magnification, ShippedIcons};
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
            ShippedIcons::On,
            TileArt::Themed("/i/term.svg".into()),
        ),
        (
            Some(file("term", "/i/term-symbolic.svg")),
            ShippedIcons::On,
            TileArt::Symbolic("/i/term-symbolic.svg".into()),
        ),
        (
            Some(file("term", "i/term.svg")),
            ShippedIcons::On,
            TileArt::Glyph(Icon::Window),
        ),
        (
            Some(app("term", AppIcon::None)),
            ShippedIcons::On,
            TileArt::Glyph(Icon::Window),
        ),
        (None, ShippedIcons::On, TileArt::Glyph(Icon::Window)),
        (
            Some(file("com.system76.CosmicTerm", "/i/term.svg")),
            ShippedIcons::On,
            TileArt::Shipped(ShippedApp::Terminal),
        ),
        (
            Some(app("org.gnome.Nautilus", AppIcon::None)),
            ShippedIcons::On,
            TileArt::Shipped(ShippedApp::Files),
        ),
        (
            Some(file("com.system76.CosmicTerm", "/i/term.svg")),
            ShippedIcons::Off,
            TileArt::Themed("/i/term.svg".into()),
        ),
    ];
    for (entry, shipped, want) in cases {
        assert_eq!(tile_art(&kind, entry.as_ref(), shipped), want, "{want:?}");
    }
    assert_eq!(
        tile_art(&ItemKind::Trash, None, ShippedIcons::On),
        TileArt::Glyph(Icon::Trash)
    );
}

fn cx(style: IconStyle, scale: u32, assets: Option<&AssetDir>) -> IconContext<'_> {
    IconContext {
        look: IconLook {
            style,
            tint: Tint {
                hue: 268.0,
                chroma: 0.06,
            },
        },
        scale: Scale(scale),
        assets,
        resolver: IconResolver {
            seen: Generation::default(),
        },
    }
}

fn word(drawn: &Drawn) -> String {
    let source = |source: &IconSource| match source {
        IconSource::Glyph(icon) => format!("glyph {icon:?}"),
        IconSource::Symbolic(e) => format!("symbolic {} {:?}", e.url.as_str(), e.size),
        IconSource::Image(e) => format!("image {} {:?}", e.url.as_str(), e.size),
    };
    match drawn {
        Drawn::Plated(s) => format!("plated {}", source(s)),
        Drawn::Bare(s) => format!("bare {}", source(s)),
    }
}

/// A 48 px tile that magnifies to 96: our icon bare at the tile's side from the file for 96 at
/// the scale, theme icons on the plate at their 72 % share. Colour and Muted are plain files;
/// the first render of a Monochrome one shows its untinted source (the cache fills behind it).
#[test]
fn a_tile_draws_its_art_at_its_size_in_its_style() {
    let dir = AssetDir(PathBuf::from("/q/apps"));
    let dock = DockSettings::default();
    let rest = TileSizes::new(48.0, &dock);
    let cases = [
        (
            TileArt::Shipped(ShippedApp::Terminal),
            IconStyle::Colour,
            120,
            "bare image file:///q/apps/terminal/96.png Px(IconPx(48))",
        ),
        (
            TileArt::Shipped(ShippedApp::Terminal),
            IconStyle::Muted,
            180,
            "bare image file:///q/apps/terminal/muted/256.png Px(IconPx(48))",
        ),
        (
            TileArt::Shipped(ShippedApp::Mail),
            IconStyle::Monochrome,
            120,
            "bare image file:///q/apps/mail/monochrome/96.png Px(IconPx(48))",
        ),
        (
            TileArt::Themed("/i/firefox.svg".into()),
            IconStyle::Colour,
            120,
            "plated image file:///i/firefox.svg Px(IconPx(28))",
        ),
        (
            TileArt::Themed("/i/firefox.svg".into()),
            IconStyle::Monochrome,
            120,
            "plated image file:///i/firefox.svg Px(IconPx(28))",
        ),
        (
            TileArt::Symbolic("/i/a-symbolic.svg".into()),
            IconStyle::Monochrome,
            120,
            "plated symbolic file:///i/a-symbolic.svg Px(IconPx(28))",
        ),
        (
            TileArt::Glyph(Icon::Trash),
            IconStyle::Monochrome,
            120,
            "plated glyph Trash",
        ),
    ];
    for (art, style, scale, want) in cases {
        let got = drawn(art.clone(), rest, &cx(style, scale, Some(&dir)));
        assert_eq!(word(&got), want, "{art:?} {style:?} {scale}");
    }
    assert_eq!(
        drawn(
            TileArt::Shipped(ShippedApp::Files),
            rest,
            &cx(IconStyle::Colour, 120, None)
        ),
        Drawn::Plated(IconSource::Glyph(Icon::Window)),
        "no quire files found: the fallback glyph"
    );
}

/// Without magnification the file is picked for the tile's rest size: 48 is 48.png at 1x,
/// 72.png at 1.5x, 96.png at 2x and at 1.75x (the nearest drawn size up).
#[test]
fn without_magnification_the_rest_size_picks_the_file() {
    let dir = AssetDir(PathBuf::from("/q/apps"));
    let dock = DockSettings {
        magnification: Magnification::Off,
        ..DockSettings::default()
    };
    let sizes = TileSizes::new(48.0, &dock);
    for (scale, file) in [(120, 48), (180, 72), (240, 96), (210, 96)] {
        let got = drawn(
            TileArt::Shipped(ShippedApp::Photos),
            sizes,
            &cx(IconStyle::Colour, scale, Some(&dir)),
        );
        assert_eq!(
            word(&got),
            format!("bare image file:///q/apps/photos/{file}.png Px(IconPx(48))"),
            "{scale}"
        );
    }
}

#[test]
fn icons_take_their_share_of_the_plate_in_whole_pixels() {
    let dock = DockSettings::default();
    let size = |side: f64| match drawn(
        TileArt::Themed("/i/term.svg".into()),
        TileSizes::new(side, &dock),
        &cx(IconStyle::Colour, 120, None),
    ) {
        Drawn::Plated(IconSource::Image(e)) => e.size,
        other => panic!("{other:?}"),
    };
    // A 48 px tile's plate is 38.64 px: the icon 27.8.
    assert_eq!(size(48.0), IconSize::Px(IconPx(28)));
    // Fully magnified (96 px): 55.6.
    assert_eq!(size(96.0), IconSize::Px(IconPx(56)));
}
