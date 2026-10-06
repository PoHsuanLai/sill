//! The `SheetView` to props table, and the events' way back. Pure: no renderer.

use ds::components::content::provider_mark::MarkProvider;
use ds::prelude::Check;
use ds_shell::accounts::hidden::Hidden;
use ds_shell::accounts::model::{
    AccountChoice, AllowScope, Attempt, ChoiceKey, ConsentAnswer, CopyState, FieldProblem,
    FieldRole, FieldText, FormField, Limitation, ProblemKind, ProviderEntry, ProviderKey,
    ProviderPick, Requirement, ServiceKey, ServiceLine, ServiceOffer, SignInFault,
};
use porter_core::consent::{AccountChoice as AskChoice, ConsentAsk, GrantScope, Usage};
use porter_core::sheet::{
    Entry, FieldKind, FieldSpec, FieldValue, Presence, ProblemKind as PorterProblem, ProviderRow,
    Review, ReviewView, RowKind, ServiceRow, ServiceState, SheetView, SignInFault as PorterFault,
    SignInView, UserCode,
};
use porter_core::{
    AbsentReason, AccountId, AccountLabel, AppId, AppName, CapabilityKind, DataClass, EndpointUrl,
    Isolation, LimitReason, ProviderId, SecretText, Toggle, WebUrl,
};
use sill_model::accounts::sheet::{
    AccountsSheets, CopyMark, ListKey, SheetAction, SheetEvent, SheetHandle, step,
};

use super::events;
use super::props::{ConsentProps, ProvidersProps, ReviewProps, SignInProps, Step, step_of};
use super::stage::{Here, drawn};

fn provider(id: &str) -> ProviderId {
    ProviderId::parse(id).unwrap_or_else(|e| panic!("{e}"))
}

fn account(id: &str) -> AccountId {
    AccountId::parse(id).unwrap_or_else(|e| panic!("{e}"))
}

fn app(name: &str) -> AppId {
    AppId {
        name: AppName::parse(name).unwrap_or_else(|e| panic!("{e}")),
        isolation: Isolation::Flatpak,
    }
}

/// `view` opened as sheet "a", then `acts` done to it; the props of what is on screen.
fn shown(view: SheetView, acts: Vec<SheetAction>) -> Option<Step> {
    let handle = SheetHandle::parse("a").unwrap_or_else(|| panic!("handle"));
    let (mut sheets, _) = step(
        AccountsSheets::default(),
        SheetEvent::Opened(handle.clone(), view),
    );
    for action in acts {
        sheets = step(sheets, SheetEvent::Acted(handle.clone(), action)).0;
    }
    sheets.open.first().and_then(step_of)
}

fn spec(kind: FieldKind, entry: Entry, presence: Presence) -> FieldSpec {
    FieldSpec {
        kind,
        entry,
        presence,
        prefill: None,
    }
}

fn row(kind: CapabilityKind, state: ServiceState, limit: Option<LimitReason>) -> ServiceRow {
    ServiceRow { kind, state, limit }
}

fn line(key: &str, name: &str, offer: ServiceOffer, limit: Option<Limitation>) -> ServiceLine {
    ServiceLine {
        key: ServiceKey(key.to_owned()),
        name: name.to_owned(),
        offer,
        limit,
    }
}

#[test]
fn every_sheet_view_becomes_the_props_of_its_part() {
    let consent = SheetView::Consent(ConsentAsk {
        app: app("org.quire.Photos"),
        kind: CapabilityKind::Storage,
        class: DataClass::AppOwn,
        usage: Usage::Interactive,
        accounts: vec![
            AskChoice {
                account: account("home"),
                label: AccountLabel("ada@home.example".to_owned()),
                provider: provider("nextcloud"),
            },
            AskChoice {
                account: account("work"),
                label: AccountLabel("ada@work.example".to_owned()),
                provider: provider("microsoft"),
            },
        ],
    });
    let providers = SheetView::Providers(vec![
        ProviderRow {
            id: provider("nextcloud"),
            label: "Nextcloud".to_owned(),
            mark: "nextcloud".to_owned(),
            kind: RowKind::Provider,
        },
        ProviderRow {
            id: provider("google"),
            label: "Google".to_owned(),
            mark: "google".to_owned(),
            kind: RowKind::Provider,
        },
    ]);
    let form = SheetView::SignIn(SignInView {
        provider: provider("nextcloud"),
        fields: vec![
            spec(FieldKind::Address, Entry::Plain, Presence::Required),
            spec(FieldKind::Password, Entry::Secret, Presence::Required),
            spec(FieldKind::Server, Entry::Plain, Presence::Optional),
        ],
        problem: Some(porter_core::sheet::FieldProblem {
            field: FieldKind::Password,
            problem: PorterProblem::Refused,
        }),
    });
    let review = SheetView::Review(ReviewView {
        provider: provider("microsoft"),
        review: Review {
            label: AccountLabel("ada@work.example".to_owned()),
            services: vec![
                row(
                    CapabilityKind::Mail,
                    ServiceState::Offered(Toggle::On),
                    None,
                ),
                row(
                    CapabilityKind::Storage,
                    ServiceState::Offered(Toggle::Off),
                    Some(LimitReason::AppFolderOnly),
                ),
                row(
                    CapabilityKind::Photos,
                    ServiceState::Absent(AbsentReason::UnverifiedBuild),
                    None,
                ),
            ],
            endpoints: Vec::new(),
        },
        allow: Some(app("org.quire.Mail")),
    });
    let url = WebUrl::parse("https://login.example.org/auth?x=1").unwrap_or_else(|e| panic!("{e}"));
    let device = EndpointUrl::parse("https://example.org/device").unwrap_or_else(|e| panic!("{e}"));
    let cases: Vec<(&str, SheetView, Option<Step>)> = vec![
        (
            "consent",
            consent,
            Some(Step::Consent(ConsentProps {
                app: "Photos".to_owned(),
                request: "keep its data in your files".to_owned(),
                choices: vec![
                    AccountChoice {
                        key: ChoiceKey("home".to_owned()),
                        label: "ada@home.example".to_owned(),
                        provider: MarkProvider::Imap,
                    },
                    AccountChoice {
                        key: ChoiceKey("work".to_owned()),
                        label: "ada@work.example".to_owned(),
                        provider: MarkProvider::Microsoft,
                    },
                ],
                chosen: None,
            })),
        ),
        (
            "providers",
            providers,
            Some(Step::Providers(ProvidersProps {
                providers: vec![
                    ProviderEntry {
                        key: ProviderKey("nextcloud".to_owned()),
                        label: "Nextcloud".to_owned(),
                        mark: MarkProvider::Imap,
                    },
                    ProviderEntry {
                        key: ProviderKey("google".to_owned()),
                        label: "Google".to_owned(),
                        mark: MarkProvider::Google,
                    },
                ],
                query: String::new(),
                cursor: None,
            })),
        ),
        (
            "sign in",
            form,
            Some(Step::SignIn(SignInProps {
                provider: "Nextcloud".to_owned(),
                mark: MarkProvider::Imap,
                fields: vec![
                    FormField {
                        role: FieldRole::Address,
                        requirement: Requirement::Required,
                        text: FieldText::Plain(String::new()),
                    },
                    FormField {
                        role: FieldRole::Password,
                        requirement: Requirement::Required,
                        text: FieldText::Secret(Hidden::new("")),
                    },
                    FormField {
                        role: FieldRole::Server,
                        requirement: Requirement::Optional,
                        text: FieldText::Plain(String::new()),
                    },
                ],
                problem: Some(FieldProblem {
                    role: FieldRole::Password,
                    kind: ProblemKind::Refused,
                    attempt: Attempt(0),
                }),
            })),
        ),
        (
            "browser",
            SheetView::BrowserWait {
                provider: provider("google"),
                url,
            },
            Some(Step::Browser {
                provider: "Google".to_owned(),
                url: "https://login.example.org/auth?x=1".to_owned(),
                copied: CopyState::Idle,
            }),
        ),
        (
            "code",
            SheetView::ShowCode {
                provider: provider("microsoft"),
                user_code: UserCode("ABCD-EFGH".to_owned()),
                url: device,
            },
            Some(Step::Code {
                provider: "Microsoft".to_owned(),
                code: "ABCD-EFGH".to_owned(),
                url: "https://example.org/device".to_owned(),
                copied: CopyState::Idle,
            }),
        ),
        (
            "review",
            review,
            Some(Step::Review(ReviewProps {
                account: "ada@work.example".to_owned(),
                services: vec![
                    line("mail", "Mail", ServiceOffer::Offered(Check::On), None),
                    line(
                        "storage",
                        "Files",
                        ServiceOffer::Offered(Check::Off),
                        Some(Limitation::AppFolderOnly),
                    ),
                    line(
                        "photos",
                        "Photos",
                        ServiceOffer::Absent(Limitation::UnverifiedBuild),
                        None,
                    ),
                ],
                allow: Some("Mail".to_owned()),
            })),
        ),
        (
            "working",
            SheetView::Working(provider("nextcloud")),
            Some(Step::Working {
                provider: "Nextcloud".to_owned(),
            }),
        ),
        (
            "failed",
            SheetView::Failed {
                provider: provider("nextcloud"),
                fault: PorterFault::NeedsClientId,
            },
            Some(Step::Failed {
                provider: "Nextcloud".to_owned(),
                why: SignInFault::NeedsClientId,
            }),
        ),
        ("done", SheetView::Done, None),
    ];
    for (name, view, props) in cases {
        assert_eq!(shown(view, vec![]), props, "{name}");
    }
}

#[test]
fn the_draft_reaches_the_props() {
    let form = || {
        SheetView::SignIn(SignInView {
            provider: provider("nextcloud"),
            fields: vec![
                spec(FieldKind::Address, Entry::Plain, Presence::Required),
                spec(FieldKind::Password, Entry::Secret, Presence::Required),
            ],
            problem: None,
        })
    };
    let Some(Step::SignIn(props)) = shown(
        form(),
        vec![
            SheetAction::Type(FieldKind::Address, FieldValue::Plain("ada".to_owned())),
            SheetAction::Type(
                FieldKind::Password,
                FieldValue::Secret(SecretText::new("pw")),
            ),
        ],
    ) else {
        panic!("no form");
    };
    assert_eq!(props.fields[0].text, FieldText::Plain("ada".to_owned()));
    assert_eq!(props.fields[1].text, FieldText::Secret(Hidden::new("pw")));
    assert!(!format!("{props:?}").contains("pw\""), "{props:?}");

    let Some(Step::Providers(props)) = shown(
        SheetView::Providers(vec![]),
        vec![
            SheetAction::Query("nex".to_owned()),
            SheetAction::Cursor(ListKey::Other),
        ],
    ) else {
        panic!("no list");
    };
    assert_eq!(props.query, "nex");
    assert_eq!(props.cursor, Some(ProviderPick::Other));

    let url = WebUrl::parse("https://login.example.org/").unwrap_or_else(|e| panic!("{e}"));
    let browser = SheetView::BrowserWait {
        provider: provider("google"),
        url,
    };
    let Some(Step::Browser { copied, .. }) = shown(browser, vec![SheetAction::Copied]) else {
        panic!("no browser step");
    };
    assert_eq!(copied, CopyState::Copied);
    assert_eq!(CopyMark::Copied, CopyMark::Copied);
}

#[test]
fn a_review_switch_shows_what_the_person_chose() {
    let review = SheetView::Review(ReviewView {
        provider: provider("nextcloud"),
        review: Review {
            label: AccountLabel("ada".to_owned()),
            services: vec![row(
                CapabilityKind::Calendar,
                ServiceState::Offered(Toggle::On),
                None,
            )],
            endpoints: Vec::new(),
        },
        allow: None,
    });
    let Some(Step::Review(props)) = shown(
        review,
        vec![SheetAction::Toggle(CapabilityKind::Calendar, Toggle::Off)],
    ) else {
        panic!("no review");
    };
    assert_eq!(props.services[0].offer, ServiceOffer::Offered(Check::Off));
}

#[test]
fn the_title_area_carries_the_steps_own_title() {
    let cases = [
        (SheetView::Providers(vec![]), "Add Account"),
        (
            serde_json::from_str(
                r#"{"kind":"consent","v":{"app":{"name":"org.quire.Photos","isolation":"flatpak"},
                "kind":"storage","class":"app_own","usage":"interactive","accounts":[]}}"#,
            )
            .unwrap_or_else(|e| panic!("{e}")),
            "Account Access",
        ),
        (
            SheetView::Working(provider("nextcloud")),
            "Signing in to Nextcloud",
        ),
        (
            SheetView::Failed {
                provider: provider("google"),
                fault: PorterFault::Refused,
            },
            "Could not add Google",
        ),
    ];
    for (view, title) in cases {
        assert_eq!(
            shown(view, vec![]).map(|step| step.title()).as_deref(),
            Some(title)
        );
    }
}

#[test]
fn only_the_banners_output_draws_and_a_finished_add_draws_nothing() {
    let handle = SheetHandle::parse("a").unwrap_or_else(|| panic!("handle"));
    let (sheets, _) = step(
        AccountsSheets::default(),
        SheetEvent::Opened(handle, SheetView::Working(provider("nextcloud"))),
    );
    assert!(drawn(&sheets, Here::Yes).is_some());
    assert!(drawn(&sheets, Here::No).is_none());
}

#[test]
fn events_become_what_the_person_did() {
    assert_eq!(
        events::consent(ConsentAnswer::Allow {
            account: ChoiceKey("home".to_owned()),
            scope: AllowScope::Always
        }),
        Some(SheetAction::Allow(GrantScope::Always))
    );
    assert_eq!(
        events::consent(ConsentAnswer::Deny),
        Some(SheetAction::Deny)
    );
    assert_eq!(
        events::consent(ConsentAnswer::Dismiss),
        Some(SheetAction::Cancel)
    );
    assert_eq!(
        events::consent(ConsentAnswer::AddAccount),
        Some(SheetAction::AddAccount)
    );
    assert_eq!(
        events::chosen(&ChoiceKey("home".to_owned())),
        Some(SheetAction::Choose(account("home")))
    );
    assert_eq!(events::chosen(&ChoiceKey("Not An Id".to_owned())), None);
    assert_eq!(events::list_key(&ProviderPick::Other), Some(ListKey::Other));
    assert_eq!(
        events::list_key(&ProviderPick::Provider(ProviderKey("google".to_owned()))),
        Some(ListKey::Provider(provider("google")))
    );
    assert_eq!(
        events::typed(FieldRole::Password, FieldText::Secret(Hidden::new("pw"))),
        SheetAction::Type(
            FieldKind::Password,
            FieldValue::Secret(SecretText::new("pw"))
        )
    );
    assert_eq!(
        events::toggled(&ServiceKey("calendar".to_owned()), Check::On),
        Some(SheetAction::Toggle(CapabilityKind::Calendar, Toggle::On))
    );
    assert_eq!(
        events::toggled(&ServiceKey("nothing".to_owned()), Check::On),
        None
    );
}

#[test]
fn the_words_follow_the_table() {
    use super::words::{app_name, mark_of, request_phrase, service_key, service_of};
    assert_eq!(app_name(&app("org.quire.Photos")), "Photos");
    assert_eq!(app_name(&app("com.example.my_app-2")), "my app 2");
    assert_eq!(mark_of("gemini"), MarkProvider::Google);
    assert_eq!(mark_of("lm-studio"), MarkProvider::Imap);
    assert_eq!(mark_of("local"), MarkProvider::Local);
    assert_eq!(
        request_phrase(CapabilityKind::Mail, DataClass::Mail, Usage::Background),
        "use your email in the background"
    );
    for kind in [
        CapabilityKind::Mail,
        CapabilityKind::ImageGen,
        CapabilityKind::KeyValue,
    ] {
        assert_eq!(service_of(&service_key(kind)), Some(kind));
    }
}
