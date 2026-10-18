//! The brief sheet's body (design `brief-confirm.md` section 2): the words helpers would work
//! from, in a field the person edits, the spans the router flagged listed with a Remove each, the
//! pieces beneath, and the two buttons. It replaces the facts, the notes and the allow choices of
//! the plain sheet; the title, the arming and Escape are the card's.
//!
//! "Use this brief" takes the field's text, byte for byte, and stays disabled until the whole
//! text has been in view once ([`Reach`]), so a long brief cannot be confirmed unread. There is no
//! Always, no Never and no tick to say it was read: the control is that the person is shown the
//! exact text that becomes the instruction, with the dangerous parts hard to miss and cheap to
//! remove.

use dioxus::prelude::*;
use ds::components::controls::button::Button;
use ds::components::controls::button_model::Answers;
use ds::components::fields::text_field_model::FieldRows;
use ds::machine::MachineRef;
use ds::prelude::*;
use ds::root::common::Common;
use sill_model::intents::sheet::{
    BRIEF_MAX_BYTES, BriefFlag, BriefView, Reach, ScrollSpot, SheetChoice, SheetView, remapped,
    removed, span_text,
};
use sill_model::untrusted::for_display;

use super::gate::{AnswerGate, GateIn};

/// The buttons' element ids (the tests', and a person's stylesheet's, handles).
pub(super) const USE_ID: &str = "confirm-brief-use";
pub(super) const CANCEL_ID: &str = "confirm-brief-cancel";
const FIELD_ID: &str = "confirm-brief-text";

/// How many characters of one flagged span the list draws.
const FLAG_SHOWN_CHARS: usize = 80;

/// The brief as the person has it so far: the field, the flags that still apply to it, and
/// whether the whole of it has been in view.
#[derive(Clone, Copy)]
pub(super) struct BriefState {
    text: Signal<String>,
    flags: Signal<Vec<BriefFlag>>,
    reach: Signal<Reach>,
}

/// The brief state for `view`'s card: empty for a plain sheet. Called every time the card draws,
/// so the hooks run in the same order whatever the sheet is.
pub(super) fn use_brief(view: &SheetView) -> BriefState {
    let start = view.brief.clone();
    let text = use_signal(|| start.as_ref().map(|b| b.text.clone()).unwrap_or_default());
    let flags = use_signal(|| start.as_ref().map(|b| b.flags.clone()).unwrap_or_default());
    let reach = use_signal(|| Reach::of_text(&text.peek()));
    BriefState { text, flags, reach }
}

/// Whether "Use this brief" can be pressed: the card takes input, the text has been in view, and
/// there are words that fit the bound.
fn usable(state: BriefState, input: Availability) -> Availability {
    let text = state.text.read();
    let fine = !text.trim().is_empty() && text.len() <= BRIEF_MAX_BYTES;
    match (input, *state.reach.read(), fine) {
        (Availability::Enabled, Reach::Seen, true) => Availability::Enabled,
        _ => Availability::Disabled,
    }
}

/// The body of the brief sheet. `send` carries the text out as the answer.
pub(super) fn brief_body(
    state: BriefState,
    brief: &BriefView,
    gate: MachineRef<AnswerGate>,
    input: Availability,
    send: Callback<String>,
) -> Element {
    let BriefState {
        mut text,
        mut flags,
        mut reach,
    } = state;
    let use_it = usable(state, input);
    let pieces = brief.pieces.clone();
    let too_long = text.read().len() > BRIEF_MAX_BYTES;
    rsx! {
        div { class: "sill-brief-hint", "Edit it until it says what you want. Helpers will do what it says." }
        div {
            class: "sill-brief-field",
            onscroll: move |event: ScrollEvent| {
                let spot = ScrollSpot {
                    top: event.scroll_top(),
                    height: event.scroll_height(),
                    client: event.client_height(),
                };
                reach.set(reach().after(spot));
            },
            TextField {
                label: "The brief",
                kind: FieldKind::Multiline,
                rows: FieldRows::Ten,
                value: text(),
                availability: input,
                oninput: move |next: String| {
                    let moved = remapped(&text.peek(), &next, &flags.peek());
                    flags.set(moved);
                    reach.set(reach().after_edit(&next));
                    text.set(next);
                },
                common: Common { id: Some(FIELD_ID.to_owned()), ..Common::default() },
            }
        }
        if too_long {
            div { class: "sill-brief-note", "This is longer than helpers can take. Shorten it." }
        }
        {flag_list(state, input)}
        if !pieces.is_empty() {
            div { class: "sill-brief-pieces",
                div { class: "sill-brief-heading", "Pieces" }
                for (place , piece) in pieces.iter().enumerate() {
                    div { key: "{place}", class: "sill-brief-piece", "{piece}" }
                }
            }
        }
        div { class: "sill-brief-next", "Next: helpers will work from this." }
        if *reach.read() == Reach::Unseen {
            div { class: "sill-brief-note", "Scroll to the end to use it." }
        }
        div { class: "sill-confirm-footer",
            Button {
                label: "Cancel",
                answers: Answers::Escape,
                availability: input,
                onclick: move |_| gate.send(GateIn::Press(SheetChoice::Deny)),
                common: Common { id: Some(CANCEL_ID.to_owned()), ..Common::default() },
            }
            Button {
                label: "Use this brief",
                answers: Answers::Return,
                availability: use_it,
                onclick: move |_| send.call(text.peek().clone()),
                common: Common { id: Some(USE_ID.to_owned()), ..Common::default() },
            }
        }
    }
}

/// "Look at these": each flagged span underlined, why it is listed, and a Remove that deletes it
/// from the field.
fn flag_list(state: BriefState, input: Availability) -> Element {
    let BriefState {
        mut text,
        mut flags,
        mut reach,
    } = state;
    let rows: Vec<(BriefFlag, String)> = flags
        .read()
        .iter()
        .map(|flag| {
            let shown = for_display(&span_text(&text.read(), flag.span), FLAG_SHOWN_CHARS);
            (*flag, shown)
        })
        .collect();
    if rows.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "sill-brief-flags",
            div { class: "sill-brief-heading", "Look at these" }
            for (place , (flag , shown)) in rows.into_iter().enumerate() {
                div { key: "{place}-{shown}", class: "sill-brief-flag",
                    div { class: "sill-brief-flag-what",
                        Label { text: TextLine::Runs(vec![TextRun::new(shown, RunTone::Underline)]) }
                        div { class: "sill-brief-flag-why", "{flag.kind.sentence()}" }
                    }
                    Button {
                        label: "Remove",
                        availability: input,
                        onclick: move |_| {
                            let (next, kept) = removed(&text.peek(), &flags.peek(), flag);
                            reach.set(reach().after_edit(&next));
                            text.set(next);
                            flags.set(kept);
                        },
                    }
                }
            }
        }
    }
}
