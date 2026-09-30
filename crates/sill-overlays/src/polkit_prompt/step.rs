//! What the prompt's surface shows, pure: the request the agent wants on this output, kept a
//! moment past its going so quire's sheet can play its exit, and whether the surface is mapped.
//! A new request replaces a leaving one at once (keyed by its cookie, it enters again).

use ds::prelude::*;

use super::model::PolkitPromptView;

/// The surface's stage.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Stage {
    /// Nothing drawn; the surface is unmapped.
    #[default]
    Empty,
    /// The request is up.
    Up(PolkitPromptView),
    /// The request has gone; its sheet is playing its exit.
    Leaving(PolkitPromptView),
}

/// What reaches the stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageIn {
    /// The request the agent shows on this output now (`None`: none, or another output).
    Want(Option<PolkitPromptView>),
    /// The sheet's exit settled (quire's `on_hidden`).
    Settled,
}

/// Whether the surface is mapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mapped {
    Yes,
    No,
}

/// `stage` after `input`.
pub fn advance(stage: Stage, input: StageIn) -> Stage {
    match (stage, input) {
        (_, StageIn::Want(Some(view))) => Stage::Up(view),
        (Stage::Up(view), StageIn::Want(None)) => Stage::Leaving(view),
        (Stage::Leaving(_), StageIn::Settled) => Stage::Empty,
        (stage, StageIn::Want(None) | StageIn::Settled) => stage,
    }
}

impl Stage {
    /// Whether the surface is mapped: from the first frame of a request to its exit's settle.
    pub fn mapped(&self) -> Mapped {
        match self {
            Stage::Empty => Mapped::No,
            Stage::Up(_) | Stage::Leaving(_) => Mapped::Yes,
        }
    }

    /// The view drawn and quire's `shown` for it.
    pub fn drawn(&self) -> Option<(&PolkitPromptView, Shown)> {
        match self {
            Stage::Empty => None,
            Stage::Up(view) => Some((view, Shown::Visible)),
            Stage::Leaving(view) => Some((view, Shown::Hidden)),
        }
    }
}

#[cfg(test)]
mod tests {
    use ds::components::content::avatar::{
        AvatarFace, AvatarShape, AvatarSize, AvatarTone, person_hue,
    };
    use ds::prelude::*;
    use ds_shell::lock::vocab::{LockUser, PromptState};
    use sill_model::polkit_agent::Cookie;

    use super::{Mapped, Stage, StageIn, advance};
    use crate::polkit_prompt::model::PolkitPromptView;

    fn view(cookie: &str, state: PromptState) -> PolkitPromptView {
        PolkitPromptView {
            cookie: Cookie(cookie.to_owned()),
            action: "Run it?".to_owned(),
            detail: None,
            user: LockUser::new(
                "ada".to_owned(),
                AvatarFace {
                    initial: 'A',
                    size: AvatarSize::Size48,
                    tone: AvatarTone::Person(person_hue("ada")),
                    shape: AvatarShape::Round,
                },
            ),
            state,
        }
    }

    #[test]
    fn a_request_enters_leaves_and_unmaps_once_settled() {
        let a = || view("a", PromptState::Idle);
        let a_wrong = || view("a", PromptState::Wrong);
        let b = || view("b", PromptState::Checking);
        let cases = [
            (Stage::Empty, StageIn::Want(Some(a())), Stage::Up(a())),
            (
                Stage::Up(a()),
                StageIn::Want(Some(a_wrong())),
                Stage::Up(a_wrong()),
            ),
            (Stage::Up(a()), StageIn::Want(None), Stage::Leaving(a())),
            (
                Stage::Leaving(a()),
                StageIn::Want(None),
                Stage::Leaving(a()),
            ),
            (Stage::Leaving(a()), StageIn::Settled, Stage::Empty),
            (
                Stage::Leaving(a()),
                StageIn::Want(Some(b())),
                Stage::Up(b()),
            ),
            (Stage::Up(a()), StageIn::Settled, Stage::Up(a())),
            (Stage::Empty, StageIn::Want(None), Stage::Empty),
            (Stage::Empty, StageIn::Settled, Stage::Empty),
        ];
        for (from, input, want) in cases {
            assert_eq!(
                advance(from.clone(), input.clone()),
                want,
                "{from:?} {input:?}"
            );
        }
    }

    #[test]
    fn mapped_from_the_request_to_the_settle() {
        assert_eq!(Stage::Empty.mapped(), Mapped::No);
        assert_eq!(
            Stage::Up(view("a", PromptState::Idle)).mapped(),
            Mapped::Yes
        );
        assert_eq!(
            Stage::Leaving(view("a", PromptState::Idle)).mapped(),
            Mapped::Yes
        );
        let shown = |stage: &Stage| stage.drawn().map(|(_, shown)| shown);
        assert_eq!(shown(&Stage::Empty), None);
        assert_eq!(
            shown(&Stage::Up(view("a", PromptState::Idle))),
            Some(Shown::Visible)
        );
        assert_eq!(
            shown(&Stage::Leaving(view("a", PromptState::Idle))),
            Some(Shown::Hidden)
        );
    }
}
