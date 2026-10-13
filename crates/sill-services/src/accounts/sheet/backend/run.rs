//! The loop: passive until `Serve`, then the calls from accountd and the person's actions, each a
//! [`SheetEvent`] fed to the pure step; what the step asks (an `Input` signal) is carried out here.

use std::collections::HashMap;

use porter_core::sheet::SheetInput;
use porter_dbus::SHEET_PATH;
use sill_launcher::OpenTarget;
use sill_model::accounts::sheet::{
    AccountsSheetCommand, AccountsSheets, Admit, SheetEvent, SheetHandle, SheetLink, SheetOut,
    admits, step,
};
use sill_model::launch::{LaunchCommand, OpenRequest};
use tokio::sync::{mpsc, watch};
use zbus::Connection;
use zbus::fdo;

use super::callers::{Caller, Host};
use super::objects::Call;
use super::serve;
use crate::accounts::sheet::{AccountsSheetLinks, AccountsSheetService};
use crate::control::Control;
use crate::env::Bus;
use crate::owner;
use crate::service::ServiceIo;

pub(crate) async fn run(bus: Bus, links: AccountsSheetLinks, io: ServiceIo<AccountsSheetService>) {
    let ServiceIo {
        state: tx,
        mut commands,
    } = io;
    loop {
        match commands.recv().await {
            Some(AccountsSheetCommand::Serve) => break,
            // Nothing is answered before the daemon says to serve.
            Some(AccountsSheetCommand::Act(..)) => {}
            None => return,
        }
    }
    let (calls_tx, mut calls) = mpsc::unbounded_channel();
    let connection = match serve::take(&bus, calls_tx).await {
        Ok(connection) => connection,
        Err(trouble) => {
            tx.send_modify(|state| state.link = SheetLink::Unavailable(trouble));
            // Held until the service is dropped, so its handle keeps answering sends.
            while commands.recv().await.is_some() {}
            return;
        }
    };
    let (left_tx, mut left) = mpsc::unbounded_channel();
    watch_services(&connection, &left_tx);
    let mut live = Live {
        sheets: AccountsSheets::default(),
        tx,
        connection,
        launch: links.launch,
        openers: HashMap::new(),
    };
    live.feed(SheetEvent::Link(SheetLink::Serving));
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(AccountsSheetCommand::Act(handle, action)) => {
                    live.feed(SheetEvent::Acted(handle, action));
                }
                Some(AccountsSheetCommand::Serve) => {}
                None => break,
            },
            Some(call) = calls.recv() => live.call(call),
            Some(host) = left.recv() => live.drop_sheets_of(host),
        }
    }
    live.feed(SheetEvent::Link(SheetLink::Passive));
}

struct Live {
    sheets: AccountsSheets,
    tx: watch::Sender<AccountsSheets>,
    connection: Connection,
    launch: Control<LaunchCommand>,
    /// Who opened each handle: the connection `Input` goes to, and the only one that may change it.
    openers: HashMap<SheetHandle, Caller>,
}

impl Live {
    /// A service left the bus or was replaced: its sheets come down, the others stay.
    fn drop_sheets_of(&mut self, host: Host) {
        let gone: Vec<SheetHandle> = self
            .openers
            .iter()
            .filter(|(_, opener)| opener.host == host)
            .map(|(handle, _)| handle.clone())
            .collect();
        for handle in gone {
            self.openers.remove(&handle);
            self.feed(SheetEvent::Closed(handle));
        }
    }

    fn feed(&mut self, event: SheetEvent) {
        let (next, out) = step(std::mem::take(&mut self.sheets), event);
        self.tx.send_if_modified(|published| {
            let changed = *published != next;
            if changed {
                published.clone_from(&next);
            }
            changed
        });
        self.sheets = next;
        for effect in out {
            self.carry_out(effect);
        }
    }

    fn carry_out(&self, effect: SheetOut) {
        let (handle, input) = match effect {
            SheetOut::Reply(handle, input) => (handle, input),
            SheetOut::OpenPage(url) => {
                // A stopped launch service has nobody to open it.
                let _ = self.launch.send(LaunchCommand::Open(OpenRequest {
                    target: OpenTarget::Url(url.as_str().to_owned()),
                    token: None,
                }));
                return;
            }
        };
        let Some(opener) = self.openers.get(&handle).cloned() else {
            return;
        };
        let connection = self.connection.clone();
        tokio::spawn(async move {
            // Never logged: the text may hold a password. A failure says only that it failed.
            let Some(json) = encode(&input) else { return };
            let sent = connection
                .emit_signal(
                    Some(opener.name.as_str()),
                    SHEET_PATH,
                    "org.quire.AccountsSheet1",
                    "Input",
                    &(handle.as_str(), json.as_str()),
                )
                .await;
            if sent.is_err() {
                eprintln!("sill: accounts sheet: telling the service what the person did failed");
            }
        });
    }

    fn call(&mut self, call: Call) {
        match call {
            Call::Open {
                handle,
                view,
                opener,
                answer,
            } => {
                let verdict = match admits(&self.sheets, &handle) {
                    Admit::Yes => Ok(()),
                    Admit::Duplicate => Err(fdo::Error::InvalidArgs("handle in use".to_owned())),
                    Admit::Full => Err(fdo::Error::LimitsExceeded("too many sheets".to_owned())),
                    Admit::Unknown => Err(fdo::Error::UnknownObject("no such sheet".to_owned())),
                };
                if verdict.is_ok() {
                    self.openers.insert(handle.clone(), opener);
                    self.feed(SheetEvent::Opened(handle, *view));
                }
                let _ = answer.send(verdict);
            }
            Call::Update {
                handle,
                view,
                from,
                answer,
            } => {
                let verdict = self.owned(&handle, &from);
                if verdict.is_ok() {
                    self.feed(SheetEvent::Updated(handle, *view));
                }
                let _ = answer.send(verdict);
            }
            Call::Close {
                handle,
                from,
                answer,
            } => {
                let verdict = self.owned(&handle, &from);
                if verdict.is_ok() {
                    self.feed(SheetEvent::Closed(handle.clone()));
                    self.openers.remove(&handle);
                }
                let _ = answer.send(verdict);
            }
        }
    }

    /// Whether `from` opened `handle`.
    fn owned(&self, handle: &SheetHandle, from: &Caller) -> fdo::Result<()> {
        match self.openers.get(handle) {
            Some(opener) if opener.name == from.name => Ok(()),
            Some(_) => Err(fdo::Error::AccessDenied(
                "only the connection that opened a sheet drives it".to_owned(),
            )),
            None => Err(fdo::Error::UnknownObject("no such sheet".to_owned())),
        }
    }
}

fn encode(input: &SheetInput) -> Option<String> {
    serde_json::to_string(input).ok()
}

/// Tell the loop each time one of the services leaves or is replaced.
fn watch_services(connection: &Connection, left: &mpsc::UnboundedSender<Host>) {
    for host in Host::ALL {
        let connection = connection.clone();
        let left = left.clone();
        tokio::spawn(async move {
            loop {
                owner::appeared(&connection, host.bus()).await;
                owner::changed(&connection, host.bus()).await;
                if left.send(host).is_err() {
                    return;
                }
            }
        });
    }
}
