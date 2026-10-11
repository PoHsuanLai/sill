//! What the person has picked, and the answer that goes back.

use super::ask::{Ask, Plurality, Pointer};
use super::plan::{Candidates, Plan, Source, Tab, plan};
use super::step::Outcome;

/// The person's picks so far.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picks {
    pub tab: Tab,
    pub chosen: Vec<Source>,
    pub pointer: Pointer,
}

impl Picks {
    /// Nothing picked, on the plan's first tab, with its first pointer mode.
    pub fn start(plan: &Plan) -> Picks {
        Picks {
            tab: plan.tabs.first().copied().unwrap_or(Tab::Screens),
            chosen: Vec::new(),
            pointer: plan.pointers.first().copied().unwrap_or(Pointer::Hidden),
        }
    }
}

/// `picks` after the person pressed `source`: with one source allowed it replaces the pick (a
/// second press of the picked one keeps it); with several it adds or removes it. A source the
/// plan does not offer changes nothing.
pub fn toggle(plan: &Plan, plurality: Plurality, picks: Picks, source: Source) -> Picks {
    if !plan.offers(&source) {
        return picks;
    }
    let chosen = match (plurality, picks.chosen.contains(&source)) {
        (Plurality::One, _) => vec![source],
        (Plurality::Many, true) => picks.chosen.into_iter().filter(|s| *s != source).collect(),
        (Plurality::Many, false) => {
            let mut chosen = picks.chosen;
            chosen.push(source);
            chosen
        }
    };
    Picks { chosen, ..picks }
}

/// Whether Allow can be pressed: something offered is picked.
pub fn can_allow(plan: &Plan, picks: &Picks) -> bool {
    picks.chosen.iter().any(|source| plan.offers(source))
}

/// The call's answer: `(kind, name)` sources and the pointer mode's value. An empty list means
/// the person cancelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub sources: Vec<(String, String)>,
    pub cursor: u32,
}

impl Reply {
    /// Cancelled.
    pub fn cancelled() -> Reply {
        Reply {
            sources: Vec::new(),
            cursor: 0,
        }
    }
}

/// The answer to Allow: the picked sources that are still offered, in the plan's order (screens
/// first), and the pointer mode when the plan offers it, else its first. Nothing offered and
/// picked is a cancel.
pub fn reply(plan: &Plan, picks: &Picks) -> Reply {
    let sources: Vec<(String, String)> = plan
        .screens
        .iter()
        .chain(&plan.windows)
        .filter(|choice| picks.chosen.contains(&choice.source))
        .map(|choice| {
            let (kind, name) = choice.source.wire();
            (kind.to_owned(), name.to_owned())
        })
        .collect();
    if sources.is_empty() {
        return Reply::cancelled();
    }
    let pointer = if plan.pointers.contains(&picks.pointer) {
        picks.pointer
    } else {
        plan.pointers.first().copied().unwrap_or(Pointer::Hidden)
    };
    Reply {
        sources,
        cursor: pointer.wire(),
    }
}

/// The reply to how the person ended a request, against what exists now.
pub(super) fn outcome_reply(ask: &Ask, candidates: &Candidates, outcome: &Outcome) -> Reply {
    match outcome {
        Outcome::Cancel => Reply::cancelled(),
        Outcome::Allow(picks) => reply(&plan(ask, candidates), picks),
    }
}
