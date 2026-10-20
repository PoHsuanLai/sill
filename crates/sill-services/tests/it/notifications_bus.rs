//! The notifications server on a private `dbus-daemon` (`sill-testkit`), never the
//! session bus: the person's own daemon holds `org.freedesktop.Notifications` there (on this
//! machine, plasmashell). Without a `dbus-daemon` they say so and skip.
//!
//! The first proves the private bus itself; the rest check `notifications/live.rs`.

use std::collections::HashMap;
use std::time::Duration;

use sill_model::notifications::{
    ActionKey, Activation, ActivationToken, NameHolder, NotesState, NotificationId,
    NotificationsCommand, NotificationsPolicy, ServerMode,
};
use sill_services::control::Control;
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::notifications::{self, BUS_NAME, NotificationsService};
use sill_services::service::spawn;
use sill_testkit::{Heard, PrivateBus, Signals, TempDir};
use tokio::sync::watch;
use zbus::Message;
use zbus::zvariant::OwnedValue;

pub(crate) const SETTLE: Duration = Duration::from_secs(3);

/// Whether `receiver` satisfies `want` within `limit`.
pub(crate) async fn wait_for<T>(
    receiver: &mut watch::Receiver<T>,
    limit: Duration,
    want: impl Fn(&T) -> bool,
) -> bool {
    tokio::time::timeout(limit, receiver.wait_for(|value| want(value)))
        .await
        .is_ok_and(|seen| seen.is_ok())
}

pub(crate) async fn connect(bus: &PrivateBus) -> zbus::Connection {
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

/// The service on `bus`, its history kept under `scratch`; nothing else reached.
fn start(
    runtime: &tokio::runtime::Runtime,
    bus: &PrivateBus,
    scratch: &TempDir,
) -> (watch::Receiver<NotesState>, Control<NotificationsCommand>) {
    let env = Env {
        buses: Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone())),
        ..Env::isolated(scratch.path())
    };
    let service = spawn::<NotificationsService>(runtime.handle(), &env, Default::default());
    (service.state, service.control)
}

fn own() -> NotificationsPolicy {
    NotificationsPolicy {
        server: ServerMode::Own,
        ..NotificationsPolicy::default()
    }
}

#[test]
fn the_private_bus_starts_empty_and_activates_nothing() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let Some(bus) = PrivateBus::start() else {
        return;
    };
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
fn a_passive_service_never_asks_for_the_name_but_reports_its_holder() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("notes-passive");
    let (mut notes, control) = start(&runtime, &bus, &scratch);
    let seen = runtime.block_on(wait_for(&mut notes, SETTLE, |n: &NotesState| {
        n.live.server.holder == NameHolder::Nobody
    }));
    assert!(seen, "{:?}", notes.borrow().live.server);
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
fn owning_serves_notify_and_close_with_their_signal() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("notes-own");
    let (mut notes, control) = start(&runtime, &bus, &scratch);
    control
        .send(NotificationsCommand::Policy(own()))
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(runtime.block_on(wait_for(&mut notes, SETTLE, |n| {
        n.live.server.holder == NameHolder::Sill
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
        assert!(wait_for(&mut notes, SETTLE, |n| n.live.banners.len() == 1).await);
        let mut signals = Signals::listen(&connection).await;
        call(&connection, "CloseNotification", &(id,))
            .await
            .unwrap_or_else(|e| panic!("CloseNotification: {e}"));
        assert!(wait_for(&mut notes, SETTLE, |n| n.live.banners.is_empty()).await);
        assert_eq!(
            signals.next(SETTLE).await,
            Some(Heard::Closed { id, reason: 3 })
        );
        let again = call(&connection, "CloseNotification", &(id,)).await;
        assert!(again.is_err(), "an id no longer held: {again:?}");
    });
}

/// Call a method of the notifications object on `connection`.
pub(crate) async fn call<B>(
    connection: &zbus::Connection,
    method: &str,
    body: &B,
) -> zbus::Result<Message>
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
pub(crate) async fn notify(connection: &zbus::Connection, app: &str, actions: Vec<&str>) -> u32 {
    let hints: HashMap<&str, OwnedValue> = HashMap::new();
    let body = (app, 0u32, "", "Hello", "world", actions, hints, -1i32);
    call(connection, "Notify", &body)
        .await
        .unwrap_or_else(|e| panic!("Notify: {e}"))
        .body()
        .deserialize()
        .unwrap_or_else(|e| panic!("{e}"))
}

/// A service owning the name on `bus`, its history kept under `scratch`.
pub(crate) fn owning(
    runtime: &tokio::runtime::Runtime,
    bus: &PrivateBus,
    scratch: &TempDir,
) -> (watch::Receiver<NotesState>, Control<NotificationsCommand>) {
    let (mut notes, control) = start(runtime, bus, scratch);
    control
        .send(NotificationsCommand::Policy(own()))
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(runtime.block_on(wait_for(&mut notes, SETTLE, |n| {
        n.live.server.holder == NameHolder::Sill
    })));
    (notes, control)
}

#[test]
fn capabilities_and_server_information_answer_as_the_spec_says() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("notes-capabilities");
    let _service = owning(&runtime, &bus, &scratch);
    runtime.block_on(async {
        let connection = connect(&bus).await;
        let words: Vec<String> = call(&connection, "GetCapabilities", &())
            .await
            .unwrap_or_else(|e| panic!("GetCapabilities: {e}"))
            .body()
            .deserialize()
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(words, sill_model::notifications::CAPABILITIES);
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
fn an_action_is_invoked_then_closed_and_a_swipe_keeps_the_history() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("notes-action");
    let (mut notes, control) = owning(&runtime, &bus, &scratch);
    runtime.block_on(async {
        let connection = connect(&bus).await;
        let mut signals = Signals::listen(&connection).await;
        // Another program on the bus: it hears what is broadcast, not what is for the app.
        let bystander = connect(&bus).await;
        let mut overheard = Signals::listen(&bystander).await;
        let id = notify(&connection, "act", vec!["default", "Open"]).await;
        assert!(wait_for(&mut notes, SETTLE, |n| n.live.banners.len() == 1).await);
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
        assert_eq!(heard, [Some(token), Some(action), Some(closed.clone())]);
        assert_eq!(
            overheard.next(SETTLE).await,
            Some(closed),
            "the token and the action go to the app that posted the notification, not to the bus"
        );

        let swiped = notify(&connection, "swipe", vec![]).await;
        assert!(wait_for(&mut notes, SETTLE, |n| n.live.banners.len() == 1).await);
        control
            .send(NotificationsCommand::SwipeAway(NotificationId(swiped)))
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(wait_for(&mut notes, SETTLE, |n| n.live.banners.is_empty()).await);
        assert_eq!(
            signals.next(SETTLE).await,
            None,
            "a kept swipe tells the app nothing: the notification is still live in the center"
        );
        assert!(
            wait_for(&mut notes, SETTLE, |n| n
                .history
                .find(NotificationId(swiped))
                .is_some())
            .await,
            "a swiped banner stays in the center"
        );
    });
}

#[test]
fn the_history_survives_a_restart() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("notes-restart");
    let (mut notes, control) = owning(&runtime, &bus, &scratch);
    let first = runtime.block_on(async {
        let connection = connect(&bus).await;
        let id = notify(&connection, "keep", vec![]).await;
        assert!(wait_for(&mut notes, SETTLE, |n| n.history.len() == 1).await);
        id
    });
    drop((notes, control));
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
    let (mut notes, _control) = owning(&runtime, &bus, &scratch);
    assert!(runtime.block_on(wait_for(&mut notes, SETTLE, |n| n.history.len() == 1)));
    let next = runtime.block_on(async { notify(&connect(&bus).await, "keep", vec![]).await });
    assert!(
        next > first,
        "ids continue after a restart: {first} then {next}"
    );
}

#[test]
fn notify_send_and_gdbus_reach_the_server() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("notes-notify-send");
    let (mut notes, control) = owning(&runtime, &bus, &scratch);
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
    assert!(runtime.block_on(wait_for(&mut notes, SETTLE, |n| n.live.banners.len() == 1)));
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
    // gdbus is not the connection notify-send posted from, so it may not close it.
    let refused = std::process::Command::new("gdbus")
        .args(
            [
                &dest[..],
                &path[..],
                &[
                    "--method",
                    "org.freedesktop.Notifications.CloseNotification",
                    &id,
                ],
            ]
            .concat(),
        )
        .env("DBUS_SESSION_BUS_ADDRESS", &bus.address)
        .output()
        .unwrap_or_else(|e| panic!("gdbus: {e}"));
    assert!(!refused.status.success(), "another connection closed it");
    control
        .send(NotificationsCommand::Dismiss(NotificationId(
            id.parse().unwrap_or(0),
        )))
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(runtime.block_on(wait_for(&mut notes, SETTLE, |n| n.live.banners.is_empty())));
}

#[test]
fn another_holder_is_never_replaced_and_the_name_is_taken_when_free() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let other = runtime.block_on(async {
        let other = connect(&bus).await;
        other
            .request_name(BUS_NAME)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        other
    });
    let scratch = TempDir::new("notes-other-holder");
    let (mut notes, control) = start(&runtime, &bus, &scratch);
    control
        .send(NotificationsCommand::Policy(own()))
        .unwrap_or_else(|e| panic!("{e}"));
    let pid = std::process::id();
    assert!(runtime.block_on(wait_for(&mut notes, SETTLE, |n| matches!(
        &n.live.server.holder,
        NameHolder::Other(owner) if owner.pid == Some(pid)
    ))));
    runtime.block_on(async {
        other
            .release_name(BUS_NAME)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    });
    assert!(runtime.block_on(wait_for(&mut notes, SETTLE, |n| {
        n.live.server.holder == NameHolder::Sill
    })));
}
