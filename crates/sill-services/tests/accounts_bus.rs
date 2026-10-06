//! The accounts service on a private bus: a stand-in accountd, and the real notifications
//! server, so `NeedsReauth` becomes a notice with a "Sign in again" button, the button reaches
//! accountd's `Reauthenticate`, and the account being well again takes the notice back. Everything
//! waits on a published state or on the stand-in's record; a test skips, saying so, without a
//! `dbus-daemon`. Never the person's session bus.

mod accounts_support;

use std::time::Duration;

use accounts_support::{ACCOUNT, FakeAccountd, Listing};
use sill_model::accounts::{AccountKey, AccountsLink, AccountsState, shown_for, sign_in_key};
use sill_model::notifications::{
    Activation, NotesState, NotificationId, NotificationsCommand, NotificationsPolicy, ServerMode,
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

fn press(shell: &Shell, id: NotificationId) {
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

    press(&shell, id);
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
    press(&shell, id);
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
