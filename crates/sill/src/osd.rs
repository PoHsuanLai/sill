//! The daemon's half of the OSD (design/20 §1.7): one indicator per output, spawned hidden and
//! kept warm, and the decision of which changes show and where.
//!
//! **Which changes show.** Only the ones sill makes itself: `sill display volume ...` and
//! `sill display brightness ...`, which are what the volume and brightness keys run (`dist/
//! cosmic`), so a key press always shows. A level another app sets (a player's own slider,
//! `pactl`, the monitor's buttons) never does: macOS shows its indicator for the keys only, and
//! following every PipeWire change would flash it whenever an app ramps its stream. `sill debug
//! osd` shows a made-up change and moves nothing real. `osd.enabled = "off"` silences all three.
//!
//! **Where.** Volume: the output of the active window (the daemon's stand-in for the output under
//! the pointer), else the first output. Brightness: the outputs the key moved, chosen the way the
//! displays service chooses them (`display.brightness_keys_target`: that same output, or every
//! output), less those with no brightness to show.

use shell_host::{HostCtx, OutputId, OutputInfo, Root};
use sill_ipc::{DisplayRequest, FakeMute, FakeOsd, OsdTopic, Response};
use sill_model::ids::OutputName;
use sill_model::units::Percent;
use sill_model::volume::Mute;
use sill_services::displays::brightness::targets;
use sill_services::{Displays, KeysTarget, Services};
use sill_settings::{OsdEnabled, ShellFile};
use sill_surfaces::surfaces::osd::{Level, LevelSource, OsdKind, OsdLink, OsdProps, osd_layer};
use tokio::sync::watch;

use crate::msg::ShellMsg;
use crate::outputs::Outputs;

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

/// A made-up change's kind and level.
pub fn fake_of(fake: FakeOsd) -> (OsdKind, Level) {
    let kind = match fake.topic {
        OsdTopic::Volume => OsdKind::Volume,
        OsdTopic::Brightness => OsdKind::Brightness,
    };
    let mute = match fake.mute {
        FakeMute::Audible => Mute::Audible,
        FakeMute::Muted => Mute::Muted,
    };
    let percent = Percent(u16::from(fake.percent.min(100)));
    (kind, Level { percent, mute })
}

/// Show `kind` from `level` on the outputs it belongs to, unless `osd.enabled` is off.
fn show(
    link: &OsdLink,
    services: &Services,
    outputs: &Outputs,
    file: &ShellFile,
    kind: OsdKind,
    level: LevelSource,
) {
    if file.osd.enabled == OsdEnabled::Off {
        return;
    }
    let infos: Vec<OutputInfo> = outputs.outputs.iter().map(|(i, _)| i.clone()).collect();
    let focus = crate::displays::focused_output(&services.cosmic_wl.state.borrow().toplevels);
    let keys = crate::displays::policy_from(&file.display).keys_target;
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

/// `sill debug osd`: show a made-up change.
pub fn fake(
    link: &OsdLink,
    services: &Services,
    outputs: &Outputs,
    file: &ShellFile,
    fake: FakeOsd,
) -> Response {
    let (kind, level) = fake_of(fake);
    show(
        link,
        services,
        outputs,
        file,
        kind,
        LevelSource::Fixed(level),
    );
    Response::Done
}

/// Spawn an indicator, hidden, for every output whose row has none yet (the launcher's warm
/// pattern). `root` adds the contexts every surface gets.
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    settings: &watch::Sender<ShellFile>,
    link: &OsdLink,
    root: impl Fn(Root) -> Root,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, s)| s.osd.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    let file = settings.borrow().clone();
    for info in missing {
        let props = OsdProps {
            output: info.clone(),
        };
        let root = root(Root::with_props(sill_surfaces::Osd, props)).context(link.clone());
        let config = osd_layer(&info, &file.osd);
        let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
        surfaces.osd = crate::daemon::spawn_hidden(ctx, config, root, &info);
        outputs.set(info, surfaces);
    }
}

/// The indicator's namespace on `info`, which `debug stats` reports.
pub fn osd_namespace(info: &OutputInfo) -> String {
    osd_layer(info, &Default::default()).namespace
}

#[cfg(test)]
mod tests;
