//! The indicator's render: a transparent Osd root (it paints nothing, and gives the fade its
//! motion tokens) holding the card box, which fades in and out as a whole; in it, a painted Osd
//! root on the active Space's look is the card, holding the glyph and the level bar.

use dioxus::prelude::*;
use ds::{Availability, BlurState, Ds, Glyph, IconSize, Material, RootChrome, Slider};
use shell_host::OutputInfo;
use sill_services::OutputName;

use crate::hooks::{
    use_appearance, use_displays, use_ds_scale, use_settings, use_volume, use_workspaces,
};
use crate::surfaces::launcher::panel::active_look;
use crate::surfaces::osd::drive::{times, use_follow_position, use_osd};
use crate::surfaces::osd::layer::{CARD_HEIGHT, CARD_ID, CARD_WIDTH};
use crate::surfaces::osd::link::{OsdKind, OsdLink};
use crate::surfaces::osd::reading::{fill, glyph, label, level_of};

/// The indicator's stylesheet (lint-clean under `Profile::Strict`, `tests/osd_lint.rs`).
pub(crate) const OSD_CSS: &str = include_str!("../../style/osd.css");

/// The indicator on `output`.
pub(crate) fn osd(output: OutputInfo) -> Element {
    let ds_scale = use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(OutputName);
    let look = active_look(&use_workspaces()(), on.as_ref(), &file.spaces);
    let appearance = environment.settings.appearance.appearance();
    let motion = ds::resolve(appearance, look.theme, environment.system).motion;
    let link = use_context::<OsdLink>();
    use_follow_position(&file.osd);
    let drawn = use_osd(&link, output.id, times(&file.osd, motion));
    let volume = use_volume()();
    let displays = use_displays()();
    let kind = drawn.change.as_ref().map_or(OsdKind::Volume, |c| c.kind);
    let level = drawn
        .change
        .as_ref()
        .and_then(|c| level_of(c.kind, c.level, &volume, &displays, on.as_ref()));
    let tint = environment.tint_alpha();
    let stack = environment.material_stack();
    rsx! {
        Ds {
            scale: ds_scale,
            appearance,
            system: environment.system,
            look: look.clone(),
            material: Material::Osd,
            chrome: Some(RootChrome::Transparent),
            blur: BlurState::Available,
            style { {OSD_CSS} }
            div { class: "sill-osd-room",
                div {
                    id: CARD_ID,
                    class: "sill-osd",
                    "data-look": drawn.phase.look().slug(),
                    style: "width:{CARD_WIDTH}px",
                    Ds {
                        scale: ds_scale,
                        appearance,
                        system: environment.system,
                        look,
                        material: Material::Osd,
                        blur: BlurState::Available,
                        stylesheet: ds::Inject::Host,
                        tint_alpha: Some(tint),
                        stack: Some(stack),
                        div {
                            class: "sill-osd-row",
                            style: "height:{CARD_HEIGHT}px",
                            Glyph { icon: glyph(kind, level), size: IconSize::Bar }
                            div { class: "sill-osd-level",
                                Slider {
                                    label: label(kind).to_owned(),
                                    value: fill(level),
                                    availability: Availability::Disabled,
                                    onchange: move |_| {},
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
