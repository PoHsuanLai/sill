use companion_wire::{
    ProposalId, RoutineForm, RoutineLine, RoutineProposal, RoutineReply, RoutineShown,
    RoutinesView, WhenForm,
};
use docket_core::helper::HelperName;
use docket_core::routine::{Missed, RoutineId};
use prov::HelperId;

use super::*;

fn helper(text: &str) -> HelperId {
    HelperId::parse(text).unwrap_or_else(|e| panic!("{e}"))
}

fn routine(n: u64, owner: &str, state: RoutineShown) -> RoutineLine {
    RoutineLine {
        id: RoutineId(n),
        helper: helper(owner),
        helper_name: HelperName::parse("Research").unwrap_or_else(|e| panic!("{e}")),
        goal: "Read the news".to_owned(),
        when: "Every day at 8:00".to_owned(),
        state,
        next: None,
        last: None,
    }
}

fn feed(msgs: Vec<RoutinesMsg>) -> (RoutinesModel, Vec<RoutinesCommand>) {
    msgs.into_iter().fold(
        (RoutinesModel::default(), Vec::new()),
        |(model, mut sent), msg| {
            let (next, commands) = update(model, msg);
            sent.extend(commands);
            (next, sent)
        },
    )
}

fn listed(lines: Vec<RoutineLine>) -> RoutinesMsg {
    RoutinesMsg::Listed(RoutinesView { routines: lines })
}

#[test]
fn the_list_is_kept_and_each_helper_sees_its_own() {
    let (model, _) = feed(vec![listed(vec![
        routine(1, "h-1", RoutineShown::On),
        routine(2, "h-2", RoutineShown::Paused),
    ])]);
    let goals: Vec<RoutineId> = model.of_helper(&helper("h-2")).map(|l| l.id).collect();
    assert_eq!(goals, [RoutineId(2)]);
}

#[test]
fn a_routine_is_made_only_after_the_person_confirms_and_a_missed_time_is_skipped() {
    let (model, sent) = feed(vec![
        RoutinesMsg::OpenSheet(helper("h-1")),
        RoutinesMsg::EditGoal("  Read the news  ".to_owned()),
        RoutinesMsg::SetRepeat(Repeat::Weekdays),
        RoutinesMsg::EditTime("7:30".to_owned()),
    ]);
    assert!(sent.is_empty(), "nothing goes before the confirm");
    assert!(matches!(model.sheet, Sheet::Adding(_)));
    let (_, sent) = update(model, RoutinesMsg::Confirm);
    let want = RoutineForm {
        helper: helper("h-1"),
        goal: "Read the news".to_owned(),
        when: WhenForm::At("weekdays 7:30".to_owned()),
        missed: Missed::Skip,
        steps: 12,
    };
    assert_eq!(sent, [RoutinesCommand::Add(Box::new(want))]);
}

#[test]
fn what_is_missing_from_the_sheet_is_said_and_nothing_is_sent() {
    // (goal, repeat, time, hours, sentence)
    let cases = [
        ("", Repeat::Daily, "08:00", "2", "Say what it is to do."),
        (
            "Read",
            Repeat::Daily,
            "late",
            "2",
            "Say a time of day like 08:00.",
        ),
        (
            "Read",
            Repeat::EveryFewHours,
            "08:00",
            "40",
            "Say a number of hours from 1 to 12.",
        ),
    ];
    for (goal, repeat, time, hours, sentence) in cases {
        let (model, sent) = feed(vec![
            RoutinesMsg::OpenSheet(helper("h-1")),
            RoutinesMsg::EditGoal(goal.to_owned()),
            RoutinesMsg::SetRepeat(repeat),
            RoutinesMsg::EditTime(time.to_owned()),
            RoutinesMsg::EditHours(hours.to_owned()),
            RoutinesMsg::Confirm,
        ]);
        assert!(sent.is_empty(), "{sentence}");
        assert_eq!(model.notice.as_deref(), Some(sentence));
    }
}

#[test]
fn an_answer_closes_the_sheet_and_reads_the_list_and_a_refusal_keeps_it_open() {
    let open = feed(vec![RoutinesMsg::OpenSheet(helper("h-1"))]).0;
    let (made, sent) = update(
        open.clone(),
        RoutinesMsg::Replied(RoutineReply::Made(RoutineId(3))),
    );
    assert_eq!(
        (made.sheet, sent),
        (Sheet::Closed, vec![RoutinesCommand::List])
    );
    let (refused, sent) = update(
        open.clone(),
        RoutinesMsg::Replied(RoutineReply::Refused("That helper is paused.".to_owned())),
    );
    assert_eq!(refused.sheet, open.sheet);
    assert!(sent.is_empty());
    assert_eq!(refused.notice.as_deref(), Some("That helper is paused."));
}

#[test]
fn pausing_and_resuming_go_straight_and_removing_asks_first() {
    let (model, sent) = feed(vec![
        listed(vec![routine(1, "h-1", RoutineShown::On)]),
        RoutinesMsg::Pause(RoutineId(1)),
        RoutinesMsg::Resume(RoutineId(1)),
        RoutinesMsg::Remove(RoutineId(1)),
    ]);
    assert_eq!(
        sent,
        [
            RoutinesCommand::Pause(RoutineId(1)),
            RoutinesCommand::Resume(RoutineId(1))
        ],
        "a removal nobody asked about is not done"
    );
    let (model, _) = update(model, RoutinesMsg::AskRemove(RoutineId(1)));
    assert_eq!(model.removal, RoutineRemoval::Asked(RoutineId(1)));
    let (model, sent) = update(model, RoutinesMsg::Remove(RoutineId(1)));
    assert_eq!(sent, [RoutinesCommand::Remove(RoutineId(1))]);
    assert_eq!(model.removal, RoutineRemoval::Unasked);
}

#[test]
fn a_question_about_a_routine_that_went_is_dropped() {
    let (model, _) = feed(vec![
        listed(vec![routine(1, "h-1", RoutineShown::On)]),
        RoutinesMsg::AskRemove(RoutineId(1)),
        listed(Vec::new()),
    ]);
    assert_eq!(model.removal, RoutineRemoval::Unasked);
}

#[test]
fn the_words_say_how_a_routine_stands_and_that_a_missed_run_is_skipped() {
    assert_eq!(
        MISSED_SKIPPED,
        "If your computer is off at that time, the run is skipped."
    );
    assert_eq!(state_label(RoutineShown::Paused), "Paused");
    assert_eq!(last_run_line(&routine(1, "h-1", RoutineShown::On)), None);
    assert_eq!(repeat_label(Repeat::Weekdays), "Weekdays");
}

fn proposal(n: u64, owner: &str, when: WhenForm) -> RoutineProposal {
    RoutineProposal {
        id: ProposalId(n),
        helper: helper(owner),
        form: RoutineForm {
            helper: helper(owner),
            goal: "Read the news".to_owned(),
            when,
            missed: Missed::Skip,
            steps: 12,
        },
        why: "You asked about the news twice.".to_owned(),
    }
}

#[test]
fn a_suggestion_makes_nothing_until_accepted_and_not_now_forgets_it() {
    let (model, sent) = feed(vec![
        RoutinesMsg::Proposed(vec![
            proposal(1, "h-1", WhenForm::At("daily 08:00".to_owned())),
            proposal(2, "h-2", WhenForm::At("weekdays 07:30".to_owned())),
        ]),
        RoutinesMsg::AcceptProposal(ProposalId(9)),
    ]);
    assert!(
        sent.is_empty(),
        "nothing is made, and an unknown one is ignored"
    );
    let mine: Vec<ProposalId> = model.proposed_to(&helper("h-2")).map(|p| p.id).collect();
    assert_eq!(mine, [ProposalId(2)]);
    let (accepting, sent) = update(model.clone(), RoutinesMsg::AcceptProposal(ProposalId(1)));
    assert_eq!(sent, [RoutinesCommand::AcceptProposal(ProposalId(1))]);
    let (_, sent) = update(
        accepting,
        RoutinesMsg::Accepted(RoutineReply::Made(RoutineId(4))),
    );
    assert_eq!(
        sent,
        [RoutinesCommand::List, RoutinesCommand::ListProposals]
    );
    let (declined, sent) = update(model, RoutinesMsg::DeclineProposal(ProposalId(1)));
    assert_eq!(sent, [RoutinesCommand::DeclineProposal(ProposalId(1))]);
    assert_eq!(declined.proposals.len(), 1);
}

#[test]
fn a_refused_acceptance_says_why() {
    let (model, _) = feed(vec![RoutinesMsg::Accepted(RoutineReply::Refused(
        "That helper is paused.".to_owned(),
    ))]);
    assert_eq!(model.notice.as_deref(), Some("That helper is paused."));
}

#[test]
fn a_suggestion_is_worded_plainly_and_its_reason_is_quoted() {
    let daily = proposal(1, "h-1", WhenForm::At("daily 08:00".to_owned()));
    assert_eq!(proposal_when(&daily), "Every day at 08:00");
    assert_eq!(proposal_what(&daily), "Read the news");
    assert_eq!(
        proposal_why(&daily).as_deref(),
        Some("\u{201c}You asked about the news twice.\u{201d}")
    );
    let weekdays = proposal(2, "h-1", WhenForm::At("weekdays 07:30".to_owned()));
    assert_eq!(proposal_when(&weekdays), "Weekdays at 07:30");
    let hours = proposal(3, "h-1", WhenForm::At("every 2 hours".to_owned()));
    assert_eq!(proposal_when(&hours), "Every 2 hours");
    let silent = RoutineProposal {
        why: "  ".to_owned(),
        ..daily
    };
    assert_eq!(proposal_why(&silent), None);
}
