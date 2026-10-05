//! The computer-use runs as rows: what each was asked, where it stands and the controls its
//! state offers. A search lists the runs still going; asked alone (the Runs filter) it lists every
//! run cuad still holds, the ended ones too.
//!
//! Enter on a run is "Watch" where there is a window to watch, and nothing else: stopping or
//! taking over is always a deliberate pick from the row's actions.

use ds_core::word::Word;
use sill_launcher::{
    Action, ActionId, ActionKeys, Activation, Batch, Choice, CompanionAct, IconRef, Item, ItemId,
    Provider, ProviderKind, Query, ResultSink, RunControl, Scope, Score, Subject,
};
use sill_model::companion::{Companion, CompanionLink, RunFacts, run_controls, run_state_of};
use tokio::sync::watch;

/// Rows from cuad's runs.
#[derive(Debug, Clone)]
pub struct RunsProvider {
    pub companion: watch::Receiver<Companion>,
}

impl Provider for RunsProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Runs
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        if query.is_cancelled() {
            return;
        }
        let state = self.companion.borrow().clone();
        let alone = query.scope == Scope::Only(ProviderKind::Runs);
        let items = match state.link {
            CompanionLink::Passive => Vec::new(),
            CompanionLink::Ready => rows(&state, alone),
        };
        sink.push(Batch {
            provider: ProviderKind::Runs,
            generation: query.generation,
            items,
        });
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        let Subject::Run(run) = &item.subject else {
            return Activation::Nothing;
        };
        let control = match choice {
            Choice::Primary => item
                .actions
                .iter()
                .find(|action| action.id.0 == RunControl::Watch.slug())
                .map(|_| RunControl::Watch),
            Choice::Action(ActionId(id)) => RunControl::parse(id),
        };
        control.map_or(Activation::Nothing, |control| {
            Activation::Companion(CompanionAct::Run {
                run: run.clone(),
                control,
            })
        })
    }
}

fn rows(state: &Companion, alone: bool) -> Vec<Item> {
    state
        .runs
        .values()
        .filter(|run| alone || run_state_of(run.state, run.outcome).is_live())
        .enumerate()
        .map(|(at, run)| row(state, run, at))
        .collect()
}

fn row(state: &Companion, run: &RunFacts, at: usize) -> Item {
    let standing = run_state_of(run.state, run.outcome);
    let undoable = state
        .journal
        .iter()
        .filter(|entry| entry.run.as_ref() == Some(&run.run))
        .filter(|entry| matches!(entry.state, docket_core::UndoState::Available))
        .count();
    let controls = run_controls(
        standing,
        run.mode,
        u32::try_from(undoable).unwrap_or(u32::MAX),
    );
    let subtitle = match &run.window {
        Some(window) => format!("{} · {window}", standing.label()),
        None => standing.label().to_owned(),
    };
    Item {
        id: ItemId(format!("run:{}", run.run)),
        provider: ProviderKind::Runs,
        title: run.goal.clone(),
        subtitle,
        icon: IconRef::Glyph("play".to_owned()),
        score: Score(u32::try_from(1000usize.saturating_sub(at)).unwrap_or(0)),
        actions: controls
            .into_iter()
            .map(|control| Action {
                id: ActionId(control.slug().to_owned()),
                label: control.label().to_owned(),
                keys: ActionKeys::default(),
            })
            .collect(),
        subject: Subject::Run(run.run.clone()),
    }
}

#[cfg(test)]
mod tests {
    use cua_bus::{CuaOutcome, RunStateSlug};
    use docket_core::RunMode;
    use prov::RunId;
    use sill_launcher::{Cancel, Query, VecSink};

    use super::*;

    fn run(id: &str, goal: &str, state: RunStateSlug, mode: RunMode) -> RunFacts {
        RunFacts {
            run: RunId::parse(id).unwrap_or_else(|e| panic!("{e}")),
            goal: goal.to_owned(),
            window: Some("Tables".to_owned()),
            step: 2,
            state,
            outcome: None,
            mode,
            thought: None,
            budget: None,
        }
    }

    fn companion() -> Companion {
        let runs = [
            run(
                "r-1",
                "book a table",
                RunStateSlug::Acting,
                RunMode::AgentWorkspace,
            ),
            RunFacts {
                outcome: Some(CuaOutcome::Done),
                ..run(
                    "r-2",
                    "find a flight",
                    RunStateSlug::Finished,
                    RunMode::InPlace,
                )
            },
        ];
        Companion {
            link: CompanionLink::Ready,
            runs: runs.into_iter().map(|r| (r.run.clone(), r)).collect(),
            ..Companion::default()
        }
    }

    fn rows(scope: Scope) -> Vec<Item> {
        let (_tx, rx) = watch::channel(companion());
        let provider = RunsProvider { companion: rx };
        let query = Query {
            scope,
            ..Query::begin("", &Cancel::new())
        };
        let mut sink = VecSink::default();
        provider.query(&query, &mut sink);
        sink.0.into_iter().flat_map(|batch| batch.items).collect()
    }

    #[test]
    fn a_search_lists_the_runs_still_going_and_the_filter_lists_every_run() {
        let titles = |items: &[Item]| items.iter().map(|i| i.title.clone()).collect::<Vec<_>>();
        assert_eq!(titles(&rows(Scope::All)), ["book a table"]);
        assert_eq!(
            titles(&rows(Scope::Only(ProviderKind::Runs))),
            ["book a table", "find a flight"]
        );
    }

    #[test]
    fn a_row_offers_the_controls_its_state_does() {
        let found = rows(Scope::Only(ProviderKind::Runs));
        let actions = |at: usize| -> Vec<&str> {
            found[at].actions.iter().map(|a| a.label.as_str()).collect()
        };
        assert_eq!(actions(0), ["Watch", "Pause", "Take over", "Stop"]);
        assert_eq!(
            actions(1),
            Vec::<&str>::new(),
            "nothing of a finished run was undoable"
        );
        assert_eq!(found[0].subtitle, "Running · Tables");
    }

    #[test]
    fn enter_watches_and_never_stops_and_a_pick_runs_the_control_it_names() {
        let (_tx, rx) = watch::channel(companion());
        let provider = RunsProvider { companion: rx };
        let found = rows(Scope::Only(ProviderKind::Runs));
        let run = RunId::parse("r-1").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            provider.activate(&found[0], &Choice::Primary),
            Activation::Companion(CompanionAct::Run {
                run: run.clone(),
                control: RunControl::Watch
            })
        );
        assert_eq!(
            provider.activate(&found[0], &Choice::Action(ActionId("stop".to_owned()))),
            Activation::Companion(CompanionAct::Run {
                run,
                control: RunControl::Stop
            })
        );
        assert_eq!(
            provider.activate(&found[1], &Choice::Primary),
            Activation::Nothing,
            "a run with no window to watch has no default"
        );
    }

    #[test]
    fn without_the_companion_there_are_no_runs() {
        let (_tx, rx) = watch::channel(Companion::default());
        let provider = RunsProvider { companion: rx };
        let mut sink = VecSink::default();
        provider.query(&Query::begin("", &Cancel::new()), &mut sink);
        assert!(sink.0.iter().all(|b| b.items.is_empty()));
    }
}
