//! What the polkit prompt hands quire's `PolkitPrompt`: pure.

use ds::{AvatarFace, AvatarShape, AvatarSize, AvatarTone, LockUser, PromptState, person_hue};
use sill_services::Polkit;
use sill_services::polkit_agent::{Cookie, LastTry, PromptPhase, PromptView};

/// quire's `PolkitPrompt` props that follow the request (the rest are the surface's handlers).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PolkitPromptView {
    /// A new cookie remounts the prompt (its entrance plays again).
    pub cookie: Cookie,
    /// `action`: the action's own sentence.
    pub action: String,
    /// `detail`: the action id, then polkitd's details as `key: value` lines; `None` with none.
    pub detail: Option<String>,
    /// `user`: the identity asked for, an initials face at 48.
    pub user: LockUser,
    /// `state`: `Starting` and `Checking` are `Checking` (the field closed); `Asking` is `Idle`,
    /// or `Wrong` right after a failed try (`LastTry::Wrong`: it shakes once and empties).
    pub state: PromptState,
}

/// The view of the request on screen; `None` when there is none.
pub fn prompt_view(polkit: &Polkit) -> Option<PolkitPromptView> {
    let view = polkit.current.as_ref()?;
    Some(PolkitPromptView {
        cookie: view.cookie.clone(),
        action: view.message.clone(),
        detail: detail_of(view),
        user: user_of(&view.user.0),
        state: state_of(&view.phase, view.last),
    })
}

/// The detail line block for `view` (see [`PolkitPromptView::detail`]).
pub fn detail_of(view: &PromptView) -> Option<String> {
    let lines: Vec<String> = std::iter::once(view.action_id.0.clone())
        .filter(|id| !id.is_empty())
        .chain(
            view.details
                .iter()
                .map(|(key, value)| format!("{key}: {value}")),
        )
        .collect();
    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn state_of(phase: &PromptPhase, last: LastTry) -> PromptState {
    match (phase, last) {
        (PromptPhase::Starting | PromptPhase::Checking, _) => PromptState::Checking,
        (PromptPhase::Asking { .. }, LastTry::None) => PromptState::Idle,
        (PromptPhase::Asking { .. }, LastTry::Wrong) => PromptState::Wrong,
    }
}

/// The identity asked for: the login name and an initials face (quire draws it at 48).
fn user_of(name: &str) -> LockUser {
    let initial = name
        .chars()
        .next()
        .map_or('?', |c| c.to_uppercase().next().unwrap_or(c));
    LockUser::new(
        name.to_owned(),
        AvatarFace {
            initial,
            size: AvatarSize::Size48,
            tone: AvatarTone::Person(person_hue(name)),
            shape: AvatarShape::Round,
        },
    )
}

#[cfg(test)]
mod tests {
    use ds::PromptState;
    use sill_services::Polkit;
    use sill_services::auth::UserName;
    use sill_services::polkit_agent::{
        ActionId, Cookie, Echo, LastTry, PromptPhase, PromptView, Registration,
    };

    use super::{detail_of, prompt_view};
    use crate::surfaces::user_picture::initial_of;

    fn view(phase: PromptPhase, last: LastTry) -> PromptView {
        PromptView {
            cookie: Cookie("c1".to_owned()),
            action_id: ActionId("org.example.run".to_owned()),
            message: "Run it?".to_owned(),
            icon: String::new(),
            details: vec![("program".to_owned(), "/usr/bin/true".to_owned())],
            user: UserName("ada".to_owned()),
            phase,
            last,
            notes: Vec::new(),
        }
    }

    fn polkit(view: PromptView) -> Polkit {
        Polkit {
            registration: Registration::Registered,
            current: Some(view),
            waiting: 0,
        }
    }

    #[test]
    fn the_view_follows_the_phase() {
        assert_eq!(prompt_view(&Polkit::default()), None);
        let asking = || PromptPhase::Asking {
            prompt: "Password: ".to_owned(),
            echo: Echo::Hidden,
        };
        let cases = [
            (asking(), LastTry::None, PromptState::Idle),
            (asking(), LastTry::Wrong, PromptState::Wrong),
            (PromptPhase::Starting, LastTry::Wrong, PromptState::Checking),
            (PromptPhase::Checking, LastTry::None, PromptState::Checking),
        ];
        for (phase, last, want) in cases {
            let shown =
                prompt_view(&polkit(view(phase.clone(), last))).unwrap_or_else(|| panic!("shown"));
            assert_eq!(shown.state, want, "{phase:?} {last:?}");
            assert_eq!(shown.action, "Run it?");
            assert_eq!(shown.user.name, "ada");
            assert_eq!(initial_of(&shown.user), Some('A'));
        }
    }

    #[test]
    fn the_details() {
        assert_eq!(
            detail_of(&view(PromptPhase::Starting, LastTry::None)).as_deref(),
            Some("org.example.run\nprogram: /usr/bin/true")
        );
        let mut bare = view(PromptPhase::Starting, LastTry::None);
        bare.details.clear();
        assert_eq!(detail_of(&bare).as_deref(), Some("org.example.run"));
    }
}
