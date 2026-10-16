//! The sheet's root.
//!
//! While the capture service holds a permission question and the `BannerLink` names this output,
//! the surface maps itself (`SurfaceOp::SetVisibility`) and a `Ds { material: Sheet, extent:
//! Viewport }` holds one quire `Alert` per question, keyed by its id: the app's icon, the
//! headline, the app's name set strong with what it says, and the two buttons. The yes button
//! comes first, so it is the default (Return); the no button is the one that cancels, so Escape
//! presses it. Nothing is styled under `.ds-alert*`, `.ds-sheet*` or `.ds-button*`.

use dioxus::prelude::*;
use ds::components::overlays::alert_model::{AlertButton, AlertRole};
use ds::prelude::*;
use ds::root::common::Common;
use shell_host::{OutputInfo, use_watch};
use sill_launcher::AppId;
use sill_model::capture_pick::{PermissionAsking, PermissionOutcome};
use sill_services::capture_pick::CaptureService;
use sill_ui_kit::app_icons::{LogicalPx, use_tile_icons};
use sill_ui_kit::hooks::use_service_part;
use sill_ui_kit::links::BannerLink;
use sill_ui_kit::root::SurfaceRoot;
use sill_ui_kit::sheet_stage::{SheetStage, settle, use_sheet_stage};

use super::io::use_end;
use super::layer::PANEL_ID;
use super::words::{app_named, words};

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
                QuestionCard { key: "{asking.id.0}", output: output.clone(), asking, shown, stage }
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
    let icons = use_tile_icons(&output, LogicalPx(ICON_PX));
    let ask = asking.ask;
    let said = words(&ask);
    let icon = app_named(&icons.index, &ask.app_label).map_or_else(
        || IconSource::Glyph(Icon::Window),
        |id: &AppId| icons.source(id),
    );
    let mut runs = vec![TextRun::new(said.app, RunTone::Strong)];
    if !said.detail.is_empty() {
        runs.push(TextRun::new(format!(": {}", said.detail), RunTone::Plain));
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
            shown: Some(shown),
            on_hidden: move |()| settle(stage),
            // The sheet carries the blur region's id, so `layer.rs`'s element region resolves.
            common: Common { id: Some(PANEL_ID.to_owned()), ..Common::default() },
        }
    }
}

/// The stage on `output`, following the question and the banners' output.
fn use_stage(output: OutputInfo) -> Signal<SheetStage<PermissionAsking>> {
    let asking = use_service_part::<CaptureService, _>(|picker| picker.permission.clone());
    let link = use_hook(|| try_consume_context::<BannerLink>().unwrap_or_default());
    let shows = use_watch(link.subscribe());
    use_sheet_stage(move || {
        let current = asking();
        shows
            .read()
            .shows_on(output.id)
            .then_some(current)
            .flatten()
    })
}
