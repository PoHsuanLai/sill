//! The shell's commands as Quire requests: windows by identifier, apps by key, Spaces by id.

use casement_protocols::vocab::ShellRole;
use desktop_client::toplevel::ToplevelFlag;
use desktop_client::{
    AppId, AppKey, CaptureRequest, DesktopCommand as Wire, DesktopState, OutputName, Rect, Size,
    SpaceId as WireSpaceId, SpaceKey, SpaceTarget, ToplevelIdentifier, ToplevelKey,
};
use porter_core::SpaceId;
use sill_model::desktop::DesktopCommand;

use super::surfaces::ShellSurfaces;
use crate::desktop_wl::Refusal;
use crate::desktop_wl::command::find_window;

/// The wire requests that carry `command`, in order. Activating a minimised window restores it
/// first; a minimise target names the dock's surface on the window's output.
pub fn quire_wire_of(
    command: &DesktopCommand,
    state: &DesktopState,
    surfaces: &ShellSurfaces,
) -> Result<Vec<Wire>, Refusal> {
    let window = |id: &str| find_window(state, id);
    let app = |key: &str| app_key(state, key);
    let space = |id: &SpaceId| space_key(state, id);
    let one = |request: Wire| -> Result<Vec<Wire>, Refusal> { Ok(vec![request]) };
    match command {
        DesktopCommand::Activate(id) => {
            let key = window(id)?;
            let minimised = state
                .toplevels
                .get(key)
                .is_some_and(|t| t.flags.contains(ToplevelFlag::Minimized));
            let restore = minimised.then_some(Wire::Unminimise(key));
            Ok(restore.into_iter().chain([Wire::Activate(key)]).collect())
        }
        DesktopCommand::Close(id) => one(Wire::Close(window(id)?)),
        DesktopCommand::Minimise(id) => one(Wire::Minimise(window(id)?)),
        DesktopCommand::Unminimise(id) => one(Wire::Unminimise(window(id)?)),
        DesktopCommand::Place(id, target) => one(Wire::Place {
            toplevel: window(id)?,
            target: *target,
        }),
        DesktopCommand::ToSpace(id, to) => one(Wire::MoveToSpace {
            toplevel: window(id)?,
            space: space(to)?,
        }),
        DesktopCommand::SwitchSpace(to) => one(Wire::ActivateSpace(space(to)?)),
        DesktopCommand::SetMinimiseTarget { toplevel, rect } => {
            let key = window(toplevel)?;
            let surface = surfaces
                .dock_for(output_of(state, key))
                .ok_or(Refusal::NoSurface(ShellRole::Dock))?;
            let (x, y, w, h) = *rect;
            one(Wire::SetMinimiseTarget {
                toplevel: key,
                surface,
                rect: Rect::new(x, y, w, h),
            })
        }
        DesktopCommand::ClearMinimiseTarget(id) => one(Wire::ClearMinimiseTarget(window(id)?)),
        DesktopCommand::AssignSpaces(id, assign) => one(Wire::AssignSpaces {
            toplevel: window(id)?,
            assign: *assign,
        }),
        DesktopCommand::ActivateApp(key) => one(Wire::ActivateApp(app(key)?)),
        DesktopCommand::HideApp(key) => one(Wire::HideApp(app(key)?)),
        DesktopCommand::UnhideApp(key) => one(Wire::UnhideApp(app(key)?)),
        DesktopCommand::HideOtherApps(key) => one(Wire::HideOtherApps(app(key)?)),
        DesktopCommand::QuitApp(key) => one(Wire::QuitApp(app(key)?)),
        DesktopCommand::ForceQuitApp(key) => one(Wire::ForceQuitApp(app(key)?)),
        DesktopCommand::ReopenApp(key) => one(Wire::ReopenApp(app(key)?)),
        DesktopCommand::CreateSpace(output) => one(Wire::CreateSpace {
            output: OutputName::new(output.as_str()),
        }),
        DesktopCommand::RemoveSpace(id) => one(Wire::RemoveSpace(space(id)?)),
        DesktopCommand::MoveSpace {
            space: id,
            position,
        } => one(Wire::MoveSpace {
            space: space(id)?,
            position: *position,
        }),
        DesktopCommand::Overview { mode, toggle } => one(Wire::Overview {
            mode: *mode,
            toggle: *toggle,
        }),
        DesktopCommand::ShowDesktop(toggle) => one(Wire::ShowDesktop(*toggle)),
        DesktopCommand::PickSwitcher(index) => one(Wire::PickSwitcher(*index)),
        DesktopCommand::CancelSwitcher => one(Wire::CancelSwitcher),
        DesktopCommand::PickWindowMenu(id, entry) => one(Wire::PickWindowMenu {
            window: ToplevelIdentifier::new(id.as_str()),
            entry: *entry,
        }),
        DesktopCommand::PickSplit(id) => one(Wire::PickSplit(ToplevelIdentifier::new(id.as_str()))),
        DesktopCommand::CancelSplit => one(Wire::CancelSplit),
        DesktopCommand::DivideSplit(id, share) => one(Wire::DivideSplit {
            window: ToplevelIdentifier::new(id.as_str()),
            share: *share,
        }),
        DesktopCommand::CaptureSpace {
            output,
            space,
            size: (w, h),
        } => one(Wire::CaptureOutput(CaptureRequest::of_space(
            OutputName::new(output.as_str()),
            WireSpaceId::new(space.as_str()),
            Size::new(*w, *h),
        ))),
        DesktopCommand::CaptureOutput { output } => one(Wire::CaptureOutput(CaptureRequest::new(
            OutputName::new(output.as_str()),
        ))),
        DesktopCommand::SetSpaceTargets { output, targets } => one(Wire::SetSpaceTargets {
            output: OutputName::new(output.as_str()),
            targets: targets
                .iter()
                .map(|(space, (x, y, w, h))| {
                    SpaceTarget::new(WireSpaceId::new(space.as_str()), Rect::new(*x, *y, *w, *h))
                })
                .collect(),
        }),
        DesktopCommand::SetNewSpaceTarget { output, rect } => one(Wire::SetNewSpaceTarget {
            output: OutputName::new(output.as_str()),
            rect: rect.map(|(x, y, w, h)| Rect::new(x, y, w, h)),
        }),
        DesktopCommand::AnswerPermission { ask, answer } => one(Wire::AnswerPermission {
            ask: *ask,
            answer: *answer,
        }),
    }
}

fn output_of(state: &DesktopState, key: ToplevelKey) -> Option<&str> {
    state
        .quire
        .window(key)
        .and_then(|window| window.output.as_ref())
        .map(OutputName::as_str)
}

fn app_key(state: &DesktopState, key: &str) -> Result<AppKey, Refusal> {
    state
        .quire
        .app_by_id(&AppId::new(key))
        .map(|app| app.key)
        .ok_or_else(|| Refusal::UnknownApp(key.to_owned()))
}

fn space_key(state: &DesktopState, id: &SpaceId) -> Result<SpaceKey, Refusal> {
    state
        .quire
        .space_by_id(&WireSpaceId::new(id.as_str()))
        .map(|space| space.key)
        .ok_or_else(|| Refusal::UnknownSpace(id.clone()))
}
