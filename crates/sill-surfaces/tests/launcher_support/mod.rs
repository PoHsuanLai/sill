//! Fake services for the launcher's tests and `examples/launcher_demo.rs`: the bar's fakes, a
//! search runner that answers every `SearchCommand::Query` on the frozen results `watch`
//! channel with a new generation (as `sill_services::results::spawn` will once launcher-core
//! lands), and providers that say what a fixed set of rows does.

#![allow(dead_code)] // Each test binary uses its own subset.

#[path = "../support/fake.rs"]
pub mod fake;

use std::sync::{Arc, Mutex};

use sill_launcher::{
    Action, ActionId, Activation, AppId, Choice, DesktopAction, Generation, IconRef, Item, ItemId,
    Provider, ProviderKind, Query, ResultSink, Score, SettingsTarget, SystemCommand, WindowKey,
};
use sill_services::{Control, Results, SearchCommand, Services};
use sill_surfaces::surfaces::launcher::LauncherProviders;
use tokio::sync::watch;

pub const FIREFOX: &str = "org.mozilla.firefox";
pub const FILES: &str = "com.system76.CosmicFiles";
pub const TERM: &str = "com.system76.CosmicTerm";
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
        icon: IconRef::None,
        score: Score(score),
        actions: Vec::new(),
    }
}

fn with_actions(mut item: Item, actions: &[(&str, &str)]) -> Item {
    item.actions = actions
        .iter()
        .map(|(id, label)| Action {
            id: ActionId((*id).to_owned()),
            label: (*label).to_owned(),
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
            row(Apps, FILES, "Files", "COSMIC File Manager", 800),
            &[("new-window", "New Window")],
        ),
        row(
            Windows,
            "10",
            "Firefox — sill launcher",
            "Firefox, workspace 2",
            750,
        ),
        row(Apps, TERM, "Terminal", "COSMIC Terminal", 700),
        row(Settings, "wireless", "Wi-Fi", "Network & Wireless", 500),
        row(Settings, "display", "Display", "Resolution and scale", 450),
        row(System, "lock", "Lock Screen", "loginctl lock-session", 300),
        row(System, "log-out", "Log Out", "End this session", 250),
        row(System, "restart", "Restart", "", 200),
    ]
}

/// The rows a query finds: every row for the empty query, else those whose title or subtitle
/// holds it (case-insensitively); the calculator answers only a query with a digit.
pub fn search(query: &str) -> Vec<Item> {
    let needle = query.trim().to_lowercase();
    catalogue()
        .into_iter()
        .filter(|item| match item.provider {
            ProviderKind::Calculator => needle.chars().any(|c| c.is_ascii_digit()),
            _ => {
                needle.is_empty()
                    || item.title.to_lowercase().contains(&needle)
                    || item.subtitle.to_lowercase().contains(&needle)
            }
        })
        .collect()
}

/// Fake services whose search is answered by a runner thread, and their senders.
pub fn services() -> (Services, LauncherFeeds) {
    let (mut services, bar) = fake::services();
    let (results_tx, results_rx) = watch::channel(Results::default());
    let (control, mut commands) = Control::channel();
    services.results = results_rx;
    services.search = control;
    let searches = Arc::new(Mutex::new(Vec::new()));
    let (log, tx) = (searches.clone(), results_tx.clone());
    std::thread::spawn(move || {
        let mut generation = 0;
        while let Some(command) = commands.blocking_recv() {
            if let SearchCommand::Query(text) = &command {
                generation += 1;
                tx.send_replace(Results {
                    generation: Generation(generation),
                    items: search(text),
                });
            }
            if let Ok(mut log) = log.lock() {
                log.push(command);
            }
        }
    });
    let feeds = LauncherFeeds {
        bar,
        results: results_tx,
        searches,
    };
    (services, feeds)
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
                target: SettingsTarget::Cosmic(item.id.0.clone()),
            },
            (ProviderKind::System, _) => Activation::System(SystemCommand::Lock),
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
