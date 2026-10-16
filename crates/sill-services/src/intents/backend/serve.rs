//! Taking the bus: one connection, the names in the order that keeps intentd's trust check true.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use docket_dbus::callers::{Identity, intentd_only};
use docket_dbus::{CONFIRM_BUS, CONFIRM_PATH};
use sill_model::intents::LinkTrouble;
use sill_model::screenshots::Shots;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::watch;
use zbus::Connection;
use zbus::fdo::RequestNameFlags;

use super::objects::{Call, ConfirmObject};
use super::provider::{NoSummon, ShellContext, ShellProvider, manifest};
use crate::app_menu::{MenuAgent, MenuLinks};
use crate::callers::CallerSource;
use crate::env::Bus;
use crate::intents::ShellCall;

/// The connection that serves the provider and the sheet.
pub(super) struct Seat {
    pub connection: Connection,
}

/// Connect to `bus` and claim `org.quire.Shell` (the provider; it is what gives the connection
/// its roles at intentd), then `org.quire.Confirm1`: intentd trusts the owner of the second only
/// while it also owns the first, so the sheet is never reachable before it is trusted.
pub(super) async fn take(
    bus: &Bus,
    callers: &dyn CallerSource,
    calls: UnboundedSender<Call>,
    shell: UnboundedSender<ShellCall>,
    menus: MenuLinks,
    shots: watch::Receiver<Shots>,
    proc_root: PathBuf,
) -> Result<Seat, LinkTrouble> {
    let connection = match bus.connect().await {
        Ok(Some(connection)) => connection,
        Ok(None) => return Err(LinkTrouble::NoBus),
        Err(error) => return Err(LinkTrouble::Refused(error.to_string())),
    };
    let manifest = manifest().map_err(LinkTrouble::Refused)?;
    let app = manifest.manifest().app.clone();
    let intentd = Arc::new(intentd_at(&connection, &proc_root));
    let peers = callers.peers_on(connection.clone());
    let provider = ShellProvider {
        manifest,
        calls: shell,
        menus: MenuAgent::over(connection.clone(), menus.clone(), proc_root, peers.clone()),
        windows: menus,
        shots,
    };
    docket_client::serve_on_with(
        &connection,
        provider,
        ShellContext { app },
        NoSummon,
        intentd,
    )
    .await
    .map_err(|error| trouble(error.to_string()))?;
    connection
        .object_server()
        .at(
            CONFIRM_PATH,
            ConfirmObject {
                calls,
                serials: Arc::new(AtomicU64::new(0)),
                peers,
            },
        )
        .await
        .map_err(|error| LinkTrouble::Refused(error.to_string()))?;
    let reply = connection
        .request_name_with_flags(CONFIRM_BUS, RequestNameFlags::DoNotQueue.into())
        .await
        .map_err(|error| trouble(error.to_string()))?;
    match reply {
        zbus::fdo::RequestNameReply::PrimaryOwner | zbus::fdo::RequestNameReply::AlreadyOwner => {
            Ok(Seat { connection })
        }
        zbus::fdo::RequestNameReply::InQueue | zbus::fdo::RequestNameReply::Exists => {
            Err(LinkTrouble::NameTaken(format!("{CONFIRM_BUS} is taken")))
        }
    }
}

/// Who may call the provider: intentd's unit only. Under the system's `/proc` its main process is
/// asked of the systemd manager; any other root is a fixture tree (`<root>/<pid>/cgroup`, and
/// `<root>/units/intentd.service` for the main process), which is how a test names intentd.
fn intentd_at(connection: &Connection, proc_root: &Path) -> Identity {
    let fixture = (proc_root != Path::new("/proc")).then_some(proc_root);
    Identity::on(connection.clone(), intentd_only(), fixture)
}

/// A name that is taken says so in its text (docket's "is taken", zbus's "already taken");
/// anything else is the bus refusing.
fn trouble(text: String) -> LinkTrouble {
    if text.contains("taken") {
        LinkTrouble::NameTaken(text)
    } else {
        LinkTrouble::Refused(text)
    }
}
