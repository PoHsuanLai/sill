//! Editing the brief: pure functions over the text and its flagged spans. Spans are in characters
//! of the text, and the person's edits move them: a span the person typed into is dropped (it no
//! longer says what the router flagged), one before the edit stays, one after it shifts.

use super::brief::{BriefFlag, BriefSpan};

/// How many lines the field shows before its text scrolls (quire's `FieldRows::Ten`).
const FIELD_LINES: usize = 10;

/// The fewest characters per line the field is assumed to hold; counting low means a text counts
/// as needing a scroll when it may not, never the other way.
const LINE_CHARS: usize = 48;

/// How close to the end, in pixels, a scroll counts as having reached it.
const END_SLACK: f64 = 2.0;

/// Where a scrolling box stands, as its scroll event reports it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollSpot {
    pub top: f64,
    pub height: i32,
    pub client: i32,
}

/// Whether the whole text has been in view once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    Unseen,
    Seen,
}

impl Reach {
    /// The state for a text before any scroll: seen at once when it fits the field.
    pub fn of_text(text: &str) -> Reach {
        if lines_of(text) <= FIELD_LINES {
            Reach::Seen
        } else {
            Reach::Unseen
        }
    }

    /// The state after a scroll: seen once the end is in view, and seen stays seen.
    pub fn after(self, spot: ScrollSpot) -> Reach {
        let end = spot.top + f64::from(spot.client) + END_SLACK >= f64::from(spot.height);
        match (self, end) {
            (Reach::Seen, _) | (Reach::Unseen, true) => Reach::Seen,
            (Reach::Unseen, false) => Reach::Unseen,
        }
    }
}

impl Reach {
    /// The state after the text changed to `text`: a text that now fits the field has been seen.
    pub fn after_edit(self, text: &str) -> Reach {
        match (self, Reach::of_text(text)) {
            (Reach::Seen, _) | (Reach::Unseen, Reach::Seen) => Reach::Seen,
            (Reach::Unseen, Reach::Unseen) => Reach::Unseen,
        }
    }
}

/// How many lines `text` takes at the narrowest the field is assumed to be.
fn lines_of(text: &str) -> usize {
    text.split('\n')
        .map(|line| line.chars().count().div_ceil(LINE_CHARS).max(1))
        .sum()
}

/// The characters of `text` that `span` covers.
pub fn span_text(text: &str, span: BriefSpan) -> String {
    text.chars()
        .skip(span.start)
        .take(span.end.saturating_sub(span.start))
        .collect()
}

/// `text` with the span of `flag` deleted, and the other flags moved to match.
pub fn removed(text: &str, flags: &[BriefFlag], flag: BriefFlag) -> (String, Vec<BriefFlag>) {
    let next: String = text
        .chars()
        .enumerate()
        .filter(|(place, _)| *place < flag.span.start || *place >= flag.span.end)
        .map(|(_, c)| c)
        .collect();
    let kept = flags
        .iter()
        .copied()
        .filter(|f| *f != flag)
        .collect::<Vec<_>>();
    let moved = remapped(text, &next, &kept);
    (next, moved)
}

/// `flags` of `old`, as they stand in `new` after an edit: the changed region is what the two
/// texts do not share at their start and end.
pub fn remapped(old: &str, new: &str, flags: &[BriefFlag]) -> Vec<BriefFlag> {
    let (old, new): (Vec<char>, Vec<char>) = (old.chars().collect(), new.chars().collect());
    let shared = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let room = old.len().min(new.len()) - shared;
    let tail = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(room)
        .take_while(|(a, b)| a == b)
        .count();
    let after = old.len() - tail;
    flags
        .iter()
        .filter_map(|flag| {
            let BriefSpan { start, end } = flag.span;
            if end <= shared {
                Some(*flag)
            } else if start >= after {
                let shift = |at: usize| (at + new.len()).saturating_sub(old.len());
                let span = BriefSpan {
                    start: shift(start),
                    end: shift(end),
                };
                Some(BriefFlag { span, ..*flag })
            } else {
                None
            }
        })
        .collect()
}
