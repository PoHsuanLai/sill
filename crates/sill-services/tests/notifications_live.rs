//! The notifications server on a private `dbus-daemon` (`support/private_bus.rs`), never the
//! session bus: the person's own daemon holds `org.freedesktop.Notifications` there (on this
//! machine, plasmashell). `#[ignore]`d because they start a bus: run with
//! `cargo test -p sill-services --test notifications_live -- --ignored --nocapture`.
//!
//! The first proves the private bus itself; the rest are lane a's done criteria (FINDINGS "The
//! M6 fill-in") and pass once `notifications/live.rs` is filled.

mod support;

use std::collections::HashMap;
use std::time::Duration;

use sill_services::notifications::{self, BUS_NAME, BusChoice, HistoryPlace, Setup};
use sill_services::{
    NameHolder, Notifications, NotificationsCommand, NotificationsPolicy, ServerMode,
};
use support::private_bus::PrivateBus;
use tokio::sync::watch;
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
        bus: BusChoice::Address(bus.address.clone()),
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
#[ignore = "M6 fill: a-server"]
fn a_passive_service_never_asks_for_the_name_but_reports_its_holder() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let bus = PrivateBus::start();
    let (mut live, _history, control) = notifications::spawn_with(runtime.handle(), setup(&bus));
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
#[ignore = "M6 fill: a-server"]
fn owning_serves_notify_and_close_with_their_signal() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let bus = PrivateBus::start();
    let (mut live, _history, control) = notifications::spawn_with(runtime.handle(), setup(&bus));
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
        connection
            .call_method(
                Some(BUS_NAME),
                notifications::OBJECT_PATH,
                Some(BUS_NAME),
                "CloseNotification",
                &(id,),
            )
            .await
            .unwrap_or_else(|e| panic!("CloseNotification: {e}"));
        assert!(wait_for(&mut live, SETTLE, |n| n.banners.is_empty()).await);
    });
}

#[test]
#[ignore = "M6 fill: a-server"]
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
    let (mut live, _history, control) = notifications::spawn_with(runtime.handle(), setup(&bus));
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
