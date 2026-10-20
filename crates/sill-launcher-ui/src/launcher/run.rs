//! Carrying out a [`LauncherEffect`]: the effects end of choosing a row. A settings launch
//! asks the compositor for an activation token first, from the panel while it is still mapped,
//! so the app can take focus (the same two steps as the daemon's `sill::launch`, which a
//! surface cannot reach). The insertion into the field is the panel's own (`events.rs`).

use shell_host::{OutputId, SurfaceHandle};
use sill_model::clipboard::ClipboardCommand;
use sill_model::companion::CompanionCommand;
use sill_model::launch::{LaunchCommand, OpenRequest};
use sill_services::ServiceError;
use sill_ui_kit::services::Services;

use crate::launcher::activate::LauncherEffect;
use crate::launcher::companion::command_of;
use sill_ui_kit::links::{ActionLink, Origin};

/// Where an effect is carried out from: the panel's surface (a launch's token), its output
/// and the link an action of the shell is asked on (the daemon asks first, picks the output and
/// fetches the token).
#[derive(Clone)]
pub struct EffectFrom {
    pub surface: Option<SurfaceHandle>,
    pub output: OutputId,
    pub actions: Option<ActionLink>,
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
            let token = sill_ui_kit::surface::token_from(from.surface.as_ref(), None).await;
            services
                .launch
                .control
                .send(LaunchCommand::Open(OpenRequest { target, token }))
        }
        LauncherEffect::OpenSettings(link) => {
            sill_ui_kit::surface::open_settings_with_token(
                from.surface.as_ref(),
                &services.launch.control,
                link,
            )
            .await
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
        LauncherEffect::Do(action) => {
            match &from.actions {
                Some(actions) => {
                    actions.do_it(action, Origin::of(from.surface.as_ref(), from.output));
                }
                None => eprintln!("sill: launcher: no daemon here to do {action:?}"),
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
