//! Where a chosen row's [`Activation`] goes. The runner carries out the ones a service it holds
//! can do (windows through `windows`); the rest belong to the
//! launcher surface or the daemon. Pure mapping, table-tested; the sending is [`Outlets::carry`].

use sill_launcher::Activation;
use sill_model::windows::WlCommand;

use sill_services::control::Control;

/// What carrying out an activation takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Effect {
    /// A command to the `windows` thread.
    Wl(WlCommand),
    /// Not the runner's to do: the clipboard (`Copy`), the query field (`Insert`), a settings
    /// page, a session command, or nothing.
    Elsewhere,
}

/// Where `activation` goes.
pub(crate) fn effect(activation: &Activation) -> Effect {
    match activation {
        Activation::FocusWindow { window } => Effect::Wl(WlCommand::Activate(*window)),
        Activation::CloseWindow { window } => Effect::Wl(WlCommand::Close(*window)),
        Activation::Copy { .. }
        // A launch needs the surface's activation token, so the surface sends it.
        | Activation::Launch { .. }
        | Activation::Insert { .. }
        | Activation::OpenSettings { .. }
        | Activation::System(_)
        // Opening needs the surface's activation token, and the clipboard entry goes to the
        // clipboard service from the surface.
        | Activation::Open { .. }
        | Activation::SetClipboard { .. }
        | Activation::ForgetClipboard { .. }
        // The companion's acts need the panel's own context (what was kept, where it was asked
        // from), so the surface sends them.
        | Activation::Intent(_)
        | Activation::Companion(_)
        | Activation::Nothing => Effect::Elsewhere,
    }
}

/// The services the runner can send effects to; a missing one drops its effects (the results
/// service given its providers alone, [`super::Sources::Providers`], has none).
#[derive(Debug, Clone, Default)]
pub(crate) struct Outlets {
    pub(crate) wl: Option<Control<WlCommand>>,
}

impl Outlets {
    /// Send `effect` where it goes.
    pub(crate) fn carry(&self, effect: Effect) {
        let sent = match (effect, &self.wl) {
            (Effect::Wl(command), Some(wl)) => wl.send(command),
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
    use sill_launcher::{Activation, AppId, DesktopAction, SettingsLink, SystemCommand, WindowKey};
    use sill_model::windows::WlCommand;

    #[test]
    fn windows_go_to_the_compositor_and_the_rest_elsewhere() {
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
                Effect::Elsewhere,
            ),
            (
                Activation::Launch {
                    app: AppId("foot".to_owned()),
                    action: DesktopAction::Named("new-window".to_owned()),
                },
                Effect::Elsewhere,
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
                    target: SettingsLink("page:appearance".to_owned()),
                },
                Effect::Elsewhere,
            ),
            (
                Activation::System(SystemCommand::Session(sill_launcher::SessionAction::Lock)),
                Effect::Elsewhere,
            ),
            (Activation::Nothing, Effect::Elsewhere),
        ];
        for (activation, want) in cases {
            assert_eq!(effect(&activation), want, "{activation:?}");
        }
    }
}
