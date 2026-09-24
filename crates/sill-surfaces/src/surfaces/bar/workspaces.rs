//! The workspace indicator: one pill per workspace of this output's group, active sealed, drag to reorder (`move_before`/`move_after`), right-click for Edit Space.

#![allow(unused_variables, dead_code)] // Freeze stubs

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The workspace indicator: one pill per workspace of this output's group, active sealed, drag to reorder (`move_before`/`move_after`), right-click for Edit Space.
#[component]
pub fn WorkspaceIndicator(output: OutputInfo) -> Element {
    todo!("M1 bar-ui")
}
