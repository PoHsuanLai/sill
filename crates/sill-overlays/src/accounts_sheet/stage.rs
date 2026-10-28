//! What the surface shows, pure: the sheet accountd has on screen (the first it opened that
//! shows anything) when the banners' output is this one, and whether the surface is mapped.

use sill_model::accounts::sheet::AccountsSheets;
use sill_model::accounts::sheet::SheetHandle;
use sill_ui_kit::sheet_stage::Mapped;

use super::props::{Step, step_of};

/// One step on screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawn {
    pub handle: SheetHandle,
    pub step: Step,
}

/// What `sheets` shows on this output, `here` being whether the banners' output is this one.
pub fn drawn(sheets: &AccountsSheets, here: bool) -> Option<Drawn> {
    if !here {
        return None;
    }
    sheets.current().and_then(|sheet| {
        step_of(sheet).map(|step| Drawn {
            handle: sheet.handle.clone(),
            step,
        })
    })
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
