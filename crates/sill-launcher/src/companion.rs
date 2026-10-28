//! What a companion row asks the daemon to do, beyond running an intent: ask, press a card's
//! button, promote an answer into its app, steer a run, undo, settle a memory, switch the
//! panel's mode. Pure vocabulary; the daemon's companion service carries each out.

use almanac_core::MemoryItem;
use docket_core::UndoScope;
use docket_vocab::CardActionId;
use ds_core::vocab::MemoryVerb;
use prov::{HelperId, RunId, SessionId, TaskId};
use serde::{Deserialize, Serialize};

/// What the panel is for: finding things, or talking to the companion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LauncherMode {
    /// The search palette.
    #[default]
    Search,
    /// The field is a prompt.
    Companion,
}

/// What a person can do to a run from its row.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    ds_core::word::Word,
)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum RunControl {
    /// Look at the window it works in.
    Watch,
    /// Take the window over.
    TakeOver,
    /// Give the window back.
    HandBack,
    /// Pause it.
    Pause,
    /// Carry on.
    Resume,
    /// End it.
    Stop,
    /// Undo what it did, newest first.
    UndoAll,
}

/// What a person can do to a helper from its row.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    ds_core::word::Word,
)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum HelperControl {
    /// Stop it taking jobs.
    Pause,
    /// Let it take jobs again.
    Resume,
    /// Take it away, after asking.
    Remove,
}

/// One thing a companion row does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum CompanionAct {
    /// Send the prompt with this opening's context chips.
    Ask {
        /// What was typed.
        prompt: String,
    },
    /// A card's button.
    Card {
        /// The answer.
        task: TaskId,
        /// Which button.
        action: CardActionId,
    },
    /// Open the answer's object in its app, and close.
    Promote {
        /// The answer.
        task: TaskId,
    },
    /// Steer a run.
    Run {
        /// The run.
        run: RunId,
        /// How.
        control: RunControl,
    },
    /// Undo what was done.
    Undo {
        /// How much.
        scope: UndoScope,
    },
    /// Do something to a memory row.
    Memory {
        /// The row.
        item: MemoryItem,
        /// What.
        verb: MemoryVerb,
    },
    /// Do something to a helper.
    Helper {
        /// The helper.
        helper: HelperId,
        /// What.
        control: HelperControl,
    },
    /// Start a new helper's card.
    NewHelper,
    /// Bring a session started in a terminal back, and open its conversation.
    BringBack {
        /// The session.
        session: SessionId,
    },
    /// Switch the panel's mode in place.
    Mode(LauncherMode),
}

#[cfg(test)]
mod tests {
    use super::{CompanionAct, HelperControl, LauncherMode, RunControl};
    use almanac_core::MemoryItem;
    use docket_core::UndoScope;
    use docket_vocab::{CardActionId, UndoId};
    use ds_core::vocab::MemoryVerb;
    use prov::{RunId, TaskId};

    /// Every form's wire text, pinned: the daemon and the panel agree on these.
    #[test]
    fn every_companion_act_has_a_pinned_wire_form_and_round_trips() {
        let task = TaskId::parse("t-3").unwrap_or_else(|e| panic!("{e}"));
        let run = RunId::parse("r-9").unwrap_or_else(|e| panic!("{e}"));
        let cases: Vec<(CompanionAct, &str)> = vec![
            (
                CompanionAct::Ask {
                    prompt: "forward the receipts".to_owned(),
                },
                r#"{"kind":"ask","v":{"prompt":"forward the receipts"}}"#,
            ),
            (
                CompanionAct::Card {
                    task: task.clone(),
                    action: CardActionId::parse("send").unwrap_or_else(|e| panic!("{e}")),
                },
                r#"{"kind":"card","v":{"task":"t-3","action":"send"}}"#,
            ),
            (
                CompanionAct::Promote { task },
                r#"{"kind":"promote","v":{"task":"t-3"}}"#,
            ),
            (
                CompanionAct::Run {
                    run,
                    control: RunControl::TakeOver,
                },
                r#"{"kind":"run","v":{"run":"r-9","control":"take_over"}}"#,
            ),
            (
                CompanionAct::Undo {
                    scope: UndoScope::Entry(UndoId(41)),
                },
                r#"{"kind":"undo","v":{"scope":{"kind":"entry","v":41}}}"#,
            ),
            (
                CompanionAct::Helper {
                    helper: prov::HelperId::parse("h-2").unwrap_or_else(|e| panic!("{e}")),
                    control: HelperControl::Remove,
                },
                r#"{"kind":"helper","v":{"helper":"h-2","control":"remove"}}"#,
            ),
            (CompanionAct::NewHelper, r#"{"kind":"new_helper"}"#),
            (
                CompanionAct::BringBack {
                    session: prov::SessionId::parse("s-4").unwrap_or_else(|e| panic!("{e}")),
                },
                r#"{"kind":"bring_back","v":{"session":"s-4"}}"#,
            ),
            (
                CompanionAct::Mode(LauncherMode::Companion),
                r#"{"kind":"mode","v":"companion"}"#,
            ),
        ];
        for (act, wire) in cases {
            let written = serde_json::to_string(&act).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(written, wire, "{act:?}");
            let read: CompanionAct = serde_json::from_str(wire).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(read, act, "{wire}");
        }
    }

    #[test]
    fn a_memory_act_round_trips_with_its_row_and_verb() {
        let item = MemoryItem::Fact(almanac_core::FactId::mint(1, [7; 10]));
        let act = CompanionAct::Memory {
            item,
            verb: MemoryVerb::Forget,
        };
        let written = serde_json::to_string(&act).unwrap_or_else(|e| panic!("{e}"));
        let read: CompanionAct = serde_json::from_str(&written).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(read, act, "{written}");
    }
}
