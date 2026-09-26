//! A stand-in for polkitd's Authority on a private bus (FINDINGS "M11 freeze" F547): it owns
//! `org.freedesktop.PolicyKit1` there, records the agent that registers, and on command calls
//! that agent's `BeginAuthentication` and `CancelAuthentication`, printing what happens. It is
//! how `dev/accept-polkit.sh` plays polkitd without polkitd.
//!
//! The address is an argument, never `DBUS_SYSTEM_BUS_ADDRESS`, so this cannot reach the real
//! system bus (where polkitd holds the name) by accident.
//!
//! Commands, one per stdin line: `begin <cookie> [action-id]`, `cancel <cookie>`. Output, one
//! per stdout line: `ready`, `registered <kind> session-id=<id> path=<path>`, `unregistered
//! <kind> session-id=<id> path=<path>`, `result <cookie> authorized|cancelled|failed`,
//! `cancel-sent <cookie>`, `no-agent <cookie>`.
//!
//! Usage: `cargo run -p sill-services --example polkit_authority -- <bus address>`

use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use zbus::message::Header;
use zbus::zvariant::{OwnedValue, Value};
use zbus::{Connection, interface};

const NAME: &str = "org.freedesktop.PolicyKit1";
const PATH: &str = "/org/freedesktop/PolicyKit1/Authority";
const AGENT_INTERFACE: &str = "org.freedesktop.PolicyKit1.AuthenticationAgent";

type Subject = (String, HashMap<String, OwnedValue>);

/// The registered agent: its bus name and object path.
#[derive(Debug, Clone)]
struct AgentAt {
    sender: String,
    path: String,
}

#[derive(Debug, Clone, Default)]
struct Authority {
    agent: Arc<Mutex<Option<AgentAt>>>,
}

#[interface(name = "org.freedesktop.PolicyKit1.Authority")]
impl Authority {
    async fn register_authentication_agent(
        &self,
        #[zbus(header)] header: Header<'_>,
        subject: Subject,
        _locale: String,
        object_path: String,
    ) -> zbus::fdo::Result<()> {
        let sender = header.sender().map(|s| s.to_string()).unwrap_or_default();
        say(&format!(
            "registered {}",
            subject_text(&subject, &object_path)
        ));
        if let Ok(mut agent) = self.agent.lock() {
            *agent = Some(AgentAt {
                sender,
                path: object_path,
            });
        }
        Ok(())
    }

    async fn unregister_authentication_agent(
        &self,
        subject: Subject,
        object_path: String,
    ) -> zbus::fdo::Result<()> {
        say(&format!(
            "unregistered {}",
            subject_text(&subject, &object_path)
        ));
        if let Ok(mut agent) = self.agent.lock() {
            *agent = None;
        }
        Ok(())
    }
}

fn subject_text((kind, details): &Subject, path: &str) -> String {
    let id = details
        .get("session-id")
        .and_then(|v| <&str>::try_from(v).ok())
        .unwrap_or("?");
    format!("{kind} session-id={id} path={path}")
}

fn say(line: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

fn main() {
    let Some(address) = std::env::args().nth(1) else {
        eprintln!("usage: polkit_authority <private bus address>");
        std::process::exit(2);
    };
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    runtime.block_on(serve(address));
}

async fn serve(address: String) {
    let authority = Authority::default();
    let connection = zbus::connection::Builder::address(address.as_str())
        .and_then(|b| b.name(NAME))
        .and_then(|b| b.serve_at(PATH, authority.clone()))
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("connecting to {address}: {e}"));
    say("ready");
    let mut lines = stdin_lines();
    while let Some(line) = lines.recv().await {
        let words: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
        let agent = authority.agent.lock().ok().and_then(|a| a.clone());
        match (words.first().map(String::as_str), words.get(1), agent) {
            (Some(_), Some(cookie), None) => say(&format!("no-agent {cookie}")),
            (Some("begin"), Some(cookie), Some(agent)) => {
                let action = words.get(2).cloned();
                tokio::spawn(begin(connection.clone(), agent, cookie.clone(), action));
            }
            (Some("cancel"), Some(cookie), Some(agent)) => {
                cancel(&connection, &agent, cookie).await;
            }
            _ => eprintln!("polkit_authority: unknown command {line:?}"),
        }
    }
}

/// stdin's lines, read on a thread of their own.
fn stdin_lines() -> mpsc::UnboundedReceiver<String> {
    let (tx, rx) = mpsc::unbounded_channel();
    std::thread::spawn(move || {
        for line in std::io::stdin().lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                return;
            }
        }
    });
    rx
}

/// `BeginAuthentication` for this user, as polkitd sends it for `pkexec /usr/bin/true`.
async fn begin(connection: Connection, agent: AgentAt, cookie: String, action: Option<String>) {
    let uid =
        std::fs::metadata("/proc/self").map_or(0, |m| std::os::unix::fs::MetadataExt::uid(&m));
    let uid = Value::U32(uid)
        .try_to_owned()
        .unwrap_or_else(|e| panic!("{e}"));
    let identities: Vec<Subject> = vec![(
        "unix-user".to_owned(),
        HashMap::from([("uid".to_owned(), uid)]),
    )];
    let details = HashMap::from([
        ("program".to_owned(), "/usr/bin/true".to_owned()),
        ("polkit.subject-pid".to_owned(), "4242".to_owned()),
    ]);
    let body = (
        action.unwrap_or_else(|| "org.sill.test.run".to_owned()),
        "Authentication is required to run the sill test program".to_owned(),
        String::new(),
        details,
        cookie.clone(),
        identities,
    );
    let answer = connection
        .call_method(
            Some(agent.sender.as_str()),
            agent.path.as_str(),
            Some(AGENT_INTERFACE),
            "BeginAuthentication",
            &body,
        )
        .await;
    let word = match answer {
        Ok(_) => "authorized".to_owned(),
        Err(zbus::Error::MethodError(name, _, _)) if name.ends_with(".Cancelled") => {
            "cancelled".to_owned()
        }
        Err(zbus::Error::MethodError(name, _, _)) => format!("failed {name}"),
        Err(other) => format!("failed {other}"),
    };
    say(&format!("result {cookie} {word}"));
}

async fn cancel(connection: &Connection, agent: &AgentAt, cookie: &str) {
    let sent = connection
        .call_method(
            Some(agent.sender.as_str()),
            agent.path.as_str(),
            Some(AGENT_INTERFACE),
            "CancelAuthentication",
            &(cookie,),
        )
        .await;
    match sent {
        Ok(_) => say(&format!("cancel-sent {cookie}")),
        Err(error) => say(&format!("cancel-failed {cookie} {error}")),
    }
}
