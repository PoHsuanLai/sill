//! The words a refusal and a time wear on a card.

use docket_core::{ArgFault, BudgetKind, CallRefusal, ConfirmEnd, DenyCode, Reveal, TimeRange};
use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::ViewCx;

/// A refused call, in a sentence a person can act on. Total: every refusal has words.
pub fn refusal_words(refusal: &CallRefusal) -> String {
    match refusal {
        CallRefusal::App(_) => "The app said no.".to_owned(),
        CallRefusal::Denied(code) => format!("Not allowed ({}).", deny_words(code)),
        CallRefusal::Unconfirmed(end) => match end {
            ConfirmEnd::Refused => "You said no.".to_owned(),
            ConfirmEnd::Dismissed => "It was dismissed.".to_owned(),
            ConfirmEnd::Expired => "It was not answered in time.".to_owned(),
            ConfirmEnd::Cancelled => "It was withdrawn.".to_owned(),
        },
        CallRefusal::Halted(_) => "Everything is paused.".to_owned(),
        CallRefusal::Paused(_) => "Paused until you say so.".to_owned(),
        CallRefusal::OverBudget(kind) => format!("Over its budget ({}).", budget_words(*kind)),
        CallRefusal::NoSuchAction(_) => "The app has no such action.".to_owned(),
        CallRefusal::BadArgs { why, .. } => format!("A detail is wrong ({}).", arg_words(*why)),
        CallRefusal::AppUnavailable(_) => "The app is not running.".to_owned(),
        CallRefusal::Timeout => "The app did not answer in time.".to_owned(),
    }
}

fn deny_words(code: &DenyCode) -> &'static str {
    match code {
        DenyCode::OutsideTask => "outside what you asked",
        DenyCode::NotAllowed => "not for an agent to do",
        DenyCode::NeedsUser => "needs you",
        DenyCode::Repeated => "refused before",
    }
}

fn budget_words(kind: BudgetKind) -> &'static str {
    match kind {
        BudgetKind::Calls => "calls",
        BudgetKind::Writes => "writes",
        BudgetKind::Outbound => "outbound actions",
        BudgetKind::Destructive => "destructive actions",
        BudgetKind::Rate => "calls a minute",
        BudgetKind::FanOut => "things at once",
        BudgetKind::Chain => "steps in a chain",
        BudgetKind::Wall => "time",
        BudgetKind::Reviews => "reviews",
        BudgetKind::Spend => "spend",
    }
}

fn arg_words(fault: ArgFault) -> &'static str {
    match fault {
        ArgFault::Missing => "missing",
        ArgFault::WrongType => "wrong kind",
        ArgFault::OutOfRange => "out of range",
        ArgFault::UnknownHandle => "something it no longer holds",
        ArgFault::CrossSpace => "from another Space",
    }
}

/// The plain text of a revealed string: the text itself, what the router resolved a handle to,
/// or an ellipsis until it has.
pub(super) fn plain(text: &Reveal<String>, cx: &ViewCx<'_>) -> String {
    match text {
        Reveal::Plain(text) => text.clone(),
        Reveal::Handle(handle) => (cx.display)(*handle).unwrap_or_else(|| "…".to_owned()),
    }
}

/// When a stretch of time is, in words: `Tue 3 Nov, 14:00 to 15:00` in the person's zone.
pub(super) fn when(range: &TimeRange, zone: &TimeZone) -> String {
    let at = |seconds: prov::UnixSeconds| {
        Timestamp::from_second(seconds.0)
            .ok()
            .map(|stamp| stamp.to_zoned(zone.clone()))
    };
    match (at(range.from), at(range.to)) {
        (Some(from), Some(to)) => format!(
            "{}, {} to {}",
            from.strftime("%a %-d %b"),
            from.strftime("%H:%M"),
            to.strftime("%H:%M")
        ),
        _ => String::new(),
    }
}
