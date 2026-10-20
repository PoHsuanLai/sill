//! Carrying out a [`LauncherEffect`]: the effects end of choosing a row. A settings launch
//! asks the compositor for an activation token first, from the panel while it is still mapped,
//! so the app can take focus (the same two steps as the daemon's `sill::launch`, which a
//! surface cannot reach). The insertion into the field is the panel's own (`events.rs`).

use shell_host::{OutputId, SurfaceHandle};
use sill_model::clipboard::ClipboardCommand;
use sill_model::companion::CompanionCommand;
use sill_model::launch::{LaunchCommand, OpenRequest, SettingsVisit};
use sill_services::ServiceError;
use sill_ui_kit::services::Services;

use crate::launcher::activate::{LauncherEffect, Unreachable};
use crate::launcher::companion::command_of;
use sill_ui_kit::links::{PowerMenuLink, Preselect};

/// The app id Settings runs under: what its activation token names.
const SETTINGS_APP: &str = "org.quire.Settings";

/// Where an effect is carried out from: the panel's surface (a launch's token), its output
/// and the power menu's link (a Log Out, Restart or Shut Down row opens it there).
#[derive(Clone)]
pub struct EffectFrom {
    pub surface: Option<SurfaceHandle>,
    pub output: OutputId,
    pub power_menu: Option<PowerMenuLink>,
}

/// Do `effect` with `services`, from `from`.
pub async fn run_effect(
    effect: LauncherEffect,
    from: &EffectFrom,
    services: &Services,
) -> Result<(), ServiceError> {
    match effect {
        LauncherEffect::Launch { app, action } => {
            sill_ui_kit::surface::launch_with_token(
                from.surface.as_ref(),
                &services.launch.control,
                app,
                action,
                Vec::new(),
            )
            .await
        }
        LauncherEffect::Open(target) => {
            let token = match &from.surface {
                Some(surface) => sill_ui_kit::surface::activation_token(surface, None).await,
                None => None,
            };
            services
                .launch
                .control
                .send(LaunchCommand::Open(OpenRequest { target, token }))
        }
        LauncherEffect::OpenSettings(slug) => {
            let token = match &from.surface {
                Some(surface) => {
                    sill_ui_kit::surface::activation_token(surface, Some(SETTINGS_APP.to_owned()))
                        .await
                }
                None => None,
            };
            services
                .launch
                .control
                .send(LaunchCommand::OpenSettingsAt(SettingsVisit { slug, token }))
        }
        LauncherEffect::SetClipboard(entry) => services
            .clipboard
            .control
            .send(ClipboardCommand::Offer(entry)),
        LauncherEffect::ForgetClipboard(entry) => services
            .clipboard
            .control
            .send(ClipboardCommand::Forget(entry)),
        LauncherEffect::Copy(text) => {
            if let Some(surface) = &from.surface {
                surface.copy_text(text);
            }
            Ok(())
        }
        LauncherEffect::Unreachable(why) => {
            eprintln!("sill: launcher: {}", unreachable_text(&why));
            Ok(())
        }
        LauncherEffect::Session(action) => services
            .session
            .control
            .send(sill_model::session::SessionCommand::Run(action)),
        LauncherEffect::PowerMenu(action) => {
            match &from.power_menu {
                Some(link) => link.open(from.output, Preselect::Action(action)),
                None => eprintln!("sill: launcher: no power menu here to confirm {action:?} in"),
            }
            Ok(())
        }
        LauncherEffect::Ask(input) => services
            .companion
            .control
            .send(CompanionCommand::Ask(input)),
        LauncherEffect::Intent(call) => services
            .companion
            .control
            .send(CompanionCommand::Perform(call)),
        LauncherEffect::Companion(act) => match command_of(act) {
            Some(command) => services.companion.control.send(command),
            None => Ok(()),
        },
        LauncherEffect::ByRunner | LauncherEffect::Insert(_) | LauncherEffect::Nothing => Ok(()),
    }
}

fn unreachable_text(why: &Unreachable) -> String {
    match why {
        Unreachable::System(command) => {
            format!("{command:?} is the daemon's, which no surface reaches")
        }
    }
}
