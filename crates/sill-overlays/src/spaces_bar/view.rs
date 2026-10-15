//! The bar's render: one transparent viewport root holding, from the moment Mission Control
//! starts to open until its leave has settled, a strip with quire's `WorkspacePills` (one pill
//! per Space of this output, the one on show marked) centred, and a toolbar `Button` with a plus
//! at the right edge. The strip's top padding is the menu bar's height, so the pills sit under
//! it. [`super::io`] follows the link and writes the clicks; this file only draws.

use dioxus::prelude::*;
use ds::base::press::{PointerButton, Press};
use ds::components::controls::button_model::{Bezel, ImagePosition};
use ds::prelude::*;
use shell_host::OutputInfo;
use sill_model::desktop::BarSpace;
use sill_services::AppsService;
use sill_shell_kit::prelude::*;

use super::io::use_bar;
use sill_ui_kit::hooks::{use_service, use_settings};
use sill_ui_kit::links::SpacesBarLink;
use sill_ui_kit::root::SurfaceRoot;

/// The bar's stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const SPACES_BAR_CSS: &str = include_str!("style.css");

pub(super) fn spaces_bar(output: OutputInfo) -> Element {
    let link = use_context::<SpacesBarLink>();
    let bar = use_bar(&link, &output);
    let apps = use_service::<AppsService>()();
    let menu_bar = use_settings()().bar.height_px.0;
    let drawn = bar.presented.presence.drawn_slug();
    let spaces = bar.spaces(&apps);
    rsx! {
        SurfaceRoot {
            output: output.clone(),
            material: Material::Sheet,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            AppStyle { css: SPACES_BAR_CSS }
            if let Some(drawn) = drawn {
                div {
                    class: "sill-spaces-bar",
                    "data-presence": drawn,
                    "data-pulse": bar.presented.alias.slug(),
                    style: "padding-top:{menu_bar}px;",
                    div { class: "sill-spaces-side" }
                    WorkspacePills { label: "Spaces",
                        for space in spaces {
                            {pill(&bar, space)}
                        }
                    }
                    div { class: "sill-spaces-side",
                        Button {
                            label: "Add Space",
                            bezel: Bezel::Toolbar,
                            image: ImagePosition::Only,
                            icon: Some(IconSource::from(Icon::Plus)),
                            onclick: {
                                let bar = bar.clone();
                                move |press: Press| {
                                    if press.button == PointerButton::Primary {
                                        bar.add();
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

/// The pill for one Space: marked when it is the one on show; a primary press switches to it.
fn pill(bar: &super::io::Bar, space: BarSpace) -> Element {
    let current = if space.current {
        Selection::Selected
    } else {
        Selection::Unselected
    };
    let by_press = bar.clone();
    let pressed = space.clone();
    rsx! {
        WorkspacePill {
            key: "{space.id}",
            label: space.label,
            current,
            onclick: move |press: Press| {
                if press.button == PointerButton::Primary {
                    by_press.switch(&pressed);
                }
            },
        }
    }
}
