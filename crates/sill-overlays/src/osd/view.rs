//! The indicator's render: one transparent Osd root (design/20 §1.7) holding quire's `Osd` card.
//! The card paints its own tinted frame, owns its own entrance and exit fade, and calls
//! `on_hidden` once the exit has settled, so `io.rs` can unmap the surface then; sill's own
//! hold machine (`step.rs`) only decides when the card is shown.

use dioxus::prelude::*;
use ds::components::controls::level_indicator::LevelStyle;
use ds::prelude::*;
use ds::root::common::Common;
use ds_shell::osd::OsdPosition;
use ds_shell::prelude::*;
use ds_shell::tokens::osd::OsdMetrics;
use shell_host::{OutputInfo, Visibility};
use sill_model::ids::OutputName;

use super::io::{times, use_follow_position, use_osd};
use super::layer::CARD_ID;
use super::reading::{card_level, level_of, title};
use sill_services::{DisplaysService, VolumeService};
use sill_ui_kit::hooks::{use_service, use_settings};
use sill_ui_kit::links::{OsdKind, OsdLink};
use sill_ui_kit::root::SurfaceRoot;

/// The indicator's stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const OSD_CSS: &str = include_str!("style.css");

/// The level's style: the capsule, the user's pick. Not a setting: the OSD has no look key, and
/// this is the one place.
const LOOK: LevelStyle = LevelStyle::Continuous;

/// `sill_settings::OsdPosition` as quire's `ds_shell::osd::OsdPosition`.
fn position_of(position: sill_settings::OsdPosition) -> OsdPosition {
    match position {
        sill_settings::OsdPosition::TopRight => OsdPosition::TopRight,
        sill_settings::OsdPosition::BottomCentre => OsdPosition::BottomCentre,
    }
}

/// The indicator on `output`.
pub(super) fn osd(output: OutputInfo) -> Element {
    let file = use_settings()();
    let on = output.name.clone().map(OutputName);
    let link = use_context::<OsdLink>();
    use_follow_position(&file.osd);
    let drawn = use_osd(&link, output.id, times(&file.osd));
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
            output: output.clone(),
            material: Material::Osd,
            chrome: Some(RootChrome::Transparent),
            AppStyle { css: OSD_CSS }
            div {
                style: OsdMetrics { margin: Px(f32::from(file.osd.margin_px.0)) }.style_attr(),
                Osd {
                    shown,
                    label: Some(title(kind, &volume, &displays, on.as_ref())),
                    level: Some(card_level(kind, level)),
                    position: position_of(file.osd.position),
                    style: LOOK,
                    common: Common { id: Some(CARD_ID.to_owned()), ..Common::default() },
                    on_hidden: drawn.on_hidden,
                }
            }
        }
    }
}
