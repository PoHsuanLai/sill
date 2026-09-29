//! A dock tile for an app with its own hicolor icon (COSMIC Terminal) draws that icon on
//! quire's neutral plate under every icon style, tinted to the same hue under Muted or
//! Monochrome. Every entry, COSMIC's own included, takes the theme's icon.

mod dock_support;

use dioxus::prelude::*;
use ds::icon::{IconStyle, Tint};
use sill_launcher::AppId;
use sill_model::units::Cpx;
use sill_settings::{SettingsFeed, ShellFile};
use sill_ui_kit::app_icons::IconLook;
use sill_ui_kit::surfaces::dock::machine::{TileApp, TileId};
use sill_ui_kit::surfaces::dock::tile::{Tile, TileLook};
use sill_ui_kit::surfaces::dock::{DockItem, ItemKind};

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

fn render(style: IconStyle) -> String {
    let (services, _feeds) = dock_support::services();
    let file = ShellFile::default();
    let look = TileLook(IconLook {
        style,
        tint: Tint {
            hue: 268.0,
            chroma: 0.06,
        },
        scheme: ds::Scheme::Light,
    });
    let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { look });
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(file));
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// Under every style the terminal's own hicolor icon sits on the neutral plate, never bare.
#[test]
fn a_mapped_app_still_draws_its_own_icon_on_the_plate() {
    for style in [IconStyle::Colour, IconStyle::Muted, IconStyle::Monochrome] {
        let html = render(style);
        assert!(
            html.contains("data-family=\"neutral\""),
            "{style:?}: {html}"
        );
        assert!(
            html.contains("data-kind=\"image\""),
            "{style:?}: an image, not the window glyph\n{html}"
        );
        assert!(
            !html.contains("sill-dock-art"),
            "{style:?}: no bare art\n{html}"
        );
    }
}

/// Under Muted or Monochrome the plate itself carries the icon style:
/// `IconView`'s `plate_tint` writes `data-icon-style` and the tinted stops and ink as custom
/// properties (`--plate-base-l` … `--plate-ink-d`), so quire's own stylesheet re-colours the
/// plate the same hue as the icon on it, in both schemes at once.
#[test]
fn the_plate_carries_the_icon_style_under_muted_and_monochrome() {
    for style in [IconStyle::Muted, IconStyle::Monochrome] {
        let html = render(style);
        let word = if style == IconStyle::Muted {
            "muted"
        } else {
            "monochrome"
        };
        assert!(
            html.contains(&format!("data-icon-style=\"{word}\"")),
            "{style:?}\n{html}"
        );
        for var in ["--plate-base-l:", "--plate-deep-l:", "--plate-ink-l:"] {
            assert!(html.contains(var), "{style:?}: {var}\n{html}");
        }
        for var in ["--plate-base-d:", "--plate-deep-d:", "--plate-ink-d:"] {
            assert!(html.contains(var), "{style:?}: {var}\n{html}");
        }
    }
}
