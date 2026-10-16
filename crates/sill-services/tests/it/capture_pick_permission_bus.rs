//! The permission question's host side on a private bus: a stand-in transom (the owner of the
//! portal backend's name) calls `AskPermission` on the connection the seat holds and gets each
//! answer the shell steps through `PermissionAnswer`: 0 for the yes button, 1 for the no button,
//! 2 for a closed sheet. A stranger is refused and draws nothing, a second question while one is
//! open is refused at once, and the app's words reach the sheet cleaned. A test skips, saying so,
//! without a `dbus-daemon`. Never the person's session bus.

use sill_model::capture_pick::{CaptureCommand, CapturePicker, PermissionOutcome};
use sill_testkit::{PrivateBus, TempDir};

use crate::capture_pick_bus::{Shell1Proxy, connect, serving, wait_for};

#[tokio::test(flavor = "multi_thread")]
async fn transom_asks_and_each_button_comes_back_as_its_number() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let scratch = TempDir::new("capture-pick-permission");
    let (mut service, transom) = serving(&bus, &scratch).await;
    let proxy = Shell1Proxy::new(&transom)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    const CASES: &[(PermissionOutcome, u32)] = &[
        (PermissionOutcome::Grant, 0),
        (PermissionOutcome::Deny, 1),
        (PermissionOutcome::Closed, 2),
    ];
    for &(outcome, want) in CASES {
        let asked = {
            let proxy = proxy.clone();
            tokio::spawn(async move {
                proxy
                    .ask_permission(
                        "Notes\u{202e}",
                        "Share the\nscreen?",
                        "Notes wants to see it",
                        "Only while you share.",
                        "Share",
                        "",
                    )
                    .await
            })
        };
        assert!(
            wait_for(&mut service.state, |s: &CapturePicker| s
                .permission
                .is_some())
            .await,
            "{outcome:?}: the sheet was not opened"
        );
        // One at a time, of any kind: the second call is refused at once.
        assert!(
            proxy
                .ask_permission("Other", "t", "s", "b", "Yes", "No")
                .await
                .is_err(),
            "{outcome:?}"
        );
        assert!(
            proxy.pick_color("Other").await.is_err(),
            "{outcome:?}: a colour is a question of the same kind"
        );
        let (id, ask) = {
            let state = service.state.borrow();
            let asking = state.permission.clone().unwrap_or_else(|| panic!("open"));
            (asking.id, asking.ask)
        };
        assert_eq!(ask.app_label, "Notes", "{outcome:?}: direction marks go");
        assert_eq!(ask.title, "Share the screen?", "{outcome:?}");
        assert_eq!(ask.grant_label, "Share", "{outcome:?}");
        assert_eq!(
            ask.deny_label, "Don\u{2019}t Allow",
            "{outcome:?}: an empty button takes the usual word"
        );
        let _ = service
            .control
            .send(CaptureCommand::PermissionAnswer(id, outcome));
        let answer = asked
            .await
            .unwrap_or_else(|e| panic!("{e}"))
            .unwrap_or_else(|e| panic!("{outcome:?}: {e}"));
        assert_eq!(answer, want, "{outcome:?}");
        assert!(
            service.state.borrow().permission.is_none(),
            "{outcome:?}: the sheet is free again"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_caller_that_is_not_the_portal_backend_is_refused_and_draws_nothing() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let scratch = TempDir::new("capture-pick-permission-forged");
    let (service, _transom) = serving(&bus, &scratch).await;
    let forger = connect(&bus, None).await;
    let proxy = Shell1Proxy::new(&forger)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        proxy
            .ask_permission("Evil", "Allow?", "", "", "Yes", "No")
            .await
            .is_err()
    );
    assert!(service.state.borrow().permission.is_none());
}
