//! Carrying out a [`LauncherEffect`]: the effects end of choosing a row. A settings launch
//! asks the compositor for an activation token first, from the panel while it is still mapped,
//! so the app can take focus (the same two steps as the daemon's `sill::launch`, which a
//! surface cannot reach). The insertion into the field is the panel's own (`events.rs`).

use shell_host::SurfaceHandle;
use sill_services::{LaunchCommand, LaunchRequest, ServiceError, Services};

use crate::surfaces::launcher::activate::{LauncherEffect, Unreachable};

/// Do `effect` with `services`, from `surface` when there is one.
pub async fn run_effect(
    effect: LauncherEffect,
    surface: Option<SurfaceHandle>,
    services: &Services,
) -> Result<(), ServiceError> {
    match effect {
        LauncherEffect::Launch { app, action } => {
            let token = match &surface {
                Some(surface) => surface.activation_token(Some(app.0.clone())).await,
                None => None,
            };
            services.launch.send(LaunchCommand::Launch(LaunchRequest {
                app,
                action,
                files: Vec::new(),
                uris: Vec::new(),
                token,
            }))
        }
        LauncherEffect::Copy(text) => {
            if let Some(surface) = &surface {
                surface.copy_text(text);
            }
            Ok(())
        }
        LauncherEffect::Unreachable(why) => {
            eprintln!("sill: launcher: {}", unreachable_text(&why));
            Ok(())
        }
        LauncherEffect::Session(action) => services
            .session_control
            .send(sill_services::SessionCommand::Run(action)),
        // M5 agent (d): `PowerMenuLink::open(output, Preselect::Action(action))`; the panel has
        // the link and its output, this function does not yet (FINDINGS "M5 freeze").
        LauncherEffect::PowerMenu(_) => Ok(()),
        LauncherEffect::ByRunner | LauncherEffect::Insert(_) | LauncherEffect::Nothing => Ok(()),
    }
}

fn unreachable_text(why: &Unreachable) -> String {
    match why {
        Unreachable::System(command) => {
            format!("{command:?} needs a session service the surfaces do not have yet")
        }
        Unreachable::SettingsKey(key) => {
            format!("no settings app to open {key:?} in yet")
        }
    }
}
