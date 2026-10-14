//! Open windows, by title and app name.

use nucleo_matcher::Matcher;
use sill_launcher::{
    Action, ActionId, Activation, Batch, Choice, FrecencyScore, IconRef, Item, ItemId, Needle,
    Provider, ProviderKind, Query, RankWeights, ResultSink, Score, WindowKey, matching, rank_text,
};
use tokio::sync::watch;

use sill_model::apps::{AppIcon, AppIndex, match_app};
use sill_model::windows::{WindowModel, Workspaces};

/// The id of a window row's one action: asks the daemon to close it (`Activation::CloseWindow`).
const CLOSE_ACTION: &str = "close";

/// Rows for every toplevel matching the query, most recent first on ties (the order
/// [`Toplevels`](sill_model::windows::Toplevels) already keeps).
#[derive(Debug)]
pub struct WindowsProvider {
    pub windows: watch::Receiver<WindowModel>,
    pub apps: watch::Receiver<AppIndex>,
}

impl Provider for WindowsProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Windows
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        if query.is_cancelled() {
            return;
        }
        let needle = Needle::parse(&query.text);
        let weights = RankWeights::DEFAULT;
        let mut matcher = matching::matcher();
        let WindowModel {
            workspaces,
            toplevels,
        } = self.windows.borrow().clone();
        let apps = self.apps.borrow().clone();

        let mut items = Vec::new();
        for window in &toplevels.windows {
            let app = match_app(&window.app_id, None, &apps).and_then(|id| apps.get(&id));
            let app_name = app.map_or(window.app_id.as_str(), |a| a.name.as_str());
            let workspace = window
                .workspaces
                .first()
                .and_then(|key| workspace_name(&workspaces, *key));

            let mut texts: Vec<&str> = Vec::with_capacity(2);
            if !window.title.is_empty() {
                texts.push(window.title.as_str());
            }
            texts.push(app_name);

            let Some(score) = best_score(&needle, &texts, &weights, &mut matcher) else {
                continue;
            };
            items.push(Item {
                id: ItemId(window.key.0.to_string()),
                provider: ProviderKind::Windows,
                title: if window.title.is_empty() {
                    app_name.to_owned()
                } else {
                    window.title.clone()
                },
                subtitle: subtitle_of(app_name, workspace.as_deref()),
                icon: icon_of(app.map(|a| &a.icon)),
                score,
                actions: vec![Action {
                    id: ActionId(CLOSE_ACTION.to_owned()),
                    label: "Close".to_owned(),
                    keys: sill_launcher::ActionKeys::None,
                }],
                subject: sill_launcher::Subject::None,
            });
        }

        if query.is_cancelled() {
            return;
        }
        // Stable: ties keep `toplevels`' own most-recently-activated order.
        items.sort_by_key(|item| std::cmp::Reverse(item.score));
        sink.push(Batch {
            provider: ProviderKind::Windows,
            generation: query.generation,
            items,
        });
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        match choice {
            Choice::Primary => Activation::FocusWindow {
                window: window_key_of(item),
            },
            // The runner routes `CloseWindow` to `WlCommand::Close`.
            Choice::Action(_) => Activation::CloseWindow {
                window: window_key_of(item),
            },
        }
    }
}

/// `title, app name` when `title` is present, `app name` alone otherwise, no workspace shown
/// (a window on no workspace, or on every workspace, states nothing about one).
fn subtitle_of(app_name: &str, workspace: Option<&str>) -> String {
    match workspace {
        Some(workspace) if !workspace.is_empty() => format!("{app_name} — {workspace}"),
        _ => app_name.to_owned(),
    }
}

/// The best rank of any of `texts` against `needle`, or `None` when none match. Windows are not
/// scored by frecency (no per-window history is kept; [`FrecencyScore::NONE`] leaves ranking to
/// the match and prefix terms alone).
fn best_score(
    needle: &Needle,
    texts: &[&str],
    weights: &RankWeights,
    matcher: &mut Matcher,
) -> Option<Score> {
    texts
        .iter()
        .filter_map(|text| rank_text(needle, text, FrecencyScore::NONE, weights, matcher))
        .max()
}

/// The name of the workspace `key`, if any group still has it (21-SPACES; a window's workspace
/// can outlive its own `Toplevel` update by one `done`).
fn workspace_name(workspaces: &Workspaces, key: sill_model::ids::WorkspaceKey) -> Option<String> {
    workspaces
        .groups
        .iter()
        .flat_map(|group| &group.workspaces)
        .find(|workspace| workspace.key == key)
        .map(|workspace| workspace.name.clone())
}

/// The row's icon: the matched app's resolved icon, or the `window` glyph (design/08-ICONS.md
/// §1.6: "Window ... windows provider, dock menu") when no app matched or it has none.
fn icon_of(icon: Option<&AppIcon>) -> IconRef {
    match icon {
        Some(AppIcon::File(path)) => IconRef::File(path.clone()),
        Some(AppIcon::None) | None => IconRef::Glyph("window".to_owned()),
    }
}

/// The [`WindowKey`] a row's id encodes (`query` only ever produces one from
/// `window.key.0.to_string()`; a malformed id — never produced by this provider — activates
/// key 0 rather than panicking).
fn window_key_of(item: &Item) -> WindowKey {
    WindowKey(item.id.0.parse().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::WindowsProvider;
    use sill_launcher::{
        AppId, Cancel, Choice, IconRef, Item, ItemId, Provider, ProviderKind, Query, Score,
        VecSink, WindowKey,
    };
    use sill_model::apps::{App, AppIcon, AppIndex, AppsEvent, DbusActivation, Listing, Terminal};
    use sill_model::ids::{GroupKey, OutputName, WorkspaceKey};
    use sill_model::windows::{
        Activity, Sizing, Toplevel, ToplevelEvent, Toplevels, WindowModel, WindowState, Workspace,
        WorkspaceGroup, Workspaces, toplevels::step,
    };
    use std::path::PathBuf;

    fn window(key: u64, title: &str, app_id: &str, workspaces: Vec<WorkspaceKey>) -> Toplevel {
        Toplevel {
            key: WindowKey(key),
            app_id: app_id.to_owned(),
            title: title.to_owned(),
            activity: Activity::Inactive,
            window: WindowState::Normal,
            sizing: Sizing::Floating,
            workspaces,
            outputs: Vec::new(),
        }
    }

    fn toplevels(windows: Vec<Toplevel>) -> Toplevels {
        windows.into_iter().fold(Toplevels::default(), |t, w| {
            step(t, ToplevelEvent::Opened(w))
        })
    }

    fn provider(windows: Vec<Toplevel>, apps: Vec<App>) -> WindowsProvider {
        let (_atx, arx) = tokio::sync::watch::channel(sill_model::apps::step(
            AppIndex::default(),
            AppsEvent::Scanned(apps),
        ));
        let (_stx, srx) = tokio::sync::watch::channel(WindowModel {
            workspaces: Workspaces {
                groups: vec![WorkspaceGroup {
                    key: GroupKey(1),
                    outputs: vec![OutputName("DP-1".to_owned())],
                    workspaces: vec![
                        workspace(WorkspaceKey(1), "1"),
                        workspace(WorkspaceKey(2), "2"),
                    ],
                }],
            },
            toplevels: toplevels(windows),
        });
        WindowsProvider {
            windows: srx,
            apps: arx,
        }
    }

    fn workspace(key: WorkspaceKey, name: &str) -> Workspace {
        Workspace {
            key,
            name: name.to_owned(),
            stable_id: String::new(),
            activity: sill_model::windows::Activity::Inactive,
            urgency: sill_model::units::Urgency::Calm,
            visibility: sill_model::units::Visibility::Shown,
            coordinates: Vec::new(),
            capabilities: Default::default(),
            pinning: Default::default(),
        }
    }

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
            categories: Vec::new(),
            keyboard_profile: None,
            keywords: Vec::new(),
            listing: Listing::Shown,
            path: PathBuf::from(format!("/usr/share/applications/{id}.desktop")),
        }
    }

    fn rows(provider: &WindowsProvider, text: &str) -> Vec<(String, String)> {
        let cancel = Cancel::new();
        let query = Query::begin(text, &cancel);
        let mut sink = VecSink::default();
        provider.query(&query, &mut sink);
        sink.0
            .into_iter()
            .flat_map(|batch| batch.items)
            .map(|item| (item.title, item.subtitle))
            .collect()
    }

    #[test]
    fn a_query_table_matches_title_or_app_name() {
        let windows = vec![
            window(
                1,
                "Inbox — Thunderbird",
                "thunderbird",
                vec![WorkspaceKey(1)],
            ),
            window(2, "", "org.mozilla.firefox", vec![WorkspaceKey(2)]),
            window(3, "notes.txt", "foot", vec![]),
        ];
        let apps = vec![app("org.mozilla.firefox", "Firefox"), app("foot", "Foot")];
        let provider = provider(windows, apps);

        const CASES: &[(&str, &[&str])] = &[
            ("inbox", &["Inbox — Thunderbird"]),
            ("thunderbird", &["Inbox — Thunderbird"]),
            ("firefox", &["Firefox"]), // empty title falls back to the app name
            ("notes", &["notes.txt"]),
            ("zzz", &[]),
        ];
        for &(text, want) in CASES {
            let got: Vec<String> = rows(&provider, text).into_iter().map(|(t, _)| t).collect();
            assert_eq!(got, want, "{text:?}");
        }
    }

    #[test]
    fn the_subtitle_names_the_app_and_workspace() {
        let windows = vec![window(1, "Inbox", "thunderbird", vec![WorkspaceKey(2)])];
        let apps = vec![app("thunderbird", "Thunderbird")];
        let provider = provider(windows, apps);
        assert_eq!(
            rows(&provider, "inbox"),
            vec![("Inbox".to_owned(), "Thunderbird — 2".to_owned())]
        );
    }

    #[test]
    fn ties_keep_the_toplevels_most_recently_activated_order() {
        // `window` builds every toplevel inactive; `toplevels` keeps insertion order for ties.
        let windows = vec![
            window(1, "term one", "foot", vec![]),
            window(2, "term two", "foot", vec![]),
        ];
        let provider = provider(windows, vec![app("foot", "Foot")]);
        let got: Vec<String> = rows(&provider, "term")
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        assert_eq!(got, vec!["term one", "term two"]);
    }

    #[test]
    fn no_app_match_falls_back_to_the_raw_app_id_and_the_window_glyph() {
        let windows = vec![window(1, "Mystery", "com.example.unknown", vec![])];
        let provider = provider(windows, Vec::new());
        let cancel = Cancel::new();
        let query = Query::begin("mystery", &cancel);
        let mut sink = VecSink::default();
        provider.query(&query, &mut sink);
        let item = &sink.0[0].items[0];
        assert_eq!(item.subtitle, "com.example.unknown");
        assert_eq!(item.icon, IconRef::Glyph("window".to_owned()));
    }

    #[test]
    fn a_cancelled_query_yields_nothing() {
        let provider = provider(vec![window(1, "term", "foot", vec![])], Vec::new());
        let cancel = Cancel::new();
        let query = Query::begin("term", &cancel);
        cancel.begin();
        let mut sink = VecSink::default();
        provider.query(&query, &mut sink);
        assert!(sink.0.is_empty());
    }

    #[test]
    fn activate_focuses_on_primary_and_closes_on_the_close_action() {
        let provider = provider(Vec::new(), Vec::new());
        let item = Item {
            id: ItemId("42".to_owned()),
            provider: ProviderKind::Windows,
            title: "term".to_owned(),
            subtitle: String::new(),
            icon: IconRef::None,
            score: Score(0),
            actions: Vec::new(),
            subject: sill_launcher::Subject::None,
        };
        assert_eq!(
            provider.activate(&item, &Choice::Primary),
            sill_launcher::Activation::FocusWindow {
                window: WindowKey(42)
            }
        );
        assert_eq!(
            provider.activate(
                &item,
                &Choice::Action(sill_launcher::ActionId("close".to_owned()))
            ),
            sill_launcher::Activation::CloseWindow {
                window: WindowKey(42)
            }
        );
    }
}
