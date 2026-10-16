//! Proving who owns a menu's bus name. Owning `org.quire.AppMenu.<id>` proves nothing: any
//! process may claim it. The system's account of the owner (peephole's standing for the unique
//! name) names its app, and the name counts only when it is the menu name quire derives for that
//! app.

use docket_client::Transport;
use peephole_bus::peephole_core::Standing;
use sill_model::app_menu::{Identity, MenuApp, MenuName};
use zbus::fdo::DBusProxy;
use zbus::names::BusName;

use super::MenuAgent;
use super::read::Served;
use crate::app_menu::MenuNameOf;
use std::path::Path;

impl<T: Transport> MenuAgent<T> {
    /// Whom `served`'s owner is proven to be.
    pub(super) async fn identity(&self, served: &Served) -> Identity {
        let app = match self.peers.who(served.owner.as_str()).await {
            Ok(standing) => app_of(&standing),
            Err(_) => None,
        };
        match app {
            Some(app) => verified(&app, &served.menu.name, self.links.name_of),
            None => match self.pid_of(served).await {
                Some(pid) => unknown(&self.proc_root, pid),
                None => Identity::Unverified,
            },
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

/// The app a standing names: a table row, a sandbox, or the scope a root-owned program was
/// started in. A user-writable image names nobody.
fn app_of(standing: &Standing) -> Option<String> {
    match standing {
        Standing::Service { app, .. } | Standing::Sandboxed { app, .. } => {
            Some(app.as_str().to_owned())
        }
        Standing::Program {
            scope_app: Some(app),
            ..
        } => Some(app.as_str().to_owned()),
        _ => None,
    }
}

/// Whether `app` is the app whose menu name is `claimed`.
fn verified(app: &str, claimed: &MenuName, name_of: MenuNameOf) -> Identity {
    let Ok(name) = porter_core::AppName::parse(app) else {
        return Identity::Unverified;
    };
    match name_of(&MenuApp(app.to_owned())) {
        Some(menu) if menu == *claimed => Identity::Verified(name),
        Some(_) | None => Identity::Unverified,
    }
}

/// Whom process `pid` of `proc_root` is, when its standing names no app. When its scope says it
/// is a Flatpak app and the metadata cannot be read (this process is Landlock-confined), the
/// owner is [`Identity::Unreadable`] and the reason is logged, never silently taken for a native
/// app.
fn unknown(proc_root: &Path, pid: u32) -> Identity {
    match flatpak_scope(proc_root, pid) {
        FlatpakScope::Unreadable => {
            eprintln!(
                "sill: app menu: process {pid} is a Flatpak app whose /proc/{pid}/root/.flatpak-info \
                 cannot be read from here; it is shown as an unknown app"
            );
            Identity::Unreadable
        }
        FlatpakScope::NotOne => Identity::Unverified,
    }
}

/// What a process's cgroup says about Flatpak.
#[derive(Debug, PartialEq, Eq)]
enum FlatpakScope {
    /// Flatpak's own scope (`app-flatpak-<id>-<n>.scope`) around a process whose metadata is not
    /// readable.
    Unreadable,
    /// Anything else.
    NotOne,
}

fn flatpak_scope(proc_root: &Path, pid: u32) -> FlatpakScope {
    let dir = proc_root.join(pid.to_string());
    let in_flatpak_scope = std::fs::read_to_string(dir.join("cgroup")).is_ok_and(|text| {
        text.lines()
            .filter_map(|line| line.rsplit('/').next())
            .any(|leaf| leaf.starts_with("app-flatpak-") && leaf.ends_with(".scope"))
    });
    if in_flatpak_scope && std::fs::read_to_string(dir.join("root/.flatpak-info")).is_err() {
        FlatpakScope::Unreadable
    } else {
        FlatpakScope::NotOne
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use sill_model::app_menu::{Identity, MenuApp, MenuName};
    use sill_testkit::TempDir;

    use super::{unknown, verified};

    fn name_of(app: &MenuApp) -> Option<MenuName> {
        Some(MenuName(format!("org.quire.AppMenu.{}", app.0)))
    }

    /// A `/proc` with process 7 in `leaf`'s scope, and its Flatpak metadata when `info` is given.
    fn proc_with(root: &Path, leaf: &str, info: Option<&str>) {
        let dir = root.join("7");
        std::fs::create_dir_all(dir.join("root")).unwrap_or_else(|e| panic!("{e}"));
        let scope = format!("0::/user.slice/user@1000.service/app.slice/{leaf}\n");
        std::fs::write(dir.join("cgroup"), scope).unwrap_or_else(|e| panic!("{e}"));
        if let Some(info) = info {
            std::fs::write(dir.join("root/.flatpak-info"), info).unwrap_or_else(|e| panic!("{e}"));
        }
    }

    #[test]
    fn an_app_counts_only_under_its_own_menu_name() {
        let claimed = MenuName("org.quire.AppMenu.dev.notes".to_owned());
        let notes = porter_core::AppName::parse("dev.notes").unwrap_or_else(|e| panic!("{e}"));
        let cases = [
            ("its own name", "dev.notes", Identity::Verified(notes)),
            ("another app's name", "dev.other", Identity::Unverified),
            ("not an app name", "", Identity::Unverified),
        ];
        for (name, app, want) in cases {
            assert_eq!(verified(app, &claimed, name_of), want, "{name}");
        }
    }

    #[test]
    fn an_unreadable_flatpak_app_is_an_unknown_app_and_anything_else_is_unverified() {
        let cases = [
            (
                "metadata unreadable",
                "app-flatpak-dev.notes-4242.scope",
                Identity::Unreadable,
            ),
            (
                "a native scope is not a Flatpak app",
                "app-dev.other-4242.scope",
                Identity::Unverified,
            ),
        ];
        for (name, leaf, want) in cases {
            let scratch = TempDir::new("app-menu-identity");
            proc_with(scratch.path(), leaf, None);
            assert_eq!(unknown(scratch.path(), 7), want, "{name}");
        }
    }
}
