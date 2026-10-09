//! The workspaces and their looks on a private bus: the property answers with what the daemon
//! published, a change is announced, and a name someone else owns is reported. A test skips,
//! saying so, without a `dbus-daemon`. Never the person's session bus.

use std::time::Duration;

use sill_model::workspace_looks::{
    PublishedWorkspace, WorkspaceLooks, WorkspaceLooksCommand, WorkspaceLooksLink,
    WorkspaceLooksTrouble,
};
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::service::{Handle, spawn};
use sill_services::workspace_looks::WorkspaceLooksService;
use sill_testkit::{PrivateBus, TempDir};
use tokio::sync::watch;
use zbus::connection::Builder;
use zbus::export::futures_core::Stream;
use zbus::{Connection, proxy};

const SETTLE: Duration = Duration::from_secs(5);

type Wire = (String, String, String, String, bool);

#[proxy(
    interface = "org.quire.Workspaces1",
    default_service = "org.quire.Workspaces1",
    default_path = "/org/quire/Workspaces1"
)]
trait Workspaces {
    #[zbus(property)]
    fn workspaces(&self) -> zbus::Result<Vec<Wire>>;
}

async fn wait_for<T>(receiver: &mut watch::Receiver<T>, want: impl Fn(&T) -> bool) -> bool {
    tokio::time::timeout(SETTLE, receiver.wait_for(|value| want(value)))
        .await
        .is_ok_and(|seen| seen.is_ok())
}

fn workspace(id: &str, active: bool) -> PublishedWorkspace {
    PublishedWorkspace {
        id: id.to_owned(),
        name: format!("Space {id}"),
        output: "DP-1".to_owned(),
        look: r#"{"dots":[]}"#.to_owned(),
        active,
    }
}

fn start(bus: &PrivateBus) -> (Handle<WorkspaceLooksService>, TempDir) {
    let scratch = TempDir::new("workspace-looks-bus");
    let env = Env {
        buses: Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone())),
        ..Env::isolated(scratch.path())
    };
    let handle = spawn::<WorkspaceLooksService>(&tokio::runtime::Handle::current(), &env, ());
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
        .send(WorkspaceLooksCommand::Publish(vec![workspace("a", true)]));
    let _ = shell.control.send(WorkspaceLooksCommand::Serve);
    assert!(
        wait_for(&mut shell.state, |s: &WorkspaceLooks| s.link
            == WorkspaceLooksLink::Serving)
        .await
    );

    let connection = client(&bus).await;
    let proxy = WorkspacesProxy::new(&connection)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let first = proxy.workspaces().await.unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        first,
        vec![(
            "a".to_owned(),
            "Space a".to_owned(),
            "DP-1".to_owned(),
            r#"{"dots":[]}"#.to_owned(),
            true
        )]
    );

    let mut changes = proxy.receive_workspaces_changed().await;
    let _ = shell.control.send(WorkspaceLooksCommand::Publish(vec![
        workspace("a", false),
        workspace("b", true),
    ]));
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
    assert_eq!((seen[0].4, seen[1].4), (false, true));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_taken_name_is_reported() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let _owner = Builder::address(bus.address.as_str())
        .and_then(|b| b.name("org.quire.Workspaces1"))
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let (mut shell, _scratch) = start(&bus);
    let _ = shell.control.send(WorkspaceLooksCommand::Serve);
    assert!(
        wait_for(&mut shell.state, |s: &WorkspaceLooks| s.link
            == WorkspaceLooksLink::Unavailable(
                WorkspaceLooksTrouble::NameTaken
            ))
        .await,
        "{:?}",
        shell.state.borrow().link
    );
}
