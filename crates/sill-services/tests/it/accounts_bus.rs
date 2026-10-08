//! The accounts service on a private bus: a stand-in accountd, and the real notifications
//! server, so `NeedsReauth` becomes a notice with a "Sign in again" button, the button reaches
//! accountd's `Reauthenticate`, and the account being well again takes the notice back. Everything
//! waits on a published state or on the stand-in's record; a test skips, saying so, without a
//! `dbus-daemon`. Never the person's session bus.

#[path = "accounts_support/mod.rs"]
pub(crate) mod accounts_support;

use std::time::Duration;

use accounts_support::{ACCOUNT, FakeAccountd, Listing};
use sill_model::accounts::{AccountKey, AccountsLink, AccountsState, shown_for, sign_in_key};
use sill_model::notifications::{
    ActionKey, Activation, NotesState, NotificationId, NotificationsCommand, NotificationsPolicy,
    ServerMode,
};
use sill_services::accounts::{AccountsLinks, AccountsService};
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::notifications::NotificationsService;
use sill_services::service::{Handle, spawn};
use sill_testkit::{PrivateBus, TempDir};
use tokio::sync::watch;

const SETTLE: Duration = Duration::from_secs(5);
/// How long a thing that must not happen is given to happen.
const QUIET: Duration = Duration::from_millis(600);

async fn wait_for<T>(receiver: &mut watch::Receiver<T>, want: impl Fn(&T) -> bool) -> bool {
    tokio::time::timeout(SETTLE, receiver.wait_for(|value| want(value)))
        .await
        .is_ok_and(|seen| seen.is_ok())
}

/// The shell's two services on `bus`, the notifications server holding its name.
struct Shell {
    notes: Handle<NotificationsService>,
    accounts: Handle<AccountsService>,
    _scratch: TempDir,
}

async fn shell(bus: &PrivateBus) -> Shell {
    let scratch = TempDir::new("accounts-bus");
    let env = Env {
        buses: Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone())),
        ..Env::isolated(scratch.path())
    };
    let runtime = tokio::runtime::Handle::current();
    let mut notes = spawn::<NotificationsService>(&runtime, &env, ());
    let _ = notes
        .control
        .send(NotificationsCommand::Policy(NotificationsPolicy {
            server: ServerMode::Own,
            ..NotificationsPolicy::default()
        }));
    assert!(
        wait_for(&mut notes.state, |n: &NotesState| n.live.server.holder
            == sill_model::notifications::NameHolder::Sill)
        .await,
        "{:?}",
        notes.state.borrow().live.server
    );
    let accounts = spawn::<AccountsService>(
        &runtime,
        &env,
        AccountsLinks {
            notifications: notes.control.clone(),
            notes: notes.state.clone(),
        },
    );
    Shell {
        notes,
        accounts,
        _scratch: scratch,
    }
}

fn key() -> AccountKey {
    AccountKey::new(ACCOUNT)
}

/// Wait until the notice for the work account is held, and say which it is.
async fn notice_shown(shell: &mut Shell) -> NotificationId {
    assert!(
        wait_for(&mut shell.notes.state, |n: &NotesState| shown_for(
            n,
            &key()
        )
        .is_some())
        .await,
        "no notice for the account"
    );
    shown_for(&shell.notes.state.borrow(), &key()).unwrap_or_else(|| panic!("the notice left"))
}

async fn notice_gone(shell: &mut Shell) -> bool {
    wait_for(&mut shell.notes.state, |n: &NotesState| {
        shown_for(n, &key()).is_none()
    })
    .await
}

async fn needing(shell: &mut Shell, count: usize) -> bool {
    wait_for(&mut shell.accounts.state, |s: &AccountsState| {
        s.link == AccountsLink::Present && s.needing.len() == count
    })
    .await
}

/// The person presses the notice's button. A person is slower than the service's reading of the
/// notes (it keeps the id when it first sees the notice), so the press waits a beat.
async fn press(shell: &Shell, id: NotificationId) {
    tokio::time::sleep(Duration::from_millis(150)).await;
    let _ = shell
        .notes
        .control
        .send(NotificationsCommand::InvokeAction {
            id,
            action: sign_in_key(&key()),
            token: Activation::NoToken,
        });
}

async fn until_reauthenticated(accountd: &FakeAccountd, times: usize) -> bool {
    tokio::time::timeout(SETTLE, async {
        while accountd.reauthentications() < times {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .is_ok()
}

#[tokio::test(flavor = "multi_thread")]
async fn needs_reauth_posts_a_notice_and_its_button_signs_in_again() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let accountd = FakeAccountd::start(&bus, "ok", Listing::Of(vec![])).await;
    let mut shell = shell(&bus).await;
    assert!(needing(&mut shell, 0).await, "accountd was not met");
    assert_eq!(accountd.reauthentications(), 0);

    accountd.refuse().await;
    let id = notice_shown(&mut shell).await;
    let notes = shell.notes.state.borrow().clone();
    let note = notes
        .live
        .banners
        .iter()
        .map(|banner| &banner.note)
        .find(|note| note.id == id)
        .unwrap_or_else(|| panic!("the notice is not a banner: {notes:?}"));
    assert_eq!(note.summary, "Work mail needs you to sign in again");
    assert_eq!(note.buttons().count(), 1);
    assert_eq!(
        note.buttons().map(|b| b.label.as_str()).collect::<Vec<_>>(),
        ["Sign in again"]
    );
    assert!(needing(&mut shell, 1).await);

    press(&shell, id).await;
    assert!(
        until_reauthenticated(&accountd, 1).await,
        "Reauthenticate was never called"
    );
    // The account is Ok again, as a finished sign-in leaves it: nothing needs the person.
    assert!(
        needing(&mut shell, 0).await,
        "{:?}",
        shell.accounts.state.borrow()
    );
    assert!(notice_gone(&mut shell).await);
    assert_eq!(accountd.reauthentications(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_testing_app_sign_out_says_google_does_it_every_seven_days() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let accountd = FakeAccountd::start(&bus, "ok", Listing::Of(vec![])).await;
    let mut shell = shell(&bus).await;
    assert!(needing(&mut shell, 0).await, "accountd was not met");

    accountd.refuse_testing_app_expired().await;
    let id = notice_shown(&mut shell).await;
    let notes = shell.notes.state.borrow().clone();
    let body = notes
        .live
        .banners
        .iter()
        .map(|banner| &banner.note)
        .find(|note| note.id == id)
        .map(|note| note.body.text());
    assert_eq!(
        body.as_deref(),
        Some("Google signs this app out every 7 days until it is verified.")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_refused_first_call_is_tried_again_and_the_account_is_told() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    // Three refusals fit inside the backoff (100, 250, 500 ms); the fourth try is let through.
    let _accountd = FakeAccountd::start(
        &bus,
        "needs_reauth",
        Listing::Refusing(3, vec![(ACCOUNT, "Work mail")]),
    )
    .await;
    let mut shell = shell(&bus).await;
    notice_shown(&mut shell).await;
    assert!(needing(&mut shell, 1).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_refusal_past_the_backoff_waits_for_the_name_to_change() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    // Five refusals outlast the four waits: only a new owner of the name starts it over.
    let first = FakeAccountd::start(&bus, "ok", Listing::Refusing(5, vec![])).await;
    let mut shell = shell(&bus).await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(shell.accounts.state.borrow().link, AccountsLink::Absent);
    drop(first);
    let _second = FakeAccountd::start(
        &bus,
        "needs_reauth",
        Listing::Of(vec![(ACCOUNT, "Work mail")]),
    )
    .await;
    assert!(needing(&mut shell, 1).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_notice_goes_when_the_account_is_well_without_the_button() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let accountd = FakeAccountd::start(&bus, "ok", Listing::Of(vec![])).await;
    let mut shell = shell(&bus).await;
    assert!(needing(&mut shell, 0).await);
    accountd.refuse().await;
    notice_shown(&mut shell).await;

    accountd.recover().await;
    assert!(notice_gone(&mut shell).await, "the notice stayed");
    assert!(needing(&mut shell, 0).await);
    assert_eq!(accountd.reauthentications(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_account_already_refused_when_accountd_is_met_is_told() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let _accountd = FakeAccountd::start(
        &bus,
        "needs_reauth",
        Listing::Of(vec![(ACCOUNT, "Work mail")]),
    )
    .await;
    let mut shell = shell(&bus).await;
    notice_shown(&mut shell).await;
    assert!(needing(&mut shell, 1).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_action_signal_from_any_other_connection_signs_nothing_in() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let accountd = FakeAccountd::start(&bus, "ok", Listing::Of(vec![])).await;
    let mut shell = shell(&bus).await;
    assert!(needing(&mut shell, 0).await);
    accountd.refuse().await;
    let id = notice_shown(&mut shell).await;

    // Another program says the person pressed the button.
    let stranger = zbus::connection::Builder::address(bus.address.as_str())
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    stranger
        .emit_signal(
            None::<&str>,
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "ActionInvoked",
            &(id.0, sign_in_key(&key()).0),
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    tokio::time::sleep(QUIET).await;
    assert_eq!(accountd.reauthentications(), 0);

    // The same words from the notifications server do.
    press(&shell, id).await;
    assert!(until_reauthenticated(&accountd, 1).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_accountd_with_no_list_is_followed_by_its_signal() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let accountd = FakeAccountd::start(&bus, "ok", Listing::None).await;
    let mut shell = shell(&bus).await;
    assert!(needing(&mut shell, 0).await, "accountd was not met");
    accountd.refuse().await;
    notice_shown(&mut shell).await;
}

/// A stand-in notifications server: another connection holding the name, which says what it likes
/// was invoked, as the real one does. The shell's notifications service does not ask for the name
/// (`Passive`), so what it holds is the notices sill posted and nothing the stand-in says changes
/// that.
struct FakeServer {
    connection: zbus::Connection,
}

impl FakeServer {
    async fn start(bus: &PrivateBus) -> FakeServer {
        let connection = zbus::connection::Builder::address(bus.address.as_str())
            .unwrap_or_else(|e| panic!("{e}"))
            .name(sill_services::notifications::BUS_NAME)
            .unwrap_or_else(|e| panic!("{e}"))
            .build()
            .await
            .unwrap_or_else(|e| panic!("the stand-in server: {e}"));
        FakeServer { connection }
    }

    async fn close(&self, id: NotificationId) {
        self.connection
            .emit_signal(
                None::<&str>,
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "NotificationClosed",
                &(id.0, 2_u32),
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    }

    async fn invoke(&self, id: NotificationId, key: &ActionKey) {
        self.connection
            .emit_signal(
                None::<&str>,
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "ActionInvoked",
                &(id.0, key.0.as_str()),
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    }
}

/// The shell with its notifications service passive, so the stand-in server holds the name.
async fn shell_under(bus: &PrivateBus) -> (Shell, FakeServer) {
    let server = FakeServer::start(bus).await;
    let scratch = TempDir::new("accounts-bus-id");
    let env = Env {
        buses: Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone())),
        ..Env::isolated(scratch.path())
    };
    let runtime = tokio::runtime::Handle::current();
    let notes = spawn::<NotificationsService>(&runtime, &env, ());
    let accounts = spawn::<AccountsService>(
        &runtime,
        &env,
        AccountsLinks {
            notifications: notes.control.clone(),
            notes: notes.state.clone(),
        },
    );
    (
        Shell {
            notes,
            accounts,
            _scratch: scratch,
        },
        server,
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn the_server_naming_the_notice_sill_posted_signs_in_once() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let accountd = FakeAccountd::start(&bus, "ok", Listing::Of(vec![])).await;
    let (mut shell, server) = shell_under(&bus).await;
    assert!(needing(&mut shell, 0).await);
    accountd.refuse().await;
    let id = notice_shown(&mut shell).await;
    server.invoke(id, &sign_in_key(&key())).await;
    assert!(until_reauthenticated(&accountd, 1).await);
    tokio::time::sleep(QUIET).await;
    assert_eq!(accountd.reauthentications(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_id_sill_did_not_post_for_the_account_does_nothing() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let accountd = FakeAccountd::start(&bus, "ok", Listing::Of(vec![])).await;
    let (mut shell, server) = shell_under(&bus).await;
    assert!(needing(&mut shell, 0).await);
    accountd.refuse().await;
    let id = notice_shown(&mut shell).await;

    // Another notification's id, an id nothing holds, and the notice's own id with another
    // account's key.
    let other = AccountKey::new("/org/quire/Accounts1/account/other");
    server
        .invoke(NotificationId(id.0 + 100), &sign_in_key(&key()))
        .await;
    server
        .invoke(NotificationId(id.0 + 1), &sign_in_key(&key()))
        .await;
    server.invoke(id, &sign_in_key(&other)).await;
    tokio::time::sleep(QUIET).await;
    assert_eq!(accountd.reauthentications(), 0);

    server.invoke(id, &sign_in_key(&key())).await;
    assert!(until_reauthenticated(&accountd, 1).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_closed_notice_does_nothing() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let accountd = FakeAccountd::start(&bus, "ok", Listing::Of(vec![])).await;
    let (mut shell, server) = shell_under(&bus).await;
    assert!(needing(&mut shell, 0).await);
    accountd.refuse().await;
    let id = notice_shown(&mut shell).await;

    // The person closes it; the account still needs them, but this id is no longer a notice.
    let _ = shell.notes.control.send(NotificationsCommand::Dismiss(id));
    assert!(notice_gone(&mut shell).await);
    server.close(id).await;
    server.invoke(id, &sign_in_key(&key())).await;
    tokio::time::sleep(QUIET).await;
    assert_eq!(accountd.reauthentications(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn after_the_account_is_well_and_needs_them_again_only_the_new_id_works() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let accountd = FakeAccountd::start(&bus, "ok", Listing::Of(vec![])).await;
    let (mut shell, server) = shell_under(&bus).await;
    assert!(needing(&mut shell, 0).await);
    accountd.refuse().await;
    let old = notice_shown(&mut shell).await;
    accountd.recover().await;
    assert!(notice_gone(&mut shell).await);
    assert!(needing(&mut shell, 0).await);

    accountd.refuse().await;
    let new = notice_shown(&mut shell).await;
    assert_ne!(old, new, "a fresh post is a new notification");
    server.invoke(old, &sign_in_key(&key())).await;
    tokio::time::sleep(QUIET).await;
    assert_eq!(accountd.reauthentications(), 0);

    server.invoke(new, &sign_in_key(&key())).await;
    assert!(until_reauthenticated(&accountd, 1).await);
}
