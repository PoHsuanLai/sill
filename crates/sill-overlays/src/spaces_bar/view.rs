//! The bar's render: one transparent viewport root holding, under the menu bar's height, quire's
//! `SpacesBar` with the output's Spaces (the one on show marked, a picture on each that has
//! one, over the output's wallpaper) and the plus. quire's bar plays its own drop-in and lift;
//! [`super::io`] follows the link, writes the clicks and publishes where the cards and the plus
//! lie; this file only draws.

use dioxus::prelude::*;
use ds::prelude::*;
use shell_host::OutputInfo;
use sill_services::AppsService;

use super::io::{use_bar, use_unmap};
use super::layer::BAR_ID;
use sill_ui_kit::hooks::{use_service, use_settings};
use sill_ui_kit::links::SpacesBarLink;
use sill_ui_kit::root::SurfaceRoot;

pub(super) fn spaces_bar(output: OutputInfo) -> Element {
    rsx! {
        SurfaceRoot {
            output: output.clone(),
            material: Material::Sheet,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            SpacesStrip { output }
        }
    }
}

/// The strip itself, inside the root: quire's bar reads the motion level the root provides.
#[component]
fn SpacesStrip(output: OutputInfo) -> Element {
    let link = use_context::<SpacesBarLink>();
    let bar = use_bar(&link, &output);
    let apps = use_service::<AppsService>();
    let menu_bar = use_settings()().bar.height_px.0;
    let on_hidden = use_unmap();
    let tiles = use_memo({
        let bar = bar.clone();
        move || bar.tiles(&apps())
    });
    let probe = bar.probe;
    let mounted =
        use_hook(|| EventHandler::new(move |event: MountedEvent| probe.on_mounted(event)));
    let (by_pick, by_add) = (bar.clone(), bar.clone());
    let on_layout = bar.on_layout;
    let add_drop = bar.add_drop();
    rsx! {
        div {
            class: "sill-spaces-bar",
            style: "padding-top:{menu_bar}px;",
            SpacesBar {
                label: "Spaces",
                spaces: tiles(),
                shown: bar.shown(),
                expansion: Some(BarExpansion::Expanded),
                add_drop,
                on_layout,
                on_pick: move |key: BarSpaceKey| by_pick.switch(&key),
                on_add: move |()| by_add.add(),
                on_hidden: move |()| on_hidden.call(()),
                common: Common {
                    id: Some(BAR_ID.to_owned()),
                    mounted: Some(mounted),
                    ..Common::default()
                },
            }
        }
    }
}
