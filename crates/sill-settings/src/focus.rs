//! Focus modes (design/20 §1.5: the control center's Focus / Do Not Disturb module): the state
//! the control center sets and a later notifications milestone reads.
//!
//! Stored in `notifications.dnd` (design/22 §3.12, design/13 §13.6), the one key design/22 has
//! for it: `FocusMode` is that key's reading, not a second copy of it. A mode beyond Do Not
//! Disturb (Sleep, Work) needs a key that can name it, `notifications.focus`, which would
//! replace `dnd`; until design/22 has that row there is only one mode to be in (FINDINGS "M5
//! freeze").

use serde::{Deserialize, Serialize};

use crate::file::ShellFile;
use crate::notifications::Dnd;

/// The focus mode in effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusMode {
    #[default]
    Off,
    /// No banners and no sounds, except critical ones.
    DoNotDisturb,
}

impl FocusMode {
    /// Every mode, in the control center's order.
    pub const ALL: [FocusMode; 2] = [FocusMode::Off, FocusMode::DoNotDisturb];
}

/// The mode `file` stores.
pub fn focus_mode(file: &ShellFile) -> FocusMode {
    match file.notifications.dnd {
        Dnd::On => FocusMode::DoNotDisturb,
        Dnd::Off => FocusMode::Off,
    }
}

/// `file` storing `mode`, every other key as it was.
pub fn with_focus_mode(file: ShellFile, mode: FocusMode) -> ShellFile {
    let dnd = match mode {
        FocusMode::DoNotDisturb => Dnd::On,
        FocusMode::Off => Dnd::Off,
    };
    let mut file = file;
    file.notifications.dnd = dnd;
    file
}

#[cfg(test)]
mod tests {
    use super::{FocusMode, focus_mode, with_focus_mode};
    use crate::file::ShellFile;
    use crate::notifications::Dnd;

    #[test]
    fn every_mode_round_trips_through_the_file_and_nothing_else_moves() {
        for mode in FocusMode::ALL {
            let base = ShellFile::default();
            let stored = with_focus_mode(base.clone(), mode);
            assert_eq!(focus_mode(&stored), mode, "{mode:?}");
            let back = with_focus_mode(stored, FocusMode::Off);
            assert_eq!(back, base, "{mode:?}: only notifications.dnd moved");
        }
        let mut on = ShellFile::default();
        on.notifications.dnd = Dnd::On;
        assert_eq!(focus_mode(&on), FocusMode::DoNotDisturb);
    }
}
