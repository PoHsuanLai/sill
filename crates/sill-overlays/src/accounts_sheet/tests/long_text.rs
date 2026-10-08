//! What a long word, a long address and a long link do to the props, and how the provider list is
//! ordered for a search.

use ds_shell::accounts::model::ProviderPick;
use porter_core::consent::{AccountChoice as AskChoice, ConsentAsk, Usage};
use porter_core::sheet::{ProviderKind, ProviderRow, Review, ReviewView, RowKind, SheetView};
use porter_core::{AccountLabel, CapabilityKind, DataClass, WebUrl};
use sill_model::accounts::sheet::SheetAction;

use super::super::props::Step;
use super::{account, app, provider, shown};

fn row(id: &str, label: &str) -> ProviderRow {
    ProviderRow {
        id: provider(id),
        label: label.to_owned(),
        mark: id.to_owned(),
        kind: RowKind::Provider,
        auth: ProviderKind::Service,
    }
}

fn labels_after(query: &str, rows: Vec<ProviderRow>) -> Vec<String> {
    let Some(Step::Providers(props)) = shown(
        SheetView::Providers(rows),
        vec![SheetAction::Query(query.to_owned())],
    ) else {
        panic!("no provider list");
    };
    props
        .providers
        .into_iter()
        .map(|entry| entry.label)
        .collect()
}

fn listed() -> Vec<ProviderRow> {
    vec![
        row("anthropic", "Anthropic"),
        row("google-ai", "Google (Gemini)"),
        row("google", "Google"),
        row("fastmail", "Fastmail"),
        row("nextcloud", "Nextcloud"),
    ]
}

#[test]
fn an_exact_label_ranks_first_then_a_prefix_then_the_rest() {
    assert_eq!(
        labels_after("Google", listed())[..2],
        ["Google", "Google (Gemini)"]
    );
    assert_eq!(
        labels_after("  google ", listed())[..2],
        ["Google", "Google (Gemini)"]
    );
    // Neither exact: the labels that start with it come before the ones that only hold it.
    assert_eq!(
        labels_after(
            "mail",
            vec![
                row("a", "Webmail"),
                row("b", "Mail.ru"),
                row("c", "Fastmail")
            ]
        )[..],
        ["Mail.ru", "Webmail", "Fastmail"]
    );
    // Nothing typed: the list is as porter sent it.
    assert_eq!(
        labels_after("", listed()),
        [
            "Anthropic",
            "Google (Gemini)",
            "Google",
            "Fastmail",
            "Nextcloud"
        ]
    );
}

#[test]
fn the_cursor_keeps_its_row_whatever_the_order() {
    let Some(Step::Providers(props)) = shown(
        SheetView::Providers(listed()),
        vec![
            SheetAction::Query("Google".to_owned()),
            SheetAction::Cursor(sill_model::accounts::sheet::ListKey::Provider(provider(
                "google-ai",
            ))),
        ],
    ) else {
        panic!("no provider list");
    };
    assert!(matches!(props.cursor, Some(ProviderPick::Provider(key)) if key.0 == "google-ai"));
}

fn consent_for(id: &str, label: &str) -> SheetView {
    SheetView::Consent(ConsentAsk {
        app_label: None,
        app: app(id),
        kind: CapabilityKind::Mail,
        class: DataClass::Mail,
        usage: Usage::Interactive,
        accounts: vec![AskChoice {
            account: account("home"),
            label: AccountLabel(label.to_owned()),
            provider: provider("nextcloud"),
        }],
        session: None,
    })
}

#[test]
fn a_long_app_id_is_cut_short_with_an_ellipsis_at_its_end() {
    let long = "ApplicationNameThatKeepsGoingAndGoingAndGoingForever";
    let Some(Step::Consent(props)) = shown(
        consent_for(&format!("org.example.{long}"), "a@b.test"),
        vec![],
    ) else {
        panic!("no consent");
    };
    assert!(props.app.chars().count() <= 28, "{}", props.app);
    assert!(props.app.starts_with("ApplicationName"), "{}", props.app);
    assert!(props.app.ends_with('\u{2026}'), "{}", props.app);
    let Some(Step::Consent(props)) = shown(consent_for("org.quire.Photos", "a@b.test"), vec![])
    else {
        panic!("no consent");
    };
    assert_eq!(props.app, "Photos");
}

#[test]
fn a_long_account_label_is_cut_short_on_the_review_and_the_consent_line() {
    let long = format!("{}@rig.test", "a".repeat(300));
    let Some(Step::Consent(props)) = shown(consent_for("org.quire.Photos", &long), vec![]) else {
        panic!("no consent");
    };
    let label = &props.choices[0].label;
    assert!(
        label.chars().count() <= 48 && label.ends_with('\u{2026}'),
        "{label}"
    );
    let review = SheetView::Review(ReviewView {
        allow_label: None,
        provider: provider("nextcloud"),
        row: None,
        review: Review {
            label: AccountLabel(long),
            endpoints: vec![],
            services: vec![],
        },
        allow: Some(app(
            "org.example.ApplicationNameThatKeepsGoingAndGoingAndGoing",
        )),
    });
    let Some(Step::Review(props)) = shown(review, vec![]) else {
        panic!("no review");
    };
    assert!(props.account.chars().count() <= 48 && props.account.ends_with('\u{2026}'));
    assert!(props.allow.is_some_and(|app| app.chars().count() <= 28));
}

#[test]
fn the_browser_step_prints_the_host_and_copies_the_whole_address() {
    let whole = format!(
        "https://login.example.org/oauth2/v2.0/authorize?client_id=abc&scope={}",
        "x".repeat(600)
    );
    let url = WebUrl::parse(&whole).unwrap_or_else(|e| panic!("{e}"));
    let Some(Step::Browser { shown, url, .. }) = shown(
        SheetView::BrowserWait {
            provider: provider("microsoft"),
            row: None,
            url,
        },
        vec![],
    ) else {
        panic!("no browser step");
    };
    assert_eq!(shown, "login.example.org/\u{2026}");
    assert_eq!(url, whole);
}

#[test]
fn an_address_with_nothing_after_the_host_is_the_host() {
    let url = WebUrl::parse("https://example.org").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(super::super::words::short_link(&url), "example.org");
}

#[test]
fn a_title_names_the_provider_by_its_rows_label() {
    let mut fake = row("fake-mail", "Fake Mail");
    fake.kind = RowKind::Provider;
    let Some(step) = shown(
        SheetView::Working {
            provider: provider("fake-mail"),
            row: Some(fake),
        },
        vec![],
    ) else {
        panic!("no step");
    };
    assert_eq!(step.title(), "Signing in to Fake Mail");
    let Some(step) = shown(
        SheetView::Working {
            provider: provider("fake-mail"),
            row: None,
        },
        vec![],
    ) else {
        panic!("no step");
    };
    assert_eq!(step.title(), "Signing in to fake-mail");
}
