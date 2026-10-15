//! The launcher in companion mode on the fake services: the answer's card in the pane, the keys
//! the mode adds (Escape's two meanings, command-Enter, command-period), the switch of mode in
//! place, and what an Enter on the Ask row sends. The companion service is a fake: its state is
//! seeded and its commands are read back, so what these check is the panel's.

use crate::launcher_harness;

use std::time::Duration;

use docket_core::Keep;
use ds_harness::{Driver, Query};
use launcher_harness::launcher_support::companion::{
    HAIKU, companion, declined, draft, fresh, routed, task, thinking,
};
use launcher_harness::{Open, Press, is_open, open, open_in, selected_title, titles};
use sill_launcher::{LauncherMode, ProviderKind, WindowKey};
use sill_model::companion::{CompanionCommand, LauncherOrigin, Summon};
use sill_model::results::SearchCommand;

fn commands(open: &mut Open) -> Vec<CompanionCommand> {
    let mut seen = Vec::new();
    while let Ok(command) = open.feeds.bar.companion_commands.try_recv() {
        seen.push(command);
    }
    seen
}

fn last_search(open: &Open) -> SearchCommand {
    open.feeds
        .searches
        .lock()
        .unwrap_or_else(|e| panic!("{e}"))
        .last()
        .cloned()
        .unwrap_or_else(|| panic!("no search was sent"))
}

fn settled(open: &mut Open) {
    open.harness.advance(Duration::from_millis(50));
}

#[test]
fn answer_card_in_aside() {
    let open = open_in("", LauncherMode::Companion, |bar| {
        bar.companion
            .send_replace(companion(vec![draft()], fresh()));
    });
    let h = &open.harness;
    assert_eq!(titles(h), ["reply to Sam"], "the answer is the row");
    assert_eq!(
        h.count(".ds-palette-aside .ds-answer-card"),
        1,
        "its card is the palette's aside, with nothing to open"
    );
    assert!(
        h.html().contains("Send"),
        "the card's button is the pane's action: {}",
        h.html()
    );
}

const HAIKU_LINE: &str = "Answered by Claude Haiku 4.5 via OpenRouter (already loaded)";

#[test]
fn the_card_footer_says_how_the_answer_was_reached_in_the_wire_crates_words() {
    let open = open_in("", LauncherMode::Companion, |bar| {
        bar.companion
            .send_replace(companion(vec![routed(draft(), HAIKU)], fresh()));
    });
    let h = &open.harness;
    assert_eq!(h.count(".ds-palette-aside .sill-route-line"), 1);
    assert_eq!(
        h.text_of(".sill-route-line").as_deref(),
        Some(HAIKU_LINE),
        "exactly the line companion_wire::footer_line prints"
    );
}

#[test]
fn an_answer_with_no_notes_has_no_footer_line() {
    let open = open_in("", LauncherMode::Companion, |bar| {
        bar.companion
            .send_replace(companion(vec![draft()], fresh()));
    });
    assert_eq!(open.harness.count(".ds-answer-card"), 1);
    assert_eq!(open.harness.count(".sill-route-line"), 0);
}

#[test]
fn the_footer_line_follows_the_notes_as_they_change() {
    let mut open = open_in("", LauncherMode::Companion, |bar| {
        bar.companion
            .send_replace(companion(vec![draft()], fresh()));
    });
    assert_eq!(open.harness.count(".sill-route-line"), 0);
    open.feeds
        .bar
        .companion
        .send_replace(companion(vec![routed(draft(), HAIKU)], fresh()));
    settled(&mut open);
    assert_eq!(
        open.harness.text_of(".sill-route-line").as_deref(),
        Some(HAIKU_LINE)
    );
}

#[test]
fn a_declined_named_model_shows_its_reason_not_a_generic_error() {
    let why = "Kimi K2.6 cannot answer: it is not installed on this computer.";
    let open = open_in("", LauncherMode::Companion, |bar| {
        bar.companion
            .send_replace(companion(vec![declined(why)], fresh()));
    });
    let html = open.harness.html();
    assert!(html.contains(why), "the card says why: {html}");
}

#[test]
fn an_answer_row_is_selected_into_the_pane_and_a_search_row_is_not() {
    let mut open = open_in("hi", LauncherMode::Companion, |bar| {
        bar.companion
            .send_replace(companion(vec![draft()], fresh()));
    });
    assert_eq!(
        selected_title(&open.harness).as_deref(),
        Some("Ask the companion")
    );
    assert_eq!(
        open.harness.count(".ds-answer-card"),
        0,
        "composing: no card yet"
    );
    open.press(Press::Down);
    assert_eq!(
        selected_title(&open.harness).as_deref(),
        Some("reply to Sam")
    );
    assert_eq!(open.harness.count(".ds-palette-aside .ds-answer-card"), 1);
}

#[test]
fn double_tap_in_open_launcher_switches_mode_in_place() {
    let mut open = open("fire");
    let serial = open.link.now().serial();
    assert!(titles(&open.harness).contains(&"Firefox".to_owned()));
    open.link.switch_mode(LauncherMode::Companion);
    settled(&mut open);
    assert_eq!(
        open.link.now().serial(),
        serial,
        "the same opening: no remount"
    );
    assert_eq!(
        last_search(&open),
        SearchCommand::Filtered {
            text: String::new(),
            provider: ProviderKind::Companion,
        },
        "the field emptied and the rows are the companion's"
    );
    assert!(
        !titles(&open.harness).contains(&"Firefox".to_owned()),
        "the search's rows went"
    );
    // Back out: the search it came from returns, query and rows.
    open.link.switch_mode(LauncherMode::Search);
    settled(&mut open);
    assert_eq!(open.link.now().serial(), serial);
    assert_eq!(last_search(&open), SearchCommand::Query("fire".to_owned()));
    assert!(titles(&open.harness).contains(&"Firefox".to_owned()));
}

#[test]
fn escape_in_the_prompt_asks_the_summon_to_dismiss_and_closes_a_panel_nobody_summoned() {
    let mut summoned = open_in("", LauncherMode::Companion, |bar| {
        bar.companion.send_replace(companion(vec![], fresh()));
    });
    summoned.press(Press::Escape);
    assert_eq!(commands(&mut summoned), [CompanionCommand::Dismiss]);
    assert!(
        is_open(&summoned.link),
        "the summon decides what Escape closes"
    );

    let mut alone = open_in("", LauncherMode::Companion, |_| {});
    alone.press(Press::Escape);
    settled(&mut alone);
    assert!(
        !is_open(&alone.link),
        "nothing summoned it: Escape closes it"
    );
}

#[test]
fn escape_in_an_answer_goes_back_to_the_ask_row() {
    let mut open = open_in("hi", LauncherMode::Companion, |bar| {
        bar.companion
            .send_replace(companion(vec![draft()], fresh()));
    });
    open.press(Press::Down);
    assert_eq!(
        selected_title(&open.harness).as_deref(),
        Some("reply to Sam")
    );
    open.press(Press::Escape);
    assert_eq!(
        selected_title(&open.harness).as_deref(),
        Some("Ask the companion")
    );
    assert!(
        commands(&mut open).is_empty(),
        "back is the panel's, not the summon's"
    );
}

#[test]
fn command_enter_promotes_a_finished_answer_and_command_period_stops_one_being_made() {
    let mut finished = open_in("", LauncherMode::Companion, |bar| {
        bar.companion
            .send_replace(companion(vec![draft()], fresh()));
    });
    finished.press(Press::CommandPeriod);
    assert!(commands(&mut finished).is_empty(), "nothing is being made");
    finished.press(Press::CommandEnter);
    settled(&mut finished);
    assert_eq!(
        commands(&mut finished),
        [CompanionCommand::Promote(task("t-1"))]
    );
    assert!(
        !is_open(&finished.link),
        "promoting opens the app and closes the panel"
    );

    let mut working = open_in("", LauncherMode::Companion, |bar| {
        bar.companion
            .send_replace(companion(vec![thinking()], fresh()));
    });
    working.press(Press::CommandEnter);
    assert!(
        commands(&mut working).is_empty(),
        "there is no card to promote yet"
    );
    working.press(Press::CommandPeriod);
    assert_eq!(
        commands(&mut working),
        [CompanionCommand::Cancel(task("t-2"))]
    );
    assert!(is_open(&working.link), "stopping an answer keeps the panel");
}

#[test]
fn enter_on_the_ask_row_sends_the_prompt_with_the_window_it_was_summoned_over() {
    let mut open = open_in("forward the receipts", LauncherMode::Companion, |bar| {
        let over = Summon::InLauncher {
            origin: LauncherOrigin::Fresh {
                window: Some(WindowKey(10)),
            },
        };
        bar.companion.send_replace(companion(vec![], over));
    });
    assert_eq!(
        selected_title(&open.harness).as_deref(),
        Some("Ask the companion")
    );
    open.press(Press::Enter);
    settled(&mut open);
    let sent = commands(&mut open);
    let [CompanionCommand::Ask(ask)] = sent.as_slice() else {
        panic!("one ask: {sent:?}");
    };
    assert_eq!(ask.prompt, "forward the receipts");
    assert_eq!(ask.window, Some(WindowKey(10)));
    assert_eq!(ask.keep.window, Keep::Kept);
    assert!(is_open(&open.link), "the panel stays to show the answer");
}

#[test]
fn tab_in_a_prompt_goes_round_the_companions_rows_runs_and_memory() {
    let mut open = open_in("", LauncherMode::Companion, |bar| {
        bar.companion.send_replace(companion(vec![], fresh()));
    });
    let mut seen = Vec::new();
    for _ in 0..4 {
        open.harness
            .send(ds_harness::Input::key(ds::prelude::ShortcutKey::Tab));
        settled(&mut open);
        seen.push(last_search(&open));
    }
    let only = |provider| SearchCommand::Filtered {
        text: String::new(),
        provider,
    };
    assert_eq!(
        seen,
        [
            only(ProviderKind::Runs),
            only(ProviderKind::Memory),
            only(ProviderKind::Companion),
            only(ProviderKind::Runs),
        ],
        "Tab never leaves the companion's own rows"
    );
}

fn taipei() -> sill_model::companion::AnswerFacts {
    use companion_wire::{AnswerBody, AnswerPhase};
    use docket_core::Reveal;
    let lines = [
        "I'll check my memory to see what I've remembered about this question.",
        "The capital of Taiwan is **Taipei**.",
    ];
    launcher_harness::launcher_support::companion::answer(
        "t-5",
        "capital of Taiwan",
        AnswerPhase::Done,
        AnswerBody::Text {
            lines: lines.map(|line| Reveal::Plain(line.to_owned())).to_vec(),
        },
    )
}

#[test]
fn a_finished_text_answer_shows_the_answer_in_its_row_and_every_line_in_the_pane() {
    let open = open_in("", LauncherMode::Companion, |bar| {
        bar.companion
            .send_replace(companion(vec![taipei()], fresh()));
    });
    let h = &open.harness;
    let html = h.html();
    assert!(
        html.contains("The capital of Taiwan is Taipei."),
        "the row says the answer: {html}"
    );
    assert_eq!(h.count(".ds-palette-aside .sill-answer-title"), 1);
    assert_eq!(h.count(".ds-palette-aside .sill-answer-line"), 2, "{html}");
    assert!(html.contains("I&#x27;ll check my memory") || html.contains("I'll check my memory"));
    assert_eq!(
        h.text_of(".ds-palette-aside .ds-run").as_deref(),
        Some("Taipei"),
        "the bold words are a strong run, not asterisks"
    );
    assert!(html.contains(r#"data-tone="strong""#), "{html}");
    assert!(!html.contains("**"), "no marks show: {html}");
}

fn working(phase: companion_wire::AnswerPhase, said: &[&str]) -> sill_model::companion::AnswerFacts {
    use companion_wire::AnswerBody;
    let mut answer = launcher_harness::launcher_support::companion::answer(
        "t-6",
        "what do I know about Sam",
        phase,
        AnswerBody::Text { lines: Vec::new() },
    );
    answer.wire.progress = said.iter().map(|line| (*line).to_owned()).collect();
    answer
}

const SAID: [&str; 5] = [
    "one",
    "I'll check **my memory**",
    "three",
    "four",
    "Looking at your *calendar*",
];

#[test]
fn while_working_the_row_says_the_newest_progress_and_the_pane_lists_the_newest_three() {
    let open = open_in("", LauncherMode::Companion, |bar| {
        bar.companion.send_replace(companion(
            vec![working(companion_wire::AnswerPhase::Thinking, &SAID)],
            fresh(),
        ));
    });
    let h = &open.harness;
    let html = h.html();
    assert!(html.contains("Looking at your calendar"), "the row: {html}");
    assert_eq!(
        h.count(".ds-palette-aside .sill-answer-progress-line"),
        4,
        "three lines and the fold: {html}"
    );
    assert!(html.contains("+2 earlier"), "{html}");
    assert!(!html.contains("**"), "no marks show: {html}");
    assert_eq!(h.count(".ds-palette-aside .sill-answer-progress .ds-progress"), 1);
}

#[test]
fn a_finished_answer_hides_the_working_out_in_the_row_and_the_pane() {
    use companion_wire::AnswerPhase;
    for phase in [AnswerPhase::Done, AnswerPhase::Failed, AnswerPhase::Cancelled] {
        let open = open_in("", LauncherMode::Companion, |bar| {
            bar.companion
                .send_replace(companion(vec![working(phase.clone(), &SAID)], fresh()));
        });
        let h = &open.harness;
        let html = h.html();
        assert_eq!(h.count(".sill-answer-progress"), 0, "{phase:?}: {html}");
        assert!(!html.contains("Looking at your calendar"), "{phase:?}: {html}");
    }
}

#[test]
fn a_working_answer_with_nothing_said_shows_the_thinking_words_and_no_list() {
    let open = open_in("", LauncherMode::Companion, |bar| {
        bar.companion.send_replace(companion(
            vec![working(companion_wire::AnswerPhase::Thinking, &[])],
            fresh(),
        ));
    });
    let h = &open.harness;
    assert!(h.html().contains("Thinking"));
    assert_eq!(h.count(".sill-answer-progress"), 0);
}
