//! The consent sheet's root: one quire `Sheet` over the whole output while the voice service asks
//! on the banners' output. The title says what is being asked, one sentence says what listening
//! means, and the buttons are quire's: Not Now (Escape) and Allow (the default). Nothing is styled
//! under `.ds-sheet*` or `.ds-button*`.

use dioxus::prelude::*;
use ds::components::controls::button::Button;
use ds::components::controls::button_model::Answers;
use ds::components::overlays::sheet_attach::Attach;
use ds::components::overlays::sheet_width::SheetWidth;
use ds::focus::soon::focus_soon;
use ds::prelude::*;
use ds::root::common::Common;
use shell_host::{OutputInfo, use_watch};
use sill_model::voice::{ConsentAnswer, ConsentSheet, VoiceCommand};
use sill_services::VoiceService;

use super::VOICE_CONSENT_CSS;
use super::layer::PANEL_ID;
use sill_ui_kit::hooks::{use_control, use_service_part};
use sill_ui_kit::links::BannerLink;
use sill_ui_kit::root::SurfaceRoot;
use sill_ui_kit::sheet_stage::{SheetStage, settle, use_sheet_stage};

/// The sheet's words: what is asked, and what listening means.
const TITLE: &str = "Talk to the assistant?";
const BODY: &str = "The assistant listens only while you hold \u{2318}. What you say is turned into text on this computer and is not kept.";

/// The one thing the stage holds: the consent question is up (it has no value of its own).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Asked;

/// The buttons' element ids (the tests', and a person's stylesheet's, handles).
pub(super) const ALLOW_ID: &str = "voice-consent-allow";
pub(super) const NOT_NOW_ID: &str = "voice-consent-not-now";

pub(super) fn voice_consent(output: OutputInfo) -> Element {
    let stage = use_stage(output.clone());
    let shown = stage().drawn().map(|(_, shown)| shown);
    rsx! {
        SurfaceRoot {
            output: output.clone(),
            material: Material::Sheet,
            extent: RootExtent::Viewport,
            AppStyle { css: VOICE_CONSENT_CSS }
            if let Some(shown) = shown {
                ConsentCard { shown, stage }
            }
        }
    }
}

#[component]
fn ConsentCard(shown: Shown, stage: Signal<SheetStage<Asked>>) -> Element {
    let control = use_control::<VoiceService>();
    let answer = use_callback(move |choice: ConsentAnswer| {
        let _ = control.send(VoiceCommand::Answer(choice));
    });
    rsx! {
        Sheet {
            label: TITLE,
            onclose: move |()| answer.call(ConsentAnswer::NotNow),
            shown: Some(shown),
            on_hidden: move |()| settle(stage),
            attach: Attach::Centre,
            width: SheetWidth::Narrow,
            common: Common {
                id: Some(PANEL_ID.to_owned()),
                ..Common::default()
            },
            div {
                class: "sill-consent",
                // The keyboard starts on the card; the sheet's own close (Escape) is Not Now.
                tabindex: "0",
                onmounted: move |event: MountedEvent| focus_soon(event.data()),
                div { class: "sill-consent-title", "{TITLE}" }
                div { class: "sill-consent-body", "{BODY}" }
                div { class: "sill-consent-footer",
                    Button {
                        label: "Not Now",
                        answers: Answers::Escape,
                        onclick: move |_| answer.call(ConsentAnswer::NotNow),
                        common: Common {
                            id: Some(NOT_NOW_ID.to_owned()),
                            ..Common::default()
                        },
                    }
                    Button {
                        label: "Allow",
                        answers: Answers::Return,
                        onclick: move |_| answer.call(ConsentAnswer::Allow),
                        common: Common {
                            id: Some(ALLOW_ID.to_owned()),
                            ..Common::default()
                        },
                    }
                }
            }
        }
    }
}

/// The stage on `output`, following the service and the banners' output.
fn use_stage(output: OutputInfo) -> Signal<SheetStage<Asked>> {
    let sheet = use_service_part::<VoiceService, _>(|voice| voice.sheet);
    let link = use_hook(|| try_consume_context::<BannerLink>().unwrap_or_default());
    let shows = use_watch(link.subscribe());
    use_sheet_stage(move || {
        let asking = match shows.read().shows_on(output.id) {
            true => sheet(),
            false => ConsentSheet::Closed,
        };
        match asking {
            ConsentSheet::Asking => Some(Asked),
            ConsentSheet::Closed => None,
        }
    })
}
