//! The question's root.
//!
//! While the link holds an opening on this output the surface maps itself (`use_sheet_stage`) and
//! a `Ds { material: Sheet, extent: Viewport }` holds one quire `Alert`, keyed by the opening's
//! serial. Cancel is the Cancel button, so it is the default (Return) and what Escape presses;
//! Empty Trash is destructive. Nothing is styled under `.ds-alert*`, `.ds-sheet*` or `.ds-button*`.

use dioxus::prelude::*;
use ds::components::overlays::alert_model::{AlertButton, AlertRole};
use ds::prelude::*;
use ds::root::common::Common;
use shell_host::OutputInfo;
use sill_settings::confirm_arm;
use sill_ui_kit::hooks::use_section;
use sill_ui_kit::links::{EmptyTrashLink, EmptyTrashOpening};
use sill_ui_kit::root::SurfaceRoot;
use sill_ui_kit::sheet_stage::{SheetStage, settle};

use super::io::use_stage;
use super::surface::PANEL_ID;

/// The question, in the Mac's words.
const TITLE: &str = "Are you sure you want to permanently erase the items in the Trash?";
const MESSAGE: &str = "You can't undo this action.";

/// The buttons' element ids (the tests', and a person's stylesheet's, handles).
pub(super) const CANCEL_ID: &str = "empty-trash-cancel";
pub(super) const EMPTY_ID: &str = "empty-trash-confirm";

pub(super) fn empty_trash(output: OutputInfo) -> Element {
    let stage = use_stage(&output);
    let drawn = stage().drawn().map(|(opening, shown)| (*opening, shown));
    rsx! {
        SurfaceRoot {
            output,
            material: Material::Sheet,
            extent: RootExtent::Viewport,
            if let Some((opening, shown)) = drawn {
                Question { key: "{opening.serial.0}", shown, stage }
            }
        }
    }
}

/// One opening of the question.
#[component]
fn Question(shown: Shown, stage: Signal<SheetStage<EmptyTrashOpening>>) -> Element {
    let link = use_context::<EmptyTrashLink>();
    let companion = use_section(|file| &file.companion);
    let buttons = vec![
        AlertButton::new(
            "Cancel",
            AlertRole::Cancel,
            EventHandler::new({
                let link = link.clone();
                move |()| link.close()
            }),
        )
        .with_common(Common {
            id: Some(CANCEL_ID.to_owned()),
            ..Common::default()
        }),
        AlertButton::new(
            "Empty Trash",
            AlertRole::Destructive,
            EventHandler::new(move |()| link.confirm()),
        )
        .with_common(Common {
            id: Some(EMPTY_ID.to_owned()),
            ..Common::default()
        }),
    ];
    rsx! {
        Alert {
            title: TITLE,
            message: Some(TextLine::from(MESSAGE)),
            buttons,
            // A key or click already on its way to the window behind answers nothing.
            arm_after: Some(confirm_arm(&companion())),
            shown: Some(shown),
            on_hidden: move |()| settle(stage),
            common: Common { id: Some(PANEL_ID.to_owned()), ..Common::default() },
        }
    }
}
