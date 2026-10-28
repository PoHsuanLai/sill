//! The `SheetView` to props table, and the events' way back. Pure: no renderer.

use ds::components::content::provider_mark::MarkProvider;
use ds::prelude::Check;
use ds_shell::accounts::hidden::Hidden;
use ds_shell::accounts::model::{
    AccountChoice, AllowScope, Attempt, ChoiceKey, ConsentAnswer, CopyState, FieldProblem,
    FieldRole, FieldText, FormField, Limitation, ProblemKind, ProviderEntry, ProviderKey,
    ProviderPick, Requirement, ServiceKey, ServiceLine, ServiceOffer, SignInFault,
};
use ds_shell::prelude::{Recovery, WorkingFor};
use porter_capability::{AbsentReason, CapabilityKind, Toggle};
use porter_core::consent::{AccountChoice as AskChoice, ConsentAsk, GrantScope, Usage};
use porter_core::{
    AccountId, AccountLabel, AppId, AppName, DataClass, EndpointUrl, Isolation, LauncherSession,
    LimitReason, ProviderId, SecretText,
};
use porter_ids::WebUrl;
use porter_sheet::{
    Entry, FieldKind, FieldSpec, FieldValue, Presence, ProblemKind as PorterProblem, ProviderKind,
    ProviderRow, Review, ReviewView, RowKind, ServiceRow, ServiceState, SheetView,
    SignInFault as PorterFault, SignInView, UserCode,
};
use sill_model::accounts::sheet::{
    AccountsSheets, CopyMark, ListKey, SheetAction, SheetEvent, SheetHandle, step,
};

mod long_text;

use super::events;
use super::props::{
    ConsentProps, ProvidersProps, Purpose, ReviewProps, SignInProps, Step, step_of,
};
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
    ServiceRow {
        kind,
        state,
        limit,
        access: None,
    }
}

fn line(key: &str, name: &str, offer: ServiceOffer, limit: Option<Limitation>) -> ServiceLine {
    ServiceLine {
        key: ServiceKey(key.to_owned()),
        name: name.to_owned(),
        offer,
        limit,
    }
}

/// The row porter sends for the provider `id`: its label is the id with a capital.
fn named(id: &str) -> ProviderRow {
    let mut chars = id.chars();
    let label: String = chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default();
    ProviderRow {
        id: provider(id),
        label,
        mark: id.to_owned(),
        kind: RowKind::Provider,
        auth: ProviderKind::Service,
        mark_face: None,
        group: None,
    }
}

/// The row porter sends for an assistant whose provider file says `agent_login`.
fn agent_row() -> ProviderRow {
    ProviderRow {
        id: provider("claude-code"),
        label: "Claude Code".to_owned(),
        mark: "claude-code".to_owned(),
        kind: RowKind::Provider,
        auth: ProviderKind::AgentLogin,
        mark_face: None,
        group: None,
    }
}

#[test]
fn every_sheet_view_becomes_the_props_of_its_part() {
    let consent = SheetView::Consent(ConsentAsk::new(
        app("org.quire.Photos"),
        CapabilityKind::Storage,
        DataClass::AppOwn,
        Usage::Interactive,
        vec![
            AskChoice::new(
                account("home"),
                AccountLabel("ada@home.example".to_owned()),
                provider("nextcloud"),
            ),
            AskChoice::new(
                account("work"),
                AccountLabel("ada@work.example".to_owned()),
                provider("microsoft"),
            ),
        ],
    ));
    let providers = SheetView::Providers(vec![
        ProviderRow {
            id: provider("nextcloud"),
            label: "Nextcloud".to_owned(),
            mark: "nextcloud".to_owned(),
            kind: RowKind::Provider,
            auth: ProviderKind::Service,
            mark_face: None,
            group: None,
        },
        ProviderRow {
            id: provider("google"),
            label: "Google".to_owned(),
            mark: "google".to_owned(),
            kind: RowKind::Provider,
            auth: ProviderKind::Service,
            mark_face: None,
            group: None,
        },
        // ProviderList draws its own "Other…": the generic row is not a second one.
        ProviderRow {
            id: provider("generic-imap"),
            label: "Other email account".to_owned(),
            mark: "imap".to_owned(),
            kind: RowKind::Generic,
            auth: ProviderKind::Service,
            mark_face: None,
            group: None,
        },
    ]);
    let form = SheetView::SignIn(SignInView {
        provider: provider("nextcloud"),
        row: Some(named("nextcloud")),
        fields: vec![
            spec(FieldKind::Address, Entry::Plain, Presence::Required),
            spec(FieldKind::Password, Entry::Secret, Presence::Required),
            spec(FieldKind::Server, Entry::Plain, Presence::Optional),
        ],
        problem: Some(porter_sheet::FieldProblem {
            field: FieldKind::Password,
            problem: PorterProblem::Refused,
        }),
    });
    let review = SheetView::Review(ReviewView {
        allow_label: None,
        provider: provider("microsoft"),
        row: Some(named("microsoft")),
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
                session: None,
                purpose: Purpose::Use,
            })),
        ),
        (
            "providers",
            providers,
            Some(Step::Providers(ProvidersProps {
                providers: vec![
                    ProviderEntry::new(
                        ProviderKey("nextcloud".to_owned()),
                        "Nextcloud",
                        MarkProvider::Imap,
                    ),
                    ProviderEntry::new(
                        ProviderKey("google".to_owned()),
                        "Google",
                        MarkProvider::Google,
                    ),
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
                    FormField::new(
                        FieldRole::Address,
                        Requirement::Required,
                        FieldText::Plain(String::new()),
                    ),
                    FormField::new(
                        FieldRole::Password,
                        Requirement::Required,
                        FieldText::Secret(Hidden::new("")),
                    ),
                    FormField::new(
                        FieldRole::Server,
                        Requirement::Optional,
                        FieldText::Plain(String::new()),
                    ),
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
                row: Some(named("google")),
                url,
            },
            Some(Step::Browser {
                provider: "Google".to_owned(),
                shown: "login.example.org/\u{2026}".to_owned(),
                url: "https://login.example.org/auth?x=1".to_owned(),
                copied: CopyState::Idle,
            }),
        ),
        (
            "code",
            SheetView::ShowCode {
                provider: provider("microsoft"),
                row: Some(named("microsoft")),
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
            SheetView::Working {
                provider: provider("nextcloud"),
                row: Some(named("nextcloud")),
            },
            Some(Step::Working {
                provider: "Nextcloud".to_owned(),
                waiting_for: WorkingFor::Provider,
            }),
        ),
        (
            "working on an agent login",
            SheetView::Working {
                provider: provider("claude-code"),
                row: Some(agent_row()),
            },
            Some(Step::Working {
                provider: "Claude Code".to_owned(),
                waiting_for: WorkingFor::Agent("Claude Code".to_owned()),
            }),
        ),
        (
            "failed",
            SheetView::Failed {
                provider: provider("nextcloud"),
                row: Some(named("nextcloud")),
                fault: PorterFault::NeedsClientId,
            },
            Some(Step::Failed {
                provider: "Nextcloud".to_owned(),
                why: SignInFault::NeedsClientId,
                recovery: Some(Recovery::NoRetry),
                mark: Some(MarkProvider::Imap),
            }),
        ),
        (
            "failed on Google, which names its client id",
            SheetView::Failed {
                provider: provider("google"),
                row: Some(named("google")),
                fault: PorterFault::NeedsClientId,
            },
            Some(Step::Failed {
                provider: "Google".to_owned(),
                why: SignInFault::NeedsClientId,
                recovery: Some(Recovery::NoRetry),
                mark: Some(MarkProvider::Google),
            }),
        ),
        (
            "failed on an assistant with no launcher",
            SheetView::Failed {
                provider: provider("claude-code"),
                row: Some(agent_row()),
                fault: PorterFault::NoLauncher,
            },
            Some(Step::Failed {
                provider: "Claude Code".to_owned(),
                why: SignInFault::NoLauncher,
                recovery: Some(Recovery::NoRetry),
                mark: Some(MarkProvider::Imap),
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
            row: Some(named("nextcloud")),
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
        row: Some(named("google")),
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
        allow_label: None,
        provider: provider("nextcloud"),
        row: Some(named("nextcloud")),
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
            SheetView::Working {
                provider: provider("nextcloud"),
                row: Some(named("nextcloud")),
            },
            "Signing in to Nextcloud",
        ),
        (
            SheetView::Failed {
                provider: provider("google"),
                row: Some(named("google")),
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
        SheetEvent::Opened(
            handle,
            SheetView::Working {
                provider: provider("nextcloud"),
                row: Some(named("nextcloud")),
            },
        ),
    );
    assert!(drawn(&sheets, Here::Yes).is_some());
    assert!(drawn(&sheets, Here::No).is_none());
}

#[test]
fn this_session_only_grants_the_session_the_ask_came_from() {
    let session = LauncherSession::parse("sess-1").unwrap_or_else(|e| panic!("{e}"));
    let answer = ConsentAnswer::Allow {
        account: ChoiceKey("home".to_owned()),
        scope: AllowScope::Session,
    };
    assert_eq!(
        events::consent(answer.clone(), Some(&session)),
        Some(SheetAction::Allow(GrantScope::Session(session)))
    );
    // Without a session the alert does not offer it; a stray answer is no action.
    assert_eq!(events::consent(answer, None), None);
}

#[test]
fn events_become_what_the_person_did() {
    assert_eq!(
        events::consent(
            ConsentAnswer::Allow {
                account: ChoiceKey("home".to_owned()),
                scope: AllowScope::Always
            },
            None
        ),
        Some(SheetAction::Allow(GrantScope::Always))
    );
    assert_eq!(
        events::consent(ConsentAnswer::Deny, None),
        Some(SheetAction::Deny)
    );
    assert_eq!(
        events::consent(ConsentAnswer::Dismiss, None),
        Some(SheetAction::Cancel)
    );
    assert_eq!(
        events::consent(ConsentAnswer::AddAccount, None),
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
    assert_eq!(app_name(&app("org.quire.Photos"), None), "Photos");
    // The segment is shown as written: no lowercasing, no swapped separators.
    assert_eq!(
        app_name(&app("org.quire.Agent.claude-code"), None),
        "claude-code"
    );
    assert_eq!(app_name(&app("com.example.my_App-2"), None), "my_App-2");
    // A label the app's entry gives wins.
    assert_eq!(
        app_name(&app("org.quire.Agent.claude-code"), Some("Claude Code")),
        "Claude Code"
    );
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

#[test]
fn the_server_form_becomes_choices_hints_and_parts() {
    use ds_shell::accounts::model::{Choice, FormPart};
    use porter_sheet::{Protocol, manual_form};
    let view = |protocol| {
        SheetView::SignIn(SignInView {
            provider: provider("generic-imap"),
            row: Some(named("generic-imap")),
            fields: manual_form(protocol, Some("example.org")),
            problem: None,
        })
    };
    let Some(Step::SignIn(props)) = shown(view(Protocol::Imap), vec![]) else {
        panic!("no form");
    };
    let of = |role| {
        props
            .fields
            .iter()
            .find(|f| f.role == role)
            .unwrap_or_else(|| panic!("{role:?}"))
    };
    assert_eq!(
        of(FieldRole::Protocol).choices,
        Some(vec![
            Choice::new("imap", "Most servers (IMAP)"),
            Choice::new("pop3", "Older servers (POP)"),
            Choice::new("jmap", "Newer servers (JMAP)"),
        ])
    );
    assert_eq!(
        of(FieldRole::Protocol).text,
        FieldText::Plain("imap".to_owned())
    );
    assert_eq!(
        of(FieldRole::Security).choices,
        Some(vec![
            Choice::new("tls", "Secure from the start (SSL/TLS)"),
            Choice::new("starttls", "Secure after connecting (STARTTLS)")
        ])
    );
    assert_eq!(of(FieldRole::Port).requirement, Requirement::Optional);
    assert_eq!(of(FieldRole::Port).text, FieldText::Plain(String::new()));
    assert_eq!(of(FieldRole::Port).hint.as_deref(), Some("993"));
    assert_eq!(of(FieldRole::OutgoingPort).hint.as_deref(), Some("465"));
    assert_eq!(of(FieldRole::Server).part, Some(FormPart::Incoming));
    assert_eq!(of(FieldRole::OutgoingServer).part, Some(FormPart::Outgoing));
    assert_eq!(of(FieldRole::Username).part, Some(FormPart::SignIn));
    assert_eq!(props.problem, None);

    // The hint follows protocol and security.
    let Some(Step::SignIn(props)) = shown(
        view(Protocol::Imap),
        vec![
            SheetAction::Type(FieldKind::Protocol, FieldValue::Plain("pop3".to_owned())),
            SheetAction::Type(
                FieldKind::Security,
                FieldValue::Plain("starttls".to_owned()),
            ),
            SheetAction::Type(
                FieldKind::OutgoingSecurity,
                FieldValue::Plain("starttls".to_owned()),
            ),
        ],
    ) else {
        panic!("no form");
    };
    let hint = |role| {
        props
            .fields
            .iter()
            .find(|f| f.role == role)
            .and_then(|f| f.hint.clone())
    };
    assert_eq!(hint(FieldRole::Port).as_deref(), Some("110"));
    assert_eq!(hint(FieldRole::OutgoingPort).as_deref(), Some("587"));

    // JMAP has no outgoing fields.
    let Some(Step::SignIn(props)) = shown(
        view(Protocol::Imap),
        vec![SheetAction::Type(
            FieldKind::Protocol,
            FieldValue::Plain("jmap".to_owned()),
        )],
    ) else {
        panic!("no form");
    };
    assert!(
        props
            .fields
            .iter()
            .all(|f| f.role != FieldRole::OutgoingServer)
    );
    assert!(props.fields.iter().any(|f| f.role == FieldRole::SessionUrl));

    // A port that is no number is marked as it is typed.
    let Some(Step::SignIn(props)) = shown(
        view(Protocol::Imap),
        vec![SheetAction::Type(
            FieldKind::Port,
            FieldValue::Plain("ninety".to_owned()),
        )],
    ) else {
        panic!("no form");
    };
    assert_eq!(
        props.problem.map(|p| (p.role, p.kind)),
        Some((FieldRole::Port, ProblemKind::Invalid))
    );
}

#[test]
fn every_role_and_kind_pair_up() {
    use super::props::{kind_of, role_of};
    for kind in [
        FieldKind::Address,
        FieldKind::Server,
        FieldKind::Username,
        FieldKind::Password,
        FieldKind::ApiKey,
        FieldKind::Token,
        FieldKind::Protocol,
        FieldKind::Port,
        FieldKind::Security,
        FieldKind::OutgoingServer,
        FieldKind::OutgoingPort,
        FieldKind::OutgoingSecurity,
        FieldKind::SessionUrl,
    ] {
        assert_eq!(kind_of(role_of(kind)), kind);
    }
    assert_eq!(
        events::typed(FieldRole::Security, FieldText::Plain("starttls".to_owned())),
        SheetAction::Type(
            FieldKind::Security,
            FieldValue::Plain("starttls".to_owned())
        )
    );
}

#[test]
fn the_port_hints_are_porters_usual_ports() {
    use super::words::usual_port;
    use porter_sheet::{FieldSpec, Protocol, manual_form, refit};
    for (protocol, slug) in [(Protocol::Imap, "imap"), (Protocol::Pop3, "pop3")] {
        let fields: Vec<FieldSpec> = manual_form(protocol, None);
        for (security, outgoing_kind, kind) in [
            ("tls", FieldKind::OutgoingSecurity, FieldKind::Security),
            ("starttls", FieldKind::OutgoingSecurity, FieldKind::Security),
        ] {
            let answers = [kind, outgoing_kind].map(|kind| porter_sheet::FieldAnswer {
                kind,
                value: FieldValue::Plain(security.to_owned()),
            });
            let shaped = refit(&fields, &answers);
            let prefill = |kind| {
                shaped
                    .iter()
                    .find(|f| f.kind == kind)
                    .and_then(|f| f.prefill.clone())
            };
            assert_eq!(
                prefill(FieldKind::Port),
                Some(usual_port(slug, false, security).to_string())
            );
            assert_eq!(
                prefill(FieldKind::OutgoingPort),
                Some(usual_port(slug, true, security).to_string())
            );
        }
    }
}

#[test]
fn the_calendar_and_contacts_row_is_listed_and_the_mail_rows_are_left_to_other() {
    let generic = |id: &str, label: &str| ProviderRow {
        kind: RowKind::Generic,
        label: label.to_owned(),
        ..named(id)
    };
    let view = SheetView::Providers(vec![
        named("nextcloud"),
        generic("generic-imap", "Other email account"),
        generic("generic-jmap", "Other email account (newer servers)"),
        generic("generic-dav", "Other calendar and contacts"),
    ]);
    let Some(Step::Providers(props)) = shown(view, vec![]) else {
        panic!("no provider list");
    };
    let labels: Vec<&str> = props.providers.iter().map(|e| e.label.as_str()).collect();
    assert_eq!(labels, ["Nextcloud", "Other calendar and contacts"]);
}

#[test]
fn an_app_password_field_says_where_the_provider_makes_one() {
    const CASES: &[(&str, Option<&str>)] = &[
        (
            "icloud",
            Some("Made at account.apple.com, not your Apple Account password"),
        ),
        (
            "fastmail",
            Some("Made in Fastmail under Privacy & Security, not your password"),
        ),
        (
            "yahoo",
            Some("Made under Account security, not your Yahoo password"),
        ),
        ("nextcloud", None),
    ];
    for (id, hint) in CASES {
        let view = SheetView::SignIn(SignInView {
            provider: provider(id),
            row: Some(named(id)),
            fields: vec![
                spec(FieldKind::Address, Entry::Plain, Presence::Required),
                spec(FieldKind::AppPassword, Entry::Secret, Presence::Required),
            ],
            problem: None,
        });
        let Some(Step::SignIn(props)) = shown(view, vec![]) else {
            panic!("{id}: no form");
        };
        let field = props
            .fields
            .iter()
            .find(|f| f.role == FieldRole::AppPassword);
        assert_eq!(field.and_then(|f| f.hint.as_deref()), *hint, "{id}");
    }
}

#[test]
fn porter_faults_land_on_the_shell_faults() {
    use super::words::fault_of;
    let cases = [
        (PorterFault::NoLauncher, SignInFault::NoLauncher),
        (PorterFault::NotInstalled, SignInFault::NotInstalled),
        (PorterFault::Expired, SignInFault::TimedOut),
        (PorterFault::AlreadyAdded, SignInFault::AlreadyAdded),
    ];
    for (porter, shell) in cases {
        assert_eq!(fault_of(porter), shell);
    }
}

#[test]
fn only_a_fault_a_retry_can_fix_offers_try_again() {
    use super::words::recovery_of;
    let cases = [
        (PorterFault::NeedsClientId, Some(Recovery::NoRetry)),
        (PorterFault::NotInstalled, Some(Recovery::NoRetry)),
        (PorterFault::NoLauncher, Some(Recovery::NoRetry)),
        // The fault's own answer applies: it is no retry.
        (PorterFault::AlreadyAdded, None),
        (PorterFault::Forbidden, Some(Recovery::NoRetry)),
        (PorterFault::Unreachable, Some(Recovery::Retry)),
        (PorterFault::TimedOut, Some(Recovery::Retry)),
        (PorterFault::Refused, Some(Recovery::Retry)),
    ];
    for (fault, recovery) in cases {
        assert_eq!(recovery_of(fault), recovery, "{fault:?}");
    }
}

#[test]
fn the_microsoft_failure_without_a_client_id_has_no_try_again() {
    let failed = SheetView::Failed {
        provider: provider("microsoft"),
        row: Some(named("microsoft")),
        fault: PorterFault::NeedsClientId,
    };
    let Some(Step::Failed { recovery, why, .. }) = shown(failed, vec![]) else {
        panic!("no failure");
    };
    assert_eq!(why, SignInFault::NeedsClientId);
    assert_eq!(recovery, Some(Recovery::NoRetry));
}

#[test]
fn consent_and_review_name_the_app_as_porter_resolved_it() {
    let agent = app("org.quire.Agent.claude-code");
    let consent = SheetView::Consent(
        ConsentAsk::new(
            agent.clone(),
            CapabilityKind::Mail,
            DataClass::Mail,
            Usage::Interactive,
            Vec::new(),
        )
        .with_app_label(porter_core::AppLabel("Claude Code".to_owned())),
    );
    let Some(Step::Consent(props)) = shown(consent, vec![]) else {
        panic!("no consent");
    };
    assert_eq!(props.app, "Claude Code");
    let review = SheetView::Review(ReviewView {
        allow_label: Some(porter_core::AppLabel("Claude Code".to_owned())),
        provider: provider("nextcloud"),
        row: None,
        review: Review {
            label: AccountLabel("ada".to_owned()),
            services: Vec::new(),
            endpoints: Vec::new(),
        },
        allow: Some(agent),
    });
    let Some(Step::Review(props)) = shown(review, vec![]) else {
        panic!("no review");
    };
    assert_eq!(props.allow.as_deref(), Some("Claude Code"));
}

#[test]
fn continue_as_is_its_own_step_and_an_empty_one_is_the_ordinary_question() {
    let choice = AskChoice::new(
        account("ada"),
        AccountLabel("ada@example.org".to_owned()),
        provider("nextcloud"),
    );
    let ask = |accounts| {
        SheetView::Consent(
            ConsentAsk::new(
                app("org.quire.Mail"),
                CapabilityKind::Identity,
                DataClass::AppOwn,
                Usage::Interactive,
                accounts,
            )
            .with_purpose(porter_core::consent::AskPurpose::ContinueAs),
        )
    };
    let Some(step) = shown(ask(vec![choice]), vec![]) else {
        panic!("nothing drawn");
    };
    assert_eq!(step.title(), "Continue as");
    let Step::Consent(props) = step else {
        panic!("no consent");
    };
    assert_eq!(props.purpose, Purpose::ContinueAs);
    let Some(Step::Consent(props)) = shown(ask(Vec::new()), vec![]) else {
        panic!("no consent");
    };
    assert_eq!(props.purpose, Purpose::Use);
}
