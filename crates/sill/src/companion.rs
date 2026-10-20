//! The companion from the daemon's side: what a summon finds focused, the four verbs of
//! `sill companion`, and the launcher effects the summon's machine asks for. The machines
//! themselves run in the companion service (`sill_services::companion`); this is the host
//! thread's part, which owns the outputs and the launcher link.
//!
//! A summon's launcher effects come off the service on a channel and are handed to the host
//! thread as [`ShellMsg::Summon`], which [`apply`] carries out on the launcher link: open the
//! panel in companion mode over the focused output, switch the open panel's mode in place, or
//! close it. The launcher closing, by whatever means, is told back to the service so the summon
//! returns to rest.

use shell_host::{HostHandle, OutputId};
use sill_ipc::response::Response;
use sill_ipc::show::CompanionRequest;
use sill_launcher_ui::Launcher;
use sill_model::companion::{CompanionCommand, FocusInput, LauncherFacts, SummonOut};
use sill_model::windows::Activity;
use sill_services::Control;
use sill_ui_kit::links::{LauncherLink, Reveal};
use sill_ui_kit::services::Services;
use tokio::runtime::Handle;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::daemon::msg::ShellMsg;
use crate::ipc::dispatch::{Reach, sent};
use crate::outputs::Outputs;

/// What is focused now: the active window, its app when the Wayland app id is a bus name, and the
/// panel.
pub fn focus_of(services: &Services, launcher: &LauncherLink) -> FocusInput {
    let toplevels = services.windows.state.borrow();
    let active = toplevels
        .toplevels
        .windows
        .iter()
        .find(|window| window.activity == Activity::Active);
    FocusInput {
        window: active.map(|window| window.key),
        app: active.and_then(|window| prov::AppName::parse(&window.app_id).ok()),
        launcher: match launcher.now() {
            Reveal::Revealed(opening) => LauncherFacts::Open(opening.mode),
            Reveal::Hidden { .. } => LauncherFacts::Closed,
        },
    }
}

/// Answer `sill companion ...`: tell the service, which decides.
pub fn dispatch(reach: &Reach, request: CompanionRequest) -> Response {
    let focus = || focus_of(reach.services, &reach.links.launcher);
    let command = match request {
        CompanionRequest::Tap => CompanionCommand::Tap(focus()),
        CompanionRequest::Summon => CompanionCommand::Summon(focus()),
        CompanionRequest::Dismiss => CompanionCommand::Dismiss,
        CompanionRequest::StopAll => CompanionCommand::StopAll,
    };
    sent(reach.services.companion.control.send(command), "assistant")
}

/// Carry out one launcher effect of a summon on the host thread.
pub fn apply(outputs: &Outputs, services: &Services, launcher: &LauncherLink, out: SummonOut) {
    match out {
        SummonOut::OpenLauncher { mode, .. } => {
            if let Some(output) = launcher_output(outputs, services) {
                launcher.open_in(output, String::new(), mode);
            }
        }
        SummonOut::SwitchLauncher(mode) => launcher.switch_mode(mode),
        SummonOut::CloseLauncher => launcher.close(),
        // The service asks the app itself.
        SummonOut::AskApp { .. } => {}
    }
}

/// The output the panel opens on: [`crate::ipc::dispatch::panel_output`]'s rule.
pub fn launcher_output(outputs: &Outputs, services: &Services) -> Option<OutputId> {
    crate::ipc::dispatch::panel_output::<Launcher>(outputs, services)
}

/// Hand each launcher effect the service asks for to the host thread, and tell the service when
/// the panel closes (however it was closed).
pub fn spawn_follower(
    rt: &Handle,
    host: HostHandle<ShellMsg>,
    mut summons: UnboundedReceiver<SummonOut>,
    launcher: &LauncherLink,
    companion: Control<CompanionCommand>,
) {
    rt.spawn(async move {
        while let Some(out) = summons.recv().await {
            if host.send(ShellMsg::Summon(out)).is_err() {
                return;
            }
        }
    });
    let mut shown = launcher.subscribe();
    rt.spawn(async move {
        let mut was_open = shown.borrow().opening().is_some();
        while shown.changed().await.is_ok() {
            let is_open = shown.borrow().opening().is_some();
            if was_open && !is_open && companion.send(CompanionCommand::LauncherClosed).is_err() {
                return;
            }
            was_open = is_open;
        }
    });
}
