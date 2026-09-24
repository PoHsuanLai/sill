//! Choosing a row: the panel sends `SearchCommand::Chosen` (the runner counts it and carries
//! out window and app activations, FINDINGS F117), then asks the row's provider what it does
//! (`Provider::activate`, pure, the `Activation` vocabulary) and does the rest itself: the
//! clipboard, the field, a settings page. The mapping is pure; the panel runs it.

use std::fmt;
use std::sync::{Arc, Mutex};

use sill_launcher::{
    Activation, AppId, Choice, DesktopAction, Item, Provider, ProviderKind, SettingsTarget,
    SystemCommand,
};
use sill_services::Services;
use sill_services::providers::{
    AppsProvider, CalculatorProvider, SettingsProvider, SystemProvider, WindowsProvider,
};

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
            .map_or(Activation::Nothing, |provider| {
                provider.activate(item, choice)
            })
    }
}

/// What the panel does for an activation, once it has sent `SearchCommand::Chosen`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherEffect {
    /// The search runner carries it out on `Chosen` (windows, launches; FINDINGS F117): the
    /// panel only closes.
    ByRunner,
    /// Ask for an activation token for `app`, then send the launch (a settings page).
    Launch {
        app: AppId,
        action: DesktopAction,
    },
    /// Put the text on the clipboard.
    Copy(String),
    /// Put the text in the search field and stay open.
    Insert(String),
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

/// Whether an effect ends the launcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterEffect {
    Close,
    StayOpen,
}

impl LauncherEffect {
    /// Every effect closes the launcher but an insertion into its own field.
    pub fn after(&self) -> AfterEffect {
        match self {
            LauncherEffect::Insert(_) => AfterEffect::StayOpen,
            _ => AfterEffect::Close,
        }
    }
}

/// The effect of `activation`. `SettingsTarget::Cosmic` opens COSMIC Settings on its default
/// page: its desktop entry's `Exec` takes no page argument (FINDINGS F146).
pub fn effect_of(activation: Activation) -> LauncherEffect {
    match activation {
        Activation::Launch { .. }
        | Activation::FocusWindow { .. }
        | Activation::CloseWindow { .. } => LauncherEffect::ByRunner,
        Activation::Copy { text } => LauncherEffect::Copy(text),
        Activation::Insert { text } => LauncherEffect::Insert(text),
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
    use super::{
        AfterEffect, COSMIC_SETTINGS, LauncherEffect, LauncherProviders, Unreachable, effect_of,
    };
    use sill_launcher::{
        Activation, AppId, Choice, DesktopAction, IconRef, Item, ItemId, Provider, ProviderKind,
        Query, ResultSink, Score, SettingsTarget, SystemCommand, WindowKey,
    };
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
                LauncherEffect::ByRunner,
            ),
            (
                Activation::CloseWindow {
                    window: WindowKey(9),
                },
                LauncherEffect::ByRunner,
            ),
            (
                Activation::Insert {
                    text: "42".to_owned(),
                },
                LauncherEffect::Insert("42".to_owned()),
            ),
            (
                Activation::FocusWindow {
                    window: WindowKey(9),
                },
                LauncherEffect::ByRunner,
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
        assert_eq!(
            LauncherEffect::Insert(String::new()).after(),
            AfterEffect::StayOpen
        );
        assert_eq!(LauncherEffect::ByRunner.after(), AfterEffect::Close);
    }
}
