//! The power profile's state and the daemon's profile words.

use serde::{Deserialize, Serialize};

use crate::units::Presence;

/// What power-profiles-daemon says, as the control center draws it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PowerProfiles {
    /// Whether the daemon answered at all.
    pub presence: Presence,
    /// `ActiveProfile`.
    pub active: Profile,
    /// `Profiles`, in the daemon's order: a machine without a performance driver offers two.
    pub offered: Vec<Profile>,
    /// `PerformanceDegraded`: why Performance is running below itself, if it is.
    pub degraded: ProfileDegraded,
}

/// One power profile, in the order the control offers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    PowerSaver,
    #[default]
    Balanced,
    Performance,
}

impl Profile {
    /// Every profile, left to right in the control.
    pub const ALL: [Profile; 3] = [Profile::PowerSaver, Profile::Balanced, Profile::Performance];

    /// The daemon's word for a profile (`power-saver`, `balanced`, `performance`); `None` for any
    /// other word. Compared whole.
    pub fn from_daemon(word: &str) -> Option<Profile> {
        match word {
            "power-saver" => Some(Profile::PowerSaver),
            "balanced" => Some(Profile::Balanced),
            "performance" => Some(Profile::Performance),
            _ => None,
        }
    }

    /// The word the daemon takes for this profile.
    pub fn daemon_word(self) -> &'static str {
        match self {
            Profile::PowerSaver => "power-saver",
            Profile::Balanced => "balanced",
            Profile::Performance => "performance",
        }
    }
}

/// `PerformanceDegraded`: empty when Performance runs fully, else the daemon's reason word
/// (`lap-detected`, `high-operating-temperature`), shown as the control's detail.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ProfileDegraded {
    #[default]
    No,
    Because(String),
}
