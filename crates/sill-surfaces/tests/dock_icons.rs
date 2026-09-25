//! A dock tile for an app quire ships an icon for (COSMIC Terminal) renders that icon as an
//! `Image` source, bare over the tile, under every icon style: the Colour, Muted and Monochrome
//! files for the magnified size (the Monochrome one untinted on its first render, while the
//! cache makes the tinted raster). `dock.shipped_icons = off` puts the theme's icon back on the
//! neutral plate.

mod dock_support;

use dioxus::prelude::*;
use ds::icon::{IconStyle, Tint};
use sill_launcher::AppId;
use sill_services::Cpx;
use sill_settings::{SettingsFeed, ShellFile, ShippedIcons};
use sill_surfaces::app_icons::IconLook;
use sill_surfaces::surfaces::dock::machine::{TileApp, TileId};
use sill_surfaces::surfaces::dock::tile::{Tile, TileLook};
use sill_surfaces::surfaces::dock::{DockItem, ItemKind};

#[derive(Props, Clone, PartialEq)]
struct HarnessProps {
    look: TileLook,
}

/// The terminal's tile at rest, with `look` provided the way the dock provides it.
#[allow(non_snake_case)] // A component.
fn Harness(props: HarnessProps) -> Element {
    use_context_provider(|| Signal::new(props.look));
    let item = DockItem {
        tile: TileId(1),
        kind: ItemKind::Pinned(AppId(dock_support::TERM.to_owned())),
        app: TileApp::NotRunning,
    };
    rsx! {
        Tile { item, left: Cpx(0), size: Cpx(4800), lift: Cpx(0), badge: None }
    }
}

fn render(style: IconStyle, shipped: ShippedIcons) -> String {
    let (services, _feeds) = dock_support::services();
    let mut file = ShellFile::default();
    file.dock.shipped_icons = shipped;
    let look = TileLook(IconLook {
        style,
        tint: Tint {
            hue: 268.0,
            chroma: 0.06,
        },
    });
    let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { look });
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(file));
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn a_mapped_app_draws_quires_icon_under_every_style() {
    let cases = [
        (IconStyle::Colour, "/terminal/96.png"),
        (IconStyle::Muted, "/terminal/muted/96.png"),
        (IconStyle::Monochrome, "/terminal/monochrome/96.png"),
    ];
    for (style, file) in cases {
        let html = render(style, ShippedIcons::On);
        assert!(
            html.contains("class=\"sill-dock-art\""),
            "{style:?}: bare\n{html}"
        );
        assert!(
            html.contains("data-kind=\"image\""),
            "{style:?}: an image\n{html}"
        );
        assert!(html.contains(file), "{style:?}: {file}\n{html}");
        assert!(
            !html.contains("data-family="),
            "{style:?}: no plate\n{html}"
        );
    }
}

#[test]
fn shipped_icons_off_keeps_the_theme_icon_on_the_plate() {
    let html = render(IconStyle::Colour, ShippedIcons::Off);
    assert!(html.contains("data-family=\"neutral\""), "{html}");
    assert!(!html.contains("sill-dock-art"), "{html}");
    assert!(!html.contains("/terminal/"), "{html}");
}
