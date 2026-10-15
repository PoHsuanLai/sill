//! What the companion said next to its calls while it works: which line the answer's row shows
//! and which lines the preview lists. A finished answer shows none of it.

use companion_wire::AnswerPhase;

use super::emphasis::plain_of;
use super::model::AnswerFacts;

/// How many progress lines the preview lists; older ones fold into a count.
const LISTED: usize = 3;

/// The progress lines the preview lists, and how many older ones were folded away.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProgressShown {
    /// How many older lines are not listed.
    pub earlier: usize,
    /// The newest lines, oldest first, marks and all (the preview sets their emphasis).
    pub lines: Vec<String>,
}

/// The progress lines of `answer` as the wire sent them, trimmed, the empty ones left out, but
/// only while the answer is being made.
fn working_lines(answer: &AnswerFacts) -> Vec<&str> {
    match answer.wire.phase {
        AnswerPhase::Thinking | AnswerPhase::Streaming => answer
            .wire
            .progress
            .iter()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect(),
        AnswerPhase::NeedsYou(_)
        | AnswerPhase::Done
        | AnswerPhase::Failed
        | AnswerPhase::Cancelled => Vec::new(),
    }
}

/// The newest progress line as words alone, for the row's second line; `None` when there is none
/// to show (nothing said yet, or the answer is no longer being made).
pub(super) fn row_progress(answer: &AnswerFacts) -> Option<String> {
    working_lines(answer)
        .last()
        .map(|line| plain_of(line))
        .filter(|line| !line.is_empty())
}

/// The progress lines the preview lists: the newest three, oldest first, and the count of those
/// before them. Empty once the answer is no longer being made.
pub fn preview_progress(answer: &AnswerFacts) -> ProgressShown {
    let lines = working_lines(answer);
    let earlier = lines.len().saturating_sub(LISTED);
    ProgressShown {
        earlier,
        lines: lines.into_iter().skip(earlier).map(str::to_owned).collect(),
    }
}
