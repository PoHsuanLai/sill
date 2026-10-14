//! Choosing a row: the panel sends `SearchCommand::Chosen` (the runner counts it and carries
//! out window and app activations), then asks the row's provider what it does
//! (`Provider::activate`, pure, the `Activation` vocabulary) and does the rest itself: the
//! clipboard, the field, a settings page. The mapping is pure; the panel runs it.

use std::fmt;
use std::sync::{Arc, Mutex};

use sill_launcher::{
    Activation, AppId, Choice, ClipKey, CompanionAct, DesktopAction, Generation, Item, Mime,
    OpenTarget, Provider, ProviderKind, SettingsTarget, Subject, SystemCommand, ViewerOpen,
};
use std::path::PathBuf;

use sill_model::results::LauncherPolicy;
use sill_model::session::SessionAction;
use sill_search::providers::files::IndexSource;
use sill_search::providers::{
    AppsProvider, CalculatorProvider, ClipboardProvider, CompanionProvider, EmojiProvider,
    FilesProvider, MemoryProvider, RunsProvider, SettingsProvider, SystemProvider, ThingsProvider,
    WebProvider, WindowsProvider,
};
use sill_services::env::Buses;
use sill_ui_kit::services::Services;
use tokio::sync::watch;

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
    /// The real providers over `services`' receivers (the first five, then the four whose
    /// `activate` reads the row alone: their policy here is the default, and they are never
    /// queried, so they reach no bus).
    pub fn from_services(services: &Services) -> LauncherProviders {
        let (_, policy) = watch::channel(LauncherPolicy::default());
        let buses = Buses::absent();
        LauncherProviders(vec![
            Arc::new(AppsProvider {
                apps: services.apps.state.clone(),
                frecency: Arc::new(Mutex::default()),
            }),
            Arc::new(WindowsProvider {
                windows: services.windows.state.clone(),
                apps: services.apps.state.clone(),
            }),
            Arc::new(CalculatorProvider::default()),
            Arc::new(SettingsProvider::default()),
            Arc::new(SystemProvider::new(buses.system)),
            Arc::new(FilesProvider {
                policy: policy.clone(),
                frecency: Arc::new(Mutex::default()),
                source: IndexSource::Detect(buses.session),
                home: PathBuf::from("/"),
                viewer_store: PathBuf::from("/"),
            }),
            Arc::new(ClipboardProvider {
                history: services.clipboard.state.clone(),
            }),
            Arc::new(EmojiProvider {
                policy: policy.clone(),
                frecency: Arc::new(Mutex::default()),
            }),
            Arc::new(WebProvider { policy }),
            Arc::new(ThingsProvider {
                companion: services.companion.state.clone(),
                control: services.companion.control.clone(),
            }),
            Arc::new(CompanionProvider {
                companion: services.companion.state.clone(),
            }),
            Arc::new(RunsProvider {
                companion: services.companion.state.clone(),
            }),
            Arc::new(MemoryProvider {
                companion: services.companion.state.clone(),
                control: services.companion.control.clone(),
            }),
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
    /// The search runner carries it out on `Chosen` (windows, launches): the
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
    /// Ask for an activation token, then open the file, folder or address.
    Open(OpenTarget),
    /// Make a clipboard history entry the clipboard again.
    SetClipboard(ClipKey),
    /// Drop a clipboard history entry and stay open.
    ForgetClipboard(ClipKey),
    /// Perform an app's action as the person picked it (a thing's `open`).
    Intent(Box<docket_core::CallRequest>),
    /// Something for the companion: the panel sends it with what it knows of this opening.
    Companion(CompanionAct),
    /// A prompt, with what this opening kept, ready to send.
    Ask(sill_model::companion::AskInput),
    /// Nothing sill can do from a surface yet; logged with the reason.
    Unreachable(Unreachable),
    /// Carry out a session action at once (Lock, Suspend): `SessionCommand::Run`.
    Session(SessionAction),
    /// Open the power menu on the launcher's output, focused on this action, for the person to
    /// confirm there (Log Out, Restart, Shut Down).
    PowerMenu(SessionAction),
    Nothing,
}

/// Why an activation cannot be carried out from the launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unreachable {
    /// A system command no surface can carry out: Reload is the daemon's.
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
    /// Every effect closes the launcher but an insertion into its own field and forgetting a
    /// clipboard entry.
    pub fn after(&self) -> AfterEffect {
        match self {
            LauncherEffect::Insert(_)
            | LauncherEffect::ForgetClipboard(_)
            | LauncherEffect::Ask(_) => AfterEffect::StayOpen,
            LauncherEffect::Companion(act) => match act {
                // Promoting an answer opens its app; everything else is done from the panel,
                // which stays to show what came of it.
                CompanionAct::Promote { .. } => AfterEffect::Close,
                CompanionAct::Ask { .. }
                | CompanionAct::Card { .. }
                | CompanionAct::Run { .. }
                | CompanionAct::Undo { .. }
                | CompanionAct::Memory { .. }
                | CompanionAct::Mode(_) => AfterEffect::StayOpen,
            },
            _ => AfterEffect::Close,
        }
    }
}

/// The effect of `activation`.
pub fn effect_of(activation: Activation) -> LauncherEffect {
    match activation {
        Activation::Launch { .. }
        | Activation::FocusWindow { .. }
        | Activation::CloseWindow { .. } => LauncherEffect::ByRunner,
        Activation::Copy { text } => LauncherEffect::Copy(text),
        Activation::Insert { text } => LauncherEffect::Insert(text),
        Activation::OpenSettings {
            target: SettingsTarget::Key(key),
        } => LauncherEffect::Unreachable(Unreachable::SettingsKey(key)),
        Activation::System(command) => system_effect(command),
        Activation::Open { target } => LauncherEffect::Open(target),
        Activation::SetClipboard { entry } => LauncherEffect::SetClipboard(entry),
        Activation::ForgetClipboard { entry } => LauncherEffect::ForgetClipboard(entry),
        Activation::Intent(call) => LauncherEffect::Intent(call),
        Activation::Companion(act) => LauncherEffect::Companion(act),
        Activation::Nothing => LauncherEffect::Nothing,
    }
}

/// The files of `rows`, in the order shown: the results a file opened in the viewer is chosen from,
/// which its arrow keys walk. A folder is a row of the list but not a file to walk.
pub fn file_results(rows: &[&Item]) -> Vec<PathBuf> {
    rows.iter()
        .filter_map(|item| match &item.subject {
            Subject::File(file) if file.mime.0 != Mime::FOLDER => Some(file.path.clone()),
            Subject::File(_)
            | Subject::None
            | Subject::App(_)
            | Subject::Clip(_)
            | Subject::Emoji(_)
            | Subject::Web(_)
            | Subject::Entity(_)
            | Subject::Answer(_)
            | Subject::Run(_)
            | Subject::Activity(_)
            | Subject::Memory(_) => None,
        })
        .collect()
}

/// `effect` with the results of search `search` added when it opens a file in the viewer: the
/// provider that chose the effect saw one row, and only the panel knows the list around it.
pub fn with_results(
    effect: LauncherEffect,
    search: Generation,
    files: Vec<PathBuf>,
) -> LauncherEffect {
    match effect {
        LauncherEffect::Open(OpenTarget::Viewer(open)) => {
            LauncherEffect::Open(OpenTarget::Viewer(ViewerOpen {
                search,
                results: files,
                ..open
            }))
        }
        other => other,
    }
}

/// A session row: Lock and Suspend at once, Log Out, Restart and Shut Down through the power
/// menu (it is their confirmation); Reload is the daemon's, which no surface reaches.
fn system_effect(command: SystemCommand) -> LauncherEffect {
    match SessionAction::of_system(command) {
        Some(action @ (SessionAction::Lock | SessionAction::Suspend)) => {
            LauncherEffect::Session(action)
        }
        Some(
            action @ (SessionAction::LogOut | SessionAction::Restart | SessionAction::PowerOff),
        ) => LauncherEffect::PowerMenu(action),
        None => LauncherEffect::Unreachable(Unreachable::System(command)),
    }
}

#[cfg(test)]
mod tests {
    use super::{AfterEffect, LauncherEffect, LauncherProviders, Unreachable, effect_of};
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
            subject: sill_launcher::Subject::None,
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
                    target: SettingsTarget::Key("dock.magnification".to_owned()),
                },
                LauncherEffect::Unreachable(Unreachable::SettingsKey(
                    "dock.magnification".to_owned(),
                )),
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

    #[test]
    fn the_results_the_viewer_walks_are_the_files_shown_in_order() {
        use super::{file_results, with_results};
        use sill_launcher::{
            FileFacts, Generation, Mime, OpenTarget, Subject, UnixTime, ViewerOpen,
        };
        use std::path::PathBuf;

        let file = |path: &str, mime: &str| Item {
            subject: Subject::File(FileFacts {
                path: PathBuf::from(path),
                mime: Mime(mime.to_owned()),
                modified: UnixTime(0),
            }),
            ..item(ProviderKind::Files)
        };
        let rows = [
            file("/h/b.pdf", "application/pdf"),
            file("/h/docs", Mime::FOLDER),
            item(ProviderKind::Calculator),
            file("/h/a.png", "image/png"),
        ];
        let shown: Vec<&Item> = rows.iter().collect();
        let files = file_results(&shown);
        assert_eq!(
            files,
            [PathBuf::from("/h/b.pdf"), PathBuf::from("/h/a.png")]
        );

        let open = |results: Vec<PathBuf>, search| {
            LauncherEffect::Open(OpenTarget::Viewer(ViewerOpen {
                file: PathBuf::from("/h/a.png"),
                search,
                results,
            }))
        };
        assert_eq!(
            with_results(
                open(Vec::new(), Generation(0)),
                Generation(7),
                files.clone()
            ),
            open(files.clone(), Generation(7)),
            "a file opened in the viewer is told the list around it"
        );
        let others = [
            LauncherEffect::Open(OpenTarget::File(PathBuf::from("/h/docs"))),
            LauncherEffect::Open(OpenTarget::Play(PathBuf::from("/h/a.flac"))),
            LauncherEffect::Copy("x".to_owned()),
        ];
        for effect in others {
            assert_eq!(
                with_results(effect.clone(), Generation(7), files.clone()),
                effect,
                "{effect:?} has no list to carry"
            );
        }
    }

    /// The session rows run through the session service or open the power menu.
    #[test]
    fn session_rows_run_or_open_the_power_menu() {
        use sill_model::session::SessionAction;
        let cases = [
            (
                SystemCommand::Lock,
                LauncherEffect::Session(SessionAction::Lock),
            ),
            (
                SystemCommand::Suspend,
                LauncherEffect::Session(SessionAction::Suspend),
            ),
            (
                SystemCommand::LogOut,
                LauncherEffect::PowerMenu(SessionAction::LogOut),
            ),
            (
                SystemCommand::Restart,
                LauncherEffect::PowerMenu(SessionAction::Restart),
            ),
            (
                SystemCommand::ShutDown,
                LauncherEffect::PowerMenu(SessionAction::PowerOff),
            ),
            (
                SystemCommand::Reload,
                LauncherEffect::Unreachable(Unreachable::System(SystemCommand::Reload)),
            ),
        ];
        for (command, want) in cases {
            assert_eq!(effect_of(Activation::System(command)), want, "{command:?}");
        }
    }
}
