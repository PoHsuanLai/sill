//! The notifications server on a private `dbus-daemon` (`support/private_bus.rs`), never the
//! session bus: the person's own daemon holds `org.freedesktop.Notifications` there (on this
//! machine, plasmashell). `#[ignore]`d because they start a bus: run with
//! `cargo test -p sill-services --test notifications_live -- --ignored --nocapture`.
//!
//! The first proves the private bus itself; the rest check `notifications/live.rs`.

mod support;

use std::collections::HashMap;
use std::time::Duration;

use sill_services::env::{Bus, BusRoute};
use sill_services::notifications::{self, BUS_NAME, HistoryPlace, NotificationsHandles, Setup};
use sill_services::{
    ActionKey, Activation, ActivationToken, NameHolder, NotificationId, Notifications,
    NotificationsCommand, NotificationsPolicy, ServerMode,
};
use support::private_bus::PrivateBus;
use support::signals::{Heard, Signals};
use tokio::sync::watch;
use zbus::Message;
use zbus::zvariant::OwnedValue;

const SETTLE: Duration = Duration::from_secs(3);

/// Whether `receiver` satisfies `want` within `limit`.
async fn wait_for<T>(
    receiver: &mut watch::Receiver<T>,
    limit: Duration,
    want: impl Fn(&T) -> bool,
) -> bool {
    tokio::time::timeout(limit, receiver.wait_for(|value| want(value)))
        .await
        .is_ok_and(|seen| seen.is_ok())
}

async fn connect(bus: &PrivateBus) -> zbus::Connection {
    zbus::connection::Builder::address(bus.address.as_str())
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("connecting to the private bus: {e}"))
}

/// Who holds the notifications name on `bus`, as the bus says.
async fn owner(connection: &zbus::Connection) -> Option<String> {
    let dbus = zbus::fdo::DBusProxy::new(connection)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let name = zbus::names::BusName::try_from(BUS_NAME).unwrap_or_else(|e| panic!("{e}"));
    dbus.get_name_owner(name).await.ok().map(|o| o.to_string())
}

fn setup(bus: &PrivateBus) -> Setup {
    Setup {
        bus: Bus::session(BusRoute::Address(bus.address.clone())),
        history: HistoryPlace::NotKept,
    }
}

fn own() -> NotificationsPolicy {
    NotificationsPolicy {
        server: ServerMode::Own,
        ..NotificationsPolicy::default()
    }
}

#[test]
#[ignore = "live: starts a private dbus-daemon"]
fn the_private_bus_starts_empty_and_activates_nothing() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let bus = PrivateBus::start();
    runtime.block_on(async {
        let connection = connect(&bus).await;
        assert_eq!(owner(&connection).await, None, "nobody holds the name");
        // A call to the unowned name must fail rather than activate a daemon (the stock
        // session config would start plasmashell's notifications here).
        let reply = connection
            .call_method(
                Some(BUS_NAME),
                notifications::OBJECT_PATH,
                Some(BUS_NAME),
                "GetCapabilities",
                &(),
            )
            .await;
        assert!(reply.is_err(), "activated something: {reply:?}");
        assert_eq!(owner(&connection).await, None);
    });
}

#[test]
#[ignore = "live: starts a private dbus-daemon"]
fn a_passive_service_never_asks_for_the_name_but_reports_its_holder() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let bus = PrivateBus::start();
    let (mut live, _history, control) = notifications::spawn(runtime.handle(), setup(&bus));
    let seen = runtime.block_on(wait_for(&mut live, SETTLE, |n: &Notifications| {
        n.server.holder == NameHolder::Nobody
    }));
    assert!(seen, "{:?}", live.borrow().server);
    let _ = control.send(NotificationsCommand::Policy(NotificationsPolicy::default()));
    runtime.block_on(async {
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert_eq!(
            owner(&connect(&bus).await).await,
            None,
            "passive took the name"
        );
    });
}

#[test]
#[ignore = "live: starts a private dbus-daemon"]
fn owning_serves_notify_and_close_with_their_signal() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let bus = PrivateBus::start();
    let (mut live, _history, control) = notifications::spawn(runtime.handle(), setup(&bus));
    control
        .send(NotificationsCommand::Policy(own()))
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(runtime.block_on(wait_for(&mut live, SETTLE, |n| {
        n.server.holder == NameHolder::Sill
    })));
    runtime.block_on(async {
        let connection = connect(&bus).await;
        let hints: HashMap<&str, OwnedValue> = HashMap::new();
        let reply = connection
            .call_method(
                Some(BUS_NAME),
                notifications::OBJECT_PATH,
                Some(BUS_NAME),
                "Notify",
                &(
                    "sill-test",
                    0u32,
                    "",
                    "Hello",
                    "<b>world</b>",
                    vec!["default", "Open"],
                    hints,
                    -1i32,
                ),
            )
            .await
            .unwrap_or_else(|e| panic!("Notify: {e}"));
        let id: u32 = reply.body().deserialize().unwrap_or_else(|e| panic!("{e}"));
        assert_ne!(id, 0);
        assert!(wait_for(&mut live, SETTLE, |n| n.banners.len() == 1).await);
        let mut signals = Signals::listen(&connection).await;
        call(&connection, "CloseNotification", &(id,))
            .await
            .unwrap_or_else(|e| panic!("CloseNotification: {e}"));
        assert!(wait_for(&mut live, SETTLE, |n| n.banners.is_empty()).await);
        assert_eq!(
            signals.next(SETTLE).await,
            Some(Heard::Closed { id, reason: 3 })
        );
        let again = call(&connection, "CloseNotification", &(id,)).await;
        assert!(again.is_err(), "an id no longer held: {again:?}");
    });
}

/// Call a method of the notifications object on `connection`.
async fn call<B>(connection: &zbus::Connection, method: &str, body: &B) -> zbus::Result<Message>
where
    B: serde::Serialize + zbus::zvariant::DynamicType,
{
    connection
        .call_method(
            Some(BUS_NAME),
            notifications::OBJECT_PATH,
            Some(BUS_NAME),
            method,
            body,
        )
        .await
}

/// A `Notify` from `app` with `actions`, answering its id.
async fn notify(connection: &zbus::Connection, app: &str, actions: Vec<&str>) -> u32 {
    let hints: HashMap<&str, OwnedValue> = HashMap::new();
    let body = (app, 0u32, "", "Hello", "world", actions, hints, -1i32);
    call(connection, "Notify", &body)
        .await
        .unwrap_or_else(|e| panic!("Notify: {e}"))
        .body()
        .deserialize()
        .unwrap_or_else(|e| panic!("{e}"))
}

/// A service owning the name on `bus` with `history`.
fn owning(
    runtime: &tokio::runtime::Runtime,
    bus: &PrivateBus,
    history: HistoryPlace,
) -> NotificationsHandles {
    let setup = Setup {
        history,
        ..setup(bus)
    };
    let (mut live, history, control) = notifications::spawn(runtime.handle(), setup);
    control
        .send(NotificationsCommand::Policy(own()))
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(runtime.block_on(wait_for(&mut live, SETTLE, |n| {
        n.server.holder == NameHolder::Sill
    })));
    (live, history, control)
}

#[test]
#[ignore = "live: starts a private dbus-daemon"]
fn capabilities_and_server_information_answer_as_the_spec_says() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let bus = PrivateBus::start();
    let _service = owning(&runtime, &bus, HistoryPlace::NotKept);
    runtime.block_on(async {
        let connection = connect(&bus).await;
        let words: Vec<String> = call(&connection, "GetCapabilities", &())
            .await
            .unwrap_or_else(|e| panic!("GetCapabilities: {e}"))
            .body()
            .deserialize()
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(words, notifications::CAPABILITIES);
        let info: (String, String, String, String) = call(&connection, "GetServerInformation", &())
            .await
            .unwrap_or_else(|e| panic!("GetServerInformation: {e}"))
            .body()
            .deserialize()
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((info.0.as_str(), info.1.as_str()), ("sill", "sill"));
        assert_eq!(info.3, "1.2");
    });
}

#[test]
#[ignore = "live: starts a private dbus-daemon"]
fn an_action_is_invoked_then_closed_and_a_swipe_keeps_the_history() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let bus = PrivateBus::start();
    let (mut live, mut history, control) = owning(&runtime, &bus, HistoryPlace::NotKept);
    runtime.block_on(async {
        let connection = connect(&bus).await;
        let mut signals = Signals::listen(&connection).await;
        let id = notify(&connection, "act", vec!["default", "Open"]).await;
        assert!(wait_for(&mut live, SETTLE, |n| n.banners.len() == 1).await);
        let token = Activation::Token(ActivationToken("tok".to_owned()));
        control
            .send(NotificationsCommand::InvokeAction {
                id: NotificationId(id),
                action: ActionKey::default_action(),
                token,
            })
            .unwrap_or_else(|e| panic!("{e}"));
        let heard = [
            signals.next(SETTLE).await,
            signals.next(SETTLE).await,
            signals.next(SETTLE).await,
        ];
        let token = Heard::Token {
            id,
            token: "tok".to_owned(),
        };
        let action = Heard::Invoked {
            id,
            action: "default".to_owned(),
        };
        let closed = Heard::Closed { id, reason: 2 };
        assert_eq!(heard, [Some(token), Some(action), Some(closed)]);

        let swiped = notify(&connection, "swipe", vec![]).await;
        assert!(wait_for(&mut live, SETTLE, |n| n.banners.len() == 1).await);
        control
            .send(NotificationsCommand::SwipeAway(NotificationId(swiped)))
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(wait_for(&mut live, SETTLE, |n| n.banners.is_empty()).await);
        assert_eq!(
            signals.next(SETTLE).await,
            None,
            "a kept swipe tells the app nothing: the notification is still live in the center"
        );
        assert!(
            wait_for(&mut history, SETTLE, |h| h
                .find(NotificationId(swiped))
                .is_some())
            .await,
            "a swiped banner stays in the center"
        );
    });
}

#[test]
#[ignore = "live: starts a private dbus-daemon"]
fn the_history_survives_a_restart() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let bus = PrivateBus::start();
    let dir = std::env::temp_dir().join(format!("sill-notes-live-{}", std::process::id()));
    let file = dir.join("notifications.json");
    let _ = std::fs::remove_dir_all(&dir);
    let (live, mut history, control) = owning(&runtime, &bus, HistoryPlace::At(file.clone()));
    let first = runtime.block_on(async {
        let connection = connect(&bus).await;
        let id = notify(&connection, "keep", vec![]).await;
        assert!(wait_for(&mut history, SETTLE, |h| h.len() == 1).await);
        id
    });
    drop((live, history, control));
    // The first service gives the name back once its control is gone.
    runtime.block_on(async {
        let connection = connect(&bus).await;
        for _ in 0..30 {
            if owner(&connection).await.is_none() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert_eq!(owner(&connection).await, None, "released on exit");
    });
    let (_live, mut history, _control) = owning(&runtime, &bus, HistoryPlace::At(file));
    assert!(runtime.block_on(wait_for(&mut history, SETTLE, |h| h.len() == 1)));
    let next = runtime.block_on(async { notify(&connect(&bus).await, "keep", vec![]).await });
    assert!(
        next > first,
        "ids continue after a restart: {first} then {next}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "live: starts a private dbus-daemon"]
fn notify_send_and_gdbus_reach_the_server() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let bus = PrivateBus::start();
    let (mut live, _history, _control) = owning(&runtime, &bus, HistoryPlace::NotKept);
    // Only the child's environment names the private bus; this process never uses it.
    let run = |program: &str, args: &[&str]| {
        let out = std::process::Command::new(program)
            .args(args)
            .env("DBUS_SESSION_BUS_ADDRESS", &bus.address)
            .output()
            .unwrap_or_else(|e| panic!("{program}: {e}"));
        assert!(out.status.success(), "{program}: {out:?}");
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    };
    let id = run("notify-send", &["-p", "-a", "accept", "Hello"]);
    assert!(runtime.block_on(wait_for(&mut live, SETTLE, |n| n.banners.len() == 1)));
    let dest = ["call", "--session", "--dest", BUS_NAME];
    let path = ["--object-path", notifications::OBJECT_PATH];
    let info = run(
        "gdbus",
        &[
            &dest[..],
            &path[..],
            &[
                "--method",
                "org.freedesktop.Notifications.GetServerInformation",
            ],
        ]
        .concat(),
    );
    assert!(info.starts_with("('sill', 'sill', '"), "{info}");
    assert!(info.ends_with("'1.2')"), "{info}");
    run(
        "gdbus",
        &[
            &dest[..],
            &path[..],
            &[
                "--method",
                "org.freedesktop.Notifications.CloseNotification",
                &id,
            ],
        ]
        .concat(),
    );
    assert!(runtime.block_on(wait_for(&mut live, SETTLE, |n| n.banners.is_empty())));
}

#[test]
#[ignore = "live: starts a private dbus-daemon"]
fn another_holder_is_never_replaced_and_the_name_is_taken_when_free() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let bus = PrivateBus::start();
    let other = runtime.block_on(async {
        let other = connect(&bus).await;
        other
            .request_name(BUS_NAME)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        other
    });
    let (mut live, _history, control) = notifications::spawn(runtime.handle(), setup(&bus));
    control
        .send(NotificationsCommand::Policy(own()))
        .unwrap_or_else(|e| panic!("{e}"));
    let pid = std::process::id();
    assert!(runtime.block_on(wait_for(&mut live, SETTLE, |n| matches!(
        &n.server.holder,
        NameHolder::Other(owner) if owner.pid == Some(pid)
    ))));
    runtime.block_on(async {
        other
            .release_name(BUS_NAME)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    });
    assert!(runtime.block_on(wait_for(&mut live, SETTLE, |n| {
        n.server.holder == NameHolder::Sill
    })));
}
