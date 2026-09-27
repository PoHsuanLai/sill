//! What the last edit in Edit Widgets came to, said under the gallery (FINDINGS F1025; design/26
//! moments): an add is a Success, its check drawn on beside "Added World Clock to the Desktop"
//! (quire's `CheckMark` through `use_settle`, held `SettleHold`, then the words stay: R8); a
//! removal is a Change, its words only; a refusal is a Failure, its words shaken once (quire's
//! `use_shake`, R6). Nothing plays at rest, on the panel's opening, or on a re-render (R1, R3).
//! The reference's gallery shows an add by the widget arriving on the desktop; sill's panel
//! stands over the desktop's top right, where new widgets land (Q521), so the words say it here
//! too.

use dioxus::prelude::*;
use ds::IconSize;
use ds::detail::{
    CheckMark, Detailed, FirstShow, Moment, SettleStyle, Touch, use_detail, use_settle, use_shake,
};
use ds::widget::{WidgetEdit, WidgetLayout};

use crate::widgets::layout::Refused;
use crate::widgets::{WidgetHost, WidgetKind, kind_of, registry};

/// Which edit of this opening a line is about: the same words twice are two moments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(super) struct EditCount(pub u32);

impl EditCount {
    /// The next edit's count.
    pub fn next(self) -> EditCount {
        EditCount(self.0.saturating_add(1))
    }
}

/// One line and the edit it is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Said {
    pub words: String,
    pub edit: EditCount,
}

/// What the last edit came to.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) enum Outcome {
    /// Nothing edited since the panel opened.
    #[default]
    Quiet,
    /// A widget was added.
    Added(Said),
    /// A widget was taken away.
    Removed(Said),
    /// The edit was not made, and why.
    Refused(Said),
}

impl Outcome {
    fn said(&self) -> Option<&Said> {
        match self {
            Outcome::Quiet => None,
            Outcome::Added(said) | Outcome::Removed(said) | Outcome::Refused(said) => Some(said),
        }
    }

    /// The `data-outcome` word.
    fn slug(&self) -> &'static str {
        match self {
            Outcome::Quiet => "quiet",
            Outcome::Added(_) => "added",
            Outcome::Removed(_) => "removed",
            Outcome::Refused(_) => "refused",
        }
    }
}

impl Detailed for Outcome {
    fn moment(_from: &Self, to: &Self) -> Moment {
        match to {
            Outcome::Quiet => Moment::Rest,
            Outcome::Added(_) => Moment::Success,
            Outcome::Removed(_) => Moment::Change,
            Outcome::Refused(_) => Moment::Failure,
        }
    }

    fn first(state: &Self) -> Moment {
        match state {
            Outcome::Quiet | Outcome::Added(_) | Outcome::Removed(_) | Outcome::Refused(_) => {
                Moment::Rest
            }
        }
    }
}

/// What `edit`, made on `before`, came to, as the `count`th edit of this opening: `None` for an
/// edit that says nothing (a move).
pub(super) fn outcome_of(
    edit: &WidgetEdit,
    before: &WidgetLayout,
    made: Result<(), &Refused>,
    count: EditCount,
) -> Option<Outcome> {
    let said = |words: String| Said { words, edit: count };
    match (made, edit) {
        (Err(why), _) => Some(Outcome::Refused(said(why.words().to_owned()))),
        (Ok(()), WidgetEdit::Add { kind, host, .. }) => {
            let name = kind_of(kind).map_or_else(|| kind.as_str().to_owned(), name_of);
            Some(Outcome::Added(said(format!(
                "Added {name} to {}.",
                host_words(*host)
            ))))
        }
        (Ok(()), WidgetEdit::Remove(id)) => {
            let item = before.items().iter().find(|item| item.id == *id)?;
            let name = kind_of(&item.kind).map_or_else(|| item.kind.as_str().to_owned(), name_of);
            Some(Outcome::Removed(said(format!(
                "Removed {name} from {}.",
                host_words(item.at.host())
            ))))
        }
        (Ok(()), WidgetEdit::Resize(..) | WidgetEdit::Move(..)) => None,
    }
}

/// The widget's name as the gallery lists it.
fn name_of(kind: WidgetKind) -> String {
    registry()
        .get(&crate::widgets::kind_id(kind))
        .map_or_else(|| format!("{kind:?}"), |info| info.name.plain_text())
}

/// The surface as the gallery's headings name it.
fn host_words(host: WidgetHost) -> &'static str {
    match host {
        WidgetHost::Desktop => "the Desktop",
        WidgetHost::Tile => "the Notification Center",
    }
}

/// The line under the gallery for `outcome`; nothing while it is quiet.
#[component]
pub(super) fn EditStatus(outcome: Outcome) -> Element {
    let detail = use_detail(outcome.clone(), FirstShow::Still, Touch::Remote);
    let settling = use_settle(detail.cue(), SettleStyle::Check);
    let shake = use_shake(detail.cue()).attrs();
    let Some(said) = outcome.said() else {
        return rsx! {};
    };
    let (class, alias) = match shake {
        Some((anim, alias)) => (format!("sill-ew-status {anim}"), Some(alias)),
        None => ("sill-ew-status".to_owned(), None),
    };
    rsx! {
        div { class, "data-pulse": alias, "data-outcome": outcome.slug(), role: "status",
            span { class: "sill-ew-status-mark",
                CheckMark { settling, size: IconSize::Compact }
            }
            span { class: "sill-ew-status-words", "{said.words}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use ds::catalog::PlacementId;
    use ds::detail::{Moment, moment_table};
    use ds::widget::{LayoutError, Order, WidgetAt, WidgetEdit, WidgetLayout};
    use ds::{WidgetHost, WidgetSize};

    use super::{EditCount, Outcome, Said, outcome_of};
    use crate::widgets::layout::Refused;
    use crate::widgets::{WidgetKind, kind_id};

    fn said(words: &str, edit: u32) -> Said {
        Said {
            words: words.to_owned(),
            edit: EditCount(edit),
        }
    }

    #[test]
    fn an_add_succeeds_a_removal_changes_a_refusal_fails() {
        moment_table(&[
            (
                Outcome::Quiet,
                Outcome::Added(said("a", 1)),
                Moment::Success,
            ),
            (
                Outcome::Added(said("a", 1)),
                Outcome::Added(said("a", 2)),
                Moment::Success,
            ),
            (
                Outcome::Added(said("a", 1)),
                Outcome::Removed(said("b", 2)),
                Moment::Change,
            ),
            (
                Outcome::Removed(said("b", 2)),
                Outcome::Refused(said("c", 3)),
                Moment::Failure,
            ),
        ]);
    }

    #[test]
    fn each_edit_says_what_it_did() {
        let (before, _) = WidgetLayout::default().added(
            kind_id(WidgetKind::UpNext),
            WidgetSize::Medium,
            WidgetAt::Center(Order(0)),
        );
        let add = |kind, host| WidgetEdit::Add {
            kind: kind_id(kind),
            size: WidgetSize::Medium,
            host,
        };
        let full = Refused::Layout(LayoutError::Full);
        let cases = [
            (
                add(WidgetKind::WorldClock, WidgetHost::Desktop),
                Ok(()),
                Some(Outcome::Added(said("Added World Clock to the Desktop.", 4))),
            ),
            (
                add(WidgetKind::Battery, WidgetHost::Tile),
                Ok(()),
                Some(Outcome::Added(said(
                    "Added Batteries to the Notification Center.",
                    4,
                ))),
            ),
            (
                WidgetEdit::Remove(PlacementId(1)),
                Ok(()),
                Some(Outcome::Removed(said(
                    "Removed Up Next from the Notification Center.",
                    4,
                ))),
            ),
            (
                add(WidgetKind::Calendar, WidgetHost::Desktop),
                Err(&full),
                Some(Outcome::Refused(said(
                    "There is no room on the desktop for it.",
                    4,
                ))),
            ),
            (
                WidgetEdit::Move(PlacementId(1), WidgetAt::Center(Order(0))),
                Ok(()),
                None,
            ),
        ];
        for (edit, made, want) in cases {
            assert_eq!(
                outcome_of(&edit, &before, made, EditCount(4)),
                want,
                "{edit:?}"
            );
        }
    }
}
