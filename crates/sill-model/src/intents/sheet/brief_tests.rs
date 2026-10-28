use docket_core::ConfirmOffer;

use super::brief::{BriefFlag, BriefKind, BriefSpan};
use super::brief_edit::{Reach, ScrollSpot, remapped, removed, span_text};
use super::{Serial, SheetEvent, Sheets, step};
use crate::intents::tests::{id, request};
use prov::UnixSeconds;

fn flag(start: usize, end: usize, kind: BriefKind) -> BriefFlag {
    BriefFlag {
        span: BriefSpan { start, end },
        kind,
    }
}

const TEXT: &str = "Model a chair, see http://a.example now";

fn link() -> BriefFlag {
    flag(19, 35, BriefKind::Link)
}

#[test]
fn a_span_reads_back_by_characters() {
    assert_eq!(span_text(TEXT, link().span), "http://a.example");
    assert_eq!(
        span_text("caf\u{e9} \u{1f600} x", BriefSpan { start: 5, end: 6 }),
        "\u{1f600}"
    );
    assert_eq!(span_text("abc", BriefSpan { start: 2, end: 99 }), "c");
}

#[test]
fn removing_a_span_deletes_it_and_moves_the_rest() {
    let order = flag(36, 39, BriefKind::Order);
    let (text, flags) = removed(TEXT, &[link(), order], link());
    assert_eq!(text, "Model a chair, see  now");
    assert_eq!(flags, vec![flag(20, 23, BriefKind::Order)]);
    assert_eq!(span_text(&text, flags[0].span), "now");
}

#[test]
fn an_edit_before_a_span_shifts_it_and_inside_drops_it() {
    let flags = [link()];
    assert_eq!(
        remapped(TEXT, &format!("Please {TEXT}"), &flags),
        vec![flag(26, 42, BriefKind::Link)]
    );
    assert_eq!(
        remapped(TEXT, &TEXT.replace("a.example", "b.example"), &flags),
        vec![]
    );
    assert_eq!(
        remapped(TEXT, &format!("{TEXT} please"), &flags),
        flags.to_vec()
    );
}

#[test]
fn an_unchanged_text_keeps_every_flag() {
    assert_eq!(remapped(TEXT, TEXT, &[link()]), vec![link()]);
    assert_eq!(remapped("", "", &[]), vec![]);
}

fn spot(top: f64) -> ScrollSpot {
    ScrollSpot {
        top,
        height: 1000,
        client: 200,
    }
}

#[test]
fn use_this_brief_waits_for_the_end_to_be_seen() {
    let long = "word ".repeat(400);
    assert_eq!(Reach::of_text(&long), Reach::Unseen);
    assert_eq!(Reach::of_text(TEXT), Reach::Seen);
    let reach = Reach::Unseen.after(spot(300.0));
    assert_eq!(reach, Reach::Unseen);
    assert_eq!(reach.after(spot(799.0)), Reach::Seen);
    assert_eq!(Reach::Seen.after(spot(0.0)), Reach::Seen);
    assert_eq!(Reach::Unseen.after_edit("short"), Reach::Seen);
    assert_eq!(Reach::Unseen.after_edit(&long), Reach::Unseen);
}

#[test]
fn a_plain_sheet_ignores_a_brief_answer() {
    let (sheets, _) = step(
        Sheets::default(),
        SheetEvent::Asked(
            Serial(1),
            Box::new(request("c-1", ConfirmOffer::OnceOnly, 120)),
        ),
    );
    let (after, effects) = step(
        sheets.clone(),
        SheetEvent::BriefUsed {
            id: id("c-1"),
            text: "Model a chair".to_owned(),
            at: UnixSeconds(1),
        },
    );
    assert_eq!(after, sheets);
    assert!(effects.is_empty());
}

#[test]
fn a_brief_on_the_wire_is_drawn_with_its_flags_and_a_yes_carries_the_written_text() {
    use super::brief_wire::{allowed_once, brief_of};
    use docket_core::{BriefSheet, BriefText, ConfirmAnswer};
    use docket_vocab::LabelText;
    let label = |t: &str| LabelText::parse(t).unwrap_or_else(|e| panic!("{e}"));
    let plain = request("c-1", ConfirmOffer::OnceOnly, 60);
    assert_eq!(brief_of(&plain), None, "a plain sheet is no brief");
    let text = BriefText::parse(TEXT).unwrap_or_else(|e| panic!("{e}"));
    let sheet = BriefSheet::of(label("Lamp"), text, vec![label("Base"), label("Shade")]);
    let view = brief_of(&plain.clone().for_brief(Some(sheet))).unwrap_or_else(|| panic!("a brief"));
    assert_eq!(view.text, TEXT);
    assert_eq!(view.flags, vec![link()]);
    assert_eq!(view.pieces, vec!["Base".to_owned(), "Shade".to_owned()]);

    let receipt = || prov::ConfirmReceipt {
        id: id("c-1"),
        input: prov::InputProof::SheetFallback,
        at: UnixSeconds(0),
        covers: prov::Confidentiality::Secret,
    };
    let written = |answer: ConfirmAnswer| match answer {
        ConfirmAnswer::Allowed { written, .. } => written.map(|w| w.as_str().to_owned()),
        other => panic!("{other:?}"),
    };
    assert_eq!(
        written(allowed_once(receipt(), Some("Model a lamp".into()))).as_deref(),
        Some("Model a lamp")
    );
    // Text the router would refuse rides as none, so the brief ends not confirmed.
    assert_eq!(written(allowed_once(receipt(), Some(String::new()))), None);
    assert_eq!(written(allowed_once(receipt(), None)), None);
}
