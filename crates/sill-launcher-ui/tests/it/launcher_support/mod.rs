//! Fake services for the launcher's tests and `examples/launcher_demo.rs`: the bar's fakes, a
//! search runner that answers every `SearchCommand::Query` on the frozen results `watch`
//! channel with a new generation (as `sill_search::results::spawn` will once launcher-core
//! lands) and carries out chosen window and app rows as the real runner does, and providers
//! that say what a fixed set of rows does.

pub use sill_ui_kit::harness::fake;
pub mod companion;
pub mod m9;
mod runner;

use runner::Answer;
pub use runner::Runner;

use std::sync::{Arc, Mutex};

use sill_launcher::{
    Action, ActionId, Activation, AppId, Choice, DesktopAction, Frecency, IconRef, Item, ItemId,
    ItemKey, Placement, Provider, ProviderKind, Query, QueryShape, ResultSink, Score, SectionPlan,
    SectionSpan, SettingsTarget, SystemCommand, UnixTime, WindowKey, place,
};
use sill_launcher_ui::launcher::LauncherProviders;
use sill_model::launch::{LaunchCommand, LaunchRequest};
use sill_model::results::{Results, SearchCommand};
use sill_model::windows::WlCommand;
use sill_services::Control;
use sill_ui_kit::services::Services;
use tokio::sync::watch;

pub const FIREFOX: &str = "org.mozilla.firefox";
pub const FILES: &str = "org.gnome.Nautilus";
pub const TERM: &str = "foot";
pub const FIREFOX_WINDOW: u64 = 10;

/// The fakes' senders, and every search command the launcher sent.
pub struct LauncherFeeds {
    pub bar: fake::Feeds,
    pub results: watch::Sender<Results>,
    pub searches: Arc<Mutex<Vec<SearchCommand>>>,
}

fn row(provider: ProviderKind, id: &str, title: &str, subtitle: &str, score: u32) -> Item {
    Item {
        id: ItemId(id.to_owned()),
        provider,
        title: title.to_owned(),
        subtitle: subtitle.to_owned(),
        icon: match provider {
            ProviderKind::Apps => app_icon(id),
            _ => IconRef::None,
        },
        score: Score(score),
        actions: Vec::new(),
        subject: sill_launcher::Subject::None,
    }
}

/// An app's hicolor icon, as the app index would resolve it, when this system has one (a row
/// without one shows its provider's glyph).
fn app_icon(id: &str) -> IconRef {
    let path = std::path::PathBuf::from(format!("/usr/share/icons/hicolor/48x48/apps/{id}.svg"));
    if path.is_file() {
        IconRef::File(path)
    } else {
        IconRef::None
    }
}

fn with_actions(mut item: Item, actions: &[(&str, &str)]) -> Item {
    item.actions = actions
        .iter()
        .map(|(id, label)| Action {
            id: ActionId((*id).to_owned()),
            label: (*label).to_owned(),
            keys: sill_launcher::ActionKeys::None,
        })
        .collect();
    item
}

/// Every row the fake providers know, best first.
pub fn catalogue() -> Vec<Item> {
    use ProviderKind::{Apps, Calculator, Settings, System, Windows};
    vec![
        row(Calculator, "6*7", "= 42", "6 * 7, Enter copies", 1000),
        with_actions(
            row(Apps, FIREFOX, "Firefox", "Web Browser", 950),
            &[
                ("new-window", "New Window"),
                ("new-private-window", "New Private Window"),
            ],
        ),
        with_actions(
            row(Apps, FILES, "Files", "Example File Manager", 800),
            &[("new-window", "New Window")],
        ),
        row(
            Windows,
            "10",
            "Firefox — sill launcher",
            "Firefox, workspace 2",
            750,
        ),
        row(Apps, TERM, "Terminal", "Example Terminal", 700),
        row(Settings, "wireless", "Wi-Fi", "Network & Wireless", 500),
        row(Settings, "display", "Display", "Resolution and scale", 450),
        row(System, "lock", "Lock Screen", "loginctl lock-session", 300),
        row(System, "log-out", "Log Out", "End this session", 250),
        row(System, "restart", "Restart", "", 200),
    ]
}

/// The rows a query finds, best first: every row for the empty query, else those whose title
/// or subtitle holds it (case-insensitively); the calculator answers only a query with a
/// digit first, the web fallback only typed text, and only when the query starts `m9` (so the
/// other tests' rows stay as they are); the slow text file only the query `slow-head`.
pub fn search(query: &str) -> Vec<Item> {
    let needle = query.trim().to_lowercase();
    let mut rows: Vec<Item> = catalogue()
        .into_iter()
        .chain(m9::rows())
        .filter(|item| match item.provider {
            ProviderKind::Calculator => needle.starts_with(|c: char| c.is_ascii_digit()),
            _ => {
                needle.is_empty()
                    || item.title.to_lowercase().contains(&needle)
                    || item.subtitle.to_lowercase().contains(&needle)
            }
        })
        .collect();
    if needle.starts_with("m9") {
        rows.push(m9::web(query.trim()));
    }
    if needle.starts_with("slow-head") {
        rows.push(m9::slow_head());
    }
    rows.sort_by(|a, b| b.score.cmp(&a.score).then(a.provider.cmp(&b.provider)));
    rows
}

/// When the fake history was written.
pub const NOW: UnixTime = UnixTime(1_790_000_000);

/// What the fake history has counted: Firefox most, then Files, the Firefox window, Terminal
/// and Wi-Fi (so Recent lists those five, in that order).
pub fn frecency() -> Frecency {
    let uses: [(ProviderKind, &str, usize); 5] = [
        (ProviderKind::Apps, FIREFOX, 5),
        (ProviderKind::Apps, FILES, 4),
        (ProviderKind::Windows, "10", 3),
        (ProviderKind::Apps, TERM, 2),
        (ProviderKind::Settings, "wireless", 1),
    ];
    uses.iter()
        .fold(Frecency::default(), |history, &(kind, id, times)| {
            (0..times).fold(history, |history, _| {
                history.record(ItemKey::of(kind, &ItemId(id.to_owned())), NOW)
            })
        })
}

/// `rows` (best first) as the real runner publishes them: placed in sections, or a filtered
/// provider's rows alone.
pub fn placed(rows: &[Item], text: &str, only: Option<ProviderKind>) -> Placement {
    let plan = SectionPlan::default_plan();
    match only {
        Some(kind) => {
            let items: Vec<Item> = rows
                .iter()
                .filter(|i| i.provider == kind)
                .cloned()
                .collect();
            let spans = match (plan.rule(kind), items.len()) {
                (Some(rule), n) if n > 0 => vec![SectionSpan {
                    section: rule.section,
                    rows: n as u32,
                }],
                _ => Vec::new(),
            };
            Placement { items, spans }
        }
        None => {
            let shape = match text.trim() {
                "" => QueryShape::Empty,
                _ => QueryShape::Text,
            };
            place(rows, &plan, shape, &frecency(), NOW)
        }
    }
}

/// Fake services whose search is answered by a runner thread, and their senders (the demo's
/// window: its commands come from the window's thread).
pub fn services() -> (Services, LauncherFeeds) {
    let (services, feeds, runner) = services_with_runner();
    runner.on_thread();
    (services, feeds)
}

/// Fake services and the search runner that answers them, not yet running: a harness on the
/// virtual clock runs it as a task on its own thread ([`Runner::run`]), where each answer lands
/// within the `advance` that sent the query (off-thread work never does).
pub fn services_with_runner() -> (Services, LauncherFeeds, Runner) {
    let (mut services, bar) = fake::services();
    let (results_tx, results_rx) = watch::channel(Results::default());
    let (control, commands) = Control::channel();
    services.results.state = results_rx;
    services.results.control = control;
    let searches = Arc::new(Mutex::new(Vec::new()));
    let answer = Answer {
        results: results_tx.clone(),
        searches: searches.clone(),
        wl: services.windows.control.clone(),
        launch: services.launch.control.clone(),
        providers: providers(),
        companion: services.companion.state.clone(),
        generation: 0,
        rows: Vec::new(),
    };
    let feeds = LauncherFeeds {
        bar,
        results: results_tx,
        searches,
    };
    (services, feeds, Runner { commands, answer })
}

/// What the real runner does with a chosen row (sill-services `results/effect.rs`): windows go
/// to the compositor thread, launches to the launch service, the rest are
/// the surface's.
fn carry(
    rows: &[Item],
    key: &ItemKey,
    choice: &Choice,
    providers: &LauncherProviders,
    wl: &Control<WlCommand>,
    launch: &Control<LaunchCommand>,
) {
    let Some(item) = rows.iter().find(|i| ItemKey::of(i.provider, &i.id) == *key) else {
        return;
    };
    let _ = match providers.activation(item, choice) {
        Activation::FocusWindow { window } => wl.send(WlCommand::Activate(window)),
        Activation::CloseWindow { window } => wl.send(WlCommand::Close(window)),
        Activation::Launch { app, action } => launch.send(LaunchCommand::Launch(LaunchRequest {
            app,
            action,
            files: Vec::new(),
            uris: Vec::new(),
            token: None,
        })),
        _ => Ok(()),
    };
}

/// A provider that says what the catalogue's rows of `kind` do.
#[derive(Debug)]
struct FakeProvider(ProviderKind);

impl Provider for FakeProvider {
    fn kind(&self) -> ProviderKind {
        self.0
    }

    fn query(&self, _query: &Query, _sink: &mut dyn ResultSink) {}

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        match (self.0, choice) {
            (ProviderKind::Apps, Choice::Primary) => Activation::Launch {
                app: AppId(item.id.0.clone()),
                action: DesktopAction::Default,
            },
            (ProviderKind::Apps, Choice::Action(action)) => Activation::Launch {
                app: AppId(item.id.0.clone()),
                action: DesktopAction::Named(action.0.clone()),
            },
            (ProviderKind::Windows, _) => Activation::FocusWindow {
                window: WindowKey(item.id.0.parse().unwrap_or(0)),
            },
            (ProviderKind::Calculator, _) => Activation::Copy {
                text: "42".to_owned(),
            },
            (ProviderKind::Settings, _) => Activation::OpenSettings {
                target: SettingsTarget::Key(item.id.0.clone()),
            },
            (ProviderKind::System, _) => Activation::System(SystemCommand::Session(sill_launcher::SessionAction::Lock)),
            // The companion's kinds: the real providers say what their rows do (`tests/launcher_companion.rs`).
            (ProviderKind::Companion, choice) => sill_search::providers::CompanionProvider {
                companion: watch::channel(sill_model::companion::Companion::default()).1,
            }
            .activate(item, choice),
            (ProviderKind::Helpers, choice) => sill_search::providers::HelpersProvider {
                companion: watch::channel(sill_model::companion::Companion::default()).1,
            }
            .activate(item, choice),
            (ProviderKind::Runs, choice) => sill_search::providers::RunsProvider {
                companion: watch::channel(sill_model::companion::Companion::default()).1,
            }
            .activate(item, choice),
            (ProviderKind::Things | ProviderKind::Memory, _) => Activation::Nothing,
            // The preview kinds: their rows are `m9.rs`'s.
            (
                ProviderKind::Files
                | ProviderKind::Clipboard
                | ProviderKind::Emoji
                | ProviderKind::Web,
                choice,
            ) => m9::activate(item, choice),
        }
    }
}

/// Providers for every kind in the catalogue.
pub fn providers() -> LauncherProviders {
    LauncherProviders(
        ProviderKind::ALL
            .iter()
            .map(|kind| Arc::new(FakeProvider(*kind)) as Arc<dyn Provider>)
            .collect(),
    )
}
