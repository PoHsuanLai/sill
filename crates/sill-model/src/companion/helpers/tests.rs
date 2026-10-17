use std::collections::BTreeSet;

use companion_wire::{
    HelperCardView, HelperForm, HelperLine, HelperNeeds, HelperReply, HelpersView, JobLine,
    JobShown, ToldWire,
};
use docket_core::helper::{Ceiling, HelperModel, HelperName, HelperPhase, JobId};
use porter_core::Count;
use porter_infer::ModelRef;
use prov::{AppName, HelperId, SpaceId};
use sill_launcher::HelperControl;

use super::*;

fn id(text: &str) -> HelperId {
    HelperId::parse(text).unwrap_or_else(|e| panic!("{e}"))
}

fn app(name: &str) -> AppName {
    AppName::parse(name).unwrap_or_else(|e| panic!("{e}"))
}

fn line(
    helper: &str,
    name: &str,
    state: HelperPhase,
    needs_you: Option<HelperNeeds>,
) -> HelperLine {
    HelperLine {
        id: id(helper),
        name: HelperName::parse(name).unwrap_or_else(|e| panic!("{e}")),
        space: SpaceId::desktop(),
        doing: "Nothing to do".to_owned(),
        state,
        needs_you,
        agent: None,
    }
}

fn card(helper: &str, name: &str, model: HelperModel) -> HelperCardView {
    HelperCardView {
        line: line(helper, name, HelperPhase::Idle, None),
        role: "Reads the news each morning".to_owned(),
        model,
        reach: vec!["org.quire.Notes: look at things".to_owned()],
        steps: Count(12),
        jobs: vec![JobLine {
            id: JobId(1),
            goal: "Find the weekly digest".to_owned(),
            state: JobShown::Done,
            note: None,
        }],
    }
}

fn chosen() -> HelperModel {
    HelperModel::Named(ModelRef::parse("local/qwen3-4b").unwrap_or_else(|| panic!("model")))
}

fn feed(model: HelpersModel, msgs: Vec<HelpersMsg>) -> (HelpersModel, Vec<HelpersCommand>) {
    msgs.into_iter()
        .fold((model, Vec::new()), |(model, mut sent), msg| {
            let (next, commands) = update(model, msg);
            sent.extend(commands);
            (next, sent)
        })
}

fn listed(lines: Vec<HelperLine>, told: Vec<ToldWire>) -> HelpersMsg {
    HelpersMsg::Listed(HelpersView {
        helpers: lines,
        told,
    })
}

fn opened(helper: &str, model: HelperModel) -> HelpersModel {
    let (model, _) = feed(
        HelpersModel::default(),
        vec![
            listed(
                vec![line(helper, "Research", HelperPhase::Idle, None)],
                Vec::new(),
            ),
            HelpersMsg::OpenCard(id(helper)),
            HelpersMsg::CardRead(Box::new(card(helper, "Research", model))),
        ],
    );
    model
}

fn sentence_of(model: &HelpersModel) -> Option<&str> {
    model.notice.as_deref()
}

#[test]
fn a_card_is_saved_only_with_a_name_a_role_and_an_app() {
    // (name, role, apps, what saving says)
    let cases: [(&str, &str, &[&str], Option<&str>); 6] = [
        ("Research", "Reads the news", &["org.quire.Notes"], None),
        (
            "",
            "Reads the news",
            &["org.quire.Notes"],
            Some("Give it a name."),
        ),
        (
            "   ",
            "Reads the news",
            &["org.quire.Notes"],
            Some("Give it a name."),
        ),
        (
            "Research",
            " ",
            &["org.quire.Notes"],
            Some("Say what it is for."),
        ),
        (
            "Research",
            "Reads the news",
            &[],
            Some("Choose at least one app for it to use."),
        ),
        (
            "A name that goes on and on and on past what a card can keep",
            "Reads the news",
            &["org.quire.Notes"],
            Some("That name is too long or has a character it cannot keep."),
        ),
    ];
    for (name, role, apps, says) in cases {
        let mut msgs = vec![
            HelpersMsg::NewHelper,
            HelpersMsg::EditName(name.to_owned()),
            HelpersMsg::EditRole(role.to_owned()),
        ];
        msgs.extend(apps.iter().map(|a| HelpersMsg::ToggleApp(app(a))));
        msgs.push(HelpersMsg::Save);
        let (model, sent) = feed(HelpersModel::default(), msgs);
        assert_eq!(sentence_of(&model), says, "{name:?} {role:?} {apps:?}");
        assert_eq!(sent.len(), usize::from(says.is_none()), "{name:?} {role:?}");
    }
}

#[test]
fn a_new_card_has_no_model_and_still_saves_and_says_to_choose_one() {
    let (model, sent) = feed(
        HelpersModel::default(),
        vec![
            HelpersMsg::NewHelper,
            HelpersMsg::EditName("Research".to_owned()),
            HelpersMsg::EditRole("Reads the news".to_owned()),
            HelpersMsg::ToggleApp(app("org.quire.Notes")),
            HelpersMsg::SetCeiling(Ceiling::Change),
        ],
    );
    assert!(sent.is_empty());
    let draft = model
        .draft()
        .unwrap_or_else(|| panic!("a new card is open"));
    assert_eq!(draft.model, HelperModel::Unset);
    assert_eq!(model_hint(&draft.model), Some(CHOOSE_MODEL));
    assert_eq!(model_line(&draft.model), "Not chosen yet");
    let (_, sent) = update(model, HelpersMsg::Save);
    let [HelpersCommand::Create(form)] = sent.as_slice() else {
        panic!("saving a complete card creates the helper: {sent:?}");
    };
    let want = HelperForm {
        name: HelperName::parse("Research").unwrap_or_else(|e| panic!("{e}")),
        role: docket_core::helper::Role::parse("Reads the news").unwrap_or_else(|e| panic!("{e}")),
        space: SpaceId::desktop(),
        model: HelperModel::Unset,
        apps: vec![app("org.quire.Notes")],
        ceiling: Ceiling::Change,
        steps: Count(12),
    };
    assert_eq!(**form, want);
}

#[test]
fn choosing_a_model_changes_the_card_and_the_hint_goes() {
    let model = opened("h-1", HelperModel::Unset);
    let (model, _) = update(model, HelpersMsg::ChooseModel(chosen()));
    let draft = model.draft().unwrap_or_else(|| panic!("a card is open"));
    assert_eq!(draft.model, chosen());
    assert_eq!(model_hint(&draft.model), None);
    assert_eq!(model_line(&draft.model), "Qwen3 4b");
    let (_, sent) = update(model, HelpersMsg::Save);
    let [HelpersCommand::Edit { helper, form }] = sent.as_slice() else {
        panic!("saving an open card changes the helper: {sent:?}");
    };
    assert_eq!((helper, &form.model), (&id("h-1"), &chosen()));
}

#[test]
fn asking_to_choose_a_model_opens_the_picker() {
    let (_, sent) = update(HelpersModel::default(), HelpersMsg::PickModel);
    assert_eq!(sent, [HelpersCommand::OpenModelPicker]);
}

#[test]
fn a_helper_that_needs_the_person_wears_the_waiting_badge() {
    let cases = [
        (None, None),
        (Some(HelperNeeds::Answer), Some(WAITING_BADGE)),
        (Some(HelperNeeds::ChooseModel), Some(WAITING_BADGE)),
    ];
    for (needs, badge) in cases {
        let row = line("h-1", "Research", HelperPhase::Working, needs);
        assert_eq!(badge_of(&row), badge, "{needs:?}");
    }
    assert_eq!(WAITING_BADGE, "Waiting for you");
}

#[test]
fn what_a_paused_helper_can_be_asked_is_to_resume_and_what_a_running_one_is_to_pause() {
    let label = |phase| {
        use ds_core::word::Word;
        controls_of(phase)
            .into_iter()
            .map(Word::label)
            .collect::<Vec<_>>()
    };
    assert_eq!(label(HelperPhase::Paused), ["Resume", "Remove"]);
    for phase in [
        HelperPhase::Idle,
        HelperPhase::Working,
        HelperPhase::Waiting,
    ] {
        assert_eq!(label(phase), ["Pause", "Remove"], "{phase:?}");
    }
    assert_eq!(controls_of(HelperPhase::Paused)[0], HelperControl::Resume);
}

#[test]
fn a_refusal_is_shown_as_the_sentence_it_came_with_and_the_next_edit_clears_it() {
    let (model, sent) = feed(
        HelpersModel::default(),
        vec![
            HelpersMsg::NewHelper,
            HelpersMsg::Replied(
                Asked::Save,
                HelperReply::Refused("There is already a helper called Research.".to_owned()),
            ),
        ],
    );
    assert!(sent.is_empty());
    assert_eq!(
        sentence_of(&model),
        Some("There is already a helper called Research.")
    );
    let (model, _) = update(model, HelpersMsg::EditName("Reader".to_owned()));
    assert_eq!(sentence_of(&model), None);
    let (model, _) = update(model, HelpersMsg::Unreached);
    assert_eq!(sentence_of(&model), Some(UNREACHED));
}

#[test]
fn a_made_helper_opens_its_card_and_a_change_reads_the_list_and_the_card_again() {
    let (model, sent) = update(
        HelpersModel::default(),
        HelpersMsg::Replied(Asked::Save, HelperReply::Made(id("h-7"))),
    );
    assert_eq!(model.open, Open::Reading(id("h-7")));
    assert_eq!(
        sent,
        [
            HelpersCommand::ReadCard(id("h-7")),
            HelpersCommand::List { after: 0 }
        ]
    );
    let (_, sent) = update(model, HelpersMsg::Changed);
    assert_eq!(
        sent,
        [
            HelpersCommand::List { after: 0 },
            HelpersCommand::ReadCard(id("h-7"))
        ]
    );
    let (_, sent) = update(HelpersModel::default(), HelpersMsg::Changed);
    assert_eq!(sent, [HelpersCommand::List { after: 0 }]);
}

#[test]
fn opening_a_card_asks_once_and_a_card_that_arrives_for_nobody_is_ignored() {
    let (model, sent) = feed(
        HelpersModel::default(),
        vec![
            HelpersMsg::OpenCard(id("h-1")),
            HelpersMsg::OpenCard(id("h-1")),
        ],
    );
    assert_eq!(sent, [HelpersCommand::ReadCard(id("h-1"))]);
    let (stale, _) = update(
        model.clone(),
        HelpersMsg::CardRead(Box::new(card("h-2", "Inbox", HelperModel::Unset))),
    );
    assert_eq!(stale, model);
    let (model, _) = update(
        model,
        HelpersMsg::CardRead(Box::new(card("h-1", "Research", HelperModel::Unset))),
    );
    let Open::Card(open) = &model.open else {
        panic!("the card arrived: {:?}", model.open);
    };
    assert_eq!(open.draft.name, "Research");
    assert_eq!(open.draft.role, "Reads the news each morning");
}

#[test]
fn a_card_read_again_keeps_what_the_person_typed_and_follows_one_they_did_not() {
    let model = opened("h-1", HelperModel::Unset);
    let again = |role: &str| {
        let mut view = card("h-1", "Research", HelperModel::Unset);
        view.role = role.to_owned();
        HelpersMsg::CardRead(Box::new(view))
    };
    let (untouched, _) = update(model.clone(), again("Reads the news twice a day"));
    assert_eq!(
        untouched.draft().map(|d| d.role.as_str()),
        Some("Reads the news twice a day")
    );
    let (typed, _) = update(model, HelpersMsg::EditRole("Watches prices".to_owned()));
    let (typed, _) = update(typed, again("Reads the news twice a day"));
    assert_eq!(
        typed.draft().map(|d| d.role.as_str()),
        Some("Watches prices")
    );
    let Open::Card(open) = &typed.open else {
        panic!("a card is open");
    };
    assert_eq!(open.view.role, "Reads the news twice a day");
}

#[test]
fn a_list_keeps_new_sentences_once_and_closes_a_card_whose_helper_went() {
    let told = |n: u64, text: &str| ToldWire {
        n,
        helper: id("h-1"),
        text: text.to_owned(),
    };
    let rows = vec![line("h-1", "Research", HelperPhase::Idle, None)];
    let (model, _) = feed(
        HelpersModel::default(),
        vec![
            listed(rows.clone(), vec![told(1, "Found it."), told(2, "Done.")]),
            listed(rows.clone(), vec![told(2, "Done."), told(3, "Again.")]),
        ],
    );
    let said: Vec<&str> = model.told.iter().map(|t| t.text.as_str()).collect();
    assert_eq!(said, ["Found it.", "Done.", "Again."]);
    assert_eq!(model.told_after, 3);
    let (_, sent) = update(model.clone(), HelpersMsg::Changed);
    assert_eq!(sent, [HelpersCommand::List { after: 3 }]);

    let (open, _) = feed(model, vec![HelpersMsg::OpenCard(id("h-1"))]);
    let (gone, _) = update(open, listed(Vec::new(), Vec::new()));
    assert_eq!(gone.open, Open::Nothing);
}

#[test]
fn removing_asks_first_in_the_persons_words_and_only_what_was_asked_is_removed() {
    assert_eq!(
        remove_question("Research"),
        "Remove Research? Its finished jobs stay in your history."
    );
    let model = opened("h-1", HelperModel::Unset);
    let (unasked, sent) = update(model.clone(), HelpersMsg::Remove(id("h-1")));
    assert_eq!((unasked, sent), (model.clone(), Vec::new()));

    let (asked, _) = update(model.clone(), HelpersMsg::AskRemove(id("h-1")));
    assert_eq!(asked.removal, Removal::Asked(id("h-1")));
    let (other, sent) = update(asked.clone(), HelpersMsg::Remove(id("h-2")));
    assert_eq!((other, sent), (asked.clone(), Vec::new()));
    let (declined, _) = update(asked.clone(), HelpersMsg::CancelRemove);
    assert_eq!(declined.removal, Removal::Unasked);

    let (removed, sent) = update(asked, HelpersMsg::Remove(id("h-1")));
    assert_eq!(sent, [HelpersCommand::Remove(id("h-1"))]);
    assert_eq!(
        (removed.open, removed.removal),
        (Open::Nothing, Removal::Unasked)
    );
}

#[test]
fn a_job_needs_words_and_goes_to_the_open_helper() {
    assert_eq!(job_prompt("Research"), "Give Research a job\u{2026}");
    let model = opened("h-1", chosen());
    let (blank, sent) = update(model.clone(), HelpersMsg::GiveJob("  ".to_owned()));
    assert!(sent.is_empty());
    assert_eq!(sentence_of(&blank), Some(NO_JOB_WORDS));
    let (given, sent) = update(model, HelpersMsg::GiveJob("  find the digest ".to_owned()));
    assert_eq!(
        sent,
        [HelpersCommand::PostJob {
            helper: id("h-1"),
            goal: "find the digest".to_owned()
        }]
    );
    assert_eq!(sentence_of(&given), None);
    let (_, sent) = update(HelpersModel::default(), HelpersMsg::GiveJob("x".to_owned()));
    assert!(
        sent.is_empty(),
        "no card is open, so there is nobody to give it to"
    );
}

#[test]
fn a_card_reads_back_into_the_apps_and_limit_its_lines_say() {
    let lines = |ls: &[&str]| ls.iter().map(|l| (*l).to_owned()).collect::<Vec<_>>();
    let apps = |names: &[&str]| names.iter().map(|n| app(n)).collect::<BTreeSet<_>>();
    let cases = [
        (
            lines(&["org.quire.Notes: look at things"]),
            Reach::Chosen {
                apps: apps(&["org.quire.Notes"]),
                ceiling: Ceiling::Look,
            },
        ),
        (
            lines(&[
                "org.quire.Notes: look at things and change them (changes can be undone)",
                "org.quire.Photos: look at things and change them (changes can be undone)",
            ]),
            Reach::Chosen {
                apps: apps(&["org.quire.Notes", "org.quire.Photos"]),
                ceiling: Ceiling::Change,
            },
        ),
        (
            lines(&["org.quire.Notes: look at things, change them and send things out"]),
            Reach::Chosen {
                apps: apps(&["org.quire.Notes"]),
                ceiling: Ceiling::Send,
            },
        ),
        (
            lines(&[
                "org.quire.Notes: look at things",
                "org.quire.Photos: look at things, change them and send things out",
            ]),
            Reach::Unreadable,
        ),
        (
            lines(&["org.quire.Notes: do anything, including deleting"]),
            Reach::Unreadable,
        ),
        (lines(&["no colon here"]), Reach::Unreadable),
    ];
    for (given, want) in cases {
        assert_eq!(reach_of_lines(&given), want, "{given:?}");
    }
}

#[test]
fn a_card_whose_lines_cannot_be_read_cannot_be_saved_until_apps_are_chosen_again() {
    let mut view = card("h-1", "Research", HelperModel::Unset);
    view.reach = vec!["org.quire.Notes: do anything, including deleting".to_owned()];
    let (model, _) = feed(
        HelpersModel::default(),
        vec![
            HelpersMsg::OpenCard(id("h-1")),
            HelpersMsg::CardRead(Box::new(view)),
            HelpersMsg::Save,
        ],
    );
    assert_eq!(
        sentence_of(&model),
        Some("Choose the apps it may use again to save this card.")
    );
    let (model, sent) = feed(
        model,
        vec![
            HelpersMsg::ToggleApp(app("org.quire.Notes")),
            HelpersMsg::Save,
        ],
    );
    assert!(
        matches!(sent.as_slice(), [HelpersCommand::Edit { .. }]),
        "{sent:?}"
    );
    assert_eq!(sentence_of(&model), None);
}

#[test]
fn a_job_line_says_where_the_job_stands_and_why_a_try_did_not_finish() {
    let job = |state, note: Option<&str>| JobLine {
        id: JobId(2),
        goal: "Find the digest".to_owned(),
        state,
        note: note.map(str::to_owned),
    };
    let cases = [
        (job(JobShown::Waiting, None), "Waiting its turn"),
        (job(JobShown::Working, None), "Working on it"),
        (job(JobShown::Done, None), "Done"),
        (job(JobShown::NeedsModel, None), "Needs a model first"),
        (
            job(
                JobShown::Waiting,
                Some("It stopped: the model did not answer"),
            ),
            "Waiting its turn. It stopped: the model did not answer",
        ),
    ];
    for (line, says) in cases {
        assert_eq!(
            job_words(&line),
            ("Find the digest".to_owned(), says.to_owned())
        );
    }
}

#[test]
fn an_unknown_helper_closes_its_card_with_a_sentence() {
    let (model, _) = update(HelpersModel::default(), HelpersMsg::OpenCard(id("h-9")));
    let (model, _) = update(model, HelpersMsg::CardMissing(id("h-9")));
    assert_eq!(
        (&model.open, sentence_of(&model)),
        (&Open::Nothing, Some(GONE))
    );
}
