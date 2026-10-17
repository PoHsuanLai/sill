use docket_core::{ConfirmAnswer, ConfirmEnd, ConfirmOffer, GrantScope, Seconds};
use prov::{Confidentiality, InputProof, UnixSeconds};

use super::{Reply, Serial, SheetChoice, SheetEffect, SheetEvent, Sheets, permits, step};
use crate::intents::tests::{id, request};

const AT: UnixSeconds = UnixSeconds(1_700_000_000);

fn asked(sheets: Sheets, serial: u64, id_text: &str, offer: ConfirmOffer) -> Sheets {
    step(
        sheets,
        SheetEvent::Asked(Serial(serial), Box::new(request(id_text, offer, 120))),
    )
    .0
}

fn answer(id_text: &str, choice: SheetChoice) -> SheetEvent {
    SheetEvent::Answered {
        id: id(id_text),
        choice,
        at: AT,
    }
}

fn ids(sheets: &Sheets) -> Vec<&str> {
    sheets.queue.iter().map(|p| p.request.id.as_str()).collect()
}

#[test]
fn what_an_offer_allows() {
    use ConfirmOffer::{OnceOnly, OnceOrAlways, OnceOrFromTerminal};
    use SheetChoice::{AllowAlways, AllowFromTerminal, AllowOnce, Deny, Never};
    #[rustfmt::skip]
    let cases = [
        (OnceOnly,           Deny,              true),
        (OnceOnly,           Never,             false),
        (OnceOnly,           AllowOnce,       true),
        (OnceOnly,           AllowAlways,     false),
        (OnceOnly,           AllowFromTerminal, false),
        (OnceOrAlways,       AllowAlways,     true),
        (OnceOrAlways,       AllowFromTerminal, false),
        (OnceOrFromTerminal, AllowFromTerminal, true),
        (OnceOrFromTerminal, AllowAlways,     false),
        (OnceOrFromTerminal, AllowOnce,       true),
        (OnceOrFromTerminal, Deny,              true),
    ];
    for (offer, choice, want) in cases {
        let asked = request("c-1", offer, 120);
        assert_eq!(permits(&asked, choice), want, "{offer:?} {choice:?}");
    }
}

#[test]
fn a_question_joins_the_queue_and_starts_its_clock() {
    let (sheets, effects) = step(
        Sheets::default(),
        SheetEvent::Asked(
            Serial(7),
            Box::new(request("c-1", ConfirmOffer::OnceOnly, 90)),
        ),
    );
    assert_eq!(ids(&sheets), ["c-1"]);
    assert_eq!(
        effects,
        vec![SheetEffect::Expire {
            serial: Serial(7),
            after: Seconds(90)
        }]
    );
}

#[test]
fn questions_wait_their_turn_first_in_first_out() {
    let sheets = asked(Sheets::default(), 1, "c-1", ConfirmOffer::OnceOnly);
    let sheets = asked(sheets, 2, "c-2", ConfirmOffer::OnceOnly);
    let sheets = asked(sheets, 3, "c-3", ConfirmOffer::OnceOnly);
    assert_eq!(sheets.current().map(|p| p.request.id.as_str()), Some("c-1"));
    assert_eq!(sheets.waiting(), 2);
    let (sheets, _) = step(sheets, answer("c-1", SheetChoice::Deny));
    assert_eq!(sheets.current().map(|p| p.request.id.as_str()), Some("c-2"));
    assert_eq!(sheets.waiting(), 1);
    assert_eq!(Sheets::default().current(), None);
    assert_eq!(Sheets::default().waiting(), 0);
}

#[test]
fn a_second_question_under_the_same_id_is_dismissed_not_shown_twice() {
    let sheets = asked(Sheets::default(), 1, "c-1", ConfirmOffer::OnceOnly);
    let (after, effects) = step(
        sheets.clone(),
        SheetEvent::Asked(
            Serial(2),
            Box::new(request("c-1", ConfirmOffer::OnceOnly, 120)),
        ),
    );
    assert_eq!(after, sheets);
    assert_eq!(
        effects,
        vec![SheetEffect::Respond(
            Serial(2),
            Reply::Answer(ConfirmAnswer::Ended(ConfirmEnd::Dismissed))
        )]
    );
}

#[test]
fn each_choice_is_the_answer_it_names_with_a_receipt_of_the_sheet() {
    use ConfirmOffer::{OnceOnly, OnceOrAlways, OnceOrFromTerminal};
    let receipt = |text: &str| prov::ConfirmReceipt {
        id: id(text),
        input: InputProof::SheetFallback,
        at: AT,
        covers: Confidentiality::Secret,
    };
    #[rustfmt::skip]
    let cases = [
        (OnceOnly,           SheetChoice::AllowOnce,
            ConfirmAnswer::Allowed { scope: GrantScope::Once, receipt: receipt("c-1") }),
        (OnceOrAlways,       SheetChoice::AllowAlways,
            ConfirmAnswer::Allowed { scope: GrantScope::Always, receipt: receipt("c-1") }),
        (OnceOrFromTerminal, SheetChoice::AllowFromTerminal,
            ConfirmAnswer::AllowedFromTerminal { receipt: receipt("c-1") }),
        (OnceOrFromTerminal, SheetChoice::Deny,
            ConfirmAnswer::Ended(ConfirmEnd::Refused)),
    ];
    for (offer, choice, want) in cases {
        let sheets = asked(Sheets::default(), 4, "c-1", offer);
        let (sheets, effects) = step(sheets, answer("c-1", choice));
        assert!(sheets.queue.is_empty(), "{choice:?}");
        assert_eq!(
            effects,
            vec![SheetEffect::Respond(Serial(4), Reply::Answer(want))],
            "{choice:?}"
        );
    }
}

#[test]
fn a_choice_the_offer_does_not_allow_or_an_unknown_id_changes_nothing() {
    let sheets = asked(Sheets::default(), 1, "c-1", ConfirmOffer::OnceOnly);
    for event in [
        answer("c-1", SheetChoice::AllowAlways),
        answer("c-1", SheetChoice::AllowFromTerminal),
        answer("c-1", SheetChoice::Never),
        answer("c-9", SheetChoice::Deny),
        SheetEvent::Withdrawn(id("c-9")),
        SheetEvent::Expired(Serial(9)),
    ] {
        let (after, effects) = step(sheets.clone(), event.clone());
        assert_eq!(after, sheets, "{event:?}");
        assert!(effects.is_empty(), "{event:?}");
    }
}

#[test]
fn a_withdrawal_answers_cancelled_and_an_expiry_answers_expired() {
    let sheets = asked(Sheets::default(), 1, "c-1", ConfirmOffer::OnceOnly);
    let sheets = asked(sheets, 2, "c-2", ConfirmOffer::OnceOnly);
    let (sheets, effects) = step(sheets, SheetEvent::Withdrawn(id("c-2")));
    assert_eq!(
        effects,
        vec![SheetEffect::Respond(Serial(2), Reply::Withdrawn)]
    );
    assert_eq!(ids(&sheets), ["c-1"]);
    let (sheets, effects) = step(sheets, SheetEvent::Expired(Serial(1)));
    assert_eq!(
        effects,
        vec![SheetEffect::Respond(
            Serial(1),
            Reply::Answer(ConfirmAnswer::Ended(ConfirmEnd::Expired))
        )]
    );
    assert!(sheets.queue.is_empty());
}

#[test]
fn an_asker_that_left_leaves_nothing_to_answer() {
    let sheets = asked(Sheets::default(), 1, "c-1", ConfirmOffer::OnceOnly);
    let (sheets, effects) = step(sheets, SheetEvent::AskerGone);
    assert!(sheets.queue.is_empty());
    assert!(effects.is_empty());
}

mod view_tests {
    use docket_core::helper::HelperName;
    use docket_core::{
        ArgLine, ConfirmDetail, ConfirmOffer, EntityRef, Gesture, LabelText, Shown, TaintNote,
    };
    use prov::{AppName, Effect, Label, Labelled, Source};

    use super::super::words::{app_words, asker_words, why_words};
    use super::super::{Pending, Serial, Sheets, sheet_view};
    use crate::intents::tests::{app, request};

    fn line(label: &str, value: Shown) -> ArgLine {
        ArgLine {
            label: LabelText::parse(label).unwrap_or_else(|e| panic!("{e}")),
            value,
        }
    }

    fn sheets(request: docket_core::ConfirmRequest) -> Sheets {
        Sheets {
            queue: vec![
                Pending {
                    serial: Serial(1),
                    request,
                },
                Pending {
                    serial: Serial(2),
                    request: self::request("c-2", ConfirmOffer::OnceOnly, 120),
                },
            ],
        }
    }

    #[test]
    fn nothing_is_drawn_without_a_question() {
        assert_eq!(sheet_view(&Sheets::default()), None);
    }

    #[test]
    fn the_title_is_the_actions_own_label_and_the_byline_names_who_and_where() {
        let view = sheet_view(&sheets(request(
            "c-1",
            ConfirmOffer::OnceOrFromTerminal,
            120,
        )))
        .unwrap_or_else(|| panic!("a view"));
        assert_eq!(view.title, "Archive a thread?");
        assert_eq!(view.byline, "The terminal wants to do this in Mail.");
        assert_eq!(view.waiting, 1);
        assert_eq!(view.offer, ConfirmOffer::OnceOrFromTerminal);
        assert_eq!(view.gesture, Gesture::Press);
    }

    #[test]
    fn a_helper_asks_by_the_name_the_person_gave_it() {
        let name = HelperName::parse("Inbox").unwrap_or_else(|e| panic!("{e}"));
        let asked = request("c-1", ConfirmOffer::OnceOnly, 120).for_helper(Some(name));
        let view = sheet_view(&sheets(asked)).unwrap_or_else(|| panic!("a view"));
        assert_eq!(view.byline, "Inbox wants to do this in Mail.");
    }

    #[test]
    fn arguments_are_facts_and_somebody_elses_words_are_quoted_with_their_source() {
        let mut asked = request("c-1", ConfirmOffer::OnceOnly, 120);
        asked.lines = vec![
            line("Folder", Shown::Plain("Receipts".into())),
            line(
                "Subject",
                Shown::Quoted {
                    text: "Invoice 7".into(),
                    from: Source::Mail,
                },
            ),
        ];
        asked.count = porter_core::Count(12);
        asked.detail = ConfirmDetail::Recipients(vec![Shown::Plain("accounts@example.com".into())]);
        let view = sheet_view(&sheets(asked)).unwrap_or_else(|| panic!("a view"));
        let shown: Vec<(&str, &str, Option<&str>)> = view
            .facts
            .iter()
            .map(|f| (f.label.as_str(), f.value.as_str(), f.source.as_deref()))
            .collect();
        assert_eq!(
            shown,
            vec![
                ("Folder", "Receipts", None),
                ("Subject", "\u{201c}Invoice 7\u{201d}", Some("a message")),
                ("How many", "12", None),
                ("To", "accounts@example.com", None),
            ]
        );
    }

    #[test]
    fn the_notes_are_the_routers_reasons_what_was_read_and_the_effect_once() {
        let mut asked = request("c-1", ConfirmOffer::OnceOnly, 120);
        asked.effect = Effect::Destructive;
        asked.why = vec![
            docket_core::AskReason::Effect(Effect::Destructive),
            docket_core::AskReason::FromTerminal,
        ];
        asked.taint = TaintNote::ReadUntrusted([Source::Web, Source::Mail].into_iter().collect());
        let view = sheet_view(&sheets(asked)).unwrap_or_else(|| panic!("a view"));
        assert_eq!(
            view.notes,
            vec![
                "It cannot be undone.",
                "It was typed in a terminal.",
                "It read something from a message, a web page.",
            ],
            "the effect is said by the reason, not again"
        );
        let mut plain = request("c-1", ConfirmOffer::OnceOnly, 120);
        plain.why = Vec::new();
        let view = sheet_view(&sheets(plain)).unwrap_or_else(|| panic!("a view"));
        assert_eq!(view.notes, vec!["It changes something you can undo."]);
    }

    #[test]
    fn entities_are_quoted_and_every_actor_has_words() {
        let mut asked = request("c-1", ConfirmOffer::OnceOnly, 120);
        let thing = EntityRef {
            id: prov::EntityId {
                app: app("org.quire.Mail"),
                kind: prov::EntityKind::parse("mail.thread").unwrap_or_else(|e| panic!("{e}")),
                key: prov::EntityKey::parse("t1").unwrap_or_else(|e| panic!("{e}")),
            },
            title: Labelled::new("Digest".to_owned(), Label::trusted_user()),
            subtitle: Labelled::new(String::new(), Label::trusted_user()),
        };
        asked.detail = ConfirmDetail::Entities(vec![thing]);
        let view = sheet_view(&sheets(asked)).unwrap_or_else(|| panic!("a view"));
        assert_eq!(view.facts[0].value, "\u{201c}Digest\u{201d}");
        assert_eq!(asker_words(&prov::Actor::Cli), "The terminal");
        assert_eq!(
            asker_words(&prov::Actor::Unknown),
            "Something on this computer"
        );
        let name: AppName = app("org.quire.Mail");
        assert_eq!(app_words(&name), "Mail");
        assert_eq!(app_words(&app("org.quire.Companion")), "Assistant");
        assert!(!why_words(&docket_core::AskReason::Mass(porter_core::Count(3))).is_empty());
    }
}
