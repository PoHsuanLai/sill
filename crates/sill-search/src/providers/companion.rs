//! The companion's rows: "Ask the companion" for what is typed, the answers it has made, and the
//! journal's recent activity. Nothing shows while the service has no seat at the companion (no
//! `org.quire.Companion1` to ask), so a desktop without the companion has no such row.
//!
//! The Ask row is in every search; answers and activity show only when the filter is the
//! companion's own (`Scope::Only`), which is what the panel's companion mode asks.

use docket_core::{CardActionId, UndoState};
use sill_launcher::{
    Action, ActionId, Activation, Batch, Choice, CompanionAct, IconRef, Item, ItemId, LauncherMode,
    Provider, ProviderKind, Query, ResultSink, Scope, Score, Subject,
};
use sill_model::companion::{
    AnswerFacts, Busy, Companion, CompanionLink, activity_line, answer_line, cards_of,
};
use tokio::sync::watch;

/// The id prefix of the Ask row: the prompt follows it.
const ASK: &str = "ask:";

/// The action id that undoes a journal row.
pub const UNDO_ACTION: &str = "undo";

/// Rows from the companion's state.
#[derive(Debug, Clone)]
pub struct CompanionProvider {
    pub companion: watch::Receiver<Companion>,
}

impl Provider for CompanionProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Companion
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        if query.is_cancelled() {
            return;
        }
        let state = self.companion.borrow().clone();
        let alone = query.scope == Scope::Only(ProviderKind::Companion);
        let items = match state.link {
            CompanionLink::Passive => Vec::new(),
            CompanionLink::Ready => rows(&state, query.text.trim(), alone),
        };
        sink.push(Batch {
            provider: ProviderKind::Companion,
            generation: query.generation,
            items,
        });
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        match (&item.subject, choice) {
            (Subject::None, Choice::Primary) => {
                item.id
                    .0
                    .strip_prefix(ASK)
                    .map_or(Activation::Nothing, |prompt| {
                        Activation::Companion(CompanionAct::Ask {
                            prompt: prompt.to_owned(),
                        })
                    })
            }
            // Enter on an answer is its card's default button; an answer with none (text, a
            // working answer) has nothing to press, so Enter leaves the panel as it is.
            (Subject::Answer(_), Choice::Primary) => match item.actions.first() {
                Some(first) => self.activate(item, &Choice::Action(first.id.clone())),
                None => Activation::Companion(CompanionAct::Mode(LauncherMode::Companion)),
            },
            (Subject::Answer(task), Choice::Action(ActionId(id))) => CardActionId::parse(id)
                .map_or(Activation::Nothing, |action| {
                    Activation::Companion(CompanionAct::Card {
                        task: task.clone(),
                        action,
                    })
                }),
            // Enter on a journal row undoes it while it can be undone; an undone or expired row
            // has nothing to press.
            (Subject::Activity(entry), Choice::Primary) => {
                if item.actions.iter().any(|a| a.id.0 == UNDO_ACTION) {
                    undo(*entry)
                } else {
                    Activation::Companion(CompanionAct::Mode(LauncherMode::Companion))
                }
            }
            (Subject::Activity(entry), Choice::Action(ActionId(id))) if id == UNDO_ACTION => {
                undo(*entry)
            }
            _ => Activation::Nothing,
        }
    }
}

fn undo(entry: docket_core::UndoId) -> Activation {
    Activation::Companion(CompanionAct::Undo {
        scope: docket_core::UndoScope::Entry(entry),
    })
}

/// How many journal rows the panel lists.
const RECENT_ACTIVITY: usize = 5;

/// The rows for `text`: the Ask row first, then (alone) the answers, newest first, then the
/// latest activity.
fn rows(state: &Companion, text: &str, alone: bool) -> Vec<Item> {
    let ask = (!text.is_empty()).then(|| ask_row(text));
    let answers = alone.then(|| answer_rows(state)).into_iter().flatten();
    let activity = alone
        .then(|| {
            state
                .journal
                .iter()
                .take(RECENT_ACTIVITY)
                .enumerate()
                .map(|(at, entry)| activity_row(entry, at))
        })
        .into_iter()
        .flatten();
    ask.into_iter().chain(answers).chain(activity).collect()
}

fn ask_row(text: &str) -> Item {
    Item {
        id: ItemId(format!("{ASK}{text}")),
        provider: ProviderKind::Companion,
        title: "Ask the companion".to_owned(),
        subtitle: text.to_owned(),
        icon: IconRef::Glyph("sparkles".to_owned()),
        score: Score(1000),
        actions: Vec::new(),
        subject: Subject::None,
    }
}

/// The answers, the front one first and then the rest, newest task first.
fn answer_rows(state: &Companion) -> Vec<Item> {
    let front = state.front.as_ref();
    let mut tasks: Vec<_> = state.answers.keys().collect();
    tasks.sort_by_key(|task| (Some(*task) != front, std::cmp::Reverse((*task).clone())));
    tasks
        .into_iter()
        .enumerate()
        .filter_map(|(at, task)| state.answers.get(task).map(|answer| answer_row(answer, at)))
        .collect()
}

fn answer_row(answer: &AnswerFacts, at: usize) -> Item {
    let (title, subtitle) = answer_line(answer);
    Item {
        id: ItemId(format!("answer:{}", answer.wire.task)),
        provider: ProviderKind::Companion,
        title,
        subtitle,
        icon: IconRef::Glyph("sparkles".to_owned()),
        score: Score(u32::try_from(900usize.saturating_sub(at)).unwrap_or(0)),
        actions: card_actions(answer),
        subject: Subject::Answer(answer.wire.task.clone()),
    }
}

/// An answer's card buttons as the row's actions. While a press is going there are none, so a
/// second Enter cannot press it again; a press the companion refused offers its buttons again.
fn card_actions(answer: &AnswerFacts) -> Vec<Action> {
    match answer.busy {
        Busy::Acting { .. } => Vec::new(),
        Busy::Idle | Busy::Refused(_) => cards_of(&answer.wire.body)
            .iter()
            .map(|card| Action {
                id: ActionId(card.id.as_str().to_owned()),
                label: card.label.as_str().to_owned(),
                keys: sill_launcher::ActionKeys::default(),
            })
            .collect(),
    }
}

fn activity_row(entry: &docket_core::UndoEntry, at: usize) -> Item {
    let (title, subtitle) = activity_line(entry);
    let undoable = matches!(entry.state, UndoState::Available);
    Item {
        id: ItemId(format!("activity:{}", entry.id.0)),
        provider: ProviderKind::Companion,
        title,
        subtitle,
        icon: IconRef::Glyph("undo".to_owned()),
        score: Score(u32::try_from(100usize.saturating_sub(at)).unwrap_or(0)),
        actions: if undoable {
            vec![Action {
                id: ActionId(UNDO_ACTION.to_owned()),
                label: "Undo".to_owned(),
                keys: sill_launcher::ActionKeys::default(),
            }]
        } else {
            Vec::new()
        },
        subject: Subject::Activity(entry.id),
    }
}

#[cfg(test)]
mod tests {
    use companion_wire::{AnswerBody, AnswerPhase, AnswerWire, CardWire, FooterWire};
    use docket_core::{
        ActionRef, Args, CallRequest, CardActionId, ContextKeep, Keep, LabelText, Origin,
        TargetValue, UndoEntry, UndoId, UndoScope, UndoState,
    };
    use prov::{ActionName, Actor, AppName, Effect, TaskId, UnixSeconds};
    use sill_launcher::{Cancel, Query, VecSink};
    use sill_model::companion::{AnswerFacts, Busy};

    use super::*;

    fn task(id: &str) -> TaskId {
        TaskId::parse(id).unwrap_or_else(|e| panic!("{e}"))
    }

    fn card(id: &str) -> CardWire {
        CardWire {
            id: CardActionId::parse(id).unwrap_or_else(|e| panic!("{e}")),
            label: LabelText::parse("Send").unwrap_or_else(|e| panic!("{e}")),
            effect: Effect::Outbound,
            call: CallRequest {
                action: ActionRef {
                    app: AppName::parse("org.quire.Mail").unwrap_or_else(|e| panic!("{e}")),
                    name: ActionName::parse("mail.message.send").unwrap_or_else(|e| panic!("{e}")),
                },
                target: TargetValue::Nothing,
                args: Args::new(),
                origin: Origin::Companion,
            },
        }
    }

    fn answer(id: &str, prompt: &str, phase: AnswerPhase, body: AnswerBody) -> AnswerFacts {
        AnswerFacts {
            prompt: prompt.to_owned(),
            wire: AnswerWire {
                task: task(id),
                phase,
                body,
                progress: Vec::new(),
                footer: FooterWire {
                    served: Vec::new(),
                    sources: Vec::new(),
                    keep: ContextKeep {
                        query: Keep::Dropped,
                        results: Keep::Dropped,
                        selection: Keep::Dropped,
                        window: Keep::Dropped,
                    },
                },
            },
            busy: Busy::Idle,
            session: None,
            routing: Vec::new(),
            shown: std::collections::BTreeMap::new(),
        }
    }

    fn entry(id: u64, state: UndoState) -> UndoEntry {
        UndoEntry {
            id: UndoId(id),
            at: UnixSeconds(1),
            actor: Actor::Cli,
            action: ActionRef {
                app: AppName::parse("org.quire.Mail").unwrap_or_else(|e| panic!("{e}")),
                name: ActionName::parse("mail.thread.archive").unwrap_or_else(|e| panic!("{e}")),
            },
            said: LabelText::parse("Archived 3 threads").unwrap_or_else(|e| panic!("{e}")),
            token: docket_core::UndoToken::parse("t1").unwrap_or_else(|e| panic!("{e}")),
            run: None,
            session: None,
            state,
        }
    }

    fn state() -> Companion {
        let draft = answer(
            "t-1",
            "reply to Sam",
            AnswerPhase::Done,
            AnswerBody::DraftReply {
                to: Vec::new(),
                subject: docket_core::Reveal::Plain("Re: lunch".to_owned()),
                body: docket_core::Reveal::Plain("Yes.".to_owned()),
                actions: vec![card("send")],
            },
        );
        let thinking = answer(
            "t-2",
            "what is on today",
            AnswerPhase::Thinking,
            AnswerBody::Text { lines: Vec::new() },
        );
        let mut companion = Companion {
            link: CompanionLink::Ready,
            front: Some(task("t-2")),
            journal: vec![
                entry(5, UndoState::Available),
                entry(4, UndoState::Undone { by: Actor::Cli }),
            ],
            ..Companion::default()
        };
        companion.answers.insert(task("t-1"), draft);
        companion.answers.insert(task("t-2"), thinking);
        companion
    }

    fn query(text: &str, scope: Scope) -> Query {
        Query {
            scope,
            ..Query::begin(text, &Cancel::new())
        }
    }

    fn rows(companion: Companion, text: &str, scope: Scope) -> Vec<Item> {
        let (_tx, rx) = watch::channel(companion);
        let provider = CompanionProvider { companion: rx };
        let mut sink = VecSink::default();
        provider.query(&query(text, scope), &mut sink);
        sink.0.into_iter().flat_map(|batch| batch.items).collect()
    }

    fn titles(items: &[Item]) -> Vec<(&str, &str)> {
        items
            .iter()
            .map(|i| (i.title.as_str(), i.subtitle.as_str()))
            .collect()
    }

    #[test]
    fn without_the_companion_there_are_no_rows() {
        let passive = Companion::default();
        assert!(rows(passive, "hello", Scope::All).is_empty());
    }

    #[test]
    fn typed_text_offers_the_ask_row_and_nothing_else_in_a_search() {
        let found = rows(state(), "what time is it", Scope::All);
        assert_eq!(titles(&found), [("Ask the companion", "what time is it")]);
        assert!(
            rows(state(), "  ", Scope::All).is_empty(),
            "nothing typed, nothing offered"
        );
    }

    #[test]
    fn the_companion_filter_lists_ask_then_the_front_answer_then_the_rest_then_activity() {
        let found = rows(state(), "hi", Scope::Only(ProviderKind::Companion));
        assert_eq!(
            titles(&found),
            [
                ("Ask the companion", "hi"),
                ("what is on today", "Thinking"),
                ("reply to Sam", "Done"),
                ("Archived 3 threads", "A terminal"),
                ("Archived 3 threads", "A terminal · Undone"),
            ]
        );
        let mut sorted = found.clone();
        sorted.sort_by_key(|item| std::cmp::Reverse(item.score));
        assert_eq!(sorted, found, "the rows are already in rank order");
    }

    #[test]
    fn an_answer_with_a_press_going_offers_no_button_and_a_refused_press_offers_them_again() {
        let step = ActionRef {
            app: AppName::parse("org.quire.Mail").unwrap_or_else(|e| panic!("{e}")),
            name: ActionName::parse("mail.message.send").unwrap_or_else(|e| panic!("{e}")),
        };
        let with_busy = |busy: Busy| {
            let mut companion = state();
            if let Some(answer) = companion.answers.get_mut(&task("t-1")) {
                answer.busy = busy;
            }
            let found = rows(companion, "hi", Scope::Only(ProviderKind::Companion));
            found
                .into_iter()
                .find(|item| item.title == "reply to Sam")
                .map(|item| item.actions.len())
        };
        let send = CardActionId::parse("send").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(with_busy(Busy::Idle), Some(1));
        assert_eq!(
            with_busy(Busy::Acting {
                card: send.clone(),
                step
            }),
            Some(0),
            "a second Enter cannot press the button again"
        );
        assert_eq!(with_busy(Busy::Refused(send)), Some(1));
    }

    #[test]
    fn enter_asks_presses_a_default_button_undoes_and_otherwise_stays() {
        let (_tx, rx) = watch::channel(Companion::default());
        let provider = CompanionProvider { companion: rx };
        let found = rows(state(), "hi", Scope::Only(ProviderKind::Companion));
        let by_title = |title: &str, at: usize| {
            found
                .iter()
                .filter(|i| i.title == title)
                .nth(at)
                .cloned()
                .unwrap_or_else(|| panic!("{title}"))
        };
        let primary = |item: &Item| provider.activate(item, &Choice::Primary);
        assert_eq!(
            primary(&by_title("Ask the companion", 0)),
            Activation::Companion(CompanionAct::Ask {
                prompt: "hi".to_owned()
            })
        );
        assert_eq!(
            primary(&by_title("reply to Sam", 0)),
            Activation::Companion(CompanionAct::Card {
                task: task("t-1"),
                action: CardActionId::parse("send").unwrap_or_else(|e| panic!("{e}")),
            }),
            "an answer's Enter is its card's default button"
        );
        assert_eq!(
            primary(&by_title("what is on today", 0)),
            Activation::Companion(CompanionAct::Mode(LauncherMode::Companion)),
            "an answer with no button leaves the panel as it is"
        );
        assert_eq!(
            primary(&by_title("Archived 3 threads", 0)),
            Activation::Companion(CompanionAct::Undo {
                scope: UndoScope::Entry(UndoId(5))
            })
        );
        let undone = by_title("Archived 3 threads", 1);
        assert!(
            undone.actions.is_empty(),
            "an undone row has nothing to undo"
        );
        assert_eq!(
            primary(&undone),
            Activation::Companion(CompanionAct::Mode(LauncherMode::Companion)),
            "Enter on it does not undo it again"
        );
    }
}
