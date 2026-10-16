//! Whether the owner of an app's menu name is the app: the system's account of the owner,
//! compared with the program of the focused window.

use peephole_bus::peephole_core::Standing;
use sill_model::app_menu::{MenuClient, MenuOwner, MenuTrust, menu_trust};
use sill_model::untrusted::for_log;
use zbus::names::OwnedUniqueName;

use super::Peers;

/// Whether the menu served by `owner` (the unique name that owns the app's menu name) is the
/// menu of `client`'s program. An owner that cannot be identified is not shown.
pub(crate) async fn menu_shown(
    peers: &Peers,
    owner: &OwnedUniqueName,
    client: &MenuClient,
) -> MenuTrust {
    let who = match peers.who(owner.as_str()).await {
        Ok(standing) => owner_of(&standing),
        Err(_) => MenuOwner::Unknown,
    };
    let trust = menu_trust(&who, client);
    if trust == MenuTrust::Hidden {
        eprintln!(
            "sill: callers: the owner of the menu name of {} is not that app, so its menu is not shown",
            for_log(&client.key)
        );
    }
    trust
}

/// What a standing says of a menu's owner.
fn owner_of(standing: &Standing) -> MenuOwner {
    match standing {
        Standing::Service { app, .. } => MenuOwner::Service {
            app: app.as_str().to_owned(),
        },
        Standing::Sandboxed { app, .. } => MenuOwner::Sandboxed {
            app: app.as_str().to_owned(),
        },
        Standing::Program { exe, scope_app } => MenuOwner::Program {
            exe: exe.as_path().to_string_lossy().into_owned(),
            scope_app: scope_app.as_ref().map(|app| app.as_str().to_owned()),
        },
        // A user-writable image's scope name stays a claim of no weight: nothing about it is
        // remembered.
        _ => MenuOwner::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use peephole_bus::peephole::fake::{
        introduced_program, introduced_service, introduced_unverified,
    };
    use peephole_bus::peephole_core::{AppKey, Engine, Pin, Proof, Standing, WhyNot};
    use sill_model::app_menu::MenuOwner;

    use super::owner_of;

    #[test]
    fn a_standing_is_the_owner_it_names_and_an_unverified_process_is_nobody() {
        let service = introduced_service(
            "org.quire.Settings",
            "settings",
            Proof::Full,
            Pin::AtConnect,
        )
        .unwrap_or_else(|| panic!("a standing")); // test-only
        let program = introduced_program("/usr/bin/notes").unwrap_or_else(|| panic!("a standing")); // test-only
        let sandboxed = Standing::Sandboxed {
            app: AppKey::new("dev.notes").unwrap_or_else(|e| panic!("{e}")), // test-only
            engine: Engine::Flatpak,
        };
        let cases = [
            (
                "a desktop service",
                service,
                MenuOwner::Service {
                    app: "org.quire.Settings".to_owned(),
                },
            ),
            (
                "a program the system owns",
                program,
                MenuOwner::Program {
                    exe: "/usr/bin/notes".to_owned(),
                    scope_app: None,
                },
            ),
            (
                "a sandbox",
                sandboxed,
                MenuOwner::Sandboxed {
                    app: "dev.notes".to_owned(),
                },
            ),
            (
                "an image the person can write",
                introduced_unverified(WhyNot::ImageWritable),
                MenuOwner::Unknown,
            ),
        ];
        for (name, standing, want) in cases {
            assert_eq!(owner_of(&standing), want, "{name}");
        }
    }
}
