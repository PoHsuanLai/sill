//! The windows provider's query table against a fake toplevel list (no compositor, no thread —
//! just the reducer's own public surface, the same one `CosmicWlService` publishes through).

use std::path::PathBuf;

use sill_launcher::{AppId, Cancel, Provider, Query, VecSink, WindowKey};
use sill_services::apps::{App, AppIcon, AppIndex, AppsEvent, DbusActivation, Listing, Terminal};
use sill_services::providers::WindowsProvider;
use sill_services::{
    Activity, CosmicWl, GroupKey, OutputName, Sizing, Toplevel, ToplevelEvent, Toplevels,
    WindowState, Workspace, WorkspaceGroup, WorkspaceKey, Workspaces,
};

fn app(id: &str, name: &str) -> App {
    App {
        id: AppId(id.to_owned()),
        name: name.to_owned(),
        generic_name: String::new(),
        comment: String::new(),
        exec: id.to_owned(),
        icon: AppIcon::None,
        startup_wm_class: None,
        flatpak: None,
        terminal: Terminal::No,
        activation: DbusActivation::Exec,
        actions: Vec::new(),
        mime_types: Vec::new(),
        keywords: Vec::new(),
        listing: Listing::Shown,
        path: PathBuf::from(format!("/usr/share/applications/{id}.desktop")),
    }
}

fn window(key: u64, title: &str, app_id: &str, workspace: Option<u64>) -> Toplevel {
    Toplevel {
        key: WindowKey(key),
        app_id: app_id.to_owned(),
        title: title.to_owned(),
        activity: Activity::Inactive,
        window: WindowState::Normal,
        sizing: Sizing::Floating,
        workspaces: workspace.into_iter().map(WorkspaceKey).collect(),
        outputs: Vec::new(),
    }
}

fn workspace(key: u64, name: &str) -> Workspace {
    Workspace {
        key: WorkspaceKey(key),
        name: name.to_owned(),
        stable_id: String::new(),
        activity: Activity::Inactive,
        urgency: sill_services::Urgency::Calm,
        visibility: sill_services::Visibility::Shown,
        coordinates: Vec::new(),
        capabilities: Default::default(),
        pinning: Default::default(),
    }
}

fn provider() -> WindowsProvider {
    let windows = vec![
        window(1, "Inbox — Thunderbird", "thunderbird", Some(1)),
        window(
            2,
            "docs.rs — Mozilla Firefox",
            "org.mozilla.firefox",
            Some(2),
        ),
        window(3, "", "cosmic-term", None),
    ];
    let toplevels = windows.into_iter().fold(Toplevels::default(), |t, w| {
        sill_services::cosmic_wl::toplevels::step(t, ToplevelEvent::Opened(w))
    });
    let apps = sill_services::apps::step(
        AppIndex::default(),
        AppsEvent::Scanned(vec![
            app("thunderbird", "Thunderbird"),
            app("org.mozilla.firefox", "Firefox"),
            app("cosmic-term", "Terminal"),
        ]),
    );
    let workspaces = Workspaces {
        groups: vec![WorkspaceGroup {
            key: GroupKey(1),
            outputs: vec![OutputName("DP-1".to_owned())],
            workspaces: vec![workspace(1, "Mail"), workspace(2, "Web")],
        }],
    };
    let (_wtx, wrx) = tokio::sync::watch::channel(CosmicWl {
        workspaces,
        toplevels,
    });
    let (_atx, arx) = tokio::sync::watch::channel(apps);
    WindowsProvider {
        cosmic_wl: wrx,
        apps: arx,
    }
}

fn rows(provider: &WindowsProvider, text: &str) -> Vec<(String, String)> {
    let cancel = Cancel::new();
    let query = Query::begin(text, &cancel);
    let mut sink = VecSink::default();
    provider.query(&query, &mut sink);
    let mut rows: Vec<(String, String)> = sink
        .0
        .into_iter()
        .flat_map(|batch| batch.items)
        .map(|item| (item.title, item.subtitle))
        .collect();
    rows.sort();
    rows
}

#[test]
fn a_query_table_matches_titles_and_app_names_against_a_fake_toplevel_list() {
    let provider = provider();
    const CASES: &[(&str, &[(&str, &str)])] = &[
        ("inbox", &[("Inbox — Thunderbird", "Thunderbird — Mail")]),
        (
            "thunderbird",
            &[("Inbox — Thunderbird", "Thunderbird — Mail")],
        ),
        ("docs.rs", &[("docs.rs — Mozilla Firefox", "Firefox — Web")]),
        ("firefox", &[("docs.rs — Mozilla Firefox", "Firefox — Web")]),
        // an empty title (the terminal) falls back to its app name for both title and match
        ("terminal", &[("Terminal", "Terminal")]),
        ("no such window anywhere", &[]),
    ];
    for &(text, want) in CASES {
        let want: Vec<(String, String)> = want
            .iter()
            .map(|&(title, subtitle)| (title.to_owned(), subtitle.to_owned()))
            .collect();
        assert_eq!(rows(&provider, text), want, "{text:?}");
    }
}
