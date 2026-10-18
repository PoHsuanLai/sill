//! Installed applications, ranked by name, generic name and keywords with frecency; a matching
//! desktop action (`[Desktop Action ...]`) surfaces as its own row too, so typing "private
//! window" finds Firefox's action directly, not just Firefox itself.

use std::sync::{Arc, Mutex};

use nucleo_matcher::Matcher;
use sill_launcher::{
    Action, ActionId, ActionKeys, Activation, AppId, Batch, Choice, DesktopAction, Frecency,
    FrecencyScore, IconRef, Item, ItemId, ItemKey, Needle, Provider, ProviderKind, Query,
    RankWeights, ResultSink, Score, Subject, UnixTime, matching, rank_text,
};
use tokio::sync::watch;

use crate::clock::unix_now;

use sill_model::apps::{App, AppIcon, AppIndex, DesktopActionEntry, Listing};

/// A row's id separator between an app id and one of its desktop action ids: `\u{1}`, a control
/// character neither an app id (reverse-DNS or a bare token) nor a `[Desktop Action <id>]` id
/// may contain, so decoding never mistakes a literal id for the joiner (CONVENTIONS "substrings
/// are not tokens").
const ACTION_JOINER: char = '\u{1}';

/// How much of its own rank a desktop action keeps, in thousandths: "Check Mail" is a row of
/// KMail and ranks under KMail, as Spotlight lists an app before its actions.
const ACTION_WEIGHT: u32 = 850;

/// Rows for every listed app matching the query, and one row per matching desktop action; the
/// app row's own `actions` menu (Ctrl+K / Alt+K) lists every desktop action regardless of
/// whether it matched on its own.
#[derive(Debug)]
pub struct AppsProvider {
    pub apps: watch::Receiver<AppIndex>,
    pub frecency: Arc<Mutex<Frecency>>,
}

impl Provider for AppsProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Apps
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        if query.is_cancelled() {
            return;
        }
        let needle = Needle::parse(&query.text);
        let weights = RankWeights::DEFAULT;
        let mut matcher = matching::matcher();
        let now = unix_now();
        let index = self.apps.borrow().clone();
        let frecency = snapshot(&self.frecency);

        let mut items = Vec::new();
        for app in &index.apps {
            if app.listing == Listing::Hidden {
                continue;
            }
            items.extend(app_item(
                app,
                &frecency,
                now,
                &needle,
                &weights,
                &mut matcher,
            ));
            if needle.is_empty() {
                // Desktop actions only surface once something is actually typed (an empty query
                // already lists every app by frecency; also listing every one of their actions
                // would drown that view).
                continue;
            }
            for action in &app.actions {
                items.extend(action_item(
                    app,
                    action,
                    &frecency,
                    now,
                    &needle,
                    &weights,
                    &mut matcher,
                ));
            }
        }

        if query.is_cancelled() {
            return;
        }
        items.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.title.cmp(&b.title)));
        sink.push(Batch {
            provider: ProviderKind::Apps,
            generation: query.generation,
            items,
        });
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        let (app, own_action) = split_id(&item.id);
        let action = match (own_action, choice) {
            (Some(action_id), _) => DesktopAction::Named(action_id),
            (None, Choice::Action(action_id)) => DesktopAction::Named(action_id.0.clone()),
            (None, Choice::Primary) => DesktopAction::Default,
        };
        Activation::Launch { app, action }
    }
}

/// `app`'s own row, or `None` when nothing about it matches `needle`.
fn app_item(
    app: &App,
    frecency: &Frecency,
    now: UnixTime,
    needle: &Needle,
    weights: &RankWeights,
    matcher: &mut Matcher,
) -> Option<Item> {
    let id = ItemId(app.id.0.clone());
    let mut texts: Vec<(&str, u32)> = vec![(app.name.as_str(), NAME_WEIGHT)];
    if !app.generic_name.is_empty() {
        texts.push((app.generic_name.as_str(), GENERIC_WEIGHT));
    }
    texts.extend(
        app.keywords
            .iter()
            .map(|keyword| (keyword.as_str(), KEYWORD_WEIGHT)),
    );
    let frecency_score = frecency.score(&ItemKey::of(ProviderKind::Apps, &id), now);
    let score = best_score(needle, &texts, frecency_score, weights, matcher)?;
    Some(Item {
        id,
        provider: ProviderKind::Apps,
        title: app.name.clone(),
        subtitle: app.generic_name.clone(),
        icon: icon_of(&app.icon),
        score,
        actions: app.actions.iter().map(action_of).collect(),
        subject: Subject::App(app.id.clone()),
    })
}

/// One matching desktop action of `app`, as its own row.
fn action_item(
    app: &App,
    action: &DesktopActionEntry,
    frecency: &Frecency,
    now: UnixTime,
    needle: &Needle,
    weights: &RankWeights,
    matcher: &mut Matcher,
) -> Option<Item> {
    let id = ItemId(format!("{}{ACTION_JOINER}{}", app.id.0, action.id));
    let frecency_score = frecency.score(&ItemKey::of(ProviderKind::Apps, &id), now);
    let Score(own) = rank_text(needle, &action.name, frecency_score, weights, matcher)?;
    let score = Score(own * ACTION_WEIGHT / 1000);
    Some(Item {
        id,
        provider: ProviderKind::Apps,
        title: action.name.clone(),
        subtitle: app.name.clone(),
        icon: icon_of(&app.icon),
        score,
        actions: Vec::new(),
        subject: Subject::App(app.id.clone()),
    })
}

/// How much of its rank a match keeps by what it matched, in thousandths: the app's own name in
/// full, its generic name ("Terminal" for Konsole) and its keywords less, so an app named for what
/// was typed ranks above one that only describes itself with it.
const NAME_WEIGHT: u32 = 1000;
const GENERIC_WEIGHT: u32 = 800;
const KEYWORD_WEIGHT: u32 = 700;

/// The best rank of any of `texts` (each with its weight) against `needle`, or `None` when none
/// match.
fn best_score(
    needle: &Needle,
    texts: &[(&str, u32)],
    frecency: FrecencyScore,
    weights: &RankWeights,
    matcher: &mut Matcher,
) -> Option<Score> {
    texts
        .iter()
        .filter_map(|&(text, weight)| {
            let Score(rank) = rank_text(needle, text, frecency, weights, matcher)?;
            Some(Score(rank * weight / 1000))
        })
        .max()
}

fn icon_of(icon: &AppIcon) -> IconRef {
    match icon {
        AppIcon::None => IconRef::None,
        AppIcon::File(path) => IconRef::File(path.clone()),
    }
}

fn action_of(entry: &DesktopActionEntry) -> Action {
    Action {
        id: ActionId(entry.id.clone()),
        label: entry.name.clone(),
        keys: ActionKeys::None,
    }
}

/// The app a launcher row of this provider stands for: its id before [`ACTION_JOINER`] (an app
/// row's whole id, a desktop-action row's app part).
pub fn row_app_id(id: &ItemId) -> AppId {
    split_id(id).0
}

/// `id` split at [`ACTION_JOINER`] into its app id and, for a desktop-action row, that action's
/// id.
fn split_id(id: &ItemId) -> (AppId, Option<String>) {
    match id.0.split_once(ACTION_JOINER) {
        Some((app, action)) => (AppId(app.to_owned()), Some(action.to_owned())),
        None => (AppId(id.0.clone()), None),
    }
}

/// A read-only copy of `frecency`'s current contents; a poisoned lock (a panic elsewhere while
/// holding it) still yields the data rather than poisoning every query after it.
fn snapshot(frecency: &Mutex<Frecency>) -> Frecency {
    frecency
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
}

#[cfg(test)]
mod tests {
    use super::{AppsProvider, split_id};
    use sill_launcher::{
        AppId, Cancel, Choice, DesktopAction, Frecency, IconRef, ItemId, Provider, ProviderKind,
        Query, VecSink,
    };
    use sill_model::apps::{
        App, AppIcon, AppIndex, AppsEvent, DbusActivation, DesktopActionEntry, Listing, Terminal,
    };
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    fn app_with(id: &str, name: &str, edit: impl FnOnce(&mut App)) -> App {
        let mut app = App {
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
        };
        edit(&mut app);
        app
    }

    /// A provider over `apps`; the channel's sender is dropped at once — `query` only ever
    /// borrows the receiver's last value, so nothing needs to keep sending.
    fn provider(apps: Vec<App>) -> AppsProvider {
        let (_tx, rx) = tokio::sync::watch::channel(sill_model::apps::step(
            AppIndex::default(),
            AppsEvent::Scanned(apps),
        ));
        AppsProvider {
            apps: rx,
            frecency: Arc::new(Mutex::new(Frecency::default())),
        }
    }

    fn titles(provider: &AppsProvider, text: &str) -> Vec<String> {
        let cancel = Cancel::new();
        let query = Query::begin(text, &cancel);
        let mut sink = VecSink::default();
        provider.query(&query, &mut sink);
        sink.0
            .into_iter()
            .flat_map(|batch| batch.items)
            .map(|item| item.title)
            .collect()
    }

    #[test]
    fn a_query_table_matches_name_generic_name_and_keywords() {
        let apps = vec![
            app_with("org.mozilla.firefox", "Firefox", |a| {
                a.generic_name = "Web Browser".to_owned();
                a.keywords = vec!["internet".to_owned()];
            }),
            app_with("org.gnome.Nautilus", "Files", |a| {
                a.generic_name = "File Manager".to_owned();
            }),
            app_with("foot", "Terminal", |_| {}),
        ];
        let provider = provider(apps);
        const CASES: &[(&str, &[&str])] = &[
            ("fire", &["Firefox"]),
            ("browser", &["Firefox"]),
            ("internet", &["Firefox"]),
            ("files", &["Files"]),
            ("term", &["Terminal"]),
            ("zzz", &[]),
        ];
        for &(text, want) in CASES {
            assert_eq!(titles(&provider, text), want, "{text:?}");
        }
    }

    #[test]
    fn hidden_apps_never_list() {
        let apps = vec![app_with("hidden", "Hidden App", |a| {
            a.listing = Listing::Hidden;
        })];
        let provider = provider(apps);
        assert!(titles(&provider, "hidden").is_empty());
        assert!(titles(&provider, "").is_empty());
    }

    #[test]
    fn an_empty_query_lists_everything_but_no_desktop_actions() {
        let apps = vec![app_with("foot", "Foot", |a| {
            a.actions = vec![DesktopActionEntry {
                id: "new-window".to_owned(),
                name: "New Window".to_owned(),
                exec: "foot".to_owned(),
            }];
        })];
        let provider = provider(apps);
        assert_eq!(titles(&provider, ""), vec!["Foot"]);
    }

    #[test]
    fn a_matching_desktop_action_is_its_own_row() {
        let apps = vec![app_with("org.mozilla.firefox", "Firefox", |a| {
            a.actions = vec![DesktopActionEntry {
                id: "new-private-window".to_owned(),
                name: "New Private Window".to_owned(),
                exec: "firefox --private-window".to_owned(),
            }];
        })];
        let provider = provider(apps);
        assert_eq!(titles(&provider, "private"), vec!["New Private Window"]);
        // The app itself still lists (and carries the action in its own menu) for its own text.
        assert_eq!(titles(&provider, "firefox"), vec!["Firefox"]);
    }

    #[test]
    fn a_cancelled_query_yields_nothing() {
        let provider = provider(vec![app_with("foot", "Foot", |_| {})]);
        let cancel = Cancel::new();
        let query = Query::begin("foot", &cancel);
        cancel.begin(); // a newer keystroke overtakes `query` before it runs
        let mut sink = VecSink::default();
        provider.query(&query, &mut sink);
        assert!(sink.0.is_empty());
    }

    #[test]
    fn ids_split_at_the_action_joiner() {
        let cases: &[(&str, &str, Option<&str>)] = &[
            ("org.mozilla.firefox", "org.mozilla.firefox", None),
            (
                "org.mozilla.firefox\u{1}new-window",
                "org.mozilla.firefox",
                Some("new-window"),
            ),
        ];
        for &(id, want_app, want_action) in cases {
            let (app, action) = split_id(&ItemId(id.to_owned()));
            assert_eq!(app, AppId(want_app.to_owned()), "{id}");
            assert_eq!(action, want_action.map(str::to_owned), "{id}");
        }
    }

    #[test]
    fn activate_maps_choice_to_the_right_launch() {
        let provider = provider(Vec::new());
        let icon = IconRef::None;
        let main = sill_launcher::Item {
            id: ItemId("org.mozilla.firefox".to_owned()),
            provider: ProviderKind::Apps,
            title: "Firefox".to_owned(),
            subtitle: String::new(),
            icon: icon.clone(),
            score: sill_launcher::Score(0),
            actions: Vec::new(),
            subject: sill_launcher::Subject::None,
        };
        let secondary = sill_launcher::Item {
            id: ItemId("org.mozilla.firefox\u{1}new-window".to_owned()),
            title: "New Window".to_owned(),
            ..main.clone()
        };

        assert_eq!(
            provider.activate(&main, &Choice::Primary),
            sill_launcher::Activation::Launch {
                app: AppId("org.mozilla.firefox".to_owned()),
                action: DesktopAction::Default,
            }
        );
        assert_eq!(
            provider.activate(
                &main,
                &Choice::Action(sill_launcher::ActionId("new-window".to_owned()))
            ),
            sill_launcher::Activation::Launch {
                app: AppId("org.mozilla.firefox".to_owned()),
                action: DesktopAction::Named("new-window".to_owned()),
            }
        );
        assert_eq!(
            provider.activate(&secondary, &Choice::Primary),
            sill_launcher::Activation::Launch {
                app: AppId("org.mozilla.firefox".to_owned()),
                action: DesktopAction::Named("new-window".to_owned()),
            }
        );
    }
}
