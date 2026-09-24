//! Pure launch planning: the argument vector and the scope's unit name.

#![allow(unused_variables, dead_code)] // Freeze stubs

use sill_launcher::AppId;

use crate::apps::{App, ExecError};
use crate::launch::LaunchRequest;
use crate::launch::model::ScopeUnit;

/// The full command: `systemd-run --user --scope --collect --unit=<unit> -- <exec...>`, with
/// the terminal wrapper (`terminal`, e.g. `["cosmic-term", "-e"]`) in front of `Exec` when the
/// entry has `Terminal=true`, and the request's action's `Exec` when it names one.
pub fn launch_argv(
    app: &App,
    request: &LaunchRequest,
    unit: &ScopeUnit,
    terminal: &[String],
) -> Result<Vec<String>, ExecError> {
    todo!("M2 launch: expand_exec + wrapper + systemd-run prefix")
}

/// `app-sill-<id>-<nonce>.scope`, the id escaped the way `systemd-escape` escapes a unit name
/// component (XDG "Desktop Entry / systemd integration" naming).
pub fn scope_unit(app: &AppId, nonce: u32) -> ScopeUnit {
    todo!("M2 launch")
}
