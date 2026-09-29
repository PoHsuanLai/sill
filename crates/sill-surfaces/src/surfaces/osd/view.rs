//! The indicator's render: one transparent Osd root (design/20 §1.7) holding quire's `Osd` card.
//! The card paints its own tinted frame, owns its own entrance and exit fade, and calls
//! `on_hidden` once the exit has settled, so `drive.rs` can unmap the surface then; sill's own
//! hold machine (`machine.rs`) only decides when the card is shown.

use dioxus::prelude::*;
use ds::{BlurState, Ds, LevelLook, Material, Osd, OsdMetrics, OsdPosition, Px, RootChrome, Shown};
use shell_host::OutputInfo;
use sill_model::ids::OutputName;

use crate::hooks::{use_appearance, use_ds_scale, use_service, use_service_part, use_settings};
use crate::surfaces::launcher::panel::active_look;
use crate::surfaces::launcher::presence::Visible;
use crate::surfaces::osd::drive::{times, use_follow_position, use_osd};
use crate::surfaces::osd::layer::CARD_ID;
use crate::surfaces::osd::link::{OsdKind, OsdLink};
use crate::surfaces::osd::reading::{card_level, level_of, title};
use sill_services::{CosmicWlService, DisplaysService, VolumeService};

/// The indicator's stylesheet (lint-clean under `Profile::Strict`, `tests/osd_lint.rs`).
pub(crate) const OSD_CSS: &str = include_str!("../../style/osd.css");

/// The level's look: the capsule, the user's pick. Not a setting: the OSD has no look key, and
/// this is the one place.
const LOOK: LevelLook = LevelLook::Capsule;

/// `sill_settings::OsdPosition` as quire's `ds::OsdPosition`.
fn position_of(position: sill_settings::OsdPosition) -> OsdPosition {
    match position {
        sill_settings::OsdPosition::TopRight => OsdPosition::TopRight,
        sill_settings::OsdPosition::BottomCentre => OsdPosition::BottomCentre,
    }
}

/// The indicator on `output`.
pub(crate) fn osd(output: OutputInfo) -> Element {
    let ds_scale = use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(OutputName);
    let look = active_look(
        &use_service_part::<CosmicWlService, _>(|wl| wl.workspaces.clone())(),
        on.as_ref(),
        &file.spaces,
    );
    let appearance = environment.settings.appearance.appearance();
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
        Visible::Yes => Shown::Visible,
        Visible::No => Shown::Hidden,
    };
    let tint = environment.tint_alpha();
    let stack = environment.material_stack();
    rsx! {
        Ds {
            scale: ds_scale,
            appearance,
            system: environment.system,
            look,
            material: Material::Osd,
            chrome: Some(RootChrome::Transparent),
            blur: BlurState::Available,
            tint_alpha: Some(tint),
            stack: Some(stack),
            style { {OSD_CSS} }
            div {
                style: OsdMetrics { margin: Px(f32::from(file.osd.margin_px.0)) }.style_attr(),
                Osd {
                    shown,
                    label: Some(title(kind, &volume, &displays, on.as_ref())),
                    level: Some(card_level(kind, level)),
                    position: position_of(file.osd.position),
                    look: LOOK,
                    id: Some(CARD_ID.to_owned()),
                    on_hidden: drawn.on_hidden,
                }
            }
        }
    }
}
