//! Taking the bus: one connection, the names in the order that keeps intentd's trust check true.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use docket_dbus::callers::{Identity, intentd_only};
use docket_dbus::{CONFIRM_BUS, CONFIRM_PATH};
use docket_provider::ServeError;
use sill_model::screenshots::Shots;
use sill_model::serve::ClaimError;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::watch;
use zbus::Connection;

use super::objects::{Call, ConfirmObject};
use super::provider::{NoSummon, ShellContext, ShellProvider, manifest};
use crate::app_menu::{MenuAgent, MenuLinks};
use crate::callers::CallerSource;
use crate::env::Bus;
use crate::intents::ShellCall;
use crate::serve;

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
) -> Result<Seat, ClaimError> {
    let claimed = serve::claim(bus, CONFIRM_BUS, async move |connection| {
        let manifest = manifest().map_err(serve::refused)?;
        let app = manifest.manifest().app.clone();
        let intentd = Arc::new(intentd_at(connection, &proc_root));
        let peers = callers.peers_on(connection.clone());
        let provider = ShellProvider {
            manifest,
            calls: shell,
            menus: MenuAgent::over(connection.clone(), menus.clone(), proc_root, peers.clone()),
            windows: menus,
            shots,
        };
        docket_provider::serve_on_with(
            connection,
            provider,
            ShellContext { app },
            NoSummon,
            intentd,
        )
        .await
        .map_err(docket_trouble)?;
        let confirm = ConfirmObject {
            calls,
            serials: Arc::new(AtomicU64::new(0)),
            peers,
        };
        connection
            .object_server()
            .at(CONFIRM_PATH, confirm)
            .await
            .map(|_| ())
            .map_err(serve::refused)
    })
    .await;
    claimed.map(|connection| Seat { connection })
}

/// Who may call the provider: intentd's unit only. Under the system's `/proc` its main process is
/// asked of the systemd manager; any other root is a fixture tree (`<root>/<pid>/cgroup`, and
/// `<root>/units/intentd.service` for the main process), which is how a test names intentd.
fn intentd_at(connection: &Connection, proc_root: &Path) -> Identity {
    let fixture = (proc_root != Path::new("/proc")).then_some(proc_root);
    Identity::on(connection.clone(), intentd_only(), fixture)
}

/// docket's `serve_on_with` says a taken name with a type, as everything sill claims itself does
/// ([`serve::take_name`]); a bus that refused the name is taken too.
fn docket_trouble(error: ServeError) -> ClaimError {
    match error {
        ServeError::Taken(name) => ClaimError::NameTaken(name),
        ServeError::Bus(zbus::Error::NameTaken) => ClaimError::NameTaken(CONFIRM_BUS.to_owned()),
        other => ClaimError::Refused(other.to_string()),
    }
}
