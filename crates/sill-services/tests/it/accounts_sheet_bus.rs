//! The accounts sheet's host side on a private bus: a stand-in accountd drives it (`Open`,
//! `Update`, `Close`) and hears `Input`; the shell's own part is stepped through `Act`. Only the
//! account services may drive it (a forged caller is refused and draws nothing; who counts as a
//! service is `callers_bus.rs`'s), and what the person typed reaches accountd alone. A test skips,
//! saying so, without a `dbus-daemon`. Never the person's session bus.

use std::sync::Arc;
use std::time::Duration;

use porter_core::{ProviderId, SecretText};
use porter_dbus::accounts::AccountsSheetProxy;
use porter_sheet::{
    Entry, FieldAnswer, FieldKind, FieldSpec, FieldValue, Presence, ProviderRow, SheetInput,
    SheetView, SignInView,
};
use sill_model::accounts::sheet::{
    AccountsSheetCommand, AccountsSheets, SheetAction, SheetHandle, SheetLink,
};
use sill_model::launch::LaunchCommand;
use sill_model::serve::ClaimError;
use sill_services::accounts::sheet::{AccountsSheetLinks, AccountsSheetService};
use sill_services::control::Control;
use sill_services::env::{BusRoute, Buses, Env, NameClaims};
use sill_services::service::{Handle, spawn};
use sill_testkit::{PrivateBus, TempDir};
use std::pin::Pin;
use tokio::sync::watch;
use zbus::Connection;
use zbus::connection::Builder;
use zbus::export::futures_core::Stream;

use crate::callers_support::{Introduced, Role, Who};

const SETTLE: Duration = Duration::from_secs(5);
/// How long a thing that must not happen is given to happen.
const QUIET: Duration = Duration::from_millis(500);

async fn wait_for<T>(receiver: &mut watch::Receiver<T>, want: impl Fn(&T) -> bool) -> bool {
    tokio::time::timeout(SETTLE, receiver.wait_for(|value| want(value)))
        .await
        .is_ok_and(|seen| seen.is_ok())
}

fn provider(id: &str) -> ProviderId {
    ProviderId::parse(id).unwrap_or_else(|e| panic!("{e}"))
}

fn handle(text: &str) -> SheetHandle {
    SheetHandle::parse(text).unwrap_or_else(|| panic!("{text}"))
}

pub(crate) fn providers() -> String {
    json(&SheetView::Providers(vec![ProviderRow {
        id: provider("nextcloud"),
        label: "Nextcloud".to_owned(),
        mark: "nextcloud".to_owned(),
        kind: porter_sheet::RowKind::Provider,
        auth: porter_sheet::ProviderKind::Service,
        mark_face: None,
        group: None,
    }]))
}

fn form() -> String {
    let spec = |kind, entry| FieldSpec {
        kind,
        entry,
        presence: Presence::Required,
        prefill: None,
    };
    json(&SheetView::SignIn(SignInView {
        provider: provider("nextcloud"),
        row: None,
        fields: vec![
            spec(FieldKind::Address, Entry::Plain),
            spec(FieldKind::Password, Entry::Secret),
        ],
        problem: None,
    }))
}

fn json(view: &SheetView) -> String {
    serde_json::to_string(view).unwrap_or_else(|e| panic!("{e}"))
}

pub(crate) struct Shell {
    pub(crate) sheet: Handle<AccountsSheetService>,
    /// Who the stand-ins are, to the sheet.
    pub(crate) callers: Arc<Introduced>,
    /// What the sheet asked the launch service to do.
    launched: tokio::sync::mpsc::UnboundedReceiver<LaunchCommand>,
    _scratch: TempDir,
}

pub(crate) fn shell(bus: &PrivateBus) -> Shell {
    shell_claiming(bus, NameClaims::Claim)
}

fn shell_claiming(bus: &PrivateBus, claims: NameClaims) -> Shell {
    let scratch = TempDir::new("accounts-sheet-bus");
    let callers = Introduced::new();
    let env = Env {
        buses: Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone())),
        callers: callers.clone(),
        claims,
        ..Env::isolated(scratch.path())
    };
    let (launch, launched) = Control::channel();
    let sheet = spawn::<AccountsSheetService>(
        &tokio::runtime::Handle::current(),
        &env,
        AccountsSheetLinks { launch },
    );
    Shell {
        sheet,
        callers,
        launched,
        _scratch: scratch,
    }
}

pub(crate) async fn serving(shell: &mut Shell) {
    assert!(
        wait_for(&mut shell.sheet.state, |s: &AccountsSheets| s.link
            == SheetLink::Serving)
        .await,
        "{:?}",
        shell.sheet.state.borrow().link
    );
}

/// A connection on `bus`, holding `name` when given.
pub(crate) async fn connect(bus: &PrivateBus, name: Option<&str>) -> Connection {
    let builder = Builder::address(bus.address.as_str()).unwrap_or_else(|e| panic!("{e}"));
    let builder = match name {
        Some(name) => builder
            .name(name.to_owned())
            .unwrap_or_else(|e| panic!("{e}")),
        None => builder,
    };
    builder
        .build()
        .await
        .unwrap_or_else(|e| panic!("connecting: {e}"))
}

/// A stand-in holding `name`, whom the sheet takes for the service `role`.
async fn serviced(bus: &PrivateBus, shell: &Shell, name: &str, role: Role) -> Connection {
    standing_in(bus, shell, name, Who::Service(role)).await
}

/// A stand-in holding `name`, whom the sheet takes for `who`.
pub(crate) async fn standing_in(
    bus: &PrivateBus,
    shell: &Shell,
    name: &str,
    who: Who,
) -> Connection {
    let connection = connect(bus, Some(name)).await;
    shell.callers.make(&connection, who);
    connection
}

pub(crate) async fn host(connection: &Connection) -> AccountsSheetProxy<'static> {
    AccountsSheetProxy::new(connection)
        .await
        .unwrap_or_else(|e| panic!("{e}"))
}

async fn next<S: Stream + Unpin>(stream: &mut S) -> Option<S::Item> {
    std::future::poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)).await
}

/// The next `Input` on `inputs`, as a value.
async fn heard(inputs: &mut zbus::MessageStream) -> SheetInput {
    let message = tokio::time::timeout(SETTLE, next(inputs))
        .await
        .unwrap_or_else(|_| panic!("accountd heard nothing"))
        .unwrap_or_else(|| panic!("the stream ended"))
        .unwrap_or_else(|e| panic!("{e}"));
    let (_, input): (String, String) = message
        .body()
        .deserialize()
        .unwrap_or_else(|e| panic!("{e}"));
    serde_json::from_str(&input).unwrap_or_else(|e| panic!("{e}"))
}

async fn inputs_of(connection: &Connection) -> zbus::MessageStream {
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.quire.AccountsSheet1")
        .and_then(|b| b.member("Input"))
        .map(|b| b.build())
        .unwrap_or_else(|e| panic!("{e}"));
    zbus::MessageStream::for_match_rule(rule, connection, None)
        .await
        .unwrap_or_else(|e| panic!("{e}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn accountd_opens_updates_and_closes_and_hears_what_the_person_does() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let mut shell = shell(&bus);
    let accountd = serviced(&bus, &shell, "org.quire.Accounts1", Role::Accounts).await;
    serving(&mut shell).await;
    let mut inputs = inputs_of(&accountd).await;
    let sheet = host(&accountd).await;

    sheet
        .open("accountd-0", "", &providers())
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        wait_for(&mut shell.sheet.state, |s: &AccountsSheets| s.open.len()
            == 1)
        .await,
        "the sheet was not opened"
    );
    let _ = shell.sheet.control.send(AccountsSheetCommand::Act(
        handle("accountd-0"),
        SheetAction::Pick(sill_model::accounts::sheet::ListKey::Provider(provider(
            "nextcloud",
        ))),
    ));
    assert_eq!(
        heard(&mut inputs).await,
        SheetInput::Pick(provider("nextcloud"))
    );

    sheet
        .update("accountd-0", &form())
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        wait_for(&mut shell.sheet.state, |s: &AccountsSheets| s
            .open
            .first()
            .is_some_and(|o| matches!(o.view, SheetView::SignIn(_))))
        .await
    );
    for action in [
        SheetAction::Type(
            FieldKind::Address,
            FieldValue::Plain("ada@example.org".into()),
        ),
        SheetAction::Type(
            FieldKind::Password,
            FieldValue::Secret(SecretText::new("hunter2")),
        ),
        SheetAction::Submit,
    ] {
        let _ = shell
            .sheet
            .control
            .send(AccountsSheetCommand::Act(handle("accountd-0"), action));
    }
    assert_eq!(
        heard(&mut inputs).await,
        SheetInput::Submit(vec![
            FieldAnswer {
                kind: FieldKind::Address,
                value: FieldValue::Plain("ada@example.org".to_owned()),
            },
            FieldAnswer {
                kind: FieldKind::Password,
                value: FieldValue::Secret(SecretText::new("hunter2")),
            },
        ])
    );
    assert!(
        !format!("{:?}", shell.sheet.state.borrow()).contains("hunter2"),
        "the published state must not show a secret"
    );

    sheet
        .close("accountd-0")
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        wait_for(&mut shell.sheet.state, |s: &AccountsSheets| s
            .open
            .is_empty())
        .await
    );
}

const BROWSER: &str = r#"{"kind":"browser_wait","v":{"provider":"google","url":"https://accounts.example.org/auth"}}"#;

#[tokio::test(flavor = "multi_thread")]
async fn a_browser_step_opens_its_page_and_shown_again_opens_it_again() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let mut shell = shell(&bus);
    let accountd = serviced(&bus, &shell, "org.quire.Accounts1", Role::Accounts).await;
    serving(&mut shell).await;
    let mut inputs = inputs_of(&accountd).await;
    let sheet = host(&accountd).await;
    let opened = |shell: &mut Shell| {
        std::iter::from_fn(|| shell.launched.try_recv().ok()).collect::<Vec<_>>()
    };
    let page = || {
        LaunchCommand::Open(sill_model::launch::OpenRequest {
            target: sill_launcher::OpenTarget::Url("https://accounts.example.org/auth".to_owned()),
            token: None,
        })
    };
    sheet
        .open("accountd-0", "", BROWSER)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        wait_for(&mut shell.sheet.state, |s: &AccountsSheets| s.open.len()
            == 1)
        .await
    );
    tokio::time::sleep(QUIET).await;
    assert_eq!(
        opened(&mut shell),
        vec![page()],
        "the first open is the shell's"
    );
    // "Open Again" reaches accountd, which shows the same page again.
    let _ = shell.sheet.control.send(AccountsSheetCommand::Act(
        handle("accountd-0"),
        SheetAction::OpenAgain,
    ));
    assert_eq!(heard(&mut inputs).await, SheetInput::OpenAgain);
    sheet
        .update("accountd-0", BROWSER)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    tokio::time::sleep(QUIET).await;
    assert_eq!(
        opened(&mut shell),
        vec![page()],
        "the repeat opens it again"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_forged_caller_is_refused_and_draws_nothing() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let mut shell = shell(&bus);
    let accountd = serviced(&bus, &shell, "org.quire.Accounts1", Role::Accounts).await;
    serving(&mut shell).await;
    let forger = connect(&bus, None).await;
    let forged = host(&forger).await;

    let refused = |result: zbus::Result<()>, what: &str| match result {
        Err(zbus::Error::MethodError(name, ..)) => {
            assert_eq!(
                name.as_str(),
                "org.freedesktop.DBus.Error.AccessDenied",
                "{what}"
            );
        }
        Err(zbus::Error::FDO(error)) => {
            assert!(
                matches!(*error, zbus::fdo::Error::AccessDenied(_)),
                "{what}: {error}"
            );
        }
        other => panic!("{what}: {other:?}"),
    };
    refused(forged.open("evil", "", &providers()).await, "Open");
    tokio::time::sleep(QUIET).await;
    assert!(
        shell.sheet.state.borrow().open.is_empty(),
        "a forged Open drew"
    );

    // accountd's own sheet: the forger can neither change nor take it down.
    let sheet = host(&accountd).await;
    sheet
        .open("accountd-0", "", &providers())
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        wait_for(&mut shell.sheet.state, |s: &AccountsSheets| s.open.len()
            == 1)
        .await
    );
    refused(forged.update("accountd-0", &form()).await, "Update");
    refused(forged.close("accountd-0").await, "Close");
    let state = shell.sheet.state.borrow().clone();
    assert!(
        matches!(state.open[0].view, SheetView::Providers(_)),
        "a forged Update changed the sheet"
    );
    assert_eq!(state.open.len(), 1, "a forged Close took it down");
}

#[tokio::test(flavor = "multi_thread")]
async fn what_the_person_typed_reaches_accountd_and_nobody_else() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let mut shell = shell(&bus);
    let accountd = serviced(&bus, &shell, "org.quire.Accounts1", Role::Accounts).await;
    serving(&mut shell).await;
    let bystander = connect(&bus, None).await;
    let mut overheard = inputs_of(&bystander).await;
    let mut inputs = inputs_of(&accountd).await;
    let sheet = host(&accountd).await;
    sheet
        .open("accountd-0", "", &providers())
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let _ = shell.sheet.control.send(AccountsSheetCommand::Act(
        handle("accountd-0"),
        SheetAction::Cancel,
    ));
    assert_eq!(heard(&mut inputs).await, SheetInput::Dismiss);
    assert!(
        tokio::time::timeout(QUIET, next(&mut overheard))
            .await
            .is_err(),
        "another connection overheard the sheet"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bad_call_is_refused_and_accountd_leaving_takes_the_sheets_down() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let mut shell = shell(&bus);
    let accountd = serviced(&bus, &shell, "org.quire.Accounts1", Role::Accounts).await;
    serving(&mut shell).await;
    let sheet = host(&accountd).await;
    assert!(sheet.open("bad handle", "", &providers()).await.is_err());
    assert!(sheet.open("a", "", "{not json").await.is_err());
    assert!(sheet.open("a", "", &"x".repeat(70_000)).await.is_err());
    assert!(sheet.update("never-opened", &form()).await.is_err());
    assert!(shell.sheet.state.borrow().open.is_empty());
    sheet
        .open("a", "", &providers())
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        sheet.open("a", "", &providers()).await.is_err(),
        "a handle opened twice"
    );
    assert!(
        wait_for(&mut shell.sheet.state, |s: &AccountsSheets| s.open.len()
            == 1)
        .await
    );
    drop(sheet);
    drop(accountd);
    assert!(
        wait_for(&mut shell.sheet.state, |s: &AccountsSheets| s
            .open
            .is_empty())
        .await,
        "the sheet outlived accountd"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_passive_environment_claims_nothing_and_a_taken_name_is_never_fought_over() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let _passive = shell_claiming(&bus, NameClaims::Passive);
    tokio::time::sleep(QUIET).await;
    let stranger = connect(&bus, None).await;
    let dbus = zbus::fdo::DBusProxy::new(&stranger)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let name = zbus::names::BusName::try_from("org.quire.AccountsSheet1")
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        !dbus.name_has_owner(name.clone()).await.unwrap_or(true),
        "a passive environment claimed the name"
    );
    let _holder = connect(&bus, Some("org.quire.AccountsSheet1")).await;
    let mut shell = shell(&bus);
    assert!(
        wait_for(&mut shell.sheet.state, |s: &AccountsSheets| matches!(
            s.link,
            SheetLink::Unavailable(ClaimError::NameTaken(_))
        ))
        .await,
        "{:?}",
        shell.sheet.state.borrow().link
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn each_account_service_is_served_and_its_sheets_go_when_it_leaves() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let mut shell = shell(&bus);
    serving(&mut shell).await;
    let owners = [
        ("org.quire.Accounts1", "accounts-0", Role::Accounts),
        ("org.quire.Inference1", "inference-0", Role::Inference),
        ("org.quire.Sync1", "sync-0", Role::Sync),
    ];
    let mut held = Vec::new();
    for (name, handle, role) in owners {
        let owner = serviced(&bus, &shell, name, role).await;
        host(&owner)
            .await
            .open(handle, "", &providers())
            .await
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        held.push(owner);
    }
    assert!(
        wait_for(&mut shell.sheet.state, |s: &AccountsSheets| s.open.len()
            == 3)
        .await,
        "not every service's sheet was drawn"
    );
    // One service leaving takes down its sheet only.
    drop(held.remove(1));
    assert!(
        wait_for(&mut shell.sheet.state, |s: &AccountsSheets| s.open.len()
            == 2)
        .await,
        "inferd's sheet outlived it"
    );
}
