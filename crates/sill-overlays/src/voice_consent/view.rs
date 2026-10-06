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
use shell_host::{OutputInfo, SurfaceHandle, SurfaceOp, Visibility, use_watch};
use sill_model::voice::{ConsentAnswer, ConsentSheet, VoiceCommand};
use sill_services::VoiceService;

use super::VOICE_CONSENT_CSS;
use super::layer::PANEL_ID;
use super::stage::{Mapped, Stage, StageIn, advance};
use sill_ui_kit::hooks::{use_control, use_service_part};
use sill_ui_kit::links::BannerLink;
use sill_ui_kit::root::SurfaceRoot;

/// The sheet's words: what is asked, and what listening means.
const TITLE: &str = "Talk to Companion?";
const BODY: &str = "Companion listens only while you hold \u{2318}. What you say is turned into text on this computer and is not kept.";

/// The buttons' element ids (the tests', and a person's stylesheet's, handles).
pub(super) const ALLOW_ID: &str = "voice-consent-allow";
pub(super) const NOT_NOW_ID: &str = "voice-consent-not-now";

pub(super) fn voice_consent(output: OutputInfo) -> Element {
    let stage = use_stage(output.clone());
    let shown = match stage() {
        Stage::Empty => None,
        Stage::Up => Some(Shown::Visible),
        Stage::Leaving => Some(Shown::Hidden),
    };
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
fn ConsentCard(shown: Shown, stage: Signal<Stage>) -> Element {
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

fn settle(mut stage: Signal<Stage>) {
    let next = advance(*stage.peek(), StageIn::Settled);
    if *stage.peek() != next {
        stage.set(next);
    }
}

/// The stage on `output`, following the service and the banners' output, and the surface's
/// mapping following the stage. Nothing to map without a host (a test, an SSR render).
fn use_stage(output: OutputInfo) -> Signal<Stage> {
    let sheet = use_service_part::<VoiceService, _>(|voice| voice.sheet);
    let link = use_hook(|| try_consume_context::<BannerLink>().unwrap_or_default());
    let shows = use_watch(link.subscribe());
    let mut stage = use_signal(Stage::default);
    use_effect(move || {
        let wanted = match shows.read().shows_on(output.id) {
            true => sheet(),
            false => ConsentSheet::Closed,
        };
        let next = advance(*stage.peek(), StageIn::Want(wanted));
        if *stage.peek() != next {
            stage.set(next);
        }
    });
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(Mapped::No));
    use_effect(move || {
        let mapped = stage.read().mapped();
        if *applied.peek() == mapped {
            return;
        }
        applied.set(mapped);
        if let Some(surface) = &surface {
            surface.apply(SurfaceOp::SetVisibility(match mapped {
                Mapped::Yes => Visibility::Shown,
                Mapped::No => Visibility::Hidden,
            }));
        }
    });
    stage
}
