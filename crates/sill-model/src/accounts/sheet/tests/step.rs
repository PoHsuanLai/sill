use porter_core::AccountId;
use porter_core::SecretText;
use porter_core::Toggle;
use porter_core::capability::CapabilityKind;
use porter_core::consent::{AccountChoice, ConsentAnswer, ConsentAsk, GrantScope, Usage};
use porter_core::sheet::{
    FieldAnswer, FieldKind, FieldValue, Review, ServiceChoice, ServiceRow, ServiceState,
    SheetInput, SheetView, UserCode,
};
use porter_core::{AccountLabel, AppId, AppName, DataClass, Isolation};

use super::{form, handle, provider, rows};
use crate::accounts::sheet::{
    AccountsSheets, Admit, ListKey, SheetAction as Do, SheetEvent as Ev, SheetOut, admits, step,
};
use porter_core::sheet::SignInView;

fn run(mut sheets: AccountsSheets, events: Vec<Ev>) -> (AccountsSheets, Vec<SheetOut>) {
    let mut out = Vec::new();
    for event in events {
        let (next, said) = step(sheets, event);
        sheets = next;
        out.extend(said);
    }
    (sheets, out)
}

fn opened(view: SheetView) -> AccountsSheets {
    run(
        AccountsSheets::default(),
        vec![Ev::Opened(handle("a"), view)],
    )
    .0
}

fn reply(input: SheetInput) -> Vec<SheetOut> {
    vec![SheetOut::Reply(handle("a"), input)]
}

fn act(sheets: AccountsSheets, actions: Vec<Do>) -> (AccountsSheets, Vec<SheetOut>) {
    run(
        sheets,
        actions
            .into_iter()
            .map(|action| Ev::Acted(handle("a"), action))
            .collect(),
    )
}

fn account(id: &str) -> AccountId {
    AccountId::parse(id).unwrap_or_else(|e| panic!("{e}"))
}

fn consent() -> SheetView {
    let choice = |id: &str| AccountChoice {
        account: account(id),
        label: AccountLabel(id.to_owned()),
        provider: provider("nextcloud"),
    };
    SheetView::Consent(ConsentAsk {
        app: AppId {
            name: AppName::parse("org.quire.Photos").unwrap_or_else(|e| panic!("{e}")),
            isolation: Isolation::Flatpak,
        },
        kind: CapabilityKind::Storage,
        class: DataClass::AppOwn,
        usage: Usage::Interactive,
        accounts: vec![choice("home"), choice("work")],
    })
}

fn browser() -> SheetView {
    SheetView::BrowserWait {
        provider: provider("google"),
        url: porter_core::WebUrl::parse("https://login.example.org/a")
            .unwrap_or_else(|e| panic!("{e}")),
    }
}

fn typed(kind: FieldKind, text: &str) -> Do {
    Do::Type(kind, FieldValue::Plain(text.to_owned()))
}

fn secret(text: &str) -> Do {
    Do::Type(
        FieldKind::Password,
        FieldValue::Secret(SecretText::new(text)),
    )
}

fn answers(address: &str, password: &str) -> SheetInput {
    SheetInput::Submit(vec![
        FieldAnswer {
            kind: FieldKind::Address,
            value: FieldValue::Plain(address.to_owned()),
        },
        FieldAnswer {
            kind: FieldKind::Password,
            value: FieldValue::Secret(SecretText::new(password)),
        },
    ])
}

fn review() -> SheetView {
    let row = |kind, state| ServiceRow {
        kind,
        state,
        limit: None,
    };
    SheetView::Review(porter_core::sheet::ReviewView {
        provider: provider("nextcloud"),
        review: Review {
            label: AccountLabel("ada".to_owned()),
            services: vec![
                row(CapabilityKind::Mail, ServiceState::Offered(Toggle::On)),
                row(CapabilityKind::Calendar, ServiceState::Offered(Toggle::On)),
                row(
                    CapabilityKind::Photos,
                    ServiceState::Absent(porter_core::AbsentReason::ProviderOffersNone),
                ),
            ],
            endpoints: Vec::new(),
        },
        allow: None,
    })
}

/// One row: the sheet, what the person does, what accountd is told.
struct Case {
    name: &'static str,
    view: SheetView,
    actions: Vec<Do>,
    told: Vec<SheetInput>,
}

fn case(name: &'static str, view: SheetView, actions: Vec<Do>, told: Vec<SheetInput>) -> Case {
    Case {
        name,
        view,
        actions,
        told,
    }
}

#[test]
fn what_the_person_does_is_what_accountd_is_told() {
    let failed = SheetView::Failed {
        provider: provider("nextcloud"),
        fault: porter_core::sheet::SignInFault::Refused,
    };
    let cases = vec![
        case(
            "a provider row is picked by its id",
            rows(&["nextcloud", "generic-imap"]),
            vec![Do::Pick(ListKey::Provider(provider("nextcloud")))],
            vec![SheetInput::Pick(provider("nextcloud"))],
        ),
        case(
            "a row that is not on the list is not picked",
            rows(&["nextcloud"]),
            vec![Do::Pick(ListKey::Provider(provider("yahoo")))],
            vec![],
        ),
        case(
            "Other picks the first generic provider",
            rows(&["nextcloud", "generic-imap", "generic-dav"]),
            vec![Do::Pick(ListKey::Other)],
            vec![SheetInput::Pick(provider("generic-imap"))],
        ),
        case(
            "Other with no generic provider picks nothing",
            rows(&["nextcloud"]),
            vec![Do::Pick(ListKey::Other)],
            vec![],
        ),
        case(
            "search and cursor are the shell's, not accountd's",
            rows(&["nextcloud"]),
            vec![Do::Query("next".to_owned()), Do::Cursor(ListKey::Other)],
            vec![],
        ),
        case(
            "an empty required field is not sent",
            form(None),
            vec![typed(FieldKind::Address, "ada@example.org"), Do::Submit],
            vec![],
        ),
        case(
            "a blank required field is not sent",
            form(None),
            vec![typed(FieldKind::Address, "   "), secret("pw"), Do::Submit],
            vec![],
        ),
        case(
            "a ready form is sent whole, the plain text trimmed",
            form(None),
            vec![
                typed(FieldKind::Address, " ada@example.org "),
                secret("hunter2"),
                Do::Submit,
            ],
            vec![answers("ada@example.org", "hunter2")],
        ),
        case(
            "a prefilled address needs no typing",
            form(Some("ada@example.org")),
            vec![secret("pw"), Do::Submit],
            vec![answers("ada@example.org", "pw")],
        ),
        case(
            "a secret typed into a plain field is not a keystroke of it",
            form(None),
            vec![
                Do::Type(FieldKind::Address, FieldValue::Secret(SecretText::new("x"))),
                secret("pw"),
                Do::Submit,
            ],
            vec![],
        ),
        case(
            "after Back only a dismissal goes until accountd updates the sheet",
            failed.clone(),
            vec![Do::Back, Do::Retry, Do::Cancel],
            vec![SheetInput::Back, SheetInput::Dismiss],
        ),
        case(
            "Retry means nothing on a form",
            form(None),
            vec![Do::Retry],
            vec![],
        ),
        case(
            "Cancel is a dismissal at every step",
            SheetView::Working(provider("nextcloud")),
            vec![Do::Cancel],
            vec![SheetInput::Dismiss],
        ),
        case(
            "Allow with no pick uses the first account",
            consent(),
            vec![Do::Allow(GrantScope::Once)],
            vec![SheetInput::Answer(ConsentAnswer::Allow {
                account: account("home"),
                scope: GrantScope::Once,
            })],
        ),
        case(
            "Always Allow uses the account picked",
            consent(),
            vec![Do::Choose(account("work")), Do::Allow(GrantScope::Always)],
            vec![SheetInput::Answer(ConsentAnswer::Allow {
                account: account("work"),
                scope: GrantScope::Always,
            })],
        ),
        case(
            "an account the alert does not offer is not picked",
            consent(),
            vec![Do::Choose(account("other")), Do::Allow(GrantScope::Once)],
            vec![SheetInput::Answer(ConsentAnswer::Allow {
                account: account("home"),
                scope: GrantScope::Once,
            })],
        ),
        case(
            "Add Account on the alert answers AddAccount",
            consent(),
            vec![Do::AddAccount],
            vec![SheetInput::Answer(ConsentAnswer::AddAccount)],
        ),
        case(
            "Open Again on the browser step asks accountd to show the page again",
            browser(),
            vec![Do::OpenAgain],
            vec![SheetInput::OpenAgain],
        ),
        case(
            "Open Again means nothing elsewhere",
            form(None),
            vec![Do::OpenAgain],
            vec![],
        ),
        case(
            "Don't Allow denies",
            consent(),
            vec![Do::Deny],
            vec![SheetInput::Answer(ConsentAnswer::Deny)],
        ),
        case(
            "Escape on the alert dismisses",
            consent(),
            vec![Do::Cancel],
            vec![SheetInput::Dismiss],
        ),
        case(
            "the review confirms with its switches",
            review(),
            vec![
                Do::Toggle(CapabilityKind::Calendar, Toggle::Off),
                Do::Confirm,
            ],
            vec![SheetInput::Confirm(vec![
                ServiceChoice {
                    kind: CapabilityKind::Mail,
                    toggle: Toggle::On,
                },
                ServiceChoice {
                    kind: CapabilityKind::Calendar,
                    toggle: Toggle::Off,
                },
            ])],
        ),
        case(
            "a service the provider lacks has no switch",
            review(),
            vec![Do::Toggle(CapabilityKind::Photos, Toggle::On), Do::Confirm],
            vec![SheetInput::Confirm(vec![
                ServiceChoice {
                    kind: CapabilityKind::Mail,
                    toggle: Toggle::On,
                },
                ServiceChoice {
                    kind: CapabilityKind::Calendar,
                    toggle: Toggle::On,
                },
            ])],
        ),
        case(
            "Copied tells accountd nothing",
            SheetView::ShowCode {
                provider: provider("nextcloud"),
                user_code: UserCode("ABCD".to_owned()),
                url: porter_core::EndpointUrl::parse("https://example.org/device")
                    .unwrap_or_else(|e| panic!("{e}")),
            },
            vec![Do::Copied],
            vec![],
        ),
        case(
            "nothing is answered on a finished add",
            SheetView::Done,
            vec![Do::Back, Do::Confirm, Do::Submit],
            vec![],
        ),
    ];
    for case in cases {
        let (_, out) = act(opened(case.view), case.actions);
        let told: Vec<SheetOut> = case
            .told
            .into_iter()
            .map(|input| SheetOut::Reply(handle("a"), input))
            .collect();
        assert_eq!(out, told, "{}", case.name);
    }
}

#[test]
fn a_sent_secret_is_gone_from_the_draft_and_the_plain_text_stays() {
    let (sheets, out) = act(
        opened(form(None)),
        vec![
            typed(FieldKind::Address, "ada@example.org"),
            secret("hunter2"),
            Do::Submit,
        ],
    );
    assert_eq!(out, reply(answers("ada@example.org", "hunter2")));
    let draft = &sheets.open[0].draft;
    assert_eq!(draft.submits.0, 1);
    assert_eq!(
        draft.fields[0].value,
        FieldValue::Plain("ada@example.org".to_owned())
    );
    assert_eq!(
        draft.fields[1].value,
        FieldValue::Secret(SecretText::new(""))
    );
    assert!(!format!("{sheets:?}").contains("hunter2"));
    // A second Continue has nothing to send: the password has to be typed again.
    let (_, again) = act(sheets, vec![Do::Submit]);
    assert_eq!(again, vec![]);
}

#[test]
fn a_refused_form_comes_back_without_its_password_and_keeps_the_address() {
    let (sheets, _) = act(
        opened(form(None)),
        vec![
            typed(FieldKind::Address, "ada@example.org"),
            secret("hunter2"),
            Do::Submit,
        ],
    );
    let (sheets, _) = step(sheets, Ev::Updated(handle("a"), form(None)));
    let draft = &sheets.open[0].draft;
    assert_eq!(
        draft.fields[0].value,
        FieldValue::Plain("ada@example.org".to_owned())
    );
    assert_eq!(
        draft.fields[1].value,
        FieldValue::Secret(SecretText::new(""))
    );
    assert_eq!(draft.submits.0, 1, "the count survives for the shake");
}

#[test]
fn an_answer_goes_once_per_view_and_the_next_view_frees_the_next() {
    let pick = || Do::Pick(ListKey::Provider(provider("nextcloud")));
    let (sheets, out) = act(opened(rows(&["nextcloud"])), vec![pick(), pick()]);
    assert_eq!(
        out,
        reply(SheetInput::Pick(provider("nextcloud"))),
        "a double press"
    );
    let (sheets, out) = step(sheets, Ev::Updated(handle("a"), rows(&["nextcloud"])));
    assert!(out.is_empty());
    let (_, out) = act(sheets, vec![pick()]);
    assert_eq!(
        out,
        reply(SheetInput::Pick(provider("nextcloud"))),
        "after an update"
    );
}

#[test]
fn a_browser_step_shown_opens_its_page_and_shown_again_opens_it_again() {
    let page = || {
        SheetOut::OpenPage(
            porter_core::WebUrl::parse("https://login.example.org/a")
                .unwrap_or_else(|e| panic!("{e}")),
        )
    };
    let (sheets, out) = step(
        AccountsSheets::default(),
        Ev::Opened(handle("a"), browser()),
    );
    assert_eq!(out, vec![page()], "the first open is the host's");
    let (sheets, out) = step(sheets, Ev::Updated(handle("a"), browser()));
    assert_eq!(out, vec![page()], "the same page shown again");
    let (_, out) = step(
        sheets,
        Ev::Updated(handle("a"), SheetView::Working(provider("google"))),
    );
    assert_eq!(out, vec![], "no page, no open");
    let (_, out) = step(
        AccountsSheets::default(),
        Ev::Opened(handle("b"), form(None)),
    );
    assert_eq!(out, vec![]);
}

#[test]
fn a_new_step_starts_a_clean_draft() {
    let (sheets, _) = act(
        opened(form(None)),
        vec![typed(FieldKind::Address, "ada@example.org")],
    );
    let (sheets, _) = step(
        sheets,
        Ev::Updated(handle("a"), SheetView::Working(provider("nextcloud"))),
    );
    assert!(sheets.open[0].draft.fields.is_empty());
    let (sheets, _) = step(sheets, Ev::Updated(handle("a"), form(None)));
    assert_eq!(
        sheets.open[0].draft.fields[0].value,
        FieldValue::Plain(String::new())
    );
}

#[test]
fn sheets_queue_in_the_order_opened_and_close_by_handle() {
    let events = vec![
        Ev::Opened(handle("a"), SheetView::Working(provider("one"))),
        Ev::Opened(handle("b"), SheetView::Working(provider("two"))),
        Ev::Opened(handle("a"), SheetView::Working(provider("three"))),
    ];
    let (sheets, _) = run(AccountsSheets::default(), events);
    let handles: Vec<&str> = sheets.open.iter().map(|s| s.handle.as_str()).collect();
    assert_eq!(handles, ["a", "b"], "an open handle is not opened twice");
    assert_eq!(sheets.current().map(|s| s.handle.as_str()), Some("a"));
    let (sheets, _) = step(sheets, Ev::Closed(handle("a")));
    assert_eq!(sheets.current().map(|s| s.handle.as_str()), Some("b"));
    let (sheets, out) = step(sheets, Ev::Gone);
    assert!(sheets.open.is_empty() && out.is_empty());
}

#[test]
fn a_finished_add_draws_nothing() {
    let sheets = opened(SheetView::Done);
    assert!(sheets.current().is_none());
}

#[test]
fn admission_refuses_a_second_handle_and_a_ninth_sheet() {
    let (mut sheets, _) = run(AccountsSheets::default(), vec![]);
    assert_eq!(admits(&sheets, &handle("a")), Admit::Yes);
    for n in 0..crate::accounts::sheet::MAX_OPEN {
        let (next, _) = step(
            sheets,
            Ev::Opened(handle(&format!("h{n}")), SheetView::Done),
        );
        sheets = next;
    }
    assert_eq!(admits(&sheets, &handle("h0")), Admit::Duplicate);
    assert_eq!(admits(&sheets, &handle("late")), Admit::Full);
}

#[test]
fn handles_are_short_plain_words() {
    use crate::accounts::sheet::SheetHandle;
    for (text, ok) in [
        ("accountd-0", true),
        ("", false),
        ("a b", false),
        ("a/b", false),
        (&"x".repeat(65), false),
        (&"x".repeat(64), true),
    ] {
        assert_eq!(SheetHandle::parse(text).is_some(), ok, "{text}");
    }
}

fn manual(protocol: porter_core::sheet::Protocol) -> SheetView {
    SheetView::SignIn(SignInView {
        provider: provider("generic-imap"),
        fields: porter_core::sheet::manual_form(protocol, Some("example.org")),
        problem: None,
    })
}

fn kinds(sheets: &AccountsSheets) -> Vec<FieldKind> {
    let Some(sheet) = sheets.open.first() else {
        panic!("no sheet");
    };
    sheet.draft.fields.iter().map(|f| f.kind).collect()
}

fn text_of(sheets: &AccountsSheets, kind: FieldKind) -> Option<String> {
    sheets
        .open
        .first()?
        .draft
        .fields
        .iter()
        .find_map(|f| match &f.value {
            FieldValue::Plain(text) if f.kind == kind => Some(text.clone()),
            _ => None,
        })
}

#[test]
fn the_protocol_reshapes_the_server_form_and_keeps_what_survives() {
    use porter_core::sheet::Protocol;
    let (sheets, _) = act(
        opened(manual(Protocol::Imap)),
        vec![
            typed(FieldKind::Username, "ada"),
            typed(FieldKind::Port, "1993"),
            typed(FieldKind::Protocol, "jmap"),
        ],
    );
    let want: Vec<FieldKind> = vec![
        FieldKind::Protocol,
        FieldKind::SessionUrl,
        FieldKind::Token,
        FieldKind::Username,
    ];
    assert_eq!(kinds(&sheets), want);
    assert_eq!(
        text_of(&sheets, FieldKind::Username).as_deref(),
        Some("ada")
    );
    assert_eq!(
        text_of(&sheets, FieldKind::Protocol).as_deref(),
        Some("jmap")
    );
    assert_eq!(
        text_of(&sheets, FieldKind::SessionUrl).as_deref(),
        Some("https://example.org/.well-known/jmap")
    );

    // And back: the server fields return, the guessed host follows the protocol, a port that
    // was not typed again is empty, and the login stays.
    let (sheets, _) = act(sheets, vec![typed(FieldKind::Protocol, "pop3")]);
    assert!(kinds(&sheets).contains(&FieldKind::OutgoingServer));
    assert_eq!(
        text_of(&sheets, FieldKind::Server).as_deref(),
        Some("pop.example.org")
    );
    assert_eq!(text_of(&sheets, FieldKind::Port).as_deref(), Some(""));
    assert_eq!(
        text_of(&sheets, FieldKind::Username).as_deref(),
        Some("ada")
    );
}

#[test]
fn a_port_that_cannot_be_right_keeps_the_form_unsent() {
    use porter_core::sheet::{ProblemKind, Protocol};
    let view = manual(Protocol::Imap);
    let (sheets, out) = act(
        opened(view.clone()),
        vec![typed(FieldKind::Port, "ninety"), Do::Submit],
    );
    assert_eq!(out, vec![]);
    let Some(sheet) = sheets.open.first() else {
        panic!("no sheet");
    };
    let SheetView::SignIn(form) = &sheet.view else {
        panic!("no form");
    };
    assert_eq!(
        sheet
            .draft
            .problem(&form.fields)
            .map(|p| (p.field, p.problem)),
        Some((FieldKind::Port, ProblemKind::Invalid))
    );

    // Mended, it goes.
    let (_, out) = act(
        opened(view),
        vec![typed(FieldKind::Port, "1993"), Do::Submit],
    );
    assert_eq!(out.len(), 1);
}
