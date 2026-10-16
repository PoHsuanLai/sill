//! Who may call the shell's bus objects (SILLS-07), on a private bus. Owning the name of a service
//! proves nothing; the system's account of the caller does. Each object answers one role, and the
//! caller must also own the name of the service whose role it holds:
//!
//! - `org.quire.Shell1` (the picker, the colour, `AskPermission`): the portal backend;
//! - `org.quire.AccountsSheet1`: accountd, inferd or syncd, each as the owner of its own name;
//! - `org.quire.Confirm1`: intentd.
//!
//! Every case names what the owner of the service's name is to the shell, who calls (the owner
//! itself, or another connection) and what that caller is. A refusal is `AccessDenied` and draws
//! nothing; the cases that pass are each table's control, so a refusal proves the check and not a
//! broken setup. Stand-ins are introduced by unique name; nothing reads `/proc`. A test skips,
//! saying so, without a `dbus-daemon`. Never the person's session bus.

use std::time::Duration;

use docket_core::ConfirmOffer;
use docket_dbus::ConfirmProxy;
use sill_model::screenshots::Shots;
use sill_testkit::{PrivateBus, TempDir};
use tokio::sync::watch;
use zbus::Connection;

use crate::accounts_sheet_bus::{
    connect as connect_sheet_bus, host, providers, serving as serve_sheet, shell as sheet_shell,
    standing_in,
};
use crate::callers_support::{Role, Who};
use crate::capture_pick_bus::{Shell1Proxy, connect as connect_picker_bus, rig_as};
use crate::intents_bus::{connect as connect_intents_bus, desk_as, no_menus, request};

/// How long a thing that must not happen is given to happen.
const QUIET: Duration = Duration::from_millis(400);

const ACCESS_DENIED: &str = "org.freedesktop.DBus.Error.AccessDenied";

/// Who makes the call.
#[derive(Debug, Clone, Copy)]
enum Caller {
    /// The connection that owns the service's name.
    Owner,
    /// Another connection, which is `Who` to the shell.
    Another(Who),
}

/// What a call came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// The shell did not refuse the caller.
    LetIn,
    /// `AccessDenied`.
    Refused,
}

fn verdict<T>(result: &zbus::Result<T>) -> Verdict {
    match result {
        Err(zbus::Error::MethodError(name, ..)) if name.as_str() == ACCESS_DENIED => {
            Verdict::Refused
        }
        Err(zbus::Error::FDO(error)) if matches!(**error, zbus::fdo::Error::AccessDenied(_)) => {
            Verdict::Refused
        }
        Ok(_) | Err(_) => Verdict::LetIn,
    }
}

/// `(case, what the owner of the name is, who calls, what the shell does)`.
type Case = (&'static str, Who, Caller, Verdict);

#[tokio::test(flavor = "multi_thread")]
async fn only_the_portal_backend_that_owns_its_name_asks_the_picker() {
    use Verdict::{LetIn, Refused};
    let portal = Who::Service(Role::Portal);
    let cases: [Case; 6] = [
        ("the portal backend", portal, Caller::Owner, LetIn),
        (
            "a program of the system that took the name",
            Who::Program,
            Caller::Owner,
            Refused,
        ),
        (
            "a program nobody can name that took the name",
            Who::Nobody,
            Caller::Owner,
            Refused,
        ),
        (
            "another service of the desktop that took the name",
            Who::Service(Role::Settings),
            Caller::Owner,
            Refused,
        ),
        (
            "the portal backend's role, on a connection that does not own the name",
            portal,
            Caller::Another(portal),
            Refused,
        ),
        (
            "the real portal backend, while a program holds the name",
            Who::Program,
            Caller::Another(portal),
            Refused,
        ),
    ];
    for (name, owner, caller, want) in cases {
        let Some(bus) = PrivateBus::start() else {
            eprintln!("skipped: no dbus-daemon");
            return;
        };
        let scratch = TempDir::new("callers-picker");
        let rig = rig_as(&bus, &scratch, owner).await;
        let calling = match caller {
            Caller::Owner => rig.transom.clone(),
            Caller::Another(who) => {
                let other = connect_picker_bus(&bus, None).await;
                rig.callers.make(&other, who);
                other
            }
        };
        let proxy = Shell1Proxy::new(&calling)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        let colour = proxy.pick_color("Screenshot").await;
        assert_eq!(verdict(&colour), want, "{name}: pick_color");
        match want {
            LetIn => {
                assert!(colour.is_ok(), "{name}: {colour:?}");
            }
            Refused => {
                let picked = proxy.pick_capture_source("Evil", 3, 0, vec![1]).await;
                assert_eq!(verdict(&picked), Refused, "{name}: pick_capture_source");
                let asked = proxy
                    .ask_permission("Evil", "Allow?", "", "", "Yes", "No")
                    .await;
                assert_eq!(verdict(&asked), Refused, "{name}: ask_permission");
                let state = rig.service.state.borrow().clone();
                assert!(
                    state.asking.is_none()
                        && state.color.is_none()
                        && state.permission.is_none()
                        && state.waiting.is_empty(),
                    "{name}: a refused call drew something: {state:?}"
                );
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn only_an_account_service_that_owns_its_own_name_drives_the_accounts_sheet() {
    use Verdict::{LetIn, Refused};
    const ACCOUNTS: &str = "org.quire.Accounts1";
    const INFERENCE: &str = "org.quire.Inference1";
    const SYNC: &str = "org.quire.Sync1";
    let accounts = Who::Service(Role::Accounts);
    // `(case, the name under test, what its owner is, who calls, what the shell does)`.
    let cases: [(&str, &str, Who, Caller, Verdict); 9] = [
        ("accountd", ACCOUNTS, accounts, Caller::Owner, LetIn),
        (
            "inferd",
            INFERENCE,
            Who::Service(Role::Inference),
            Caller::Owner,
            LetIn,
        ),
        (
            "syncd",
            SYNC,
            Who::Service(Role::Sync),
            Caller::Owner,
            LetIn,
        ),
        (
            "a service holding another service's name",
            SYNC,
            accounts,
            Caller::Owner,
            Refused,
        ),
        (
            "a program of the system that took accountd's name",
            ACCOUNTS,
            Who::Program,
            Caller::Owner,
            Refused,
        ),
        (
            "a program nobody can name that took syncd's name",
            SYNC,
            Who::Nobody,
            Caller::Owner,
            Refused,
        ),
        (
            "another service of the desktop that took inferd's name",
            INFERENCE,
            Who::Service(Role::Router),
            Caller::Owner,
            Refused,
        ),
        (
            "accountd's role, on a connection that does not own the name",
            ACCOUNTS,
            accounts,
            Caller::Another(accounts),
            Refused,
        ),
        (
            "the real accountd, while a program holds its name",
            ACCOUNTS,
            Who::Program,
            Caller::Another(accounts),
            Refused,
        ),
    ];
    for (name, held, owner, caller, want) in cases {
        let Some(bus) = PrivateBus::start() else {
            eprintln!("skipped: no dbus-daemon");
            return;
        };
        let mut shell = sheet_shell(&bus);
        serve_sheet(&mut shell).await;
        let owning = standing_in(&bus, &shell, held, owner).await;
        let calling = match caller {
            Caller::Owner => owning.clone(),
            Caller::Another(who) => {
                let other = connect_sheet_bus(&bus, None).await;
                shell.callers.make(&other, who);
                other
            }
        };
        let opened = host(&calling).await.open("h-1", "", &providers()).await;
        assert_eq!(verdict(&opened), want, "{name}: {opened:?}");
        match want {
            LetIn => {
                assert!(opened.is_ok(), "{name}: {opened:?}");
            }
            Refused => {
                tokio::time::sleep(QUIET).await;
                assert!(
                    shell.sheet.state.borrow().open.is_empty(),
                    "{name}: a refused Open drew"
                );
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_intentd_that_owns_its_name_asks_the_confirmation_sheet() {
    use Verdict::{LetIn, Refused};
    let router = Who::Service(Role::Router);
    let cases: [Case; 5] = [
        ("intentd", router, Caller::Owner, LetIn),
        (
            "a program of the system that took intentd's name",
            Who::Program,
            Caller::Owner,
            Refused,
        ),
        (
            "a program nobody can name that took intentd's name",
            Who::Nobody,
            Caller::Owner,
            Refused,
        ),
        (
            "the portal backend, which took intentd's name",
            Who::Service(Role::Portal),
            Caller::Owner,
            Refused,
        ),
        (
            "intentd's role, on a connection that does not own the name",
            router,
            Caller::Another(router),
            Refused,
        ),
    ];
    for (name, owner, caller, want) in cases {
        let Some(desk) = desk_as(no_menus(), watch::channel(Shots::default()).1, owner).await
        else {
            eprintln!("skipped: no dbus-daemon");
            return;
        };
        desk.serving().await;
        let calling: Connection = match caller {
            Caller::Owner => desk.intentd.clone(),
            Caller::Another(who) => {
                let other = connect_intents_bus(&desk.bus.address).await;
                desk.callers.make(&other, who);
                other
            }
        };
        let proxy = ConfirmProxy::new(&calling)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        let text = serde_json::to_string(&request("c-1", ConfirmOffer::OnceOnly, 120))
            .unwrap_or_else(|e| panic!("{e}"));
        let asked = proxy.confirm(&text).await;
        assert_eq!(verdict(&asked), want, "{name}: {asked:?}");
        match want {
            LetIn => {
                assert!(asked.is_ok(), "{name}: {asked:?}");
            }
            Refused => {
                let withdrawn = proxy.cancel("c-1").await;
                assert_eq!(verdict(&withdrawn), Refused, "{name}: cancel");
                tokio::time::sleep(QUIET).await;
                assert!(
                    desk.seat.state.borrow().sheets.queue.is_empty(),
                    "{name}: a refused Confirm drew"
                );
            }
        }
    }
}
