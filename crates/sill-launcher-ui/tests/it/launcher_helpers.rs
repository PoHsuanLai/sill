//! The helpers in the launcher's companion mode on the fake services: Tab reaches them, a row
//! opens its card in the pane, the card says what the model holds, and its buttons send what the
//! screen's model understands. The companion service is a fake: its state is seeded and its
//! commands are read back, so what these check is the panel's.

use crate::launcher_harness;

use std::time::Duration;

use docket_core::helper::HelperModel;
use docket_core::routine::RoutineId;
use ds_harness::{Driver, Input, Query};
use launcher_harness::launcher_support::companion::{RESEARCH, helpers};
use launcher_harness::{Open, Press, open_in, selected_title, titles};
use prov::HelperId;
use sill_launcher::LauncherMode;
use sill_model::companion::{CompanionCommand, HelpersMsg, ModelGroup, ModelOffer, RoutinesMsg};

fn research() -> HelperId {
    HelperId::parse("h-1").unwrap_or_else(|e| panic!("{e}"))
}

fn commands(open: &mut Open) -> Vec<CompanionCommand> {
    let mut seen = Vec::new();
    while let Ok(command) = open.feeds.bar.companion_commands.try_recv() {
        seen.push(command);
    }
    seen
}

fn settled(open: &mut Open) {
    open.harness.advance(Duration::from_millis(50));
}

fn click(open: &mut Open, selector: &str) {
    let at = open
        .harness
        .centre(selector)
        .unwrap_or_else(|| panic!("{selector} is not laid out: {}", open.harness.html()));
    open.harness.send(Input::click(at));
    settled(open);
}

/// The panel in companion mode over `more` applied to the helpers' state, on the Helpers filter
/// with Research selected.
fn on_research(more: Vec<HelpersMsg>) -> Open {
    let mut open = open_in("", LauncherMode::Companion, |bar| {
        bar.companion.send_replace(helpers(more));
    });
    open.harness.send(Input::key(ds::prelude::ShortcutKey::Tab));
    settled(&mut open);
    open.press(Press::Down);
    settled(&mut open);
    open
}

#[test]
fn tab_reaches_the_helpers_and_the_new_helper_row_comes_first() {
    let mut open = open_in("", LauncherMode::Companion, |bar| {
        bar.companion.send_replace(helpers(Vec::new()));
    });
    open.harness.send(Input::key(ds::prelude::ShortcutKey::Tab));
    settled(&mut open);
    assert_eq!(
        titles(&open.harness),
        ["New Helper\u{2026}", "Research", "Inbox"]
    );
    let html = open.harness.html();
    assert!(
        html.contains("Waiting for you"),
        "the helper that needs the person wears the badge: {html}"
    );
    assert!(html.contains("Paused"), "a paused helper says so: {html}");
}

#[test]
fn a_selected_helper_opens_its_card_and_the_card_says_what_it_needs() {
    let mut open = on_research(Vec::new());
    assert_eq!(selected_title(&open.harness).as_deref(), Some("Research"));
    let h = &open.harness;
    assert_eq!(h.count(".ds-palette-aside .sill-helper-card"), 1);
    let html = h.html();
    for says in [
        "Reads the news each morning",
        "Choose a model so it can start",
        "Not chosen yet",
        "Find the weekly digest",
        "Found the digest.",
        "Give Research a job",
    ] {
        assert!(html.contains(says), "the card says {says:?}: {html}");
    }
    assert!(
        commands(&mut open).contains(&CompanionCommand::Helpers(HelpersMsg::OpenCard(research()))),
        "the card asks the companion for the helper it shows"
    );
}

#[test]
fn saving_commits_the_typed_words_then_saves() {
    let mut open = on_research(Vec::new());
    commands(&mut open);
    click(&mut open, "#helper-save");
    assert_eq!(
        commands(&mut open),
        [
            CompanionCommand::Helpers(HelpersMsg::EditName("Research".to_owned())),
            CompanionCommand::Helpers(HelpersMsg::EditRole(
                "Reads the news each morning".to_owned()
            )),
            CompanionCommand::Helpers(HelpersMsg::Save),
        ]
    );
}

#[test]
fn choosing_a_model_asks_for_the_list_and_a_removal_waits_for_its_yes() {
    let mut open = on_research(Vec::new());
    commands(&mut open);
    click(&mut open, "#helper-model");
    assert_eq!(
        commands(&mut open),
        [CompanionCommand::Helpers(HelpersMsg::PickModel)]
    );

    let mut asked = on_research(vec![HelpersMsg::AskRemove(research())]);
    let html = asked.harness.html();
    assert!(
        html.contains("Remove Research? Its finished jobs stay in your history."),
        "{html}"
    );
    commands(&mut asked);
    click(&mut asked, "#helper-remove-yes");
    assert_eq!(
        commands(&mut asked),
        [CompanionCommand::Helpers(HelpersMsg::Remove(research()))]
    );

    let mut declined = on_research(vec![HelpersMsg::AskRemove(research())]);
    commands(&mut declined);
    click(&mut declined, "#helper-keep");
    assert_eq!(
        commands(&mut declined),
        [CompanionCommand::Helpers(HelpersMsg::CancelRemove)]
    );
}

fn qwen() -> HelperModel {
    serde_json::from_str(r#"{"kind":"named","v":{"account":"local","model":"qwen3-4b"}}"#)
        .unwrap_or_else(|e| panic!("{e}"))
}

fn offered(groups: Vec<ModelGroup>) -> Vec<HelpersMsg> {
    vec![HelpersMsg::PickModel, HelpersMsg::ModelsOffered(groups)]
}

#[test]
fn a_model_in_the_list_is_chosen_for_the_helper_and_settings_stays_one_entry_away() {
    let group = ModelGroup {
        heading: "On this computer, with llama.cpp".to_owned(),
        offers: vec![ModelOffer {
            choice: qwen(),
            label: "Qwen3 4B".to_owned(),
        }],
    };
    let mut open = on_research(offered(vec![group]));
    let html = open.harness.html();
    for says in [
        "On this computer, with llama.cpp",
        "Qwen3 4B",
        "Add a model in Settings",
    ] {
        assert!(html.contains(says), "the list says {says:?}: {html}");
    }
    commands(&mut open);
    click(&mut open, "#helper-model-0");
    assert_eq!(
        commands(&mut open),
        [CompanionCommand::Helpers(HelpersMsg::ChooseModel(qwen()))]
    );
    click(&mut open, "#helper-model-add");
    assert_eq!(
        commands(&mut open),
        [CompanionCommand::Helpers(HelpersMsg::AddModel)]
    );
}

#[test]
fn with_no_model_at_all_the_card_says_so_and_offers_settings() {
    let open = on_research(offered(Vec::new()));
    let html = open.harness.html();
    assert!(html.contains("No model is available yet"), "{html}");
    assert_eq!(open.harness.count("#helper-model-add"), 1);
    assert_eq!(open.harness.count("#helper-model-0"), 0);
}

#[test]
fn a_refused_card_shows_the_sentence_it_came_with() {
    let open = on_research(vec![HelpersMsg::Replied(
        sill_model::companion::Asked::Save,
        companion_wire::HelperReply::Refused(
            "There is already a helper called Research.".to_owned(),
        ),
    )]);
    let html = open.harness.html();
    assert!(
        html.contains("There is already a helper called Research."),
        "{html}"
    );
}

/// Research's card as an app helper would have it: it runs Claude Code, the person is at `login`
/// with it, and one job waits for the sign-in.
fn app_card(login: &str) -> HelpersMsg {
    let card = format!(
        r#"{{"line":{RESEARCH},"role":"Reads the news each morning","model":{{"kind":"unset"}},"reach":["org.quire.Notes: look at things"],"steps":12,"jobs":[{{"id":2,"goal":"Summarise my mail","state":"needs_sign_in","note":null}}],"kind":{{"agent_app":"claude-code"}},"login":"{login}","sign_in":[{{"id":"login","name":"Claude account"}}]}}"#
    );
    HelpersMsg::CardRead(Box::new(
        serde_json::from_str(&card).unwrap_or_else(|e| panic!("{e}")),
    ))
}

#[test]
fn an_app_helper_names_its_app_and_a_sign_in_button_signs_in() {
    let mut open = on_research(vec![app_card("needs_sign_in")]);
    let html = open.harness.html();
    for says in [
        "Uses Claude Code",
        "Waiting for you to sign in",
        "Sign in to Claude Code so this job can start.",
    ] {
        assert!(html.contains(says), "the card says {says:?}: {html}");
    }
    assert_eq!(open.harness.count("#helper-sign-out"), 0);
    commands(&mut open);
    click(&mut open, "#helper-sign-in-0");
    assert_eq!(
        commands(&mut open),
        [CompanionCommand::Helpers(HelpersMsg::SignIn(
            research(),
            "login".to_owned()
        ))]
    );
    click(&mut open, "#helper-job-sign-in-0");
    assert_eq!(
        commands(&mut open).len(),
        1,
        "the job's button signs in too"
    );
}

#[test]
fn a_signed_in_app_helper_offers_sign_out_and_no_sign_in() {
    let mut open = on_research(vec![app_card("signed_in")]);
    assert!(open.harness.html().contains("Signed in"));
    assert_eq!(open.harness.count("#helper-sign-in-0"), 0);
    commands(&mut open);
    click(&mut open, "#helper-sign-out");
    assert_eq!(
        commands(&mut open),
        [CompanionCommand::Helpers(HelpersMsg::SignOut(research()))]
    );
}

/// Research's app card with where its work is and what the companion allows.
fn app_card_in(runs_in: &str, handover: &str) -> HelpersMsg {
    let card = format!(
        r#"{{"line":{RESEARCH},"role":"Reads the news each morning","model":{{"kind":"unset"}},"reach":[],"steps":12,"jobs":[],"kind":{{"agent_app":"claude-code"}},"login":"signed_in","runs_in":"{runs_in}","handover":"{handover}"}}"#
    );
    HelpersMsg::CardRead(Box::new(
        serde_json::from_str(&card).unwrap_or_else(|e| panic!("{e}")),
    ))
}

#[test]
fn an_app_helper_offers_the_terminal_and_a_helper_in_one_offers_bring_back() {
    let mut open = on_research(vec![app_card_in("desktop", "to_terminal")]);
    assert_eq!(open.harness.count("#helper-bring-back"), 0);
    assert!(open.harness.html().contains("Open in Terminal"));
    commands(&mut open);
    click(&mut open, "#helper-open-terminal");
    assert_eq!(
        commands(&mut open),
        [CompanionCommand::Helpers(HelpersMsg::OpenInTerminal(
            research()
        ))]
    );
    let mut away = on_research(vec![app_card_in("terminal", "bring_back")]);
    assert_eq!(away.harness.count("#helper-open-terminal"), 0);
    assert!(away.harness.html().contains("In Terminal"));
    commands(&mut away);
    click(&mut away, "#helper-bring-back");
    assert_eq!(
        commands(&mut away),
        [CompanionCommand::Helpers(HelpersMsg::BringBack(research()))]
    );
    let nothing = on_research(vec![app_card_in("desktop", "unavailable")]);
    assert_eq!(nothing.harness.count("#helper-open-terminal"), 0);
    assert_eq!(nothing.harness.count("#helper-bring-back"), 0);
}

fn routine_listed() -> HelpersMsg {
    let line = r#"{"id":1,"helper":"h-1","helper_name":"Research","goal":"Read the news","when":"Every day at 8:00","state":"on","next":null,"last":null}"#;
    HelpersMsg::Routines(RoutinesMsg::Listed(
        serde_json::from_str(&format!(r#"{{"routines":[{line}]}}"#))
            .unwrap_or_else(|e| panic!("{e}")),
    ))
}

#[test]
fn a_routine_is_listed_with_its_time_and_the_skip_rule_and_can_be_paused() {
    let mut open = on_research(vec![routine_listed()]);
    let html = open.harness.html();
    for says in [
        "Read the news",
        "Every day at 8:00",
        "If your computer is off at that time, the run is skipped.",
    ] {
        assert!(html.contains(says), "the card says {says:?}: {html}");
    }
    commands(&mut open);
    click(&mut open, "#helper-routine-toggle-r-1");
    assert_eq!(
        commands(&mut open),
        [CompanionCommand::Helpers(HelpersMsg::Routines(
            RoutinesMsg::Pause(RoutineId(1))
        ))]
    );
    click(&mut open, "#helper-routine-add");
    assert_eq!(
        commands(&mut open),
        [CompanionCommand::Helpers(HelpersMsg::Routines(
            RoutinesMsg::OpenSheet(research())
        ))]
    );
}

#[test]
fn the_routine_sheet_sends_nothing_until_it_is_confirmed() {
    let mut open = on_research(vec![HelpersMsg::Routines(RoutinesMsg::OpenSheet(
        research(),
    ))]);
    let html = open.harness.html();
    assert!(
        html.contains("If your computer is off at that time, the run is skipped."),
        "{html}"
    );
    commands(&mut open);
    click(&mut open, "#helper-routine-confirm");
    let sent = commands(&mut open);
    assert_eq!(
        sent.last(),
        Some(&CompanionCommand::Helpers(HelpersMsg::Routines(
            RoutinesMsg::Confirm
        )))
    );
    click(&mut open, "#helper-routine-cancel");
    assert_eq!(
        commands(&mut open),
        [CompanionCommand::Helpers(HelpersMsg::Routines(
            RoutinesMsg::CloseSheet
        ))]
    );
}

#[test]
fn automatic_is_offered_with_the_models_and_ticks_nothing_for_the_person() {
    let group = ModelGroup {
        heading: "On this computer".to_owned(),
        offers: vec![ModelOffer {
            choice: qwen(),
            label: "Qwen3 4B".to_owned(),
        }],
    };
    let mut open = on_research(offered(vec![group.clone()]));
    commands(&mut open);
    click(&mut open, "#helper-automatic");
    assert_eq!(
        commands(&mut open),
        [CompanionCommand::Helpers(HelpersMsg::StartAutomatic)]
    );
    let mut choosing = on_research(
        offered(vec![group])
            .into_iter()
            .chain([HelpersMsg::StartAutomatic])
            .collect(),
    );
    let html = choosing.harness.html();
    assert!(html.contains("Use Automatic"), "{html}");
    commands(&mut choosing);
    click(&mut choosing, "#helper-automatic-confirm");
    assert_eq!(
        commands(&mut choosing),
        [CompanionCommand::Helpers(HelpersMsg::ConfirmAutomatic)]
    );
}
