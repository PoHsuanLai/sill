//! What the `windows` thread can be told to do.

use serde::{Deserialize, Serialize};
use sill_launcher::WindowKey;

use crate::ids::WorkspaceKey;

/// A request to the compositor, carried out on the thread's own connection.
///
/// There is no minimize-rectangle command (`SetMinimizeRect`): `set_rectangle` needs the dock's
/// `wl_surface`, which lives on shell-host's connection, not this one, so it is left out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum WlCommand {
    /// Activate (and unminimize) a toplevel on the thread's seat.
    Activate(WindowKey),
    /// Ask a toplevel to close.
    Close(WindowKey),
    Minimize(WindowKey),
    Unminimize(WindowKey),
    /// Make a workspace the active one of its group.
    ActivateWorkspace(WorkspaceKey),
    /// Reorder: put `workspace` before or after `other` (bar drag reorder).
    MoveWorkspace {
        workspace: WorkspaceKey,
        relative: Relative,
        other: WorkspaceKey,
    },
    /// Pin a workspace (needs [`crate::windows::WorkspaceCap::Pin`]).
    PinWorkspace(WorkspaceKey),
    /// Unpin a workspace (needs [`crate::windows::WorkspaceCap::Pin`]).
    UnpinWorkspace(WorkspaceKey),
    /// Move a toplevel onto a workspace, on that workspace's output (needs the toplevel
    /// manager's `move_to_ext_workspace`, version 4).
    MoveToplevelToWorkspace {
        toplevel: WindowKey,
        workspace: WorkspaceKey,
    },
}

/// Which side of another workspace a moved one lands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relative {
    Before,
    After,
}

#[cfg(test)]
mod tests {
    use sill_launcher::WindowKey;

    use super::WlCommand;
    use crate::ids::WorkspaceKey;

    #[test]
    fn the_amended_commands_have_pinned_wire_lines() {
        let cases = [
            (
                WlCommand::PinWorkspace(WorkspaceKey(3)),
                r#"{"kind":"pin_workspace","v":3}"#,
            ),
            (
                WlCommand::UnpinWorkspace(WorkspaceKey(3)),
                r#"{"kind":"unpin_workspace","v":3}"#,
            ),
            (
                WlCommand::MoveToplevelToWorkspace {
                    toplevel: WindowKey(7),
                    workspace: WorkspaceKey(2),
                },
                r#"{"kind":"move_toplevel_to_workspace","v":{"toplevel":7,"workspace":2}}"#,
            ),
        ];
        for (command, line) in cases {
            assert_eq!(serde_json::to_string(&command).ok().as_deref(), Some(line));
            let back: WlCommand = serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(back, command, "{line}");
        }
    }
}
