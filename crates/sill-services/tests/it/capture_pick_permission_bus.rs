//! The permission question's host side on a private bus: a stand-in transom (the owner of the
//! portal backend's name) calls `AskPermission` on the connection the seat holds and gets each
//! answer the shell steps through `PermissionAnswer`: 0 for the yes button, 1 for the no button,
//! 2 for a closed sheet. The app's words reach the sheet cleaned. One sheet at a time: a second
//! question waits behind the first, in order, and the compositor's questions stand in the same
//! line, their answers going back to the compositor. A test skips, saying so, without a
//! `dbus-daemon`. Never the person's session bus.

use casement_protocols::vocab::Answer;
use sill_model::capture_pick::{AskId, AskedBy, CaptureCommand, CapturePicker, PermissionOutcome};
use sill_model::desktop::{Consent, ConsentId, DesktopCommand, Memory, Power, SubjectKind};
use sill_testkit::{PrivateBus, TempDir};

use crate::capture_pick_bus::{Shell1Proxy, rig, serving, wait_for};

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
        // A colour is a question of another kind: refused at once while the sheet is taken.
        assert!(
            proxy.pick_color("Other").await.is_err(),
            "{outcome:?}: a colour is a question of the same sheet"
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
            wait_for(&mut service.state, |s: &CapturePicker| s
                .permission
                .is_none())
            .await,
            "{outcome:?}: the sheet is free again"
        );
    }
}

fn grim() -> Consent {
    Consent::from_wire(
        ConsentId::new(11),
        Power::Screen,
        SubjectKind::Unrecognised,
        "/home/me/bin/grim",
        Memory::ThisTimeOnly,
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn the_portals_question_and_the_compositors_wait_in_one_line_one_sheet_at_a_time() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let scratch = TempDir::new("capture-pick-consent-line");
    let mut rig = rig(&bus, &scratch).await;
    let proxy = Shell1Proxy::new(&rig.transom)
        .await
        .unwrap_or_else(|e| panic!("{e}"));

    // The compositor's question is on the sheet; the portal's waits behind it.
    let _ = rig.service.control.send(CaptureCommand::Consent(grim()));
    assert!(
        wait_for(&mut rig.service.state, |s: &CapturePicker| s
            .permission
            .as_ref()
            .is_some_and(|a| a.by == AskedBy::Compositor(ConsentId::new(11))))
        .await,
        "the compositor's question was not put"
    );
    let portal = {
        let proxy = proxy.clone();
        tokio::spawn(async move {
            proxy
                .ask_permission("Notes", "Share?", "", "", "Share", "")
                .await
        })
    };
    assert!(
        wait_for(&mut rig.service.state, |s: &CapturePicker| s.waiting.len()
            == 1)
        .await,
        "the portal's question did not wait"
    );

    // The person says Allow Once: the compositor hears Allow, and the portal's question is next.
    let (id, ask) = {
        let state = rig.service.state.borrow();
        let asking = state.permission.clone().unwrap_or_else(|| panic!("open"));
        (asking.id, asking.ask)
    };
    assert_eq!(ask.grant_label, "Allow Once");
    let _ = rig.service.control.send(CaptureCommand::PermissionAnswer(
        id,
        PermissionOutcome::Grant,
    ));
    let told = tokio::time::timeout(std::time::Duration::from_secs(5), rig.answers.recv())
        .await
        .unwrap_or_else(|_| panic!("the compositor was told nothing"));
    assert_eq!(
        told,
        Some(DesktopCommand::AnswerPermission {
            ask: ConsentId::new(11),
            answer: Answer::Allow
        })
    );
    assert!(
        wait_for(&mut rig.service.state, |s: &CapturePicker| s
            .permission
            .as_ref()
            .is_some_and(|a| a.by == AskedBy::Portal))
        .await,
        "the portal's question did not come up"
    );
    let id = rig
        .service
        .state
        .borrow()
        .permission
        .as_ref()
        .map(|a| a.id)
        .unwrap_or(AskId(0));
    let _ = rig.service.control.send(CaptureCommand::PermissionAnswer(
        id,
        PermissionOutcome::Deny,
    ));
    let answer = portal
        .await
        .unwrap_or_else(|e| panic!("{e}"))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(answer, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_withdrawn_question_comes_down_unanswered_and_a_dismissal_is_told_as_one() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("skipped: no dbus-daemon");
        return;
    };
    let scratch = TempDir::new("capture-pick-consent-withdrawn");
    let mut rig = rig(&bus, &scratch).await;
    let _ = rig.service.control.send(CaptureCommand::Consent(grim()));
    assert!(
        wait_for(&mut rig.service.state, |s: &CapturePicker| s
            .permission
            .is_some())
        .await
    );
    let _ = rig
        .service
        .control
        .send(CaptureCommand::ConsentWithdrawn(ConsentId::new(11)));
    assert!(
        wait_for(&mut rig.service.state, |s: &CapturePicker| s
            .permission
            .is_none())
        .await
    );
    assert!(
        rig.answers.try_recv().is_err(),
        "a withdrawn question is not answered"
    );

    // A question whose sheet went away is told as a dismissal, which the compositor never keeps.
    let _ = rig.service.control.send(CaptureCommand::Consent(grim()));
    assert!(
        wait_for(&mut rig.service.state, |s: &CapturePicker| s
            .permission
            .is_some())
        .await
    );
    let id = rig
        .service
        .state
        .borrow()
        .permission
        .as_ref()
        .map(|a| a.id)
        .unwrap_or(AskId(0));
    let _ = rig.service.control.send(CaptureCommand::PermissionAnswer(
        id,
        PermissionOutcome::Closed,
    ));
    let told = tokio::time::timeout(std::time::Duration::from_secs(5), rig.answers.recv())
        .await
        .unwrap_or_else(|_| panic!("the compositor was told nothing"));
    assert_eq!(
        told,
        Some(DesktopCommand::AnswerPermission {
            ask: ConsentId::new(11),
            answer: Answer::Dismissed
        })
    );
}
