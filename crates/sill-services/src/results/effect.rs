//! Where a chosen row's [`Activation`] goes. The runner carries out the ones a service it holds
//! can do (windows through `cosmic_wl`, apps through `launch`); the rest belong to the
//! launcher surface or the daemon. Pure mapping, table-tested; the sending is [`Outlets::carry`].

use sill_launcher::Activation;

use crate::control::Control;
use sill_model::cosmic_wl::WlCommand;
use sill_model::launch::{LaunchCommand, LaunchRequest};

/// What carrying out an activation takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Effect {
    /// A command to the `cosmic_wl` thread.
    Wl(WlCommand),
    /// A command to the launch service.
    Launch(LaunchCommand),
    /// Not the runner's to do: the clipboard (`Copy`), the query field (`Insert`), a settings
    /// page, a session command, or nothing.
    Elsewhere,
}

/// Where `activation` goes.
pub(crate) fn effect(activation: &Activation) -> Effect {
    match activation {
        Activation::FocusWindow { window } => Effect::Wl(WlCommand::Activate(*window)),
        Activation::CloseWindow { window } => Effect::Wl(WlCommand::Close(*window)),
        Activation::Launch { app, action } => {
            Effect::Launch(LaunchCommand::Launch(LaunchRequest {
                app: app.clone(),
                action: action.clone(),
                files: Vec::new(),
                uris: Vec::new(),
                // The launcher surface holds no activation token to hand over yet.
                token: None,
            }))
        }
        Activation::Copy { .. }
        | Activation::Insert { .. }
        | Activation::OpenSettings { .. }
        | Activation::System(_)
        // Opening needs the surface's activation token, and the clipboard entry goes to the
        // clipboard service from the surface.
        | Activation::Open { .. }
        | Activation::SetClipboard { .. }
        | Activation::ForgetClipboard { .. }
        | Activation::Nothing => Effect::Elsewhere,
    }
}

/// The services the runner can send effects to; a missing one drops its effects (the results
/// service given its providers alone, [`super::Sources::Providers`], has none).
#[derive(Debug, Clone, Default)]
pub(crate) struct Outlets {
    pub(crate) wl: Option<Control<WlCommand>>,
    pub(crate) launch: Option<Control<LaunchCommand>>,
}

impl Outlets {
    /// Send `effect` where it goes.
    pub(crate) fn carry(&self, effect: Effect) {
        let sent = match (effect, &self.wl, &self.launch) {
            (Effect::Wl(command), Some(wl), _) => wl.send(command),
            (Effect::Launch(command), _, Some(launch)) => launch.send(command),
            // No outlet for it, or not the runner's to do.
            _ => Ok(()),
        };
        if let Err(error) = sent {
            eprintln!("sill: launcher activation: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Effect, effect};
    use sill_launcher::{
        Activation, AppId, DesktopAction, SettingsTarget, SystemCommand, WindowKey,
    };
    use sill_model::cosmic_wl::WlCommand;
    use sill_model::launch::{LaunchCommand, LaunchRequest};

    #[test]
    fn windows_go_to_cosmic_wl_apps_to_launch_and_the_rest_elsewhere() {
        let launch = |action: DesktopAction| {
            Effect::Launch(LaunchCommand::Launch(LaunchRequest {
                app: AppId("foot".to_owned()),
                action,
                files: Vec::new(),
                uris: Vec::new(),
                token: None,
            }))
        };
        let cases: Vec<(Activation, Effect)> = vec![
            (
                Activation::FocusWindow {
                    window: WindowKey(3),
                },
                Effect::Wl(WlCommand::Activate(WindowKey(3))),
            ),
            (
                Activation::CloseWindow {
                    window: WindowKey(3),
                },
                Effect::Wl(WlCommand::Close(WindowKey(3))),
            ),
            (
                Activation::Launch {
                    app: AppId("foot".to_owned()),
                    action: DesktopAction::Default,
                },
                launch(DesktopAction::Default),
            ),
            (
                Activation::Launch {
                    app: AppId("foot".to_owned()),
                    action: DesktopAction::Named("new-window".to_owned()),
                },
                launch(DesktopAction::Named("new-window".to_owned())),
            ),
            (
                Activation::Copy {
                    text: "4".to_owned(),
                },
                Effect::Elsewhere,
            ),
            (
                Activation::Insert {
                    text: "4".to_owned(),
                },
                Effect::Elsewhere,
            ),
            (
                Activation::OpenSettings {
                    target: SettingsTarget::Cosmic("display".to_owned()),
                },
                Effect::Elsewhere,
            ),
            (Activation::System(SystemCommand::Lock), Effect::Elsewhere),
            (Activation::Nothing, Effect::Elsewhere),
        ];
        for (activation, want) in cases {
            assert_eq!(effect(&activation), want, "{activation:?}");
        }
    }
}
