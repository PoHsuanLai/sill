//! The helpers the person keeps, as rows: one per helper with its name, one line of what it is
//! doing and, when it needs the person, a Waiting badge on the right; and the row that starts a
//! new one. Only the Helpers filter (`Scope::Only`) lists them, which is what the panel's
//! companion mode asks when Tab reaches it.
//!
//! The card is the preview pane's, not a row's: what Enter does on a helper is nothing, and its
//! actions are pause or resume and remove (which asks first).

use companion_wire::{HelperLine, HelperNeeds};
use docket_core::helper::HelperPhase;
use ds_core::word::Word;
use sill_launcher::{
    Action, ActionId, ActionKeys, Activation, Batch, Choice, CompanionAct, HelperAttention,
    HelperControl, HelperFacts, IconRef, Item, ItemId, Provider, ProviderKind, Query, ResultSink,
    Scope, Score, Subject,
};
use sill_model::companion::{Companion, CompanionLink, controls_of};
use tokio::sync::watch;

/// The id of the row that starts a new helper.
const NEW_ID: &str = "helper:new";

/// Rows from the companion's helpers.
#[derive(Debug, Clone)]
pub struct HelpersProvider {
    pub companion: watch::Receiver<Companion>,
}

impl Provider for HelpersProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Helpers
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        if query.is_cancelled() {
            return;
        }
        let state = self.companion.borrow().clone();
        let alone = query.scope == Scope::Only(ProviderKind::Helpers);
        let items = match (state.link, alone) {
            (CompanionLink::Ready, true) => rows(&state.helpers.lines),
            (CompanionLink::Ready, false) | (CompanionLink::Passive, _) => Vec::new(),
        };
        sink.push(Batch {
            provider: ProviderKind::Helpers,
            generation: query.generation,
            items,
        });
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        match (&item.subject, choice) {
            (Subject::NewHelper, Choice::Primary) => Activation::Companion(CompanionAct::NewHelper),
            (Subject::Helper(facts), Choice::Action(ActionId(id))) => HelperControl::parse(id)
                .map_or(Activation::Nothing, |control| {
                    Activation::Companion(CompanionAct::Helper {
                        helper: facts.id.clone(),
                        control,
                    })
                }),
            _ => Activation::Nothing,
        }
    }
}

/// The new-helper row, then one row per helper in the companion's order.
fn rows(lines: &[HelperLine]) -> Vec<Item> {
    std::iter::once(new_row())
        .chain(lines.iter().enumerate().map(|(at, line)| row(line, at)))
        .collect()
}

fn new_row() -> Item {
    Item {
        id: ItemId(NEW_ID.to_owned()),
        provider: ProviderKind::Helpers,
        title: "New Helper\u{2026}".to_owned(),
        subtitle: String::new(),
        icon: IconRef::Glyph("plus".to_owned()),
        score: Score(1000),
        actions: Vec::new(),
        subject: Subject::NewHelper,
    }
}

fn row(line: &HelperLine, at: usize) -> Item {
    Item {
        id: ItemId(format!("helper:{}", line.id)),
        provider: ProviderKind::Helpers,
        title: line.name.as_str().to_owned(),
        subtitle: line.doing.clone(),
        icon: IconRef::Glyph("group".to_owned()),
        score: Score(u32::try_from(900usize.saturating_sub(at)).unwrap_or(0)),
        actions: actions(line.state),
        subject: Subject::Helper(HelperFacts {
            id: line.id.clone(),
            attention: attention_of(line.needs_you),
        }),
    }
}

fn attention_of(needs: Option<HelperNeeds>) -> HelperAttention {
    match needs {
        Some(HelperNeeds::ChooseModel | HelperNeeds::Answer) => HelperAttention::WaitingForYou,
        None => HelperAttention::Calm,
    }
}

fn actions(phase: HelperPhase) -> Vec<Action> {
    controls_of(phase)
        .into_iter()
        .map(|control| Action {
            id: ActionId(control.slug().to_owned()),
            label: control.label().to_owned(),
            keys: ActionKeys::default(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use companion_wire::HelpersView;
    use docket_core::helper::HelperName;
    use prov::{HelperId, SpaceId};
    use sill_launcher::{Cancel, Query, VecSink};
    use sill_model::companion::{HelpersModel, HelpersMsg, update_helpers};

    use super::*;

    fn line(id: &str, name: &str, state: HelperPhase, needs: Option<HelperNeeds>) -> HelperLine {
        HelperLine {
            id: HelperId::parse(id).unwrap_or_else(|e| panic!("{e}")),
            name: HelperName::parse(name).unwrap_or_else(|e| panic!("{e}")),
            space: SpaceId::desktop(),
            doing: "Nothing to do".to_owned(),
            state,
            needs_you: needs,
            agent: None,
        }
    }

    fn companion() -> Companion {
        let view = HelpersView {
            helpers: vec![
                line(
                    "h-1",
                    "Research",
                    HelperPhase::Working,
                    Some(HelperNeeds::Answer),
                ),
                line("h-2", "Inbox", HelperPhase::Paused, None),
            ],
            told: Vec::new(),
        };
        Companion {
            link: CompanionLink::Ready,
            helpers: update_helpers(HelpersModel::default(), HelpersMsg::Listed(view)).0,
            ..Companion::default()
        }
    }

    fn rows_of(state: Companion, scope: Scope) -> Vec<Item> {
        let (_tx, rx) = watch::channel(state);
        let provider = HelpersProvider { companion: rx };
        let query = Query {
            scope,
            ..Query::begin("", &Cancel::new())
        };
        let mut sink = VecSink::default();
        provider.query(&query, &mut sink);
        sink.0.into_iter().flat_map(|batch| batch.items).collect()
    }

    #[test]
    fn only_the_helpers_filter_lists_them_and_the_new_row_comes_first() {
        let titles = |items: &[Item]| items.iter().map(|i| i.title.clone()).collect::<Vec<_>>();
        assert_eq!(
            titles(&rows_of(companion(), Scope::Only(ProviderKind::Helpers))),
            ["New Helper\u{2026}", "Research", "Inbox"]
        );
        assert!(rows_of(companion(), Scope::All).is_empty());
        assert!(
            rows_of(Companion::default(), Scope::Only(ProviderKind::Helpers)).is_empty(),
            "without the companion there are no helpers"
        );
    }

    #[test]
    fn a_row_says_what_the_helper_does_and_whether_it_needs_the_person() {
        let found = rows_of(companion(), Scope::Only(ProviderKind::Helpers));
        let facts = |at: usize| match &found[at].subject {
            Subject::Helper(facts) => facts.attention,
            other => panic!("{other:?}"),
        };
        assert_eq!(facts(1), HelperAttention::WaitingForYou);
        assert_eq!(facts(2), HelperAttention::Calm);
        assert_eq!(found[1].subtitle, "Nothing to do");
        let labels = |at: usize| -> Vec<&str> {
            found[at].actions.iter().map(|a| a.label.as_str()).collect()
        };
        assert_eq!(labels(1), ["Pause", "Remove"]);
        assert_eq!(labels(2), ["Resume", "Remove"]);
    }

    #[test]
    fn enter_on_the_new_row_starts_a_card_and_a_pick_runs_the_control_it_names() {
        let (_tx, rx) = watch::channel(companion());
        let provider = HelpersProvider { companion: rx };
        let found = rows_of(companion(), Scope::Only(ProviderKind::Helpers));
        assert_eq!(
            provider.activate(&found[0], &Choice::Primary),
            Activation::Companion(CompanionAct::NewHelper)
        );
        assert_eq!(
            provider.activate(&found[1], &Choice::Action(ActionId("remove".to_owned()))),
            Activation::Companion(CompanionAct::Helper {
                helper: HelperId::parse("h-1").unwrap_or_else(|e| panic!("{e}")),
                control: HelperControl::Remove
            })
        );
        assert_eq!(
            provider.activate(&found[1], &Choice::Primary),
            Activation::Nothing,
            "the card is the pane; Enter has nothing to do"
        );
    }
}
