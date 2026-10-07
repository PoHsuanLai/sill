//! The work areas on a private bus: the property answers with what the daemon published, a change
//! is announced, and a name someone else owns is reported, not fought over. A test skips, saying
//! so, without a `dbus-daemon`. Never the person's session bus.

use std::time::Duration;

use sill_model::work_areas::{
    Rect, WorkArea, WorkAreaCommand, WorkAreaLink, WorkAreas, WorkTrouble,
};
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::service::{Handle, spawn};
use sill_services::work_areas::WorkAreaService;
use sill_testkit::{PrivateBus, TempDir};
use tokio::sync::watch;
use zbus::connection::Builder;
use zbus::export::futures_core::Stream;
use zbus::{Connection, proxy};

const SETTLE: Duration = Duration::from_secs(5);

type Wire = (u32, String, (i32, i32, u32, u32), (i32, i32, u32, u32), u32);

#[proxy(
    interface = "org.quire.Outputs1",
    default_service = "org.quire.Outputs1",
    default_path = "/org/quire/Outputs1"
)]
trait Outputs {
    #[zbus(property)]
    fn work_areas(&self) -> zbus::Result<Vec<Wire>>;
}

async fn wait_for<T>(receiver: &mut watch::Receiver<T>, want: impl Fn(&T) -> bool) -> bool {
    tokio::time::timeout(SETTLE, receiver.wait_for(|value| want(value)))
        .await
        .is_ok_and(|seen| seen.is_ok())
}

fn area(id: u32, top: u32) -> WorkArea {
    let frame = Rect {
        x: 0,
        y: 0,
        width: 2560,
        height: 1440,
    };
    WorkArea {
        id,
        name: format!("DP-{id}"),
        frame,
        work: Rect {
            y: i32::try_from(top).unwrap_or(0),
            height: 1440 - top,
            ..frame
        },
        scale: 180,
    }
}

fn start(bus: &PrivateBus) -> (Handle<WorkAreaService>, TempDir) {
    let scratch = TempDir::new("work-areas-bus");
    let env = Env {
        buses: Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone())),
        ..Env::isolated(scratch.path())
    };
    let handle = spawn::<WorkAreaService>(&tokio::runtime::Handle::current(), &env, ());
    (handle, scratch)
}

async fn client(bus: &PrivateBus) -> Connection {
    Builder::address(bus.address.as_str())
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("connecting: {e}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn the_property_answers_and_a_change_is_announced() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let (mut shell, _scratch) = start(&bus);
    // Published before the name is taken: the first reader still sees it.
    let _ = shell
        .control
        .send(WorkAreaCommand::Publish(vec![area(1, 32)]));
    let _ = shell.control.send(WorkAreaCommand::Serve);
    assert!(
        wait_for(&mut shell.state, |s: &WorkAreas| s.link
            == WorkAreaLink::Serving)
        .await
    );

    let connection = client(&bus).await;
    let proxy = OutputsProxy::new(&connection)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let first = proxy.work_areas().await.unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        first,
        vec![(
            1,
            "DP-1".to_owned(),
            (0, 0, 2560, 1440),
            (0, 32, 2560, 1408),
            180
        )]
    );

    let mut changes = proxy.receive_work_areas_changed().await;
    let _ = shell
        .control
        .send(WorkAreaCommand::Publish(vec![area(1, 32), area(2, 0)]));
    // The stream may first hand over the value it already had; the change is the one with two.
    let seen = tokio::time::timeout(SETTLE, async {
        loop {
            let changed = std::future::poll_fn(|cx| std::pin::Pin::new(&mut changes).poll_next(cx))
                .await
                .unwrap_or_else(|| panic!("the stream ended"));
            let value = changed.get().await.unwrap_or_else(|e| panic!("{e}"));
            if value.len() == 2 {
                return value;
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("no change was announced"));
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[1].3, (0, 0, 2560, 1440));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_taken_name_is_reported() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let _owner = Builder::address(bus.address.as_str())
        .and_then(|b| b.name("org.quire.Outputs1"))
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let (mut shell, _scratch) = start(&bus);
    let _ = shell.control.send(WorkAreaCommand::Serve);
    assert!(
        wait_for(&mut shell.state, |s: &WorkAreas| s.link
            == WorkAreaLink::Unavailable(WorkTrouble::NameTaken))
        .await,
        "{:?}",
        shell.state.borrow().link
    );
}
