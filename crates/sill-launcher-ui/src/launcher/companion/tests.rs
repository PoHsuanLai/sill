use docket_core::Keep;
use prov::{RunId, TaskId};
use sill_launcher::{ProviderKind, WindowKey};
use sill_model::companion::{LauncherOrigin, Summon};
use sill_model::cosmic_wl::{Activity, Sizing, Toplevel, Toplevels, WindowState};

use super::*;
use crate::launcher::keys::Filter;

fn task() -> TaskId {
    TaskId::parse("t-1").unwrap_or_else(|e| panic!("{e}"))
}

fn run() -> RunId {
    RunId::parse("r-1").unwrap_or_else(|e| panic!("{e}"))
}

/// Design/32 section 4.4's key table, by pane.
#[test]
fn companion_keys_table() {
    use CompanionKey as K;
    use CompanionOut as O;
    type Case = (&'static str, K, Pane, Selected, Vec<CompanionOut>);
    let cases: Vec<Case> = vec![
        (
            "escape composing dismisses",
            K::Escape,
            Pane::Composing,
            Selected::Ask,
            vec![O::Dismiss],
        ),
        (
            "escape in an answer goes back",
            K::Escape,
            Pane::Showing(task()),
            Selected::Answer(task()),
            vec![O::Back],
        ),
        (
            "escape in a form goes back",
            K::Escape,
            Pane::Form(task()),
            Selected::Answer(task()),
            vec![O::Back],
        ),
        (
            "escape while thinking goes back",
            K::Escape,
            Pane::Thinking(task()),
            Selected::Answer(task()),
            vec![O::Back],
        ),
        (
            "command-enter promotes an answer",
            K::CommandEnter,
            Pane::Showing(task()),
            Selected::Answer(task()),
            vec![O::Promote(task())],
        ),
        (
            "command-enter promotes a form's answer",
            K::CommandEnter,
            Pane::Form(task()),
            Selected::Answer(task()),
            vec![O::Promote(task())],
        ),
        (
            "command-enter composing is nothing",
            K::CommandEnter,
            Pane::Composing,
            Selected::Ask,
            vec![],
        ),
        (
            "command-enter while thinking is nothing",
            K::CommandEnter,
            Pane::Thinking(task()),
            Selected::Answer(task()),
            vec![],
        ),
        (
            "command-period cancels what is being made",
            K::CommandPeriod,
            Pane::Thinking(task()),
            Selected::Answer(task()),
            vec![O::Cancel(task())],
        ),
        (
            "command-period on a run stops it",
            K::CommandPeriod,
            Pane::Composing,
            Selected::Run(run()),
            vec![O::StopRun(run())],
        ),
        (
            "command-period on the ask row is nothing",
            K::CommandPeriod,
            Pane::Composing,
            Selected::Ask,
            vec![],
        ),
        (
            "command-period on a finished answer is nothing",
            K::CommandPeriod,
            Pane::Showing(task()),
            Selected::Answer(task()),
            vec![],
        ),
    ];
    for (name, key, pane, selected, want) in cases {
        assert_eq!(companion_step(key, &pane, &selected), want, "{name}");
    }
}

#[test]
fn tab_visits_the_companions_rows_runs_and_memory_and_wraps() {
    use ProviderKind::{Companion, Memory, Runs};
    let cases = [
        (Filter::All, Way::Forward, Companion),
        (Filter::All, Way::Backward, Memory),
        (Filter::Only(Companion), Way::Forward, Runs),
        (Filter::Only(Runs), Way::Forward, Memory),
        (Filter::Only(Memory), Way::Forward, Companion),
        (Filter::Only(Companion), Way::Backward, Memory),
        (Filter::Only(Memory), Way::Backward, Runs),
        (Filter::Only(ProviderKind::Apps), Way::Forward, Companion),
    ];
    for (from, way, want) in cases {
        assert_eq!(
            companion_filter(from, way),
            Filter::Only(want),
            "{from:?} {way:?}"
        );
    }
}

fn toplevels() -> Toplevels {
    Toplevels {
        windows: vec![Toplevel {
            key: WindowKey(4),
            app_id: "org.quire.Mail".to_owned(),
            title: "Inbox".to_owned(),
            activity: Activity::Active,
            window: WindowState::Normal,
            sizing: Sizing::Floating,
            workspaces: Vec::new(),
            outputs: Vec::new(),
        }],
    }
}

#[test]
fn a_prompt_from_a_panel_summoned_over_a_window_keeps_the_window_chip_only() {
    let summon = Summon::InLauncher {
        origin: LauncherOrigin::Fresh {
            window: Some(WindowKey(4)),
        },
    };
    let ask = ask_input("forward the receipts".to_owned(), &summon, &toplevels());
    assert_eq!(ask.window, Some(WindowKey(4)));
    assert_eq!(ask.app.as_ref().map(|a| a.as_str()), Some("org.quire.Mail"));
    assert_eq!(ask.keep.window, Keep::Kept);
    assert_eq!(
        (ask.keep.query, ask.keep.results, ask.keep.selection),
        (Keep::Dropped, Keep::Dropped, Keep::Dropped)
    );
}

#[test]
fn a_prompt_from_the_search_or_over_no_window_carries_no_context() {
    for summon in [
        Summon::InLauncher {
            origin: LauncherOrigin::FromSearch,
        },
        Summon::InLauncher {
            origin: LauncherOrigin::Fresh { window: None },
        },
        Summon::Rest,
    ] {
        let ask = ask_input("what time is it".to_owned(), &summon, &toplevels());
        assert_eq!((ask.window, ask.app), (None, None), "{summon:?}");
        assert_eq!(ask.keep.window, Keep::Dropped, "{summon:?}");
    }
}

#[test]
fn each_act_but_an_ask_and_a_mode_is_one_command() {
    use docket_core::{CardActionId, UndoId, UndoScope};
    use sill_launcher::{CompanionAct, LauncherMode, RunControl};
    use sill_model::companion::CompanionCommand as C;
    let card = CardActionId::parse("send").unwrap_or_else(|e| panic!("{e}"));
    let cases: Vec<(CompanionAct, Option<C>)> = vec![
        (
            CompanionAct::Ask {
                prompt: "hi".to_owned(),
            },
            None,
        ),
        (CompanionAct::Mode(LauncherMode::Companion), None),
        (
            CompanionAct::Card {
                task: task(),
                action: card.clone(),
            },
            Some(C::Card {
                task: task(),
                action: card,
            }),
        ),
        (
            CompanionAct::Promote { task: task() },
            Some(C::Promote(task())),
        ),
        (
            CompanionAct::Run {
                run: run(),
                control: RunControl::Pause,
            },
            Some(C::Run {
                run: run(),
                control: RunControl::Pause,
            }),
        ),
        (
            CompanionAct::Undo {
                scope: UndoScope::Entry(UndoId(3)),
            },
            Some(C::Undo(UndoScope::Entry(UndoId(3)))),
        ),
    ];
    for (act, want) in cases {
        assert_eq!(command_of(act.clone()), want, "{act:?}");
    }
}
