//! The sync-hold service on a private bus: a stand-in syncd, a stand-in accountd for the account's
//! name, and the real notifications server. A hold becomes a notification whose one button asks
//! the launch service to open Settings at the account; its end takes the notification back; and
//! `Watch` is called at start and again for every new owner of `org.quire.Sync1`. Never the
//! person's session bus; a test skips, saying so, without a `dbus-daemon`.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::accounts_bus::accounts_support::{ACCOUNT, FakeAccountd, Listing};
use sill_launcher::SettingsLink;
use sill_model::launch::{LaunchCommand, SettingsOpen};
use sill_model::notifications::{
    Activation, NameHolder, NotesState, NotificationId, NotificationsCommand, NotificationsPolicy,
    ServerMode,
};
use sill_model::sync_hold::{Dataset, SyncHoldState, open_key, shown_for};
use sill_services::control::Control;
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::notifications::NotificationsService;
use sill_services::service::{Handle, spawn};
use sill_services::sync_hold::{SyncHoldLinks, SyncHoldService};
use sill_testkit::{PrivateBus, TempDir};
use tokio::sync::{mpsc, watch};
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{OwnedValue, Value};

const SETTLE: Duration = Duration::from_secs(5);
const SYNC_PATH: &str = "/org/quire/Sync1";
const DATASET: &str = "work/files";

async fn wait_for<T>(receiver: &mut watch::Receiver<T>, want: impl Fn(&T) -> bool) -> bool {
    tokio::time::timeout(SETTLE, receiver.wait_for(|value| want(value)))
        .await
        .is_ok_and(|seen| seen.is_ok())
}

type Held = HashMap<String, OwnedValue>;

fn counts(discard: u64, held: u64) -> Held {
    let owned = |value: Value<'_>| OwnedValue::try_from(value).unwrap_or_else(|e| panic!("{e}"));
    HashMap::from([
        ("discard".to_owned(), owned(Value::from(discard))),
        ("held".to_owned(), owned(Value::from(held))),
        ("account".to_owned(), owned(Value::from(ACCOUNT))),
    ])
}

/// `org.quire.Sync1` as far as the shell uses it: `Watch` is counted, and a hold that is already
/// on is told before `Watch` returns.
struct Syncd {
    watched: Arc<AtomicUsize>,
    already: Option<Held>,
}

#[zbus::interface(name = "org.quire.Sync1")]
impl Syncd {
    async fn watch(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.watched.fetch_add(1, Ordering::SeqCst);
        if let Some(held) = self.already.clone() {
            let _ = Self::needs_confirmation(&emitter, DATASET, held).await;
        }
    }

    #[zbus(signal)]
    async fn needs_confirmation(
        emitter: &SignalEmitter<'_>,
        dataset: &str,
        held: Held,
    ) -> zbus::Result<()>;
}

struct FakeSyncd {
    connection: zbus::Connection,
    watched: Arc<AtomicUsize>,
}

impl FakeSyncd {
    async fn start(bus: &PrivateBus, already: Option<Held>) -> FakeSyncd {
        let watched = Arc::new(AtomicUsize::new(0));
        let connection = zbus::connection::Builder::address(bus.address.as_str())
            .unwrap_or_else(|e| panic!("{e}"))
            .serve_at(
                SYNC_PATH,
                Syncd {
                    watched: Arc::clone(&watched),
                    already,
                },
            )
            .unwrap_or_else(|e| panic!("{e}"))
            .name("org.quire.Sync1")
            .unwrap_or_else(|e| panic!("{e}"))
            .build()
            .await
            .unwrap_or_else(|e| panic!("the stand-in syncd: {e}"));
        FakeSyncd {
            connection,
            watched,
        }
    }

    async fn tell(&self, held: Held) {
        let emitter =
            SignalEmitter::new(&self.connection, SYNC_PATH).unwrap_or_else(|e| panic!("{e}"));
        Syncd::needs_confirmation(&emitter, DATASET, held)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    }

    fn watches(&self) -> usize {
        self.watched.load(Ordering::SeqCst)
    }
}

struct Shell {
    notes: Handle<NotificationsService>,
    hold: Handle<SyncHoldService>,
    launched: mpsc::UnboundedReceiver<LaunchCommand>,
    _scratch: TempDir,
}

async fn shell(bus: &PrivateBus) -> Shell {
    let scratch = TempDir::new("sync-hold-bus");
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
            == NameHolder::Sill)
        .await
    );
    let (launch, launched) = Control::channel();
    let hold = spawn::<SyncHoldService>(
        &runtime,
        &env,
        SyncHoldLinks {
            notifications: notes.control.clone(),
            notes: notes.state.clone(),
            launch,
        },
    );
    Shell {
        notes,
        hold,
        launched,
        _scratch: scratch,
    }
}

fn dataset() -> Dataset {
    Dataset::new(DATASET)
}

async fn notice_shown(shell: &mut Shell) -> NotificationId {
    assert!(
        wait_for(&mut shell.notes.state, |n: &NotesState| shown_for(
            n,
            &dataset()
        )
        .is_some())
        .await,
        "no notification for the dataset"
    );
    shown_for(&shell.notes.state.borrow(), &dataset()).unwrap_or_else(|| panic!("it left"))
}

async fn until(what: &str, mut done: impl FnMut() -> bool) {
    let waited = tokio::time::timeout(SETTLE, async {
        while !done() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(waited.is_ok(), "{what}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_hold_is_told_opens_settings_at_the_account_and_is_withdrawn_when_it_ends() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let _accountd = FakeAccountd::start(&bus, "ok", Listing::Of(vec![])).await;
    let syncd = FakeSyncd::start(&bus, None).await;
    let mut shell = shell(&bus).await;
    until("Watch was never called", || syncd.watches() == 1).await;

    syncd.tell(counts(30, 40)).await;
    let id = notice_shown(&mut shell).await;
    let notes = shell.notes.state.borrow().clone();
    let note = notes
        .live
        .banners
        .iter()
        .map(|banner| &banner.note)
        .find(|note| note.id == id)
        .unwrap_or_else(|| panic!("not a banner: {notes:?}"));
    assert_eq!(note.summary, "Files removed online in Work mail");
    assert_eq!(
        note.body.text(),
        "30 of 40 files were removed online. Open Settings to decide."
    );
    assert_eq!(
        note.buttons().map(|b| b.label.as_str()).collect::<Vec<_>>(),
        ["Open Settings"],
        "the shell offers no removal"
    );
    assert!(
        wait_for(&mut shell.hold.state, |s: &SyncHoldState| s.holds.len()
            == 1)
        .await
    );

    // A person is slower than the service's reading of the notes.
    tokio::time::sleep(Duration::from_millis(150)).await;
    let _ = shell
        .notes
        .control
        .send(NotificationsCommand::InvokeAction {
            id,
            action: open_key(&dataset()),
            token: Activation::NoToken,
        });
    let launched = tokio::time::timeout(SETTLE, shell.launched.recv()).await;
    assert_eq!(
        launched.ok().flatten(),
        Some(LaunchCommand::OpenSettings(SettingsOpen::at(SettingsLink(
            format!("account:{ACCOUNT}")
        ))))
    );

    syncd.tell(Held::new()).await;
    assert!(
        wait_for(&mut shell.notes.state, |n: &NotesState| shown_for(
            n,
            &dataset()
        )
        .is_none())
        .await,
        "the notification stayed after the hold ended"
    );
    assert!(
        wait_for(&mut shell.hold.state, |s: &SyncHoldState| s
            .holds
            .is_empty())
        .await
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_hold_already_on_is_told_at_start_and_a_new_syncd_is_joined_again() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let _accountd = FakeAccountd::start(&bus, "ok", Listing::Of(vec![])).await;
    let syncd = FakeSyncd::start(&bus, Some(counts(40, 40))).await;
    let mut shell = shell(&bus).await;
    let id = notice_shown(&mut shell).await;
    let body = shell
        .notes
        .state
        .borrow()
        .live
        .banners
        .iter()
        .map(|banner| &banner.note)
        .find(|note| note.id == id)
        .map(|note| note.body.text());
    assert_eq!(
        body.as_deref(),
        Some("All 40 files were removed online. Open Settings to decide.")
    );
    assert_eq!(syncd.watches(), 1);

    // syncd restarts: its roster is gone, so the shell joins the new one.
    drop(syncd);
    assert!(
        wait_for(&mut shell.hold.state, |s: &SyncHoldState| s
            .holds
            .is_empty())
        .await,
        "the old syncd's hold was kept"
    );
    let again = FakeSyncd::start(&bus, None).await;
    until("the new syncd was not joined", || again.watches() == 1).await;
}
