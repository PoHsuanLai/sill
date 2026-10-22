//! Hot corners on the daemon's side: the actions a firing carries out. The corner surfaces are
//! spawned like any `Surface` (`sill_overlays::hot_corners::HotCorners`).
//!
//! A firing arrives on the [`CornerLink`] (from a corner surface's machine, or from
//! `sill debug corner`); [`spawn_hits`] follows it, picks the corner's binding for exactly the
//! modifier keys held now ([`keys::pick`]) and turns it into a [`CornerEffect`]: an action of the
//! shell, handed to the interpreter (`crate::actions`) with the corner's output as its origin; an
//! activation the launcher's own path carries out (launch, open, an app's action); the companion
//! or App Exposé; or a custom command run as a child process. A held-keys set with no binding does
//! nothing.

mod effect;
mod keys;

use std::path::PathBuf;
use std::process::Stdio;

use shell_host::{HostHandle, OutputId};
use sill_launcher::activation::Activation;
use sill_model::actions::Ask;
use sill_model::companion::CompanionCommand;
use sill_model::desktop::{MacMods, open_app_expose};
use sill_model::launch::{LaunchCommand, LaunchRequest, OpenRequest};
use sill_services::ServiceError;
use sill_settings::ShellFile;
use sill_ui_kit::links::{Asked, Links, Origin};
use sill_ui_kit::services::Services;
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::daemon::msg::ShellMsg;

pub use effect::{CornerEffect, corner_effect};
pub use keys::{keys_of, pick};

/// Everything a firing is carried out with.
struct Doing {
    host: HostHandle<ShellMsg>,
    links: Links,
    services: Services,
}

/// Follow the corner link on `rt` and carry out each hit's binding (read from `settings` at the
/// hit, for the modifiers `held` holds then): an action goes to the host thread's interpreter, a
/// command is run here, the rest through the services.
pub fn spawn_hits(
    rt: &Handle,
    links: Links,
    services: Services,
    settings: watch::Receiver<ShellFile>,
    held: watch::Receiver<MacMods>,
    host: HostHandle<ShellMsg>,
) {
    let mut hits = links.corners.subscribe();
    let doing = Doing {
        host,
        links,
        services,
    };
    rt.spawn(async move {
        loop {
            if hits.changed().await.is_err() {
                return;
            }
            let Some(hit) = *hits.borrow_and_update() else {
                continue;
            };
            let file = settings.borrow().clone();
            let mods = *held.borrow();
            let bindings =
                sill_overlays::hot_corners::corner_bindings(&file.hot_corners, hit.corner);
            let Some(binding) = pick(bindings, mods) else {
                continue;
            };
            let home = std::env::var_os("HOME").map(PathBuf::from);
            let effect = corner_effect(&binding.action, home.as_deref());
            act(effect, hit.output, &doing).await;
        }
    });
}

/// Carry out `effect` on `output`: hand an action to the host with the corner's output as its
/// origin; the launcher's path for an activation; spawn a detached child process for `Run` (a
/// custom command). What cannot be done is logged.
async fn act(effect: CornerEffect, output: OutputId, doing: &Doing) {
    let Doing {
        host,
        links,
        services,
    } = doing;
    match effect {
        CornerEffect::Nothing => {}
        CornerEffect::Do(action) => {
            let origin = Origin {
                surface: None,
                output: Some(output),
            };
            let asked = Asked {
                ask: Ask::of(action),
                origin,
            };
            // The host is gone: the daemon is ending.
            let _gone_is_dropped = host.send(ShellMsg::Action(asked));
        }
        CornerEffect::AppExpose => {
            // The host is gone: the daemon is ending.
            let _gone_is_dropped = host.send(ShellMsg::Command(open_app_expose()));
        }
        CornerEffect::Assistant => {
            let focus = crate::companion::focus_of(services, &links.launcher);
            if let Err(error) = services
                .companion
                .control
                .send(CompanionCommand::Tap(focus))
            {
                eprintln!("sill: hot corner: the assistant: {error}");
            }
        }
        CornerEffect::Activate(activation) => {
            if let Err(error) = activate(activation, services) {
                eprintln!("sill: hot corner: {error}");
            }
        }
        CornerEffect::Run { program, args } => {
            let mut command = tokio::process::Command::new(&program);
            command.args(&args).stdin(Stdio::null()).kill_on_drop(false);
            if let Err(error) = command.spawn() {
                eprintln!("sill: hot corner: running {program}: {error}");
            }
        }
        CornerEffect::Cannot(why) => eprintln!("sill: hot corner: {why}"),
    }
}

/// Carry out what a corner opens or asks of an app: a launch and an open go to the launch
/// service and an app's action to intentd through the companion service, as the launcher's own
/// rows do. A corner has no surface of its own, so no activation token goes with them.
fn activate(activation: Activation, services: &Services) -> Result<(), ServiceError> {
    match activation {
        Activation::Launch { app, action } => services.launch.control.send(LaunchCommand::Launch(
            LaunchRequest::app(app).action(action),
        )),
        Activation::Open { target } => {
            services
                .launch
                .control
                .send(LaunchCommand::Open(OpenRequest {
                    target,
                    token: None,
                }))
        }
        Activation::Intent(call) => services
            .companion
            .control
            .send(CompanionCommand::Perform(call)),
        other => {
            eprintln!("sill: hot corner: nothing carries out {other:?}");
            Ok(())
        }
    }
}
