//! What the polkit prompt shows and what its keys do: pure.

use sill_services::Polkit;
use sill_services::auth::PamNote;
use sill_services::polkit_agent::{Cookie, Echo, LastTry, PromptPhase};

/// quire's `PolkitPrompt` props, as sill computes them (sill's mirror until quire lands).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PolkitPromptView {
    pub cookie: Cookie,
    /// The identity asked for: its login name (the avatar's initials come from it).
    pub user: String,
    /// The action's sentence.
    pub message: String,
    /// The helper's prompt (`Password:` trimmed of its colon and space); `Password` while the
    /// helper starts.
    pub placeholder: String,
    pub echo: Echo,
    /// The field and Authenticate are disabled while the helper starts or checks.
    pub busy: Busy,
    /// Shake once when this is `Wrong` and the phase went back to asking.
    pub last: LastTry,
    /// The details (`action_id` first, then polkitd's details) for the `HoverCard`.
    pub details: Vec<(String, String)>,
    /// The helper's error and info lines, the newest last.
    pub notes: Vec<PamNote>,
}

/// Whether the prompt can take an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Busy {
    Ready,
    Busy,
}

/// The view of the request on screen; `None` when there is none.
pub fn prompt_view(polkit: &Polkit) -> Option<PolkitPromptView> {
    todo!("M11 fill: b-polkit ({polkit:?})")
}

/// A key the prompt acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PolkitKey {
    Enter,
    Escape,
}

/// What a key does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PolkitKeyOut {
    /// `PolkitCommand::Answer` with the field's text.
    Answer,
    /// `PolkitCommand::Dismiss`.
    Cancel,
    Nothing,
}

/// Enter answers while `Ready`, Escape always cancels (design/06).
pub fn polkit_key(key: PolkitKey, busy: Busy) -> PolkitKeyOut {
    todo!("M11 fill: b-polkit ({key:?} {busy:?})")
}

/// The helper's prompt as a placeholder: trailing `:` and whitespace trimmed; empty is
/// `Password`.
pub fn placeholder_of(phase: &PromptPhase) -> String {
    todo!("M11 fill: b-polkit ({phase:?})")
}

#[cfg(test)]
mod tests {
    use sill_services::Polkit;
    use sill_services::auth::UserName;
    use sill_services::polkit_agent::{
        ActionId, Cookie, Echo, LastTry, PromptPhase, PromptView, Registration,
    };

    use super::{Busy, PolkitKey, PolkitKeyOut, placeholder_of, polkit_key, prompt_view};

    fn polkit(phase: PromptPhase) -> Polkit {
        Polkit {
            registration: Registration::Registered,
            current: Some(PromptView {
                cookie: Cookie("c1".to_owned()),
                action_id: ActionId("org.example.run".to_owned()),
                message: "Run it?".to_owned(),
                icon: String::new(),
                details: vec![("program".to_owned(), "/usr/bin/true".to_owned())],
                user: UserName("ada".to_owned()),
                phase,
                last: LastTry::Wrong,
                notes: Vec::new(),
            }),
            waiting: 0,
        }
    }

    #[test]
    #[ignore = "M11 fill: b-polkit"]
    fn the_view_follows_the_phase() {
        assert_eq!(prompt_view(&Polkit::default()), None);
        let asking = PromptPhase::Asking {
            prompt: "Password: ".to_owned(),
            echo: Echo::Hidden,
        };
        let view = prompt_view(&polkit(asking)).unwrap_or_else(|| panic!("shown"));
        assert_eq!(view.placeholder, "Password");
        assert_eq!(view.busy, Busy::Ready);
        assert_eq!(view.last, LastTry::Wrong);
        assert_eq!(
            view.details,
            vec![
                ("action".to_owned(), "org.example.run".to_owned()),
                ("program".to_owned(), "/usr/bin/true".to_owned()),
            ]
        );
        for phase in [PromptPhase::Starting, PromptPhase::Checking] {
            let view = prompt_view(&polkit(phase)).unwrap_or_else(|| panic!("shown"));
            assert_eq!(view.busy, Busy::Busy);
        }
    }

    #[test]
    #[ignore = "M11 fill: b-polkit"]
    fn keys_and_placeholders() {
        assert_eq!(
            polkit_key(PolkitKey::Enter, Busy::Ready),
            PolkitKeyOut::Answer
        );
        assert_eq!(
            polkit_key(PolkitKey::Enter, Busy::Busy),
            PolkitKeyOut::Nothing
        );
        assert_eq!(
            polkit_key(PolkitKey::Escape, Busy::Busy),
            PolkitKeyOut::Cancel
        );
        let asking = |prompt: &str| PromptPhase::Asking {
            prompt: prompt.to_owned(),
            echo: Echo::Hidden,
        };
        assert_eq!(placeholder_of(&asking("Password: ")), "Password");
        assert_eq!(placeholder_of(&asking("Passwort:")), "Passwort");
        assert_eq!(placeholder_of(&asking("")), "Password");
        assert_eq!(placeholder_of(&PromptPhase::Starting), "Password");
    }
}
