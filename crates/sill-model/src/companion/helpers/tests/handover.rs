//! A helper's work in a terminal: what the card offers, and what the buttons and the closing of
//! the window ask the companion.

use docket_core::helper::TerminalLogin;
use prov::SessionId;

use super::*;

fn with(runs_in: RunsIn, handover: Handover) -> HelperCardView {
    HelperCardView {
        runs_in,
        handover,
        ..card("h-1", "Research", HelperModel::Unset)
    }
}

#[test]
fn the_card_offers_the_terminal_or_says_the_work_is_in_one() {
    const CASES: &[(RunsIn, Handover, HandoverShown)] = &[
        (RunsIn::Desktop, Handover::Unavailable, HandoverShown::Nothing),
        (RunsIn::Desktop, Handover::ToTerminal, HandoverShown::OffersTerminal),
        (RunsIn::Desktop, Handover::BringBack, HandoverShown::Nothing),
        (RunsIn::Terminal, Handover::BringBack, HandoverShown::InTerminal),
        (RunsIn::Terminal, Handover::Unavailable, HandoverShown::InTerminal),
    ];
    for (runs_in, handover, shown) in CASES {
        let view = with(runs_in.clone(), handover.clone());
        assert_eq!(handover_of(&view), *shown, "{runs_in:?} {handover:?}");
    }
}

#[test]
fn the_buttons_ask_the_companion_and_the_request_opens_a_window() {
    let (_, sent) = update(HelpersModel::default(), HelpersMsg::OpenInTerminal(id("h-1")));
    assert_eq!(sent, [HelpersCommand::OpenInTerminal(id("h-1"))]);
    let (_, sent) = update(HelpersModel::default(), HelpersMsg::BringBack(id("h-1")));
    assert_eq!(sent, [HelpersCommand::BringBack(id("h-1"))]);
    let request = docket_core::helper::TerminalRequest {
        helper: id("h-1"),
        program: prov::AgentProgram::parse("claude-code").unwrap_or_else(|e| panic!("{e}")),
        login: TerminalLogin::default(),
        run: None,
        handoff: Some(SessionId::parse("s-1").unwrap_or_else(|e| panic!("{e}"))),
    };
    let (_, sent) = update(
        HelpersModel::default(),
        HelpersMsg::Replied(Asked::OpenInTerminal, HelperReply::OpenTerminal(request.clone())),
    );
    assert_eq!(sent, [HelpersCommand::OpenTerminal(request)]);
}

#[test]
fn closing_the_window_brings_the_work_back_and_says_nothing_if_it_is_already_back() {
    let (_, sent) = update(
        HelpersModel::default(),
        HelpersMsg::HandoffClosed(id("h-1")),
    );
    assert_eq!(sent, [HelpersCommand::BringBackAfterClose(id("h-1"))]);
    let model = opened("h-1", HelperModel::Unset);
    let refused = HelperReply::Refused("It is not in a terminal.".to_owned());
    let (quiet, sent) = update(
        model.clone(),
        HelpersMsg::Replied(Asked::BringBackAfterClose, refused.clone()),
    );
    assert_eq!(quiet.notice, None);
    assert_eq!(
        sent,
        [
            HelpersCommand::List { after: 0 },
            HelpersCommand::ReadCard(id("h-1"))
        ]
    );
    let (told, sent) = update(model, HelpersMsg::Replied(Asked::BringBack, refused));
    assert_eq!(told.notice.as_deref(), Some("It is not in a terminal."));
    assert!(sent.is_empty());
}
