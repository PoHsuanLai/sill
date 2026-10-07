//! PaneSwitcher's specimens: at rest on each pane, a grid of tiles and a list of networks.

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::components::lists::preview::switcher::PaneSwitcher;
use ds::components::lists::row::size::RowSize;
use ds::motion::pane_slide::Pane;
use ds::prelude::*;
use sill_shell_kit::control_center::module_tile_kind::TileSpan;
use sill_shell_kit::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct PaneProps {
    pub shown: Pane,
}

pub const CASES: [(Pane, &str); 2] = [(Pane::Root, "panes-root"), (Pane::Detail, "panes-detail")];

/// A switcher at rest on `shown`, mounted there.
pub fn panes(props: PaneProps) -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Popover, stylesheet: Inject::Host,
            PaneSwitcher {
                shown: props.shown,
                root: rsx! {
                    ModuleGrid {
                        ModuleTile { glyph: Icon::Wifi, title: "Wi-Fi", status: "Home", value: Check::On, onclick: |_| {}, on_detail: |_| {} }
                        ModuleTile { glyph: Icon::Moon, title: "Focus", value: Check::Off, onclick: |_| {} }
                        ModuleTile { glyph: Icon::Play, title: "Now Playing", value: Check::Off, span: TileSpan::Full, onclick: |_| {} }
                    }
                },
                detail: rsx! {
                    List::<&'static str> {
                        label: "Networks",
                        items: vec![ListItem::row("Home", "Home", rsx! { Row { leading: RowLeading::Icon(Icon::Wifi), title: "Home", accessory: Accessory::Check(Check::On), size: RowSize::Settings, onclick: |_| {} } })],
                    }
                },
            }
        }
    }
}
