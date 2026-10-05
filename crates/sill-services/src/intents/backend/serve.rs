//! Taking the bus: one connection, the names in the order that keeps intentd's trust check true.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use docket_dbus::{CONFIRM_BUS, CONFIRM_PATH};
use sill_model::intents::LinkTrouble;
use tokio::sync::mpsc::UnboundedSender;
use zbus::Connection;
use zbus::fdo::RequestNameFlags;

use super::objects::{Call, ConfirmObject};
use super::provider::{NoSummon, ShellContext, ShellProvider, manifest};
use crate::app_menu::{MenuAgent, MenuLinks};
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
    calls: UnboundedSender<Call>,
    shell: UnboundedSender<ShellCall>,
    menus: MenuLinks,
    proc_root: PathBuf,
) -> Result<Seat, LinkTrouble> {
    let connection = match bus.connect().await {
        Ok(Some(connection)) => connection,
        Ok(None) => return Err(LinkTrouble::NoBus),
        Err(error) => return Err(LinkTrouble::Refused(error.to_string())),
    };
    let manifest = manifest().map_err(LinkTrouble::Refused)?;
    let app = manifest.manifest().app.clone();
    let provider = ShellProvider {
        manifest,
        calls: shell,
        menus: MenuAgent::over(connection.clone(), menus, proc_root),
    };
    docket_client::serve_on(&connection, provider, ShellContext { app }, NoSummon)
        .await
        .map_err(|error| trouble(error.to_string()))?;
    connection
        .object_server()
        .at(
            CONFIRM_PATH,
            ConfirmObject {
                calls,
                serials: Arc::new(AtomicU64::new(0)),
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

/// A name that is taken says so in its text (docket's "is taken", zbus's "already taken");
/// anything else is the bus refusing.
fn trouble(text: String) -> LinkTrouble {
    if text.contains("taken") {
        LinkTrouble::NameTaken(text)
    } else {
        LinkTrouble::Refused(text)
    }
}
