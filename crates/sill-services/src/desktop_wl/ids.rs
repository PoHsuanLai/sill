//! The keys and ids the wire client hands out, in sill's own types.

use desktop_client::workspace::Workspace as WireWorkspace;
use desktop_client::{
    GroupKey as WireGroup, OutputName as WireOutput, ToplevelKey, WorkspaceKey as WireWorkspaceKey,
};
use porter_core::SpaceId;
use sill_launcher::WindowKey;
use sill_model::ids::{GroupKey, OutputName, WorkspaceKey};

pub(super) fn window_key(key: ToplevelKey) -> WindowKey {
    WindowKey(key.get())
}

pub(super) fn toplevel_key(key: WindowKey) -> ToplevelKey {
    ToplevelKey::new(key.0)
}

pub(super) fn workspace_key(key: WireWorkspaceKey) -> WorkspaceKey {
    WorkspaceKey(key.get())
}

pub(super) fn wire_workspace_key(key: WorkspaceKey) -> WireWorkspaceKey {
    WireWorkspaceKey::new(key.0)
}

pub(super) fn group_key(key: WireGroup) -> GroupKey {
    GroupKey(key.get())
}

pub(super) fn output_name(name: &WireOutput) -> OutputName {
    OutputName(name.as_str().to_owned())
}

/// The Space id of a workspace: the compositor's own id as a slug when it gives one, else
/// `ws-<key>`. Never the reserved `desktop`.
pub fn space_id(workspace: &WireWorkspace) -> SpaceId {
    let fallback = || format!("ws-{}", workspace.key.get());
    let text = workspace
        .id
        .as_deref()
        .map(slug)
        .filter(|s| !s.is_empty() && s.as_str() != SpaceId::DESKTOP)
        .unwrap_or_else(fallback);
    SpaceId::parse(&text)
        .or_else(|_| SpaceId::parse(&fallback()))
        .unwrap_or_else(|_| SpaceId::desktop())
}

/// `text` as the id grammar's characters: lower case letters, digits, `.`, `_`, `-`; any
/// other character becomes `-`, and a leading `.`, `_` or `-` is dropped.
fn slug(text: &str) -> String {
    text.chars()
        .map(|c| match c.to_ascii_lowercase() {
            c if c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-') => c,
            _ => '-',
        })
        .skip_while(|c| matches!(*c, '.' | '_' | '-'))
        .collect()
}
