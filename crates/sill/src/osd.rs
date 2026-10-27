//! The daemon's half of the OSD (design/20 §1.7): the decision of which changes show and where.
//! The indicator per output is spawned like any `Surface` (`sill_overlays::Osd`).
//!
//! **Which changes show.** Only the ones sill makes itself: `sill display volume ...` and
//! `sill display brightness ...`, which are what the volume and brightness keys run (bound
//! in the compositor's shortcuts), so a key press always shows. A level another app sets (a player's own slider,
//! `pactl`, the monitor's buttons) never does: macOS shows its indicator for the keys only, and
//! following every PipeWire change would flash it whenever an app ramps its stream. `sill debug
//! osd` shows a made-up change and moves nothing real.
//!
//! **Where.** Volume: the output of the active window (the daemon's stand-in for the output under
//! the pointer), else the first output. Brightness: the outputs the key moved, chosen the way the
//! displays service chooses them (`display.brightness_keys_target`: that same output, or every
//! output), less those with no brightness to show.

use crate::outputs::Outputs;
use shell_host::{OutputId, OutputInfo};
use sill_ipc::act::DisplayRequest;
use sill_ipc::response::Response;
use sill_model::displays::brightness::targets;
use sill_model::displays::{Displays, KeysTarget};
use sill_model::ids::OutputName;
use sill_model::units::Percent;
use sill_model::volume::Mute;
use sill_settings::ShellFile;
use sill_ui_kit::links::{Level, LevelSource, OsdKind, OsdLink};
use sill_ui_kit::services::Services;

/// The outputs that show a `kind` change whose level comes from `level`: see the module doc.
pub fn osd_outputs(
    kind: OsdKind,
    level: LevelSource,
    keys: KeysTarget,
    focus: Option<&OutputName>,
    outputs: &[OutputInfo],
    displays: &Displays,
) -> Vec<OutputId> {
    let named: Vec<(OutputName, OutputId)> = outputs
        .iter()
        .filter_map(|info| Some((OutputName(info.name.clone()?), info.id)))
        .collect();
    let names: Vec<OutputName> = named.iter().map(|(name, _)| name.clone()).collect();
    let id_of = |name: &OutputName| named.iter().find(|(n, _)| n == name).map(|(_, id)| *id);
    match kind {
        OsdKind::Volume => targets(KeysTarget::PointerOutput, focus, &names)
            .into_iter()
            .filter_map(id_of)
            .chain(outputs.first().map(|info| info.id))
            .take(1)
            .collect(),
        OsdKind::Brightness => targets(keys, focus, &names)
            .into_iter()
            .filter(|name| match level {
                LevelSource::Fixed(_) => true,
                LevelSource::Live => displays.brightness(&displays.target(name)).is_some(),
            })
            .filter_map(id_of)
            .collect(),
    }
}

/// What a display request changes.
pub fn kind_of(request: DisplayRequest) -> OsdKind {
    match request {
        DisplayRequest::Brightness(_) => OsdKind::Brightness,
        DisplayRequest::Volume(_) => OsdKind::Volume,
    }
}

/// A made-up change's level: `percent` is held to 0-100.
pub fn fake_level(percent: Percent, mute: Mute) -> Level {
    let percent = Percent(percent.0.min(100));
    Level { percent, mute }
}

/// Show `kind` from `level` on the outputs it belongs to.
fn show(
    link: &OsdLink,
    services: &Services,
    outputs: &Outputs,
    file: &ShellFile,
    kind: OsdKind,
    level: LevelSource,
) {
    let infos: Vec<OutputInfo> = outputs.outputs.iter().map(|(i, _)| i.clone()).collect();
    let focus = services.windows.state.borrow().toplevels.focused_output();
    let keys = crate::policy::displays::policy_from(&file.display).keys_target;
    let ids = osd_outputs(
        kind,
        level,
        keys,
        focus.as_ref(),
        &infos,
        &services.displays.state.borrow(),
    );
    link.show(kind, ids, level);
}

/// `sill display ...` was carried out (`response`): show its change.
pub fn after_display(
    link: &OsdLink,
    services: &Services,
    outputs: &Outputs,
    file: &ShellFile,
    request: DisplayRequest,
    response: &Response,
) {
    if *response == Response::Done {
        show(
            link,
            services,
            outputs,
            file,
            kind_of(request),
            LevelSource::Live,
        );
    }
}

/// `sill debug osd`: show a made-up change of `kind` at `level`, moving no real level.
pub fn fake(
    link: &OsdLink,
    services: &Services,
    outputs: &Outputs,
    file: &ShellFile,
    kind: OsdKind,
    level: Level,
) {
    show(
        link,
        services,
        outputs,
        file,
        kind,
        LevelSource::Fixed(level),
    );
}

#[cfg(test)]
mod tests;
