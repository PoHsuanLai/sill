//! Proving who owns a menu's bus name. Owning `org.quire.AppMenu.<id>` proves nothing: any
//! process may claim it. Porter's caller identity (the owner's cgroup scope, read through the
//! pid the bus reports) names the app that launched the process, and the name counts only when
//! it is the menu name quire derives for that app.

use docket_client::Transport;
use porter_dbus::{CallerTable, ProcCallers};
use sill_model::app_menu::{Identity, MenuApp};
use zbus::fdo::DBusProxy;
use zbus::names::BusName;

use super::MenuAgent;
use super::read::Served;

impl<T: Transport> MenuAgent<T> {
    /// Whom `served`'s owner is proven to be.
    pub(super) async fn identity(&self, served: &Served) -> Identity {
        let Some(pid) = self.pid_of(served).await else {
            return Identity::Unverified;
        };
        let Some(caller) =
            ProcCallers::caller_of_pid(&self.proc_root, pid, &CallerTable::default())
        else {
            return Identity::Unverified;
        };
        let app = caller.app.name;
        let derived = (self.links.name_of)(&MenuApp(app.as_str().to_owned()));
        match derived {
            Some(name) if name == served.menu.name => Identity::Verified(app),
            Some(_) | None => Identity::Unverified,
        }
    }

    async fn pid_of(&self, served: &Served) -> Option<u32> {
        let dbus = DBusProxy::new(&self.connection).await.ok()?;
        let owner = BusName::from(served.owner.inner().clone());
        dbus.get_connection_credentials(owner)
            .await
            .ok()?
            .process_id()
    }
}
