//! The helpers in the launcher's companion mode on the fake services: Tab reaches them, a row
//! opens its card in the pane, the card says what the model holds, and its buttons send what the
//! screen's model understands. The companion service is a fake: its state is seeded and its
//! commands are read back, so what these check is the panel's.

use crate::launcher_harness;

use std::time::Duration;

use docket_core::helper::HelperModel;
use ds_harness::{Driver, Input, Query};
use launcher_harness::launcher_support::companion::helpers;
use launcher_harness::{Open, Press, open_in, selected_title, titles};
use prov::HelperId;
use sill_launcher::LauncherMode;
use sill_model::companion::{CompanionCommand, HelpersMsg, ModelGroup, ModelOffer};

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
