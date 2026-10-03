use std::collections::BTreeMap;

use docket_core::{
    AppRefusal, CallId, ChoiceId, Invocation, Origin, ParamName, TargetValue, Value,
};
use prov::{ActionName, Actor, Label, Labelled, SpaceId};

use docket_core::{UndoFault, UndoToken, Undoable};
use sill_launcher::AppId;

use super::{ShellAction, WorkspaceStep, action_of, outcome_of, undo_of};
use crate::screenshots::ShotKind;

fn invocation(action: &str, args: &[(&str, Value)]) -> Invocation {
    Invocation {
        call: CallId(1),
        action: ActionName::parse(action).unwrap_or_else(|e| panic!("{action}: {e}")),
        target: TargetValue::Nothing,
        args: args
            .iter()
            .map(|(name, value)| {
                (
                    ParamName::parse(name).unwrap_or_else(|e| panic!("{name}: {e}")),
                    Labelled {
                        value: value.clone(),
                        label: Label::trusted_user(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
        actor: Actor::Cli,
        origin: Origin::Launcher,
        space: SpaceId::parse("work").unwrap_or_else(|e| panic!("{e}")),
    }
}

fn choice(id: &str) -> Value {
    Value::Choice(ChoiceId::parse(id).unwrap_or_else(|e| panic!("{id}: {e}")))
}

#[test]
fn each_declared_action_reads_into_its_shell_action() {
    let cases: Vec<(Invocation, Result<ShellAction, AppRefusal>)> = vec![
        (
            invocation("shell.launcher.open", &[]),
            Ok(ShellAction::LauncherOpen {
                query: String::new(),
            }),
        ),
        (
            invocation(
                "shell.launcher.open",
                &[("query", Value::Text("mail".into()))],
            ),
            Ok(ShellAction::LauncherOpen {
                query: "mail".into(),
            }),
        ),
        (
            invocation("shell.workspace.switch", &[]),
            Ok(ShellAction::WorkspaceSwitch(WorkspaceStep::Next)),
        ),
        (
            invocation("shell.workspace.switch", &[("direction", choice("prev"))]),
            Ok(ShellAction::WorkspaceSwitch(WorkspaceStep::Prev)),
        ),
        (invocation("shell.lock", &[]), Ok(ShellAction::Lock)),
        (
            invocation(
                "shell.dock.pin",
                &[("app", Value::Text("firefox.desktop".into()))],
            ),
            Ok(ShellAction::DockPin {
                app: AppId("firefox.desktop".into()),
            }),
        ),
        (
            invocation(
                "shell.dock.unpin",
                &[("app", Value::Text(" firefox.desktop ".into()))],
            ),
            Ok(ShellAction::DockUnpin {
                app: AppId("firefox.desktop".into()),
            }),
        ),
        (
            invocation("shell.screenshot", &[]),
            Ok(ShellAction::Screenshot(ShotKind::Screen)),
        ),
        (
            invocation("shell.screenshot", &[("kind", choice("area"))]),
            Ok(ShellAction::Screenshot(ShotKind::Area)),
        ),
        (
            invocation("shell.reboot", &[]),
            Err(AppRefusal::Unsupported),
        ),
    ];
    for (call, want) in cases {
        assert_eq!(action_of(&call), want, "{}", call.action);
    }
}

#[test]
fn a_parameter_of_the_wrong_kind_or_outside_its_options_is_refused_by_name() {
    for call in [
        invocation(
            "shell.workspace.switch",
            &[("direction", choice("sideways"))],
        ),
        invocation(
            "shell.workspace.switch",
            &[("direction", Value::Integer(3))],
        ),
        invocation("shell.launcher.open", &[("query", Value::Integer(3))]),
        invocation("shell.screenshot", &[("kind", Value::Text("area".into()))]),
    ] {
        let Err(AppRefusal::Failed(why)) = action_of(&call) else {
            panic!("{} should be refused", call.action);
        };
        assert!(!why.0.is_empty());
        assert!(
            !why.0.contains("sideways"),
            "a refusal names the parameter, never its text: {}",
            why.0
        );
    }
}

#[test]
fn every_action_says_what_it_did() {
    for action in [
        ShellAction::LauncherOpen { query: "x".into() },
        ShellAction::WorkspaceSwitch(WorkspaceStep::Next),
        ShellAction::WorkspaceSwitch(WorkspaceStep::Prev),
        ShellAction::Lock,
        ShellAction::DockPin {
            app: AppId("a.desktop".into()),
        },
        ShellAction::DockUnpin {
            app: AppId("a.desktop".into()),
        },
        ShellAction::Screenshot(ShotKind::Window),
    ] {
        let outcome = outcome_of(&action);
        assert!(outcome.said.is_some(), "{action:?}");
        assert!(outcome.value.is_none());
    }
}

#[test]
fn a_pin_needs_its_app() {
    for call in [
        invocation("shell.dock.pin", &[]),
        invocation("shell.dock.unpin", &[("app", Value::Text("  ".into()))]),
    ] {
        assert!(
            matches!(action_of(&call), Err(AppRefusal::NeedsParam { .. })),
            "{}",
            call.action
        );
    }
}

#[test]
fn a_pin_and_an_unpin_undo_each_other_and_nothing_else_does() {
    let app = AppId("firefox.desktop".into());
    let pin = outcome_of(&ShellAction::DockPin { app: app.clone() });
    let Undoable::Yes(token) = pin.undo else {
        panic!("a pin can be taken back");
    };
    assert_eq!(token.as_str(), "dock.unpin:firefox.desktop");
    assert_eq!(
        undo_of(&token),
        Ok(ShellAction::DockUnpin { app: app.clone() })
    );
    let unpin = outcome_of(&ShellAction::DockUnpin { app: app.clone() });
    let Undoable::Yes(token) = unpin.undo else {
        panic!("an unpin can be taken back");
    };
    assert_eq!(undo_of(&token), Ok(ShellAction::DockPin { app }));
    for action in [
        ShellAction::Lock,
        ShellAction::LauncherOpen {
            query: String::new(),
        },
        ShellAction::WorkspaceSwitch(WorkspaceStep::Prev),
        ShellAction::Screenshot(ShotKind::Area),
    ] {
        assert_eq!(outcome_of(&action).undo, Undoable::No, "{action:?}");
    }
    let foreign = UndoToken::parse("mail.restore:7").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(undo_of(&foreign), Err(UndoFault::Gone));
}
