//! The helpers on a private bus: a stand-in owns `org.quire.Companion1` and its
//! `Companion1.Helpers2` object; the companion service reads the list, follows `Changed`, reads a
//! card, and sends what the person confirms. Every wait is on the service's published state or on
//! the stand-in's record, never on a clock. Never the person's session bus.

use crate::companion_support::{self, Desk, desk_or_return};

use std::sync::{Arc, Mutex};

use companion_support::{Log, app, json};
use companion_wire::{
    HelperCardView, HelperLine, HelperNeeds, HelperReply, HelpersView, LoginShown, RoutineForm,
    RoutineReply, RoutinesView, ToldWire,
};
use docket_core::helper::{HelperKind, HelperModel, HelperName, HelperPhase};
use docket_core::routine::RoutineId;
use docket_dbus::COMPANION_BUS;
use porter_core::Count;
use prov::{HelperId, SpaceId};
use sill_model::companion::{
    CompanionCommand, GONE, HelpersMsg, Open, Removal, RoutinesMsg, WAITING_BADGE, badge_of,
};
use zbus::{Connection, fdo, interface};

const PATH: &str = "/org/quire/Companion1";

fn id(text: &str) -> HelperId {
    HelperId::parse(text).unwrap_or_else(|e| panic!("{e}"))
}

fn line(helper: &str, name: &str, needs_you: Option<HelperNeeds>) -> HelperLine {
    HelperLine {
        id: id(helper),
        name: HelperName::parse(name).unwrap_or_else(|e| panic!("{e}")),
        space: SpaceId::desktop(),
        doing: "Nothing to do".to_owned(),
        state: HelperPhase::Idle,
        needs_you,
        agent: None,
        conversation: None,
    }
}

fn card(helper: &str, name: &str) -> HelperCardView {
    HelperCardView {
        line: line(helper, name, None),
        role: "Reads the news each morning".to_owned(),
        model: HelperModel::Unset,
        reach: vec!["org.quire.Notes: look at things".to_owned()],
        steps: Count(12),
        jobs: vec![
            serde_json::from_str(
                r#"{"id":1,"goal":"Find the weekly digest","state":"done","note":null}"#,
            )
            .unwrap_or_else(|e| panic!("{e}")),
        ],
        kind: HelperKind::default(),
        login: LoginShown::default(),
        sign_in: Vec::new(),
    }
}

/// What the stand-in holds, and answers with.
#[derive(Clone)]
struct Held {
    view: Arc<Mutex<HelpersView>>,
    card: Arc<Mutex<Option<HelperCardView>>>,
    reply: Arc<Mutex<HelperReply>>,
}

struct StandInHelpers {
    log: Log,
    held: Held,
}

#[interface(name = "org.quire.Companion1.Helpers2")]
impl StandInHelpers {
    async fn list(&self, after: u64) -> fdo::Result<String> {
        self.log.push(format!("list {after}"));
        Ok(json(
            &*self.held.view.lock().unwrap_or_else(|e| panic!("{e}")),
        ))
    }

    async fn card(&self, helper: String) -> fdo::Result<String> {
        self.log.push(format!("card {helper}"));
        let held = self.held.card.lock().unwrap_or_else(|e| panic!("{e}"));
        match held.as_ref() {
            Some(card) if card.line.id.as_str() == helper => Ok(json(card)),
            _ => Err(fdo::Error::Failed("no such helper".to_owned())),
        }
    }

    async fn create(&self, form: String) -> fdo::Result<String> {
        self.log.push(format!("create {form}"));
        Ok(json(
            &*self.held.reply.lock().unwrap_or_else(|e| panic!("{e}")),
        ))
    }

    async fn edit(&self, helper: String, form: String) -> fdo::Result<String> {
        self.log.push(format!("edit {helper} {form}"));
        Ok(json(
            &*self.held.reply.lock().unwrap_or_else(|e| panic!("{e}")),
        ))
    }

    async fn pause(&self, helper: String) -> fdo::Result<()> {
        self.log.push(format!("pause {helper}"));
        Ok(())
    }

    async fn resume(&self, helper: String) -> fdo::Result<()> {
        self.log.push(format!("resume {helper}"));
        Ok(())
    }

    async fn remove(&self, helper: String) -> fdo::Result<()> {
        self.log.push(format!("remove {helper}"));
        Ok(())
    }

    async fn post_job(&self, helper: String, goal: String, things: String) -> fdo::Result<String> {
        self.log.push(format!("post {helper} {goal} {things}"));
        Ok(json(
            &*self.held.reply.lock().unwrap_or_else(|e| panic!("{e}")),
        ))
    }

    async fn sign_in(&self, helper: String, way: String) -> fdo::Result<String> {
        self.log.push(format!("sign_in {helper} {way}"));
        Ok(json(
            &*self.held.reply.lock().unwrap_or_else(|e| panic!("{e}")),
        ))
    }

    async fn sign_out(&self, helper: String) -> fdo::Result<String> {
        self.log.push(format!("sign_out {helper}"));
        Ok(json(
            &*self.held.reply.lock().unwrap_or_else(|e| panic!("{e}")),
        ))
    }

    async fn routines(&self) -> fdo::Result<String> {
        self.log.push("routines".to_owned());
        Ok(json(&RoutinesView::default()))
    }

    async fn add_routine(&self, form: String) -> fdo::Result<String> {
        self.log.push(format!("add_routine {form}"));
        Ok(json(&RoutineReply::Done))
    }

    async fn pause_routine(&self, routine: String) -> fdo::Result<()> {
        self.log.push(format!("pause_routine {routine}"));
        Ok(())
    }

    async fn resume_routine(&self, routine: String) -> fdo::Result<()> {
        self.log.push(format!("resume_routine {routine}"));
        Ok(())
    }

    async fn remove_routine(&self, routine: String) -> fdo::Result<()> {
        self.log.push(format!("remove_routine {routine}"));
        Ok(())
    }
}

/// The stand-in, on the companion's name, and what it holds.
async fn companion_with(desk: &Desk, view: HelpersView) -> (Connection, Held) {
    let held = Held {
        view: Arc::new(Mutex::new(view)),
        card: Arc::new(Mutex::new(None)),
        reply: Arc::new(Mutex::new(HelperReply::Done)),
    };
    let companion = desk.peer(COMPANION_BUS).await;
    companion
        .object_server()
        .at(
            PATH,
            StandInHelpers {
                log: desk.log.clone(),
                held: held.clone(),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    (companion, held)
}

/// The stand-in says the helpers changed.
async fn changed(companion: &Connection) {
    companion
        .emit_signal(
            None::<&str>,
            PATH,
            "org.quire.Companion1.Helpers2",
            "Changed",
            &(),
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
}

fn said(held: &Held, text: &str) {
    *held.reply.lock().unwrap_or_else(|e| panic!("{e}")) = HelperReply::Refused(text.to_owned());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_list_is_read_when_the_shell_is_seated_and_again_at_each_change() {
    let mut desk = desk_or_return!();
    let view = HelpersView {
        helpers: vec![line("h-1", "Research", Some(HelperNeeds::Answer))],
        told: vec![ToldWire {
            n: 3,
            helper: id("h-1"),
            text: "Found the digest.".to_owned(),
        }],
    };
    let (companion, held) = companion_with(&desk, view).await;
    desk.seated().await;
    let first = desk
        .state(|c| (!c.helpers.lines.is_empty()).then(|| c.helpers.clone()))
        .await;
    assert_eq!(first.lines[0].name.as_str(), "Research");
    assert_eq!(badge_of(&first.lines[0]), Some(WAITING_BADGE));
    assert_eq!(first.told_after, 3);
    desk.log_has("list 0").await;

    held.view
        .lock()
        .unwrap_or_else(|e| panic!("{e}"))
        .helpers
        .push(line("h-2", "Inbox", None));
    changed(&companion).await;
    let second = desk
        .state(|c| (c.helpers.lines.len() == 2).then(|| c.helpers.clone()))
        .await;
    assert_eq!(second.lines[1].name.as_str(), "Inbox");
    desk.log_has("list 3").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_card_is_read_and_a_helper_the_companion_lacks_is_gone() {
    let mut desk = desk_or_return!();
    let (_companion, held) = companion_with(&desk, HelpersView::default()).await;
    *held.card.lock().unwrap_or_else(|e| panic!("{e}")) = Some(card("h-1", "Research"));
    desk.seated().await;

    desk.send(CompanionCommand::Helpers(HelpersMsg::OpenCard(id("h-1"))));
    let open = desk
        .state(|c| match &c.helpers.open {
            Open::Card(card) => Some(card.draft.role.clone()),
            Open::Nothing | Open::Reading(_) | Open::New(_) => None,
        })
        .await;
    assert_eq!(open, "Reads the news each morning");

    desk.send(CompanionCommand::Helpers(HelpersMsg::OpenCard(id("h-9"))));
    let notice = desk
        .state(|c| {
            matches!(c.helpers.open, Open::Reading(_) | Open::Nothing)
                .then(|| c.helpers.notice.clone())
                .flatten()
        })
        .await;
    assert_eq!(notice, GONE);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_saved_card_goes_to_the_companion_and_its_refusal_is_shown_as_it_came() {
    let mut desk = desk_or_return!();
    let (_companion, held) = companion_with(&desk, HelpersView::default()).await;
    said(&held, "There is already a helper called Research.");
    desk.seated().await;

    for msg in [
        HelpersMsg::NewHelper,
        HelpersMsg::EditName("Research".to_owned()),
        HelpersMsg::EditRole("Reads the news".to_owned()),
        HelpersMsg::ToggleApp(app("org.quire.Notes")),
        HelpersMsg::Save,
    ] {
        desk.send(CompanionCommand::Helpers(msg));
    }
    let sent = desk.log_starting("create ").await;
    assert!(sent.contains(r#""name":"Research""#), "{sent}");
    assert!(sent.contains(r#""model":{"kind":"unset"}"#), "{sent}");
    let notice = desk.state(|c| c.helpers.notice.clone()).await;
    assert_eq!(notice, "There is already a helper called Research.");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pausing_removing_and_giving_a_job_reach_the_companion_and_removal_waits_for_the_yes() {
    let mut desk = desk_or_return!();
    let (_companion, held) = companion_with(
        &desk,
        HelpersView {
            helpers: vec![line("h-1", "Research", None)],
            told: Vec::new(),
        },
    )
    .await;
    *held.card.lock().unwrap_or_else(|e| panic!("{e}")) = Some(card("h-1", "Research"));
    desk.seated().await;
    desk.state(|c| (!c.helpers.lines.is_empty()).then_some(()))
        .await;

    desk.send(CompanionCommand::Helpers(HelpersMsg::OpenCard(id("h-1"))));
    desk.state(|c| matches!(c.helpers.open, Open::Card(_)).then_some(()))
        .await;
    desk.send(CompanionCommand::Helpers(HelpersMsg::Pause(id("h-1"))));
    desk.log_has("pause h-1").await;
    desk.send(CompanionCommand::Helpers(HelpersMsg::GiveJob(
        "find the digest".to_owned(),
    )));
    desk.log_has("post h-1 find the digest []").await;

    desk.send(CompanionCommand::Helpers(HelpersMsg::AskRemove(id("h-1"))));
    desk.state(|c| (c.helpers.removal == Removal::Asked(id("h-1"))).then_some(()))
        .await;
    assert!(
        !desk.log.lines().iter().any(|l| l.starts_with("remove ")),
        "asking removes nothing"
    );
    desk.send(CompanionCommand::Helpers(HelpersMsg::Remove(id("h-1"))));
    desk.log_has("remove h-1").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn signing_in_and_out_and_the_routines_reach_the_companion() {
    let mut desk = desk_or_return!();
    let (_companion, _held) = companion_with(&desk, HelpersView::default()).await;
    desk.seated().await;
    desk.log_has("routines").await;

    desk.send(CompanionCommand::Helpers(HelpersMsg::SignIn(
        id("h-1"),
        "login".to_owned(),
    )));
    desk.log_has("sign_in h-1 login").await;
    desk.send(CompanionCommand::Helpers(HelpersMsg::SignOut(id("h-1"))));
    desk.log_has("sign_out h-1").await;

    for msg in [
        RoutinesMsg::OpenSheet(id("h-1")),
        RoutinesMsg::EditGoal("Read the news".to_owned()),
        RoutinesMsg::Confirm,
    ] {
        desk.send(CompanionCommand::Helpers(HelpersMsg::Routines(msg)));
    }
    let sent = desk.log_starting("add_routine ").await;
    let form: RoutineForm = serde_json::from_str(sent.trim_start_matches("add_routine "))
        .unwrap_or_else(|e| panic!("{e}: {sent}"));
    assert_eq!(form.goal, "Read the news");
    desk.send(CompanionCommand::Helpers(HelpersMsg::Routines(
        RoutinesMsg::Pause(RoutineId(4)),
    )));
    desk.log_has("pause_routine r-4").await;
}
