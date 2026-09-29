//! sill's own polkit agent on the daemon's side: one warm hidden prompt surface per output. The
//! daemon tells the service to register at start; it stays passive until then.
//!
//! Registering an agent is per logind session: nested in another desktop's session, the daemon
//! shares that session, whose own agent already holds it, so polkitd refuses sill's
//! (`Registration::Refused`), and that is the right outcome there. Acceptance runs the agent on
//! a private bus with a stand-in Authority, never on the system bus.

use shell_host::{HostCtx, OutputInfo, Root};
use sill_surfaces::surfaces::polkit_prompt::{PolkitPromptProps, polkit_layer};

use crate::daemon::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn a hidden polkit prompt for every output whose row lacks one (after the power menu, so
/// on the Overlay layer the prompt draws over it).
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    root: impl Fn(Root) -> Root,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, s)| s.polkit_prompt.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    for info in missing {
        let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
        let props = PolkitPromptProps {
            output: info.clone(),
        };
        let root = root(Root::with_props(sill_surfaces::PolkitPrompt, props));
        surfaces.polkit_prompt = crate::daemon::spawn_hidden(ctx, polkit_layer(&info), root, &info);
        outputs.set(info, surfaces);
    }
}

/// The prompt's namespace on `info`, which `debug stats` reports.
pub fn namespace(info: &OutputInfo) -> String {
    polkit_layer(info).namespace
}
