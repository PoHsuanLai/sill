//! sill's polkit agent against a stand-in Authority on a private bus and the stand-in helper
//! (`examples/polkit_helper.rs`), never polkitd, the system bus or PAM. The bus test is live (a
//! `dbus-daemon`): run it with `cargo test -p sill-services --test polkit_live -- --ignored
//! --nocapture`. The made-up request's test touches no bus and
//! runs with the suite.

#[path = "support/private_bus.rs"]
mod private_bus;

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use private_bus::PrivateBus;
use sill_services::auth::Secret;
use sill_services::env::{Bus, BusRoute};
use sill_services::polkit_agent::machine::MAX_TRIES;
use sill_services::polkit_agent::{
    AGENT_PATH, ActionId, Cookie, FAKE_SECRET, FakeRequest, HelperChoice, HelperRoute, LastTry,
    Polkit, PolkitCommand, PromptPhase, Registration, SessionChoice, Setup,
};
use tokio::sync::{mpsc, watch};
use zbus::message::Header;
use zbus::zvariant::{OwnedValue, Value};
use zbus::{Connection, interface};

type Subject = (String, HashMap<String, OwnedValue>);

/// What the stand-in Authority heard: `(register or unregister, subject kind, session id,
/// path)`, and who registered.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Heard {
    verb: &'static str,
    kind: String,
    session: String,
    path: String,
}

#[derive(Debug, Clone)]
struct StandIn {
    heard: mpsc::UnboundedSender<Heard>,
    agent: Arc<Mutex<Option<String>>>,
}

fn heard(verb: &'static str, (kind, details): Subject, path: String) -> Heard {
    let session = details
        .get("session-id")
        .and_then(|v| <&str>::try_from(v).ok())
        .unwrap_or("?")
        .to_owned();
    Heard {
        verb,
        kind,
        session,
        path,
    }
}

#[interface(name = "org.freedesktop.PolicyKit1.Authority")]
impl StandIn {
    async fn register_authentication_agent(
        &self,
        #[zbus(header)] header: Header<'_>,
        subject: Subject,
        _locale: String,
        object_path: String,
    ) -> zbus::fdo::Result<()> {
        if let Ok(mut agent) = self.agent.lock() {
            *agent = header.sender().map(|s| s.to_string());
        }
        let _ = self.heard.send(heard("register", subject, object_path));
        Ok(())
    }

    async fn unregister_authentication_agent(
        &self,
        subject: Subject,
        object_path: String,
    ) -> zbus::fdo::Result<()> {
        let _ = self.heard.send(heard("unregister", subject, object_path));
        Ok(())
    }
}

/// The stand-in helper, built.
fn helper_program() -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let status = Command::new(cargo)
        .args([
            "build",
            "-q",
            "-p",
            "sill-services",
            "--example",
            "polkit_helper",
        ])
        .status()
        .unwrap_or_else(|e| panic!("building polkit_helper: {e}"));
    assert!(status.success(), "building polkit_helper");
    let target = std::env::var("CARGO_TARGET_DIR").map_or_else(
        |_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"),
        PathBuf::from,
    );
    target.join("debug/examples/polkit_helper")
}

async fn until(rx: &mut watch::Receiver<Polkit>, what: &str, ok: impl FnMut(&Polkit) -> bool) {
    let waited = tokio::time::timeout(Duration::from_secs(5), rx.wait_for(ok))
        .await
        .map(|r| r.map(|_| ()));
    match waited {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("{what}: {e}"),
        Err(_) => panic!("timed out waiting for {what}: {:?}", rx.borrow()),
    }
}

/// Wait for a value published after the last one seen that satisfies `ok`.
async fn next(rx: &mut watch::Receiver<Polkit>, what: &str, mut ok: impl FnMut(&Polkit) -> bool) {
    let wait = async {
        loop {
            if rx.changed().await.is_err() || ok(&rx.borrow_and_update()) {
                return;
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(5), wait)
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {what}: {:?}", rx.borrow()));
}

fn asking(polkit: &Polkit, last: LastTry) -> bool {
    polkit
        .current
        .as_ref()
        .is_some_and(|v| matches!(v.phase, PromptPhase::Asking { .. }) && v.last == last)
}

/// `BeginAuthentication` as polkitd sends it, answered with the method's result.
async fn begin(connection: Connection, agent: String, cookie: &str) -> Result<(), String> {
    let uid = Value::U32(
        std::fs::metadata("/proc/self").map_or(0, |m| std::os::unix::fs::MetadataExt::uid(&m)),
    )
    .try_to_owned()
    .unwrap_or_else(|e| panic!("{e}"));
    let identities: Vec<Subject> = vec![(
        "unix-user".to_owned(),
        HashMap::from([("uid".to_owned(), uid)]),
    )];
    let details = HashMap::from([("program".to_owned(), "/usr/bin/true".to_owned())]);
    let body = (
        "org.sill.test.run".to_owned(),
        "Authentication is required to run a test".to_owned(),
        String::new(),
        details,
        cookie.to_owned(),
        identities,
    );
    connection
        .call_method(
            Some(agent.as_str()),
            AGENT_PATH,
            Some("org.freedesktop.PolicyKit1.AuthenticationAgent"),
            "BeginAuthentication",
            &body,
        )
        .await
        .map(|_| ())
        .map_err(|e| match e {
            zbus::Error::MethodError(name, _, _) => name.to_string(),
            other => other.to_string(),
        })
}

fn answer(control: &sill_services::Control<PolkitCommand>, cookie: &str, text: &str) {
    control
        .send(PolkitCommand::Answer {
            cookie: Cookie(cookie.to_owned()),
            secret: Secret::new(text),
        })
        .unwrap_or_else(|e| panic!("answer: {e}"));
}

#[test]
#[ignore = "live: needs dbus-daemon"]
fn the_agent_registers_answers_fails_and_cancels_on_a_private_bus() {
    let helper = helper_program();
    let bus = PrivateBus::start();
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    rt.block_on(async {
        let (heard_tx, mut heard_rx) = mpsc::unbounded_channel();
        let agent = Arc::new(Mutex::new(None));
        let stand_in = StandIn {
            heard: heard_tx,
            agent: agent.clone(),
        };
        let authority = zbus::connection::Builder::address(bus.address.as_str())
            .and_then(|b| b.name("org.freedesktop.PolicyKit1"))
            .and_then(|b| b.serve_at("/org/freedesktop/PolicyKit1/Authority", stand_in))
            .unwrap_or_else(|e| panic!("{e}"))
            .build()
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        let setup = Setup {
            bus: Bus::system(BusRoute::Address(bus.address.clone())),
            helper: HelperChoice::Use(HelperRoute::Setuid(helper)),
            session: SessionChoice::Id("c42".to_owned()),
        };
        let (mut rx, control) = sill_services::polkit_agent::spawn(&rt_handle(), setup);

        // (1) Own registers for the session at AGENT_PATH.
        control
            .send(PolkitCommand::Policy(
                sill_services::polkit_agent::AgentPolicy::Own,
            ))
            .unwrap_or_else(|e| panic!("{e}"));
        until(&mut rx, "registered", |p| {
            p.registration == Registration::Registered
        })
        .await;
        let registered = heard_rx.recv().await.unwrap_or_else(|| panic!("heard"));
        let want = |verb| Heard {
            verb,
            kind: "unix-session".to_owned(),
            session: "c42".to_owned(),
            path: AGENT_PATH.to_owned(),
        };
        assert_eq!(registered, want("register"));
        let sender = agent
            .lock()
            .ok()
            .and_then(|a| a.clone())
            .unwrap_or_else(|| panic!("no agent sender"));
        println!("registered: {registered:?}");

        // (2) A request shows, and the stand-in's secret makes the call return.
        let call = tokio::spawn(begin(authority.clone(), sender.clone(), "k1"));
        until(&mut rx, "k1 asking", |p| asking(p, LastTry::None)).await;
        let shown = rx
            .borrow()
            .current
            .clone()
            .unwrap_or_else(|| panic!("shown"));
        assert_eq!(shown.action_id, ActionId("org.sill.test.run".to_owned()));
        assert_eq!(
            shown.details,
            vec![("program".to_owned(), "/usr/bin/true".to_owned())]
        );
        answer(&control, "k1", FAKE_SECRET);
        let result = call.await.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(result, Ok(()));
        until(&mut rx, "k1 gone", |p| p.current.is_none()).await;
        println!("k1: authorized");

        // (3) Three wrong answers fail it.
        let call = tokio::spawn(begin(authority.clone(), sender.clone(), "k2"));
        until(&mut rx, "k2 asking", |p| asking(p, LastTry::None)).await;
        for _ in 1..MAX_TRIES {
            rx.borrow_and_update();
            answer(&control, "k2", "wrong");
            next(&mut rx, "k2 asked again", |p| asking(p, LastTry::Wrong)).await;
        }
        answer(&control, "k2", "wrong");
        let result = call.await.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            result,
            Err("org.freedesktop.PolicyKit1.Error.Failed".to_owned())
        );
        println!("k2: failed after three tries");

        // (4) CancelAuthentication cancels it and clears the view.
        let call = tokio::spawn(begin(authority.clone(), sender.clone(), "k3"));
        until(&mut rx, "k3 asking", |p| asking(p, LastTry::None)).await;
        authority
            .call_method(
                Some(sender.as_str()),
                AGENT_PATH,
                Some("org.freedesktop.PolicyKit1.AuthenticationAgent"),
                "CancelAuthentication",
                &("k3",),
            )
            .await
            .unwrap_or_else(|e| panic!("cancel: {e}"));
        let result = call.await.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            result,
            Err("org.freedesktop.PolicyKit1.Error.Cancelled".to_owned())
        );
        until(&mut rx, "k3 gone", |p| p.current.is_none()).await;
        println!("k3: cancelled");

        // (5) Off unregisters.
        control
            .send(PolkitCommand::Policy(
                sill_services::polkit_agent::AgentPolicy::Off,
            ))
            .unwrap_or_else(|e| panic!("{e}"));
        until(&mut rx, "passive", |p| {
            p.registration == Registration::Passive
        })
        .await;
        let unregistered = heard_rx.recv().await.unwrap_or_else(|| panic!("heard"));
        assert_eq!(unregistered, want("unregister"));
        println!("unregistered: {unregistered:?}");
    });
}

fn rt_handle() -> tokio::runtime::Handle {
    tokio::runtime::Handle::current()
}

#[test]
fn a_made_up_request_takes_the_fake_secret_and_nothing_else() {
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    rt.block_on(async {
        let (mut rx, control) = sill_services::polkit_agent::spawn(
            &rt_handle(),
            Setup {
                bus: Bus::system(BusRoute::Address(
                    "unix:path=/nonexistent/sill-test".to_owned(),
                )),
                helper: HelperChoice::Use(HelperRoute::Missing),
                session: SessionChoice::Id("never".to_owned()),
            },
        );
        control
            .send(PolkitCommand::Fake(FakeRequest {
                action_id: ActionId("org.sill.debug".to_owned()),
                message: "Made up".to_owned(),
                icon: String::new(),
            }))
            .unwrap_or_else(|e| panic!("{e}"));
        until(&mut rx, "asking", |p| asking(p, LastTry::None)).await;
        assert_eq!(
            rx.borrow().registration,
            Registration::Passive,
            "no bus touched"
        );
        let cookie = rx
            .borrow()
            .current
            .as_ref()
            .map(|v| v.cookie.0.clone())
            .unwrap_or_else(|| panic!("shown"));
        rx.borrow_and_update();
        answer(&control, &cookie, "not it");
        next(&mut rx, "checking", |p| {
            p.current
                .as_ref()
                .is_some_and(|v| v.phase == PromptPhase::Checking)
        })
        .await;
        next(&mut rx, "wrong", |p| asking(p, LastTry::Wrong)).await;
        answer(&control, &cookie, FAKE_SECRET);
        until(&mut rx, "gone", |p| p.current.is_none()).await;
    });
}
