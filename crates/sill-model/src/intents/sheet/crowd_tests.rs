//! What a crowd of rows, a very long value or a pile of questions does to the sheet: the rows are
//! counted past a few, the values are one plain line each, and the queue stops growing.

use docket_core::{ConfirmAnswer, ConfirmDetail, ConfirmEnd, ConfirmOffer, Shown};

use super::model::{SHEET_QUEUE_MAX, SHEET_ROWS_MAX, SHEET_VALUE_MAX_CHARS};
use super::{Pending, Reply, Serial, SheetEffect, SheetEvent, Sheets, sheet_view, step};
use crate::intents::tests::request;

fn sheets_with(detail: ConfirmDetail) -> Sheets {
    let mut asked = request("c-1", ConfirmOffer::OnceOnly, 120);
    asked.detail = detail;
    Sheets {
        queue: vec![Pending {
            serial: Serial(1),
            request: asked,
        }],
    }
}

#[test]
fn thousands_of_recipients_show_a_few_and_a_count_of_the_rest() {
    let who = (0..3000)
        .map(|n| Shown::Plain(format!("person{n}@example.com")))
        .collect();
    let view = sheet_view(&sheets_with(ConfirmDetail::Recipients(who)))
        .unwrap_or_else(|| panic!("a view"));
    let rows: Vec<(&str, &str)> = view
        .facts
        .iter()
        .map(|f| (f.label.as_str(), f.value.as_str()))
        .collect();
    assert_eq!(rows.len(), SHEET_ROWS_MAX + 1, "{rows:?}");
    assert_eq!(rows[0], ("To", "person0@example.com"));
    let rest = 3000 - SHEET_ROWS_MAX;
    assert_eq!(
        rows[SHEET_ROWS_MAX],
        ("To", format!("and {rest} more").as_str())
    );
}

#[test]
fn a_long_value_is_one_plain_line_that_cannot_pass_for_another_row() {
    let sneaky = format!(
        "a@example.com\nTo: b@example.com\u{202E}{}",
        "z".repeat(5000)
    );
    let view = sheet_view(&sheets_with(ConfirmDetail::Destination(Shown::Plain(
        sneaky,
    ))))
    .unwrap_or_else(|| panic!("a view"));
    let value = &view.facts[0].value;
    assert!(
        !value.contains('\n') && !value.contains('\u{202E}'),
        "{value:?}"
    );
    assert!(
        value.chars().count() <= SHEET_VALUE_MAX_CHARS + 1,
        "{} characters",
        value.chars().count()
    );
}

#[test]
fn the_queue_stops_at_its_ceiling_and_turns_the_rest_away() {
    let mut queue = Sheets::default();
    for n in 0..SHEET_QUEUE_MAX {
        let asked = request(&format!("c-{n}"), ConfirmOffer::OnceOnly, 120);
        queue = step(
            queue,
            SheetEvent::Asked(Serial(u64::try_from(n).unwrap_or(0)), Box::new(asked)),
        )
        .0;
    }
    assert_eq!(queue.queue.len(), SHEET_QUEUE_MAX);
    let extra = request("c-99", ConfirmOffer::OnceOnly, 120);
    let serial = Serial(99);
    let (queue, effects) = step(queue, SheetEvent::Asked(serial, Box::new(extra)));
    assert_eq!(queue.queue.len(), SHEET_QUEUE_MAX, "none added");
    assert_eq!(
        effects,
        vec![SheetEffect::Respond(
            serial,
            Reply::Answer(ConfirmAnswer::Ended(ConfirmEnd::Dismissed))
        )]
    );
}
