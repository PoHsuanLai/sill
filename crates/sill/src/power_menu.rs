//! The daemon's power menu half: one power menu surface per output, spawned warm and hidden (the
//! launcher's pattern).
//!
//! **Dry run.** A chosen action goes to the session service, which takes it through the daemon's
//! `SessionActions` (`sill_services::env`): logind's, or, in a debug build started with `sill run
//! --session-dry-run RECORD`, a dry run that only prints and records it. Each surface prints
//! `power_menu.backend=dry-run` as it mounts in that case, which `dev/accept-power-menu.sh` checks
//! before it presses Enter.

use shell_host::{HostCtx, OutputInfo, Root};
use sill_surfaces::surfaces::power_menu::{PowerMenuProps, power_menu_layer};

use crate::daemon::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn a hidden power menu for every output whose row has none yet, each root built by `root`
/// (the daemon's `surface_root`, which adds the links).
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    root: impl Fn(Root) -> Root,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, surfaces)| surfaces.power_menu.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    for info in missing {
        let config = power_menu_layer(&info);
        let namespace = config.namespace.clone();
        let props = PowerMenuProps {
            output: info.clone(),
        };
        let surface = root(Root::with_props(sill_surfaces::PowerMenu, props));
        match ctx.spawn_layer(config, surface) {
            Ok(id) => {
                ctx.set_visibility(id, shell_host::Visibility::Hidden);
                let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
                surfaces.power_menu = Some(id);
                outputs.set(info, surfaces);
            }
            Err(error) => eprintln!("sill: spawning {namespace} on {:?}: {error}", info.name),
        }
    }
}
