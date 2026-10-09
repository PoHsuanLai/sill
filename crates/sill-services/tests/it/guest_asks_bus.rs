//! The guest-asks service on a private bus: a stand-in inferd and the real notifications server.
//! A computer asking becomes a notification, "Let <name> use this computer's models?", with Allow
//! and Deny; a press reaches inferd's `AnswerGuest` as the word; the question being answered
//! elsewhere takes the notification back, and `NotAsking` on a late press is quiet. Questions
//! already waiting are read at start. Never the person's session bus; a test skips, saying so,
//! without a `dbus-daemon`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sill_model::guest_asks::{Computer, GuestAsksState, Verdict, choice_key, shown_for};
use sill_model::notifications::{
    Activation, NameHolder, NotesState, NotificationId, NotificationsCommand, NotificationsPolicy,
    ServerMode,
};
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::guest_asks::{GuestAsksLinks, GuestAsksService};
use sill_services::notifications::NotificationsService;
use sill_services::service::{Handle, spawn};
use sill_testkit::{PrivateBus, TempDir};
use tokio::sync::watch;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{OwnedValue, Value};

const SETTLE: Duration = Duration::from_secs(5);
const PATH: &str = "/org/quire/Inference1";
const NODE: &str = "nodeABC";

type Details = std::collections::HashMap<String, OwnedValue>;
type Row = (String, Details);

async fn wait_for<T>(receiver: &mut watch::Receiver<T>, want: impl Fn(&T) -> bool) -> bool {
    tokio::time::timeout(SETTLE, receiver.wait_for(|value| want(value)))
        .await
        .is_ok_and(|seen| seen.is_ok())
}

fn owned(value: Value<'_>) -> OwnedValue {
    OwnedValue::try_from(value).unwrap_or_else(|e| panic!("{e}"))
}

fn row(node: &str, name: &str, state: &str) -> Row {
    let details = Details::from([
        ("name".to_owned(), owned(Value::from(name))),
        ("state".to_owned(), owned(Value::from(state))),
        ("since".to_owned(), owned(Value::from(1_i64))),
    ]);
    (node.to_owned(), details)
}

fn asks(name: &str) -> Details {
    Details::from([
        ("name".to_owned(), owned(Value::from(name))),
        ("since".to_owned(), owned(Value::from(1_i64))),
    ])
}

/// What `AnswerGuest` answers when the question is gone.
#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.quire.Inference1.Error.Computer")]
enum Refusal {
    #[zbus(error)]
    ZBus(zbus::Error),
    NotAsking(String),
}

/// The stand-in's book: the rows `Guests` lists, the answers it was given, and whether a press
/// finds the question gone.
#[derive(Default)]
struct Book {
    rows: Vec<Row>,
    answers: Vec<(String, String)>,
    gone: bool,
    /// How many times `Guests` was read: the service subscribes before it reads.
    reads: usize,
}

struct Inferd {
    book: Arc<Mutex<Book>>,
}

#[zbus::interface(name = "org.quire.Inference1")]
impl Inferd {
    fn guests(&self) -> Vec<Row> {
        self.book.lock().unwrap_or_else(|e| e.into_inner()).reads += 1;
        self.book
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .rows
            .clone()
    }

    fn answer_guest(&self, node: String, answer: String) -> Result<(), Refusal> {
        let mut book = self.book.lock().unwrap_or_else(|e| e.into_inner());
        book.answers.push((node, answer));
        match book.gone {
            true => Err(Refusal::NotAsking("Nobody is asking.".to_owned())),
            false => Ok(()),
        }
    }

    #[zbus(signal)]
    async fn guest_asks(
        emitter: &SignalEmitter<'_>,
        node: &str,
        details: Details,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn guests_changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

struct FakeInferd {
    connection: zbus::Connection,
    book: Arc<Mutex<Book>>,
}

impl FakeInferd {
    async fn start(bus: &PrivateBus, rows: Vec<Row>) -> FakeInferd {
        let book = Arc::new(Mutex::new(Book {
            rows,
            ..Book::default()
        }));
        let connection = zbus::connection::Builder::address(bus.address.as_str())
            .unwrap_or_else(|e| panic!("{e}"))
            .serve_at(
                PATH,
                Inferd {
                    book: Arc::clone(&book),
                },
            )
            .unwrap_or_else(|e| panic!("{e}"))
            .name("org.quire.Inference1")
            .unwrap_or_else(|e| panic!("{e}"))
            .build()
            .await
            .unwrap_or_else(|e| panic!("the stand-in inferd: {e}"));
        FakeInferd { connection, book }
    }

    fn emitter(&self) -> SignalEmitter<'_> {
        SignalEmitter::new(&self.connection, PATH).unwrap_or_else(|e| panic!("{e}"))
    }

    async fn asks(&self, name: &str) {
        Inferd::guest_asks(&self.emitter(), NODE, asks(name))
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    }

    /// The rows are now `rows`, as `GuestsChanged` tells.
    async fn settle(&self, rows: Vec<Row>) {
        self.book.lock().unwrap_or_else(|e| e.into_inner()).rows = rows;
        Inferd::guests_changed(&self.emitter())
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    }

    fn reads(&self) -> usize {
        self.book.lock().unwrap_or_else(|e| e.into_inner()).reads
    }

    fn answers(&self) -> Vec<(String, String)> {
        self.book
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .answers
            .clone()
    }
}

struct Shell {
    notes: Handle<NotificationsService>,
    asks: Handle<GuestAsksService>,
    _scratch: TempDir,
}

async fn shell(bus: &PrivateBus) -> Shell {
    let scratch = TempDir::new("guest-asks-bus");
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
    let asks = spawn::<GuestAsksService>(
        &runtime,
        &env,
        GuestAsksLinks {
            notifications: notes.control.clone(),
            notes: notes.state.clone(),
        },
    );
    Shell {
        notes,
        asks,
        _scratch: scratch,
    }
}

fn computer() -> Computer {
    Computer::new(NODE)
}

async fn notice_shown(shell: &mut Shell) -> NotificationId {
    assert!(
        wait_for(&mut shell.notes.state, |n: &NotesState| shown_for(
            n,
            &computer()
        )
        .is_some())
        .await,
        "no notification for the computer"
    );
    shown_for(&shell.notes.state.borrow(), &computer()).unwrap_or_else(|| panic!("it left"))
}

async fn notice_gone(shell: &mut Shell, why: &str) {
    assert!(
        wait_for(&mut shell.notes.state, |n: &NotesState| shown_for(
            n,
            &computer()
        )
        .is_none())
        .await,
        "{why}"
    );
}

async fn press(shell: &Shell, id: NotificationId, verdict: Verdict) {
    // A person is slower than the service's reading of the notes.
    tokio::time::sleep(Duration::from_millis(150)).await;
    let _ = shell
        .notes
        .control
        .send(NotificationsCommand::InvokeAction {
            id,
            action: choice_key(&computer(), verdict),
            token: Activation::NoToken,
        });
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
async fn a_computer_asking_is_asked_about_and_allow_reaches_inferd_as_a_word() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let inferd = FakeInferd::start(&bus, vec![row("old", "Studio", "approved")]).await;
    let mut shell = shell(&bus).await;
    until("the service never read Guests", || inferd.reads() == 1).await;

    inferd.asks("Laptop").await;
    let id = notice_shown(&mut shell).await;
    let notes = shell.notes.state.borrow().clone();
    let note = notes
        .live
        .banners
        .iter()
        .map(|banner| &banner.note)
        .find(|note| note.id == id)
        .unwrap_or_else(|| panic!("not a banner: {notes:?}"));
    assert_eq!(note.summary, "Let Laptop use this computer's models?");
    assert_eq!(
        note.buttons().map(|b| b.label.as_str()).collect::<Vec<_>>(),
        ["Allow", "Deny"]
    );
    assert!(
        wait_for(&mut shell.asks.state, |s: &GuestAsksState| s.asks.len()
            == 1)
        .await
    );

    press(&shell, id, Verdict::Allow).await;
    until("inferd was not answered", || !inferd.answers().is_empty()).await;
    assert_eq!(inferd.answers(), [(NODE.to_owned(), "allow".to_owned())]);
    notice_gone(&mut shell, "the notification stayed after the answer").await;
    assert!(
        wait_for(&mut shell.asks.state, |s: &GuestAsksState| s
            .asks
            .is_empty())
        .await
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_question_already_waiting_is_told_at_start_and_one_answered_elsewhere_is_withdrawn() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let inferd = FakeInferd::start(
        &bus,
        vec![
            row("old", "Studio", "approved"),
            row(NODE, "Laptop", "asking"),
        ],
    )
    .await;
    let mut shell = shell(&bus).await;
    let id = notice_shown(&mut shell).await;
    assert!(
        wait_for(&mut shell.asks.state, |s: &GuestAsksState| s.asks.len()
            == 1)
        .await,
        "an approved row was taken for a question"
    );

    // Settings answers first: the notification goes, and a late press is quiet.
    inferd.book.lock().unwrap_or_else(|e| e.into_inner()).gone = true;
    press(&shell, id, Verdict::Deny).await;
    until("the press was not given to inferd", || {
        !inferd.answers().is_empty()
    })
    .await;
    assert_eq!(inferd.answers(), [(NODE.to_owned(), "deny".to_owned())]);
    notice_gone(&mut shell, "the notification stayed after the press").await;

    inferd.asks("Laptop").await;
    notice_shown(&mut shell).await;
    inferd.settle(vec![row(NODE, "Laptop", "approved")]).await;
    notice_gone(&mut shell, "the notification stayed after another answered").await;
    assert!(
        wait_for(&mut shell.asks.state, |s: &GuestAsksState| s
            .asks
            .is_empty())
        .await
    );
}
