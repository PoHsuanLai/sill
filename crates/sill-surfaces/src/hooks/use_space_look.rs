//! `use_space_look`: the SpaceLook of a workspace (design/21 §5 trigger: "`use_workspaces` ->
//! each surface's `Ds { look }`").
//!
//! quire has no `spaces.json` store yet (design/21 §10 names `ds-settings`' writer); until it
//! does this returns the design/21 §4 default for the workspace's index (FINDINGS.md, quire gap).

#![allow(unused_variables, dead_code)] // Freeze stubs

use ds::SpaceLook;
use sill_services::WorkspaceKey;
use sill_settings::SpacesSettings;

/// The look of `workspace` at position `index` in its group, with `spaces` defaults.
pub fn use_space_look(workspace: WorkspaceKey, index: usize, spaces: &SpacesSettings) -> SpaceLook {
    todo!("M1 bar-ui: ds::default_look(index) until quire ships the spaces.json store")
}
