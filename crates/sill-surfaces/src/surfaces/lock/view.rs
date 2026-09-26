//! What the lock screen hands quire's `LockClock` and `LockPrompt`, from the auth service's
//! state, the keyboard and the clock: pure.

use dioxus::prelude::Modifiers;
use ds::{
    AvatarFace, AvatarShape, AvatarSize, AvatarTone, CapsLock, ImageSource, LockUser, PromptState,
    UserPicture, person_hue,
};
use sill_services::AuthPhase;
use sill_services::auth::{Account, AvatarSource, UserName};
use sill_settings::LockClock;

/// Caps Lock from the modifiers of the last key event the surface saw (a lock surface has the
/// keyboard, so it sees them; shell-host's `modifiers_from` sets `CAPS_LOCK` from xkb's locked
/// state). Before the first key the state is not known (G180): `Off`.
pub fn caps_lock(modifiers: Modifiers) -> CapsLock {
    if modifiers.contains(Modifiers::CAPS_LOCK) {
        CapsLock::On
    } else {
        CapsLock::Off
    }
}

/// Caps Lock from shell-host's live modifiers (`use_modifiers()`, published on keyboard focus
/// and on every change, so it is known before the first key: G180 closed upstream). The surface
/// uses this; [`caps_lock`] reads a key event's modifiers.
pub fn caps_of(key: shell_host::LockKey) -> CapsLock {
    match key {
        shell_host::LockKey::Locked => CapsLock::On,
        shell_host::LockKey::Unlocked => CapsLock::Off,
    }
}

/// quire's prompt state for `phase` at `now` (whose zone words the time): `Accepted` is quire's
/// `Accepted` (its beat plays before the unlock), `Unavailable` is `Idle` (the service's notes are the
/// hint line), `LockedOut { until }` is quire's `LockedOut` with `until` as the lock clock
/// words a time (`14:07`, or `2:07` in `H12`), and one whose `until` has passed is `Idle`.
pub fn prompt_state(phase: &AuthPhase, now: &jiff::Zoned, format: LockClock) -> PromptState {
    match phase {
        AuthPhase::Idle | AuthPhase::Unavailable(_) => PromptState::Idle,
        AuthPhase::Checking => PromptState::Checking,
        AuthPhase::Accepted => PromptState::Accepted,
        AuthPhase::Wrong => PromptState::Wrong,
        AuthPhase::LockedOut { until } if until.0 > now.timestamp().as_millisecond() => {
            jiff::Timestamp::from_millisecond(until.0).map_or(PromptState::Idle, |at| {
                PromptState::LockedOut {
                    until: time_text(&at.to_zoned(now.time_zone().clone()), format),
                }
            })
        }
        AuthPhase::LockedOut { .. } => PromptState::Idle,
    }
}

/// The person as quire's prompt shows them: the real name (the login name when it is empty),
/// and their picture ([`picture_of`]).
pub fn lock_user(account: &Account) -> LockUser {
    let name = if account.name.0.trim().is_empty() {
        account.user.0.clone()
    } else {
        account.name.0.clone()
    };
    let picture = picture_of(account, &name);
    LockUser::new(name, picture)
}

/// The person's picture: their photo when the account has one (AccountsService's icon, else
/// `$HOME/.face`, read by the auth service: `sill_services::auth::avatar_source`), which quire
/// crops round; else the initial of `name` on their hue, at 64. The one place a picture is
/// chosen; a persona is not offered (the user prefers an animated emoji, a later change).
fn picture_of(account: &Account, name: &str) -> UserPicture {
    match &account.avatar {
        AvatarSource::File(path) => ImageSource::file(path).map_or_else(
            |_| UserPicture::from(face_of(name, &account.user)),
            UserPicture::Photo,
        ),
        AvatarSource::Initials => UserPicture::from(face_of(name, &account.user)),
    }
}

/// The initial on the person's hue.
fn face_of(name: &str, user: &UserName) -> AvatarFace {
    AvatarFace {
        initial: name
            .chars()
            .next()
            .map_or('?', |first| first.to_uppercase().next().unwrap_or(first)),
        size: AvatarSize::Size64,
        tone: AvatarTone::Person(person_hue(&user.0)),
        shape: AvatarShape::Round,
    }
}

/// The clock's two lines, for quire's `LockClock`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ClockLines {
    /// `14:05` (`H24`) or `2:05` (`H12`, no AM/PM, macOS's lock screen).
    pub time: String,
    /// `Saturday, 26 September` (macOS: the weekday and the date above the time).
    pub date: String,
}

/// `now` as the lock screen's clock.
pub fn clock_lines(now: &jiff::Zoned, format: LockClock) -> ClockLines {
    ClockLines {
        time: time_text(now, format),
        date: format!(
            "{}, {} {}",
            now.strftime("%A"),
            now.day(),
            now.strftime("%B")
        ),
    }
}

/// The time as the lock clock words it: `14:05`, or `2:05` (no AM/PM).
fn time_text(at: &jiff::Zoned, format: LockClock) -> String {
    let hour = at.hour();
    match format {
        LockClock::H24 => format!("{hour:02}:{:02}", at.minute()),
        LockClock::H12 => format!("{}:{:02}", (hour + 11) % 12 + 1, at.minute()),
    }
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::Modifiers;
    use ds::UserPicture;
    use ds::{CapsLock, PromptState};
    use sill_services::AuthPhase;
    use sill_services::auth::{Account, AuthFault, AvatarSource, RealName, UserName};
    use sill_services::notifications::UnixMs;
    use sill_settings::LockClock;

    use super::{ClockLines, caps_lock, caps_of, clock_lines, lock_user, prompt_state};

    fn at(text: &str) -> jiff::Zoned {
        text.parse::<jiff::Zoned>()
            .unwrap_or_else(|e| panic!("{text}: {e}"))
    }

    #[test]
    fn caps_lock_from_the_modifiers() {
        let cases = [
            (Modifiers::empty(), CapsLock::Off),
            (Modifiers::SHIFT, CapsLock::Off),
            (Modifiers::CAPS_LOCK, CapsLock::On),
            (Modifiers::CAPS_LOCK | Modifiers::SHIFT, CapsLock::On),
        ];
        for (modifiers, want) in cases {
            assert_eq!(caps_lock(modifiers), want, "{modifiers:?}");
        }
        assert_eq!(caps_of(shell_host::LockKey::Locked), CapsLock::On);
        assert_eq!(caps_of(shell_host::LockKey::Unlocked), CapsLock::Off);
    }

    #[test]
    fn prompt_states() {
        let now = at("2026-09-26T14:05:00+08:00[Asia/Taipei]");
        let ms = |z: &jiff::Zoned| UnixMs(z.timestamp().as_millisecond());
        let later = at("2026-09-26T14:07:00+08:00[Asia/Taipei]");
        let cases = [
            (AuthPhase::Idle, LockClock::H24, PromptState::Idle),
            (AuthPhase::Checking, LockClock::H24, PromptState::Checking),
            (AuthPhase::Wrong, LockClock::H24, PromptState::Wrong),
            (AuthPhase::Accepted, LockClock::H24, PromptState::Accepted),
            (
                AuthPhase::Unavailable(AuthFault::TimedOut),
                LockClock::H24,
                PromptState::Idle,
            ),
            (
                AuthPhase::LockedOut { until: ms(&later) },
                LockClock::H24,
                PromptState::LockedOut {
                    until: "14:07".to_owned(),
                },
            ),
            (
                AuthPhase::LockedOut { until: ms(&later) },
                LockClock::H12,
                PromptState::LockedOut {
                    until: "2:07".to_owned(),
                },
            ),
            (
                AuthPhase::LockedOut { until: ms(&now) },
                LockClock::H24,
                PromptState::Idle,
            ),
        ];
        for (phase, format, want) in cases {
            assert_eq!(prompt_state(&phase, &now, format), want, "{phase:?}");
        }
    }

    #[test]
    fn the_user_is_named_and_initialled() {
        let account = |user: &str, name: &str| Account {
            user: UserName(user.to_owned()),
            name: RealName(name.to_owned()),
            ..Account::default()
        };
        let ada = lock_user(&account("ada", "Ada Lovelace"));
        assert_eq!(ada.name, "Ada Lovelace");
        assert_eq!(initial(&ada), Some('A'));
        let bob = lock_user(&account("bob", ""));
        assert_eq!(bob.name, "bob");
        assert_eq!(initial(&bob), Some('B'));
        let photo = lock_user(&Account {
            avatar: AvatarSource::File("/home/ada/.face".into()),
            ..account("ada", "Ada Lovelace")
        });
        assert_eq!(
            photo.picture,
            UserPicture::Photo(ds::ImageSource("file:///home/ada/.face".to_owned()))
        );
    }

    fn initial(user: &ds::LockUser) -> Option<char> {
        match &user.picture {
            UserPicture::Face(face) => Some(face.initial),
            _ => None,
        }
    }

    #[test]
    fn clock_lines_in_both_formats() {
        let lines = |time: &str, date: &str| ClockLines {
            time: time.to_owned(),
            date: date.to_owned(),
        };
        let cases = [
            (
                "2026-09-26T14:05:00+08:00[Asia/Taipei]",
                LockClock::H24,
                lines("14:05", "Saturday, 26 September"),
            ),
            (
                "2026-09-26T14:05:00+08:00[Asia/Taipei]",
                LockClock::H12,
                lines("2:05", "Saturday, 26 September"),
            ),
            (
                "2026-01-01T00:07:00+00:00[UTC]",
                LockClock::H12,
                lines("12:07", "Thursday, 1 January"),
            ),
            (
                "2026-01-01T09:07:00+00:00[UTC]",
                LockClock::H24,
                lines("09:07", "Thursday, 1 January"),
            ),
        ];
        for (text, format, want) in cases {
            assert_eq!(clock_lines(&at(text), format), want, "{text} {format:?}");
        }
    }
}
