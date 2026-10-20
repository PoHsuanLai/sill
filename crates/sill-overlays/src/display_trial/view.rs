//! The trial sheet's root.
//!
//! While the link holds a trial and the banners' output is this one, the surface maps itself
//! (`use_sheet_stage`) and a `Ds { material: Sheet, extent: Viewport }` holds one quire `Alert`,
//! keyed by the send of the trial so a restart starts the sheet over: the headline "Keep these
//! display settings?", the line "Reverting in N seconds" counting down, and two buttons. Revert
//! comes first, so it is the default (Return) and the button Escape presses; Keep Changes
//! follows. The alert answers nothing for `companion.confirm_arm_ms` (`QUESTION_ARM_AFTER` unset) after it mounts. Nothing is styled
//! under `.ds-alert*`, `.ds-sheet*` or `.ds-button*`.

use dioxus::prelude::*;
use ds::components::overlays::alert_model::{AlertButton, AlertRole};
use ds::prelude::*;
use ds::root::common::Common;
use shell_host::OutputInfo;
use sill_model::desktop::{KEEP_LABEL, REVERT_LABEL, TRIAL_TITLE, TrialAnswer, countdown_line};
use sill_settings::confirm_arm;
use sill_ui_kit::hooks::use_section;
use sill_ui_kit::links::TrialRun;
use sill_ui_kit::root::SurfaceRoot;
use sill_ui_kit::sheet_stage::{SheetStage, settle};

use super::io::{use_answer, use_left, use_stage};
use super::surface::PANEL_ID;

/// The buttons' element ids (the tests', and a person's stylesheet's, handles).
pub(super) const REVERT_ID: &str = "display-trial-revert";
pub(super) const KEEP_ID: &str = "display-trial-keep";

pub(super) fn display_trial(output: OutputInfo) -> Element {
    let stage = use_stage(&output);
    let drawn = stage().drawn().map(|(run, shown)| (*run, shown));
    rsx! {
        SurfaceRoot {
            output,
            material: Material::Sheet,
            extent: RootExtent::Viewport,
            if let Some((run, shown)) = drawn {
                TrialCard { key: "{run.serial.0}", run, shown, stage }
            }
        }
    }
}

/// One send of a trial: its alert and its countdown.
#[component]
fn TrialCard(run: TrialRun, shown: Shown, stage: Signal<SheetStage<TrialRun>>) -> Element {
    let answer = use_answer(run.trial.trial);
    let left = use_left(run.trial.seconds);
    let companion = use_section(|file| &file.companion);
    let buttons = vec![
        AlertButton::new(
            REVERT_LABEL,
            AlertRole::Cancel,
            EventHandler::new(move |()| answer.call(TrialAnswer::Revert)),
        )
        .with_common(Common {
            id: Some(REVERT_ID.to_owned()),
            ..Common::default()
        }),
        AlertButton::new(
            KEEP_LABEL,
            AlertRole::Normal,
            EventHandler::new(move |()| answer.call(TrialAnswer::Keep)),
        )
        .with_common(Common {
            id: Some(KEEP_ID.to_owned()),
            ..Common::default()
        }),
    ];
    rsx! {
        Alert {
            title: TRIAL_TITLE,
            message: Some(TextLine::from(countdown_line(left()))),
            buttons,
            // Escape, or the sheet closing another way, presses Revert (the Cancel button).
            // A key or click already on its way to the window behind answers nothing.
            arm_after: Some(confirm_arm(&companion())),
            shown: Some(shown),
            on_hidden: move |()| settle(stage),
            // The sheet carries the blur region's id, so the layer's element region resolves.
            common: Common { id: Some(PANEL_ID.to_owned()), ..Common::default() },
        }
    }
}
