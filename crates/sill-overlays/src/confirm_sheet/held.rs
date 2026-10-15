//! The line a limit puts on the sheet: the reason it asks, in the person's words when they are
//! kept, and Undo, which takes that entry away. Undo answers nothing and leaves the sheet open:
//! the call still waits for a yes or a no, and the entry is gone from the next one on. Once the
//! service says it is gone, the question loses its line and this one with it.

use dioxus::prelude::*;
use ds::components::controls::button::Button;
use ds::prelude::*;
use ds::root::common::Common;
use sill_model::intents::sheet::SheetView;
use sill_model::intents::{StandingGrantId, UndoOffer};

/// The Undo button's element id (the tests', and a person's stylesheet's, handle).
pub(super) const UNDO_ID: &str = "confirm-undo";

/// The reason line, with Undo unless the entry is not the person's to undo (an administrator's).
pub(super) fn held_line(
    view: &SheetView,
    undo: Callback<StandingGrantId>,
    input: Availability,
) -> Element {
    let Some(held) = &view.held else {
        return rsx! {};
    };
    let (why, id) = (held.why.clone(), held.id.clone());
    rsx! {
        div { class: "sill-confirm-held",
            div { class: "sill-confirm-held-why", "{why}" }
            if held.undo == UndoOffer::Offered {
                Button {
                    label: "Undo",
                    availability: input,
                    onclick: move |_| undo.call(id.clone()),
                    common: Common { id: Some(UNDO_ID.to_owned()), ..Common::default() },
                }
            }
        }
    }
}
