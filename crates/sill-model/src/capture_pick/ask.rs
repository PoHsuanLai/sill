//! What an app asked to share, parsed from the call's numbers.

/// What kinds of source the app accepts (the portal's bit set: 1 display, 2 window).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kinds {
    Screens,
    Windows,
    Both,
}

/// One source or several.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plurality {
    One,
    Many,
}

/// How the pointer appears in the shared picture (the portal's values 1, 2, 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pointer {
    Hidden,
    Embedded,
    Metadata,
}

impl Pointer {
    /// The portal's value.
    pub fn wire(self) -> u32 {
        match self {
            Pointer::Hidden => 1,
            Pointer::Embedded => 2,
            Pointer::Metadata => 4,
        }
    }

    /// The mode for a portal value; `None` for anything but exactly 1, 2 or 4.
    pub fn from_wire(value: u32) -> Option<Pointer> {
        match value {
            1 => Some(Pointer::Hidden),
            2 => Some(Pointer::Embedded),
            4 => Some(Pointer::Metadata),
            _ => None,
        }
    }
}

/// How long the person's choice is kept for the app (the call's `remembered` number).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Remembered {
    /// Asked every time.
    #[default]
    EveryTime,
    /// Kept while the app runs.
    WhileRunning,
    /// Kept until the person takes it back in Settings.
    UntilTakenBack,
}

impl Remembered {
    /// The mode for a call's number; anything unknown is asked every time.
    pub fn from_wire(value: u32) -> Remembered {
        match value {
            1 => Remembered::WhileRunning,
            2 => Remembered::UntilTakenBack,
            _ => Remembered::EveryTime,
        }
    }
}

/// Why a call cannot be asked of the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AskError {
    /// Neither screens nor windows were allowed.
    #[error("the request allows nothing to share")]
    NothingAllowed,
}

/// A request to pick what to share.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    /// The app's name as the person knows it.
    pub app_label: String,
    pub kinds: Kinds,
    pub plurality: Plurality,
    /// The pointer modes the app accepts, ascending and without repeats; never empty.
    pub pointers: Vec<Pointer>,
    /// How long the person's choice is kept.
    pub remembered: Remembered,
}

impl Ask {
    /// The ask a call's arguments make. Unknown kind bits and cursor values are ignored; no
    /// cursor mode at all means the pointer stays out of the picture.
    pub fn from_wire(
        app_label: &str,
        kinds: u32,
        multiple: u32,
        cursor_modes: &[u32],
    ) -> Result<Ask, AskError> {
        let kinds = match (kinds & 1 != 0, kinds & 2 != 0) {
            (true, true) => Kinds::Both,
            (true, false) => Kinds::Screens,
            (false, true) => Kinds::Windows,
            (false, false) => return Err(AskError::NothingAllowed),
        };
        let mut pointers: Vec<Pointer> = cursor_modes
            .iter()
            .copied()
            .filter_map(Pointer::from_wire)
            .collect();
        pointers.sort();
        pointers.dedup();
        if pointers.is_empty() {
            pointers.push(Pointer::Hidden);
        }
        Ok(Ask {
            app_label: app_label.to_owned(),
            kinds,
            plurality: if multiple == 0 {
                Plurality::One
            } else {
                Plurality::Many
            },
            pointers,
            remembered: Remembered::EveryTime,
        })
    }

    /// The same ask, with how long the choice is kept.
    pub fn remembering(self, remembered: Remembered) -> Ask {
        Ask { remembered, ..self }
    }
}
