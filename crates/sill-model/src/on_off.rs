//! The one two-state word a setting is stored as: `"on"` or `"off"`.

use ds_core::word::Word;
use serde::{Deserialize, Serialize};

/// A switch a setting stores as the word `on` or `off` (the dock's auto-hide, Do Not Disturb,
/// scroll momentum, ...). Each key picks its own default; there is none here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum OnOff {
    On,
    Off,
}

impl OnOff {
    /// The other state.
    pub fn toggle(self) -> OnOff {
        match self {
            OnOff::On => OnOff::Off,
            OnOff::Off => OnOff::On,
        }
    }

    /// Whether this is `On`.
    pub fn is_on(self) -> bool {
        self == OnOff::On
    }
}

impl From<bool> for OnOff {
    fn from(on: bool) -> OnOff {
        if on { OnOff::On } else { OnOff::Off }
    }
}

#[cfg(test)]
mod tests {
    use ds_core::word::Word;

    use super::OnOff;

    #[test]
    fn the_stored_words_are_on_and_off() {
        let words: Vec<&str> = OnOff::ALL.iter().map(|state| state.slug()).collect();
        assert_eq!(words, ["on", "off"]);
        assert_eq!(OnOff::On.toggle(), OnOff::Off);
        assert_eq!(OnOff::Off.toggle(), OnOff::On);
        assert_eq!(OnOff::from(true), OnOff::On);
        assert!(OnOff::On.is_on() && !OnOff::Off.is_on());
    }
}
