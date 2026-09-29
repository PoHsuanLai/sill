//! The app switcher on the daemon's side: one warm hidden switcher surface per output.

use shell_host::{HostCtx, OutputInfo, Root};
use sill_surfaces::surfaces::switcher::{SwitcherPanelProps, switcher_layer};

use crate::daemon::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn a hidden switcher for every output whose row lacks one.
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    root: impl Fn(Root) -> Root,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, s)| s.switcher.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    for info in missing {
        let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
        let props = SwitcherPanelProps {
            output: info.clone(),
        };
        let root = root(Root::with_props(sill_surfaces::SwitcherPanel, props));
        surfaces.switcher =
            crate::daemon::surfaces::spawn_hidden(ctx, switcher_layer(&info), root, &info);
        outputs.set(info, surfaces);
    }
}

/// The switcher's namespace on `info`, which `debug stats` reports.
pub fn namespace(info: &OutputInfo) -> String {
    switcher_layer(info).namespace
}
