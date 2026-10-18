//! The confirmation sheet's root.
//!
//! While the intents service's queue holds a question and the `BannerLink` names this output, the
//! surface maps itself (`SurfaceHandle::set_visibility`) and a `Ds { material: Sheet, extent: Viewport }`
//! holds one `Sheet` per question, keyed by the question's id so the next one enters again and
//! starts its arming over: the title (the action's own label), who asks and where, the arguments
//! as quire's `FactList`, the reasons, and the buttons. The buttons are quire's `Button`s: Don't
//! Allow (answers Escape), the offered second answer ("Allow from the terminal: this action,
//! until logout", or "Always Allow"), and Allow, which for a destructive action is a hold
//! ([`super::gate`]). Nothing is styled under `.ds-sheet*` or `.ds-button*`.

use dioxus::prelude::*;
use ds::base::press::Press;
use ds::components::controls::button::Button;
use ds::components::controls::button_model::{Answers, Bezel, ButtonRole};
use ds::components::overlays::sheet_attach::Attach;
use ds::components::overlays::sheet_width::SheetWidth;
use ds::focus::soon::focus_soon;
use ds::machine::MachineRef;
use ds::prelude::*;
use ds::root::common::Common;
use shell_host::OutputInfo;
use sill_model::intents::sheet::{SheetChoice, SheetFact, SheetView, sheet_view};
use sill_model::intents::{ConfirmOffer, Gesture, NeverOffer};
use sill_services::IntentsService;

use super::CONFIRM_SHEET_CSS;
use super::brief_view::{brief_body, use_brief};
use super::gate::{AnswerGate, GateIn};
use super::held::held_line;
use super::io::{use_brief_answer, use_gate, use_undo};
use super::layer::PANEL_ID;
use sill_ui_kit::hooks::{use_banner_output, use_service_part};
use sill_ui_kit::root::SurfaceRoot;
use sill_ui_kit::sheet_stage::{SheetStage, settle, use_sheet_stage};

pub(super) fn confirm_sheet(output: OutputInfo) -> Element {
    let stage = use_stage(output.clone());
    let drawn = stage().drawn().map(|(view, shown)| (view.clone(), shown));
    rsx! {
        SurfaceRoot {
            output,
            material: Material::Sheet,
            extent: RootExtent::Viewport,
            AppStyle { css: CONFIRM_SHEET_CSS }
            if let Some((view, shown)) = drawn {
                SheetCard { key: "{view.id.as_str()}", view, shown, stage }
            }
        }
    }
}

/// One question's sheet and its gate.
#[component]
fn SheetCard(view: SheetView, shown: Shown, stage: Signal<SheetStage<SheetView>>) -> Element {
    let gate = use_gate(view.id.clone());
    let undo = use_undo();
    let brief = use_brief(&view);
    let send_brief = use_brief_answer(view.id.clone());
    let state = gate.state();
    let mut focus_target = use_signal(|| None::<std::rc::Rc<MountedData>>);
    let mut landed = use_signal(|| Landed::Not);
    // Once armed, the keyboard lands on the safe button: Allow for a press, Don't Allow when
    // Allow needs a hold, so a stray Return never destroys anything. It lands once: a hold
    // abandoned on the way back to armed leaves the keyboard where the person put it.
    use_effect(move || {
        if matches!(*state.read(), AnswerGate::Armed)
            && *landed.peek() == Landed::Not
            && let Some(target) = focus_target.peek().clone()
        {
            landed.set(Landed::Once);
            focus_soon(target);
        }
    });
    let input = if state.read().takes_input() {
        Availability::Enabled
    } else {
        Availability::Disabled
    };
    let holding = matches!(*state.read(), AnswerGate::Holding { .. });
    let landing = |button: Landing| {
        if view.gesture == landing_of(button) {
            Some(EventHandler::new(move |event: MountedEvent| {
                focus_target.set(Some(event.data()));
            }))
        } else {
            None
        }
    };
    let facts: Vec<Fact> = view.facts.iter().map(fact).collect();
    let (title, byline) = (view.title.clone(), view.byline.clone());
    rsx! {
        Sheet {
            label: title.clone(),
            onclose: move |()| gate.send(GateIn::Escape),
            shown: Some(shown),
            on_hidden: move |()| settle(stage),
            attach: Attach::Centre,
            width: width_of(&view),
            common: Common {
                id: Some(PANEL_ID.to_owned()),
                ..Common::default()
            },
            div {
                class: "sill-confirm",
                // The keyboard starts on the card, so Escape denies while it arms (nothing else
                // takes focus then: the buttons are disabled). A key or a pointer move here
                // makes the sheet wait again before it arms.
                tabindex: "0",
                onmounted: move |event: MountedEvent| focus_soon(event.data()),
                onkeydown: move |event: KeyboardEvent| {
                    if event.key() == Key::Escape {
                        gate.send(GateIn::Escape);
                    } else {
                        gate.send(GateIn::Activity);
                    }
                },
                onmousemove: move |_| gate.send(GateIn::Activity),
                "data-gesture": gesture_word(view.gesture),
                "data-arm": if input == Availability::Enabled { "armed" } else { "arming" },
                div { class: "sill-confirm-title", "{title}" }
                div { class: "sill-confirm-byline", "{byline}" }
                if let Some(editable) = view.brief.clone() {
                    {brief_body(brief, &editable, gate, input, send_brief)}
                } else {
                    if !facts.is_empty() {
                        FactList { facts }
                    }
                    div { class: "sill-confirm-notes",
                        for (place , note) in view.notes.iter().enumerate() {
                            div { key: "{place}", class: "sill-confirm-note", "{note}" }
                        }
                    }
                    {held_line(&view, undo, input)}
                    {offered(&view, gate, input)}
                    div { class: "sill-confirm-footer",
                        {never(&view, gate, input)}
                        Button {
                            label: "Don't Allow",
                            answers: Answers::Escape,
                            availability: input,
                            onclick: move |_| gate.send(GateIn::Press(SheetChoice::Deny)),
                            common: Common {
                                id: Some(DENY_ID.to_owned()),
                                mounted: landing(Landing::Deny),
                                ..Common::default()
                            },
                        }
                        {allow(&view, gate, input, holding, landing(Landing::Allow))}
                    }
                }
                if view.waiting > 0 {
                    div { class: "sill-confirm-waiting", {waiting_words(view.waiting)} }
                }
            }
        }
    }
}

/// A brief needs room to be read; every other question is a narrow card.
fn width_of(view: &SheetView) -> SheetWidth {
    match view.brief {
        Some(_) => SheetWidth::Regular,
        None => SheetWidth::Narrow,
    }
}

/// The buttons' element ids (the tests', and a person's stylesheet's, handles).
const ALLOW_ID: &str = "confirm-allow";
const DENY_ID: &str = "confirm-deny";
const OFFER_ID: &str = "confirm-offer";
const NEVER_ID: &str = "confirm-never";

/// Whether the keyboard has landed on its button yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Landed {
    Not,
    Once,
}

/// Which button the keyboard lands on once armed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Landing {
    Allow,
    Deny,
}

/// The gesture whose card lands on `button`: a press lands on Allow, a hold on Don't Allow.
fn landing_of(button: Landing) -> Gesture {
    match button {
        Landing::Allow => Gesture::Press,
        Landing::Deny => Gesture::HoldToConfirm,
    }
}

fn gesture_word(gesture: Gesture) -> &'static str {
    match gesture {
        Gesture::Press => "press",
        Gesture::HoldToConfirm => "hold",
    }
}

/// The second answer the router offered, as its own button above the footer.
fn offered(view: &SheetView, gate: MachineRef<AnswerGate>, input: Availability) -> Element {
    let (label, choice) = match view.offer {
        ConfirmOffer::OnceOnly => return rsx! {},
        ConfirmOffer::OnceOrAlways => ("Always Allow", SheetChoice::AllowAlways),
        ConfirmOffer::OnceOrFromTerminal => (
            "Allow from the terminal: this action, until logout",
            SheetChoice::AllowFromTerminal,
        ),
    };
    rsx! {
        div { class: "sill-confirm-offer",
            Button {
                label,
                availability: input,
                onclick: move |_| gate.send(GateIn::Press(choice)),
                common: Common { id: Some(OFFER_ID.to_owned()), ..Common::default() },
            }
        }
    }
}

/// Never, when the request offers it: this call is refused and the action is not asked again. It
/// is neither the default button nor Escape's, answers no key of its own, and is reached by Tab
/// and pressed by Return or Space like the offer button.
fn never(view: &SheetView, gate: MachineRef<AnswerGate>, input: Availability) -> Element {
    match view.never {
        NeverOffer::Withheld => rsx! {},
        NeverOffer::Offered => rsx! {
            Button {
                label: "Never",
                availability: input,
                onclick: move |_| gate.send(GateIn::Press(SheetChoice::Never)),
                common: Common { id: Some(NEVER_ID.to_owned()), ..Common::default() },
            }
        },
    }
}

/// Allow: a press, or for a destructive action a hold on the button.
fn allow(
    view: &SheetView,
    gate: MachineRef<AnswerGate>,
    input: Availability,
    holding: bool,
    mounted: Option<EventHandler<MountedEvent>>,
) -> Element {
    match view.gesture {
        Gesture::Press => rsx! {
            Button {
                label: "Allow",
                answers: Answers::Return,
                availability: input,
                onclick: move |_| gate.send(GateIn::Press(SheetChoice::AllowOnce)),
                common: Common {
                    id: Some(ALLOW_ID.to_owned()),
                    mounted,
                    ..Common::default()
                },
            }
        },
        Gesture::HoldToConfirm => rsx! {
            div {
                class: "sill-confirm-hold",
                "data-holding": if holding { "true" } else { "false" },
                onmousedown: move |_| gate.send(GateIn::HoldBegan),
                onmouseup: move |_| gate.send(GateIn::HoldEnded),
                onmouseleave: move |_| gate.send(GateIn::HoldEnded),
                // The button keeps a Return or Space keydown for itself (it reports it as an
                // activation, and a held key does not repeat it), but a keyup reaches here: so a
                // keyboard hold begins at the button's activation and ends here.
                onkeyup: move |_| gate.send(GateIn::HoldEnded),
                Button {
                    label: if holding { "Keep holding\u{2026}" } else { "Hold to Allow" },
                    role: ButtonRole::Destructive,
                    bezel: Bezel::Push,
                    availability: input,
                    // A mouse click is the release of the hold, and only the hold answers. A key
                    // activation (Return or Space on the focused button) has no point: it is
                    // reported at the origin, and it is the keyboard's way to begin the hold.
                    onclick: move |press: Press| {
                        if press.at == Point::default() {
                            gate.send(GateIn::HoldBegan);
                        }
                    },
                    common: Common {
                        id: Some(ALLOW_ID.to_owned()),
                        mounted,
                        ..Common::default()
                    },
                }
            }
        },
    }
}

fn fact(fact: &SheetFact) -> Fact {
    let value = match &fact.source {
        None => TextLine::from(fact.value.clone()),
        Some(source) => TextLine::Runs(vec![
            TextRun::new(fact.value.clone(), RunTone::Plain),
            TextRun::new(format!("  from {source}"), RunTone::Faint),
        ]),
    };
    Fact::new(fact.label.clone(), value)
}

fn waiting_words(count: usize) -> String {
    match count {
        1 => "1 more waiting".to_owned(),
        n => format!("{n} more waiting"),
    }
}

/// The stage on `output`, following the queue and the banners' output.
fn use_stage(output: OutputInfo) -> Signal<SheetStage<SheetView>> {
    let sheets = use_service_part::<IntentsService, _>(|intents| sheet_view(&intents.sheets));
    let shows = use_banner_output();
    use_sheet_stage(move || {
        let current = sheets();
        shows
            .read()
            .shows_on(output.id)
            .then_some(current)
            .flatten()
    })
}
