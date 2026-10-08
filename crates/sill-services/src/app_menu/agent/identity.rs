//! Proving who owns a menu's bus name. Owning `org.quire.AppMenu.<id>` proves nothing: any
//! process may claim it. Porter's caller identity (the owner's cgroup scope, read through the
//! pid the bus reports) names the app that launched the process, and the name counts only when
//! it is the menu name quire derives for that app.

use docket_client::Transport;
use porter_dbus::{CallerTable, ProcCallers};
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
        let Some(pid) = self.pid_of(served).await else {
            return Identity::Unverified;
        };
        proven(&self.proc_root, pid, &served.menu.name, self.links.name_of)
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

/// Whom process `pid` of `proc_root` is proven to be, given the menu name `claimed` its owner
/// holds. A Flatpak app's name comes from its sandbox's metadata; when its scope says it is a
/// Flatpak app and the metadata cannot be read (this process is Landlock-confined), the owner is
/// [`Identity::Unreadable`] and the reason is logged, never silently taken for a native app.
fn proven(proc_root: &Path, pid: u32, claimed: &MenuName, name_of: MenuNameOf) -> Identity {
    let Some(caller) = ProcCallers::caller_of_pid(proc_root, pid, &CallerTable::default()) else {
        return match flatpak_scope(proc_root, pid) {
            FlatpakScope::Unreadable => {
                eprintln!(
                    "sill: app menu: process {pid} is a Flatpak app whose /proc/{pid}/root/.flatpak-info \
                     cannot be read from here; it is shown as an unknown app"
                );
                Identity::Unreadable
            }
            FlatpakScope::NotOne => Identity::Unverified,
        };
    };
    let app = caller.app.name;
    match name_of(&MenuApp(app.as_str().to_owned())) {
        Some(name) if name == *claimed => Identity::Verified(app),
        Some(_) | None => Identity::Unverified,
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

    use super::proven;

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
    fn a_flatpak_app_is_known_by_its_metadata_and_an_unreadable_one_is_an_unknown_app() {
        const INFO: &str = "[Application]\nname=dev.notes\n[Instance]\ninstance-id=1\n";
        let claimed = MenuName("org.quire.AppMenu.dev.notes".to_owned());
        let flatpak = "app-flatpak-dev.notes-4242.scope";
        let cases = [
            (
                "metadata readable",
                flatpak,
                Some(INFO),
                Identity::Verified(
                    porter_core::AppName::parse("dev.notes").unwrap_or_else(|e| panic!("{e}")),
                ),
            ),
            ("metadata unreadable", flatpak, None, Identity::Unreadable),
            (
                "a native scope is not a Flatpak app",
                "app-dev.other-4242.scope",
                None,
                Identity::Unverified,
            ),
        ];
        for (name, leaf, info, want) in cases {
            let scratch = TempDir::new("app-menu-identity");
            proc_with(scratch.path(), leaf, info);
            assert_eq!(proven(scratch.path(), 7, &claimed, name_of), want, "{name}");
        }
    }
}
