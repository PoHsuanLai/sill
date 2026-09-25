use super::{Drawn, IconContext, TileArt, TileSizes, drawn, tile_art};
use crate::app_icons::cache::Generation;
use crate::app_icons::{IconLook, IconResolver, ShippedApp};
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

fn cx(style: IconStyle, scale: u32) -> IconContext {
    IconContext {
        look: IconLook {
            style,
            tint: Tint {
                hue: 268.0,
                chroma: 0.06,
            },
        },
        scale: Scale(scale),
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

/// Non-shipped art at a fixed, self-contained path: an exact word.
#[test]
fn theme_symbolic_and_glyph_art_draw_their_word_exactly() {
    let dock = DockSettings::default();
    let rest = TileSizes::new(48.0, &dock);
    let cases = [
        (
            TileArt::Themed("/i/firefox.svg".into()),
            IconStyle::Colour,
            "plated image file:///i/firefox.svg Px(IconPx(28))",
        ),
        (
            TileArt::Themed("/i/firefox.svg".into()),
            IconStyle::Monochrome,
            "plated image file:///i/firefox.svg Px(IconPx(28))",
        ),
        (
            TileArt::Symbolic("/i/a-symbolic.svg".into()),
            IconStyle::Monochrome,
            "plated symbolic file:///i/a-symbolic.svg Px(IconPx(28))",
        ),
        (
            TileArt::Glyph(Icon::Trash),
            IconStyle::Monochrome,
            "plated glyph Trash",
        ),
    ];
    for (art, style, want) in cases {
        let got = drawn(art.clone(), rest, &cx(style, 120));
        assert_eq!(word(&got), want, "{art:?} {style:?}");
    }
}

/// A 48 px tile that magnifies to 96: our icon bare at the tile's side, from the file for 96 at
/// the scale, under quire's real apps directory (the sibling checkout this workspace builds
/// against, `ds_settings`'s development fallback; sill FINDINGS Q71 — the path itself is no
/// longer sill's to build, so the assertion only pins the size and the style's subdirectory).
/// Colour and Muted are shown as they are; the first render of a Monochrome one shows its
/// untinted source (the cache fills behind it, off the blocking pool this plain test has none
/// of).
#[test]
fn a_shipped_tile_loads_the_magnified_sizes_file_in_its_style() {
    let dock = DockSettings::default();
    let rest = TileSizes::new(48.0, &dock);
    let cases = [
        (
            ShippedApp::Terminal,
            IconStyle::Colour,
            120,
            "terminal/96.png",
        ),
        (
            ShippedApp::Terminal,
            IconStyle::Muted,
            180,
            "terminal/muted/256.png",
        ),
        (
            ShippedApp::Mail,
            IconStyle::Monochrome,
            120,
            "mail/monochrome/96.png",
        ),
    ];
    for (app, style, scale, suffix) in cases {
        let got = drawn(TileArt::Shipped(app), rest, &cx(style, scale));
        let word = word(&got);
        assert!(
            word.starts_with("bare image file://") && word.ends_with("Px(IconPx(48))"),
            "{app:?} {style:?} {scale}: {word}"
        );
        assert!(word.contains(suffix), "{app:?} {style:?} {scale}: {word}");
    }
}

/// Without magnification the file is picked for the tile's rest size: 48 is 48.png at 1x,
/// 72.png at 1.5x, 96.png at 2x and at 1.75x (the nearest drawn size up).
#[test]
fn without_magnification_the_rest_size_picks_the_file() {
    let dock = DockSettings {
        magnification: Magnification::Off,
        ..DockSettings::default()
    };
    let sizes = TileSizes::new(48.0, &dock);
    for (scale, file) in [(120, 48), (180, 72), (240, 96), (210, 96)] {
        let got = drawn(
            TileArt::Shipped(ShippedApp::Photos),
            sizes,
            &cx(IconStyle::Colour, scale),
        );
        let word = word(&got);
        assert!(
            word.contains(&format!("photos/{file}.png")),
            "{scale}: {word}"
        );
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
        Drawn::Plated(IconSource::Image(e)) => e.size,
        other => panic!("{other:?}"),
    };
    // A 48 px tile's plate is 38.64 px: the icon 27.8.
    assert_eq!(size(48.0), IconSize::Px(IconPx(28)));
    // Fully magnified (96 px): 55.6.
    assert_eq!(size(96.0), IconSize::Px(IconPx(56)));
}
