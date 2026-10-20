//! The sheet's root.
//!
//! While the capture service holds a permission question and the `BannerLink` names this output,
//! the surface maps itself (`SurfaceHandle::set_visibility`) and a `Ds { material: Sheet, extent:
//! Viewport }` holds one quire `Alert` per question, keyed by its id: the app's icon, the
//! headline, the app's name set strong with what it says, and the two buttons. The yes button
//! comes first, so it is the default (Return); Escape answers what `escape_answer` says (the no
//! button for a portal question, a dismissal for the compositor's). The alert answers nothing for
//! `companion.confirm_arm_ms` (`PERMISSION_ARM_AFTER` unset) after it mounts. Nothing is styled under
//! `.ds-alert*`, `.ds-sheet*` or `.ds-button*`.

use dioxus::prelude::*;
use ds::components::overlays::alert_model::{AlertButton, AlertRole};
use ds::prelude::*;
use ds::root::common::Common;
use shell_host::OutputInfo;
use sill_launcher::AppId;
use sill_model::capture_pick::{PermissionAsking, PermissionOutcome};
use sill_services::capture_pick::CaptureService;
use sill_settings::confirm_arm;
use sill_ui_kit::app_icons::{LogicalPx, use_tile_icons};
use sill_ui_kit::hooks::{use_banner_output, use_section, use_service_part};
use sill_ui_kit::root::SurfaceRoot;
use sill_ui_kit::sheet_stage::{SheetStage, settle, use_sheet_stage};

use super::io::use_end;
use super::surface::PANEL_ID;
use super::words::{app_named, escape_answer, words};

/// The app icon's side in points: quire's alert draws its hero icon at 64.
const ICON_PX: u16 = 64;

/// The buttons' element ids (the tests', and a person's stylesheet's, handles).
pub(super) const GRANT_ID: &str = "permission-grant";
pub(super) const DENY_ID: &str = "permission-deny";

pub(super) fn permission_sheet(output: OutputInfo) -> Element {
    let stage = use_stage(output.clone());
    let drawn = stage()
        .drawn()
        .map(|(asking, shown)| (asking.clone(), shown));
    rsx! {
        SurfaceRoot {
            output: output.clone(),
            material: Material::Sheet,
            extent: RootExtent::Viewport,
            if let Some((asking, shown)) = drawn {
                QuestionCard { key: "{asking.id.0}", output, asking, shown, stage }
            }
        }
    }
}

/// One question's alert.
#[component]
fn QuestionCard(
    output: OutputInfo,
    asking: PermissionAsking,
    shown: Shown,
    stage: Signal<SheetStage<PermissionAsking>>,
) -> Element {
    let end = use_end(asking.id);
    let companion = use_section(|file| &file.companion);
    let icons = use_tile_icons(&output, LogicalPx(ICON_PX));
    let said = words(&asking);
    let escape = escape_answer(asking.by);
    let ask = asking.ask;
    let icon = app_named(&icons.index, &ask.app_label).map_or_else(
        || IconSource::Glyph(Icon::Window),
        |id: &AppId| icons.source(id),
    );
    let mut runs = Vec::new();
    if let Some(lead) = said.lead.clone() {
        runs.push(TextRun::new(lead, RunTone::Strong));
    }
    if !said.detail.is_empty() {
        let text = match said.lead {
            Some(_) => format!(": {}", said.detail),
            None => said.detail.clone(),
        };
        runs.push(TextRun::new(text, RunTone::Plain));
    }
    let buttons = vec![
        AlertButton::new(
            ask.grant_label,
            AlertRole::Normal,
            EventHandler::new(move |()| end.call(PermissionOutcome::Grant)),
        )
        .with_common(Common {
            id: Some(GRANT_ID.to_owned()),
            ..Common::default()
        }),
        AlertButton::new(
            ask.deny_label,
            AlertRole::Cancel,
            EventHandler::new(move |()| end.call(PermissionOutcome::Deny)),
        )
        .with_common(Common {
            id: Some(DENY_ID.to_owned()),
            ..Common::default()
        }),
    ];
    rsx! {
        Alert {
            title: said.title,
            message: Some(TextLine::Runs(runs)),
            icon: Some(icon),
            buttons,
            // Escape is heard here, not pressed as a button: what it answers is the sheet's
            // (`escape_answer`).
            on_dismiss: Some(EventHandler::new(move |()| end.call(escape))),
            // A key or click already on its way to the window behind answers nothing.
            arm_after: Some(confirm_arm(&companion())),
            shown: Some(shown),
            on_hidden: move |()| settle(stage),
            // The sheet carries the blur region's id, so the layer's element region resolves.
            common: Common { id: Some(PANEL_ID.to_owned()), ..Common::default() },
        }
    }
}

/// The stage on `output`, following the question and the banners' output.
fn use_stage(output: OutputInfo) -> Signal<SheetStage<PermissionAsking>> {
    let asking = use_service_part::<CaptureService, _>(|picker| picker.permission.clone());
    let shows = use_banner_output();
    use_sheet_stage(super::surface::PRESET, move || {
        let current = asking();
        shows
            .read()
            .shows_on(output.id)
            .then_some(current)
            .flatten()
    })
}
