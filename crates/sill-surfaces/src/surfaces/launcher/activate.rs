//! Choosing a row: its provider says what should happen (`Provider::activate`, pure, the
//! frozen `Activation` vocabulary) and the panel hands that to the service that does it — the
//! launch service, the compositor thread, the clipboard — then closes. The mapping is pure;
//! the panel runs it.

use std::fmt;
use std::sync::{Arc, Mutex};

use sill_launcher::{
    Activation, AppId, Choice, DesktopAction, Item, Provider, ProviderKind, SettingsTarget,
    SystemCommand,
};
use sill_services::providers::{
    AppsProvider, CalculatorProvider, SettingsProvider, SystemProvider, WindowsProvider,
};
use sill_services::{Services, WlCommand};

/// The desktop entry `SettingsTarget::Cosmic` opens.
pub const COSMIC_SETTINGS: &str = "com.system76.CosmicSettings";

/// The providers the panel asks what a chosen row does. The daemon's search runner holds its
/// own instances for querying; `activate` is pure, so separate instances answer the same.
/// Provided as a root context by a demo or a test to stand in for the real five.
#[derive(Clone)]
pub struct LauncherProviders(pub Vec<Arc<dyn Provider>>);

impl fmt::Debug for LauncherProviders {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kinds: Vec<ProviderKind> = self.0.iter().map(|p| p.kind()).collect();
        f.debug_tuple("LauncherProviders").field(&kinds).finish()
    }
}

impl LauncherProviders {
    /// The five real providers over `services`' receivers.
    pub fn from_services(services: &Services) -> LauncherProviders {
        LauncherProviders(vec![
            Arc::new(AppsProvider {
                apps: services.apps.clone(),
                frecency: Arc::new(Mutex::default()),
            }),
            Arc::new(WindowsProvider {
                toplevels: services.toplevels.clone(),
                apps: services.apps.clone(),
                workspaces: services.workspaces.clone(),
            }),
            Arc::new(CalculatorProvider),
            Arc::new(SettingsProvider::default()),
            Arc::new(SystemProvider::new()),
        ])
    }

    /// What choosing `choice` on `item` does, from its own provider; nothing when no provider
    /// of its kind is here.
    pub fn activation(&self, item: &Item, choice: &Choice) -> Activation {
        self.0
            .iter()
            .find(|provider| provider.kind() == item.provider)
            .map_or(Activation::Nothing, |provider| provider.activate(item, choice))
    }
}

/// What the panel does for an activation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherEffect {
    /// Ask for an activation token for `app`, then send the launch.
    Launch { app: AppId, action: DesktopAction },
    /// A command for the compositor thread.
    Wl(WlCommand),
    /// Put the text on the clipboard.
    Copy(String),
    /// Nothing sill can do from a surface yet; logged with the reason.
    Unreachable(Unreachable),
    Nothing,
}

/// Why an activation cannot be carried out from the launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unreachable {
    /// `Services` has no session control: lock, log out, suspend, restart, shut down and
    /// reload need one (FINDINGS F146).
    System(SystemCommand),
    /// No quire settings app exists to deep-link a key into.
    SettingsKey(String),
}

/// The effect of `activation`. `SettingsTarget::Cosmic` opens COSMIC Settings on its default
/// page: its desktop entry's `Exec` takes no page argument (FINDINGS F146).
pub fn effect_of(activation: Activation) -> LauncherEffect {
    match activation {
        Activation::Launch { app, action } => LauncherEffect::Launch { app, action },
        Activation::FocusWindow { window } => LauncherEffect::Wl(WlCommand::Activate(window)),
        Activation::Copy { text } => LauncherEffect::Copy(text),
        Activation::OpenSettings {
            target: SettingsTarget::Cosmic(_),
        } => LauncherEffect::Launch {
            app: AppId(COSMIC_SETTINGS.to_owned()),
            action: DesktopAction::Default,
        },
        Activation::OpenSettings {
            target: SettingsTarget::Key(key),
        } => LauncherEffect::Unreachable(Unreachable::SettingsKey(key)),
        Activation::System(command) => LauncherEffect::Unreachable(Unreachable::System(command)),
        Activation::Nothing => LauncherEffect::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::{COSMIC_SETTINGS, LauncherEffect, LauncherProviders, Unreachable, effect_of};
    use sill_launcher::{
        Activation, AppId, Choice, DesktopAction, IconRef, Item, ItemId, Provider, ProviderKind,
        Query, ResultSink, Score, SettingsTarget, SystemCommand, WindowKey,
    };
    use sill_services::WlCommand;
    use std::sync::Arc;

    #[derive(Debug)]
    struct Fixed;

    impl Provider for Fixed {
        fn kind(&self) -> ProviderKind {
            ProviderKind::Calculator
        }
        fn query(&self, _: &Query, _: &mut dyn ResultSink) {}
        fn activate(&self, item: &Item, _: &Choice) -> Activation {
            Activation::Copy {
                text: item.title.clone(),
            }
        }
    }

    fn item(provider: ProviderKind) -> Item {
        Item {
            id: ItemId("4".to_owned()),
            provider,
            title: "4".to_owned(),
            subtitle: String::new(),
            icon: IconRef::None,
            score: Score(1),
            actions: Vec::new(),
        }
    }

    #[test]
    fn the_rows_own_provider_answers() {
        let providers = LauncherProviders(vec![Arc::new(Fixed)]);
        assert_eq!(
            providers.activation(&item(ProviderKind::Calculator), &Choice::Primary),
            Activation::Copy {
                text: "4".to_owned()
            }
        );
        assert_eq!(
            providers.activation(&item(ProviderKind::Apps), &Choice::Primary),
            Activation::Nothing,
            "no apps provider here"
        );
    }

    #[test]
    fn every_activation_has_an_effect() {
        let firefox = AppId("org.mozilla.firefox".to_owned());
        let cases = vec![
            (
                Activation::Launch {
                    app: firefox.clone(),
                    action: DesktopAction::Named("new-window".to_owned()),
                },
                LauncherEffect::Launch {
                    app: firefox,
                    action: DesktopAction::Named("new-window".to_owned()),
                },
            ),
            (
                Activation::FocusWindow {
                    window: WindowKey(9),
                },
                LauncherEffect::Wl(WlCommand::Activate(WindowKey(9))),
            ),
            (
                Activation::Copy {
                    text: "42".to_owned(),
                },
                LauncherEffect::Copy("42".to_owned()),
            ),
            (
                Activation::OpenSettings {
                    target: SettingsTarget::Cosmic("display".to_owned()),
                },
                LauncherEffect::Launch {
                    app: AppId(COSMIC_SETTINGS.to_owned()),
                    action: DesktopAction::Default,
                },
            ),
            (
                Activation::OpenSettings {
                    target: SettingsTarget::Key("dock.magnification".to_owned()),
                },
                LauncherEffect::Unreachable(Unreachable::SettingsKey(
                    "dock.magnification".to_owned(),
                )),
            ),
            (
                Activation::System(SystemCommand::Lock),
                LauncherEffect::Unreachable(Unreachable::System(SystemCommand::Lock)),
            ),
            (Activation::Nothing, LauncherEffect::Nothing),
        ];
        for (activation, want) in cases {
            assert_eq!(effect_of(activation.clone()), want, "{activation:?}");
        }
    }
}
