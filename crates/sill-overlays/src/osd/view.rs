//! The indicator's render: one transparent Osd root (design/20 §1.7) holding quire's `Osd` card.
//! The card paints its own tinted frame, owns its own entrance and exit fade, and calls
//! `on_hidden` once the exit has settled, so `io.rs` can unmap the surface then; sill's own
//! hold machine (`step.rs`) only decides when the card is shown.

use dioxus::prelude::*;
use ds::components::controls::level_indicator::LevelStyle;
use ds::prelude::*;
use ds::root::common::Common;
use shell_host::{OutputInfo, Visibility};
use sill_model::ids::OutputName;
use sill_shell_kit::osd::OsdPosition;
use sill_shell_kit::prelude::*;

use super::io::{times, use_osd};
use super::layer::{CARD_ID, layer};
use super::reading::{card_level, level_of, title};
use super::tokens::OsdTokens;
use sill_services::{DisplaysService, VolumeService};
use sill_ui_kit::hooks::{use_follow_place, use_service, use_surface_tokens};
use sill_ui_kit::links::{OsdKind, OsdLink};
use sill_ui_kit::root::{SurfaceRoot, use_root_style};

/// The indicator's stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const OSD_CSS: &str = include_str!("style.css");

/// The level's style: the bezel's sixteen squares, as the Mac's volume and brightness keys draw
/// them (quire design/29 section 16); the capsule was the iOS pill. Not a setting: the OSD has no
/// look key, and this is the one place.
const LOOK: LevelStyle = LevelStyle::Discrete;

/// The indicator on `output`.
pub(super) fn osd(output: OutputInfo) -> Element {
    let on = output.name.clone().map(OutputName);
    let resolved = use_surface_tokens(use_root_style().styles, |length| OsdTokens::read(length));
    let place = layer(&output, &resolved.get());
    use_follow_place(place.size, place.margin);
    let link = use_context::<OsdLink>();
    let drawn = use_osd(&link, output.id, times());
    let volume = use_service::<VolumeService>()();
    let displays = use_service::<DisplaysService>()();
    let kind = drawn.change.as_ref().map_or(OsdKind::Volume, |c| c.kind);
    let level = drawn
        .change
        .as_ref()
        .and_then(|c| level_of(c.kind, c.level, &volume, &displays, on.as_ref()));
    let shown = match drawn.phase.visible() {
        Visibility::Shown => Shown::Visible,
        Visibility::Hidden => Shown::Hidden,
    };
    rsx! {
        SurfaceRoot {
            output,
            material: Material::Osd,
            chrome: Some(RootChrome::Transparent),
            AppStyle { css: OSD_CSS }
            div { onmounted: move |event| resolved.mounted(event),
                Osd {
                    shown,
                    label: Some(title(kind, &volume, &displays, on.as_ref())),
                    level: Some(card_level(kind, level)),
                    position: OsdPosition::TopRight,
                    style: LOOK,
                    common: Common { id: Some(CARD_ID.to_owned()), ..Common::default() },
                    on_hidden: drawn.on_hidden,
                }
            }
        }
    }
}
