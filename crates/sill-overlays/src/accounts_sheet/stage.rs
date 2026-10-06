//! What the surface shows, pure: the sheet accountd has on screen (the first it opened that
//! shows anything) when the banners' output is this one, and whether the surface is mapped.

use sill_model::accounts::sheet::AccountsSheets;
use sill_model::accounts::sheet::SheetHandle;

use super::props::{Step, step_of};

/// Whether the surface is mapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mapped {
    Yes,
    No,
}

/// Whether the banners' output is this one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Here {
    Yes,
    No,
}

/// One step on screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawn {
    pub handle: SheetHandle,
    pub step: Step,
}

/// What `sheets` shows on this output.
pub fn drawn(sheets: &AccountsSheets, here: Here) -> Option<Drawn> {
    match here {
        Here::No => None,
        Here::Yes => sheets.current().and_then(|sheet| {
            step_of(sheet).map(|step| Drawn {
                handle: sheet.handle.clone(),
                step,
            })
        }),
    }
}

impl Drawn {
    /// The surface is mapped exactly while there is a step to draw.
    pub fn mapped(drawn: Option<&Drawn>) -> Mapped {
        match drawn {
            Some(_) => Mapped::Yes,
            None => Mapped::No,
        }
    }
}
