//! The workspaces and the look each one has, published to the workspace-looks service for
//! `org.quire.Workspaces1`, so an app that keeps its Spaces from the desktop paints the active
//! workspace's look. The look is the one every shell surface draws (`sill_ui_kit::look`), as
//! quire's compact JSON.

use sill_model::workspace_looks::{WorkspaceLooksCommand, published};
use sill_ui_kit::look::use_space_look;
use sill_ui_kit::services::Services;

/// Tell the service the workspaces as they now are; nothing is sent when it already holds them.
pub fn publish(services: &Services) {
    let now = {
        let wl = services.windows.state.borrow();
        published(&wl.workspaces, |index| use_space_look(index).to_json())
    };
    if services.workspace_looks.state.borrow().workspaces == now {
        return;
    }
    // A stopped service has nobody to tell.
    let _ = services
        .workspace_looks
        .control
        .send(WorkspaceLooksCommand::Publish(now));
}
