//! What the launcher shows for a typed query, end to end through the pure part of the runner: the
//! apps and emoji providers answer, `step` merges their batches and `arranged` places them in
//! Spotlight's sections. Top Hit comes from the primary providers only, an app that starts with
//! the query outranks one that merely contains it, and letters scattered along a long title
//! (`term` in "Capture Current Monitor") come last. The apps are the ones the nested readiness run
//! found installed.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sill_launcher::{
    AppId, Cancel, Frecency, Placement, Provider, Query, Scope, Section, SectionPlan, UnixTime,
    VecSink,
};
use sill_model::apps::{
    App, AppIcon, AppIndex, AppsEvent, DbusActivation, DesktopActionEntry, Listing, Terminal, step,
};
use sill_model::results::{
    Asked, LauncherPolicy, Results, ResultsEvent, Typed, arranged, step as merge,
};
use sill_search::providers::{AppsProvider, EmojiProvider};

/// An installed app: `(name, generic name, keywords, desktop actions)`.
type Installed<'a> = (&'a str, &'a str, &'a [&'a str], &'a [&'a str]);

const INSTALLED: &[Installed] = &[
    ("KMail", "Mail Client", &[], &["Check Mail"]),
    (
        "KMail Header Theme Editor",
        "Mail Header Theme Editor",
        &[],
        &[],
    ),
    ("Mailo", "", &[], &[]),
    ("KMail Import Wizard", "", &[], &[]),
    ("NeoChat", "Matrix Client", &[], &[]),
    ("Example Terminal", "", &[], &[]),
    ("Ghostty", "", &["terminal"], &[]),
    ("Konsole", "Terminal", &[], &[]),
    ("Telegram", "", &[], &["Quit Telegram"]),
    (
        "Spectacle",
        "Screenshot Capture Utility",
        &[],
        &["Capture Current Monitor"],
    ),
    ("KTnef", "TNEF File Viewer", &[], &[]),
];

fn app((name, generic, keywords, actions): &Installed) -> App {
    App {
        id: AppId(format!("test.{}", name.to_lowercase().replace(' ', "-"))),
        name: (*name).to_owned(),
        generic_name: (*generic).to_owned(),
        comment: String::new(),
        exec: "true".to_owned(),
        icon: AppIcon::None,
        startup_wm_class: None,
        flatpak: None,
        terminal: Terminal::No,
        activation: DbusActivation::Exec,
        actions: actions
            .iter()
            .map(|action| DesktopActionEntry {
                id: action.to_lowercase().replace(' ', "-"),
                name: (*action).to_owned(),
                exec: "true".to_owned(),
            })
            .collect(),
        mime_types: Vec::new(),
        categories: Vec::new(),
        keyboard_profile: None,
        keywords: keywords.iter().map(|word| (*word).to_owned()).collect(),
        listing: Listing::Shown,
        path: PathBuf::new(),
    }
}

/// The launcher's sections for `text`, every provider asked.
fn shown(text: &str) -> Placement {
    let index = step(
        AppIndex::default(),
        AppsEvent::Scanned(INSTALLED.iter().map(app).collect()),
    );
    let (_apps_tx, apps) = tokio::sync::watch::channel(index);
    let (_policy_tx, policy) = tokio::sync::watch::channel(LauncherPolicy::default());
    let frecency = Arc::new(Mutex::new(Frecency::default()));
    let providers: Vec<Box<dyn Provider>> = vec![
        Box::new(AppsProvider {
            apps,
            frecency: frecency.clone(),
        }),
        Box::new(EmojiProvider { policy, frecency }),
    ];
    let cancel = Cancel::new();
    let query = Query::begin(text, &cancel);
    let mut results = merge(Results::default(), ResultsEvent::Began(query.generation));
    for provider in &providers {
        let mut sink = VecSink::default();
        provider.query(&query, &mut sink);
        for batch in sink.0 {
            results = merge(results, ResultsEvent::Batch(batch));
        }
    }
    let asked = Asked {
        typed: Typed::of(text),
        scope: Scope::All,
    };
    arranged(
        &results.items,
        &SectionPlan::default_plan(),
        asked,
        &Frecency::default(),
        UnixTime(1_790_000_000),
    )
}

/// The titles of `section`'s rows, in order.
fn titles(placement: &Placement, section: Section) -> Vec<&str> {
    let mut rows = placement.items.iter();
    for span in &placement.spans {
        let taken: Vec<&str> = rows
            .by_ref()
            .take(span.rows as usize)
            .map(|item| item.title.as_str())
            .collect();
        if span.section == section {
            return taken;
        }
    }
    Vec::new()
}

#[test]
fn mail_puts_the_mail_apps_first_and_no_emoji_on_top() {
    let placement = shown("mail");
    assert_eq!(titles(&placement, Section::TopHit), ["Mailo"]);
    let apps = titles(&placement, Section::Applications);
    assert_eq!(
        apps.iter().take(3).copied().collect::<Vec<_>>(),
        ["KMail", "KMail Header Theme Editor", "Check Mail"],
        "{apps:?}"
    );
    assert!(
        !titles(&placement, Section::Emoji).is_empty(),
        "emoji stay, below"
    );
    assert_eq!(
        placement.spans.first().map(|span| span.section),
        Some(Section::TopHit)
    );
}

#[test]
fn term_ranks_the_terminals_above_scattered_letters() {
    let placement = shown("term");
    assert_eq!(titles(&placement, Section::TopHit), ["Example Terminal"]);
    let apps = titles(&placement, Section::Applications);
    assert_eq!(
        apps.iter().take(2).copied().collect::<Vec<_>>(),
        ["Konsole", "Ghostty"],
        "{apps:?}"
    );
    let at = |title: &str| apps.iter().position(|row| *row == title);
    for scattered in ["Capture Current Monitor", "Telegram", "Quit Telegram"] {
        if let Some(scattered_at) = at(scattered) {
            assert!(
                scattered_at > at("Ghostty").unwrap_or(0),
                "{scattered}: {apps:?}"
            );
        }
    }
}

#[test]
fn a_whole_name_beats_a_longer_one_that_starts_with_it() {
    let placement = shown("kmail");
    assert_eq!(titles(&placement, Section::TopHit), ["KMail"]);
}
