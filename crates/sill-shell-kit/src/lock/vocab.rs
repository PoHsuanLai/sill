//! What the lock screen and the polkit prompt say about a password being asked for
//! (design/20-SURFACES.md sections 1.9 and 1.10; design/04-COMPONENTS.md section 42): whose it
//! is, how the asking is going, whether caps lock is on, and where the Space's colour reaches.

use crate::user_picture::picture::UserPicture;
use ds::base::vocab::Availability;
use ds::base::word::Word;
use ds::motion::detail::{detailed::Detailed, moment::Moment};

/// Where the current Space's colour reaches on the lock screen. The reference lock screen is
/// white type and a white glass field over the wallpaper; Arc's colour may take the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
pub enum LockLook {
    /// White type, a flat white glass field (`--lock-ink`, `--lock-glass`).
    #[default]
    Clear,
    /// The Space's colour on the small surfaces: the date line sits in a pill and the password
    /// field is painted with the Space gradient, in the Space's frame ink (`--f-grad`,
    /// `--f-ink`). The time stays white.
    Space,
}

/// How the asking is going. The caller moves it: `Checking` while the password is tried,
/// `Wrong` when it failed (the field shakes once and clears), `LockedOut` after too many tries,
/// `Accepted` once it was right (the picture plays its accept beat before the host unlocks).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum PromptState {
    /// Waiting for a password.
    #[default]
    Idle,
    /// The password is being tried: the field holds still and the enter button spins.
    Checking,
    /// It was wrong: the field plays `shake-x` once (420 ms, `--e-shake`), then empties itself.
    Wrong,
    /// Too many tries: the field is closed until `until`, the caller's own wording of the time
    /// ("9:52"), which the hint line shows as "Try again at 9:52".
    LockedOut {
        /// When it opens again, as the caller writes the time.
        until: String,
    },
    /// It was right: the field stays closed while the lock screen goes away. The picture plays
    /// Happy (the accept beat, `settle(Anim::PictureAccept)`, and an emoji's partying face);
    /// unlock the session at that settle.
    Accepted,
}

impl PromptState {
    /// The `data-state` word.
    pub(crate) fn slug(&self) -> &'static str {
        match self {
            PromptState::Idle => "idle",
            PromptState::Checking => "checking",
            PromptState::Wrong => "wrong",
            PromptState::LockedOut { .. } => "locked-out",
            PromptState::Accepted => "accepted",
        }
    }

    /// Whether the field takes typing: not while the password is tried or the prompt is shut.
    pub(crate) fn availability(&self) -> Availability {
        match self {
            PromptState::Idle | PromptState::Wrong => Availability::Enabled,
            PromptState::Checking | PromptState::LockedOut { .. } | PromptState::Accepted => {
                Availability::Disabled
            }
        }
    }

    /// Whether this is the failed state, the one that shakes.
    pub(crate) fn is_wrong(&self) -> bool {
        matches!(self, PromptState::Wrong)
    }
}

impl Detailed for PromptState {
    /// Checking is the pending try (its arrow spins, bounded, R4); arriving at Wrong is the
    /// failure (the field shakes once, R6); a lock-out closes the prompt; Accepted is the success (the
    /// picture's accept beat); anything else is a Change.
    fn moment(from: &Self, to: &Self) -> Moment {
        match (from, to) {
            (_, PromptState::Checking) => Moment::Pending,
            (_, PromptState::Wrong) => Moment::Failure,
            (_, PromptState::LockedOut { .. }) => Moment::Unavailable,
            (_, PromptState::Accepted) => Moment::Success,
            (
                PromptState::Idle
                | PromptState::Checking
                | PromptState::Wrong
                | PromptState::LockedOut { .. }
                | PromptState::Accepted,
                PromptState::Idle,
            ) => Moment::Change,
        }
    }

    /// A prompt shown mid-try is an operation already running; otherwise it is simply there.
    fn first(state: &Self) -> Moment {
        match state {
            PromptState::Checking => Moment::Pending,
            PromptState::Idle
            | PromptState::Wrong
            | PromptState::LockedOut { .. }
            | PromptState::Accepted => Moment::Rest,
        }
    }
}

/// Whether caps lock is on, which the field marks beside its button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CapsLock {
    /// On: the caps-lock arrow shows in the field.
    On,
    /// Off.
    #[default]
    Off,
}

/// The person the password belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LockUser {
    /// Their name, as the session shows it.
    pub name: String,
    /// Their picture: a letter disc, an animated emoji or their photo. The prompt draws it at its
    /// own size (64 at the lock screen, 48 in a polkit prompt), whatever size a face carries.
    pub picture: UserPicture,
}

impl LockUser {
    /// The person called `name`, shown by `picture`: an `AvatarFace`, an `EmojiId` or an
    /// `ImageSource` (their photo).
    pub fn new(name: impl Into<String>, picture: impl Into<UserPicture>) -> Self {
        LockUser {
            name: name.into(),
            picture: picture.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PromptState;
    use ds::base::vocab::Availability;

    #[test]
    fn only_idle_and_wrong_take_typing() {
        let cases = [
            (PromptState::Idle, Availability::Enabled, "idle"),
            (PromptState::Wrong, Availability::Enabled, "wrong"),
            (PromptState::Checking, Availability::Disabled, "checking"),
            (
                PromptState::LockedOut {
                    until: "9:52".into(),
                },
                Availability::Disabled,
                "locked-out",
            ),
            (PromptState::Accepted, Availability::Disabled, "accepted"),
        ];
        for (state, availability, slug) in cases {
            assert_eq!(state.availability(), availability, "{state:?}");
            assert_eq!(state.slug(), slug, "{state:?}");
        }
    }
}
