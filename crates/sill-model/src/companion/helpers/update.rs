//! The helpers screen's one transition: `(model, message) -> (model, commands)`.

use companion_wire::{HelperCardView, HelperReply, HelpersView};
use docket_core::helper::Ceiling;
use prov::{AppName, HelperId, SpaceId};

use super::automatic::{self, Automatic};
use super::draft::{Draft, Reach};
use super::model::{HelpersModel, Open, OpenCard, Removal, Told};
use super::models::{ModelGroup, ModelList};
use super::msg::{Asked, HelpersCommand, HelpersMsg};
use super::routines::{self, RoutinesCommand};
use super::words;

/// How many told sentences the model keeps.
const TOLD_KEPT: usize = 30;

/// `model` after `msg`, and what the screen asks the companion.
pub fn update(model: HelpersModel, msg: HelpersMsg) -> (HelpersModel, Vec<HelpersCommand>) {
    match msg {
        HelpersMsg::Listed(view) => (listed(model, view), Vec::new()),
        HelpersMsg::Changed => changed(model),
        HelpersMsg::CardRead(view) => (card_read(model, *view), Vec::new()),
        HelpersMsg::CardMissing(id) => (card_missing(model, &id), Vec::new()),
        HelpersMsg::AppsOffered(apps) => (HelpersModel { apps, ..model }, Vec::new()),
        HelpersMsg::Replied(asked, reply) => replied(model, asked, reply),
        HelpersMsg::Unreached => (
            said(model_list_closed_if_reading(model), words::UNREACHED),
            Vec::new(),
        ),
        HelpersMsg::OpenCard(id) => open_card(model, id),
        HelpersMsg::CloseCard => (
            HelpersModel {
                open: Open::Nothing,
                models: ModelList::Closed,
                automatic: Automatic::Off,
                notice: None,
                ..model
            },
            Vec::new(),
        ),
        HelpersMsg::NewHelper => (
            HelpersModel {
                open: Open::New(Draft::blank(SpaceId::desktop())),
                models: ModelList::Closed,
                automatic: Automatic::Off,
                notice: None,
                ..model
            },
            Vec::new(),
        ),
        HelpersMsg::EditName(name) => (edited(model, |draft| Draft { name, ..draft }), Vec::new()),
        HelpersMsg::EditRole(role) => (edited(model, |draft| Draft { role, ..draft }), Vec::new()),
        HelpersMsg::ToggleApp(app) => (edited(model, |draft| toggled(draft, app)), Vec::new()),
        HelpersMsg::SetCeiling(ceiling) => {
            (edited(model, |draft| limited(draft, ceiling)), Vec::new())
        }
        HelpersMsg::ChooseModel(chosen) => (
            HelpersModel {
                models: ModelList::Closed,
                automatic: Automatic::Off,
                ..edited(model, |draft| Draft {
                    model: chosen,
                    ..draft
                })
            },
            Vec::new(),
        ),
        HelpersMsg::PickModel => pick_model(model),
        HelpersMsg::ModelsOffered(groups) => (models_offered(model, groups), Vec::new()),
        HelpersMsg::AddModel => (
            HelpersModel {
                models: ModelList::Closed,
                ..model
            },
            vec![HelpersCommand::OpenModelPicker],
        ),
        HelpersMsg::Save => save(model),
        HelpersMsg::Pause(id) => (model, vec![HelpersCommand::Pause(id)]),
        HelpersMsg::Resume(id) => (model, vec![HelpersCommand::Resume(id)]),
        HelpersMsg::AskRemove(id) => (
            HelpersModel {
                removal: Removal::Asked(id),
                ..model
            },
            Vec::new(),
        ),
        HelpersMsg::CancelRemove => (
            HelpersModel {
                removal: Removal::Unasked,
                ..model
            },
            Vec::new(),
        ),
        HelpersMsg::Remove(id) => remove(model, id),
        HelpersMsg::GiveJob(goal) => give_job(model, &goal),
        HelpersMsg::SignIn(helper, way) => (model, vec![HelpersCommand::SignIn { helper, way }]),
        HelpersMsg::SignOut(helper) => (model, vec![HelpersCommand::SignOut(helper)]),
        HelpersMsg::TerminalClosed(helper) => (model, vec![HelpersCommand::Resume(helper)]),
        HelpersMsg::HandoffClosed(helper) => {
            (model, vec![HelpersCommand::BringBackAfterClose(helper)])
        }
        HelpersMsg::BringBack(helper) => (model, vec![HelpersCommand::BringBack(helper)]),
        HelpersMsg::OpenInTerminal(helper) => (model, vec![HelpersCommand::OpenInTerminal(helper)]),
        HelpersMsg::NoTerminal => (said(model, words::NO_TERMINAL), Vec::new()),
        HelpersMsg::SignInUnconfined => (said(model, words::SIGN_IN_DID_NOT_WORK), Vec::new()),
        HelpersMsg::StartAutomatic => (automatic::start(model), Vec::new()),
        HelpersMsg::TogglePooled(chosen) => (automatic::toggle(model, chosen), Vec::new()),
        HelpersMsg::CancelAutomatic => (automatic::cancel(model), Vec::new()),
        HelpersMsg::ConfirmAutomatic => automatic::confirm(model),
        HelpersMsg::PoolWritten(written) => (automatic::written(model, written), Vec::new()),
        HelpersMsg::Routines(msg) => routine_part(model, msg),
    }
}

/// The routines part moves by `msg`; what it asks goes out as the screen's own commands.
fn routine_part(
    model: HelpersModel,
    msg: routines::RoutinesMsg,
) -> (HelpersModel, Vec<HelpersCommand>) {
    let (part, commands) = routines::update(model.routines, msg);
    (
        HelpersModel {
            routines: part,
            ..model
        },
        commands.into_iter().map(HelpersCommand::Routines).collect(),
    )
}

/// The list of models shown, or hidden when it already is; asked for when it was not.
fn pick_model(model: HelpersModel) -> (HelpersModel, Vec<HelpersCommand>) {
    match model.models {
        ModelList::Closed => (
            HelpersModel {
                models: ModelList::Reading,
                ..model
            },
            vec![HelpersCommand::ReadModels],
        ),
        ModelList::Reading | ModelList::Offered(_) => (
            HelpersModel {
                models: ModelList::Closed,
                ..model
            },
            Vec::new(),
        ),
    }
}

/// The models arrived: shown when the person is waiting for them, otherwise ignored.
fn models_offered(model: HelpersModel, groups: Vec<ModelGroup>) -> HelpersModel {
    match model.models {
        ModelList::Reading => HelpersModel {
            models: ModelList::Offered(groups),
            ..model
        },
        ModelList::Closed | ModelList::Offered(_) => model,
    }
}

/// `model`, with a list of models that was being read given up.
fn model_list_closed_if_reading(model: HelpersModel) -> HelpersModel {
    match model.models {
        ModelList::Reading => HelpersModel {
            models: ModelList::Closed,
            ..model
        },
        ModelList::Closed | ModelList::Offered(_) => model,
    }
}

fn said(model: HelpersModel, sentence: &str) -> HelpersModel {
    HelpersModel {
        notice: Some(sentence.to_owned()),
        ..model
    }
}

/// The list arrived: keep it and the new sentences, and close a card whose helper has gone.
fn listed(model: HelpersModel, view: HelpersView) -> HelpersModel {
    let told_after = view
        .told
        .iter()
        .map(|t| t.n)
        .max()
        .map_or(model.told_after, |n| n.max(model.told_after));
    let fresh = view
        .told
        .into_iter()
        .filter(|t| t.n > model.told_after)
        .map(Told::from);
    let mut told: Vec<Told> = model.told.iter().cloned().chain(fresh).collect();
    let extra = told.len().saturating_sub(TOLD_KEPT);
    let told = told.split_off(extra);
    let still_there = |id: &HelperId| view.helpers.iter().any(|line| line.id == *id);
    let open_gone = model.open_id().is_some_and(|id| !still_there(id));
    // A card whose helper has gone closes with the sentence a missing card says, whichever
    // answer came first: a list read before the card leaves the card nothing to report.
    let (open, notice) = if open_gone {
        (Open::Nothing, Some(words::GONE.to_owned()))
    } else {
        (model.open, model.notice)
    };
    let removal = match model.removal {
        Removal::Asked(id) if !still_there(&id) => Removal::Unasked,
        other => other,
    };
    HelpersModel {
        lines: view.helpers,
        told,
        told_after,
        open,
        notice,
        removal,
        ..model
    }
}

/// The companion changed something: read the list again, and the open card.
fn changed(model: HelpersModel) -> (HelpersModel, Vec<HelpersCommand>) {
    let mut commands = vec![
        HelpersCommand::List {
            after: model.told_after,
        },
        HelpersCommand::Routines(RoutinesCommand::List),
        HelpersCommand::Routines(RoutinesCommand::ListProposals),
    ];
    commands.extend(model.open_id().cloned().map(HelpersCommand::ReadCard));
    (model, commands)
}

/// A card arrived: it replaces the open card of that helper, keeping what the person typed over
/// the old one; a card nobody is waiting for is ignored.
fn card_read(model: HelpersModel, view: HelperCardView) -> HelpersModel {
    let draft = match &model.open {
        Open::Reading(id) if *id == view.line.id => Some(Draft::of(&view)),
        Open::Card(old) if old.view.line.id == view.line.id => {
            if old.draft == Draft::of(&old.view) {
                Some(Draft::of(&view))
            } else {
                Some(old.draft.clone())
            }
        }
        Open::Reading(_) | Open::Card(_) | Open::New(_) | Open::Nothing => None,
    };
    let Some(draft) = draft else {
        return model;
    };
    HelpersModel {
        open: Open::Card(Box::new(OpenCard { view, draft })),
        ..model
    }
}

fn card_missing(model: HelpersModel, id: &HelperId) -> HelpersModel {
    if model.open_id() == Some(id) {
        HelpersModel {
            open: Open::Nothing,
            notice: Some(words::GONE.to_owned()),
            ..model
        }
    } else {
        model
    }
}

/// A call was answered.
fn replied(
    model: HelpersModel,
    asked: Asked,
    reply: HelperReply,
) -> (HelpersModel, Vec<HelpersCommand>) {
    let model = HelpersModel {
        notice: None,
        ..model
    };
    match (asked, reply) {
        // The window closed after the work was already brought back: nothing is wrong.
        (Asked::BringBackAfterClose, HelperReply::Refused(_)) => refreshed(model),
        (_, HelperReply::Refused(sentence)) => (
            HelpersModel {
                notice: Some(sentence),
                ..model
            },
            Vec::new(),
        ),
        (_, HelperReply::OpenTerminal(request)) => {
            (model, vec![HelpersCommand::OpenTerminal(request)])
        }
        (Asked::Save, HelperReply::Made(id)) => {
            let after = model.told_after;
            (
                HelpersModel {
                    open: Open::Reading(id.clone()),
                    ..model
                },
                vec![HelpersCommand::ReadCard(id), HelpersCommand::List { after }],
            )
        }
        (
            Asked::Save
            | Asked::SignIn
            | Asked::SignOut
            | Asked::OpenInTerminal
            | Asked::BringBack
            | Asked::BringBackAfterClose,
            HelperReply::Done,
        )
        | (Asked::Job, HelperReply::Done | HelperReply::Posted(_)) => refreshed(model),
        (Asked::Job, HelperReply::Made(_))
        | (Asked::Save, HelperReply::Posted(_))
        | (
            Asked::SignIn
            | Asked::SignOut
            | Asked::OpenInTerminal
            | Asked::BringBack
            | Asked::BringBackAfterClose,
            HelperReply::Made(_) | HelperReply::Posted(_),
        ) => (model, Vec::new()),
    }
}

/// Read the list again, and the open card.
fn refreshed(model: HelpersModel) -> (HelpersModel, Vec<HelpersCommand>) {
    let after = model.told_after;
    let mut commands = vec![HelpersCommand::List { after }];
    commands.extend(model.open_id().cloned().map(HelpersCommand::ReadCard));
    (model, commands)
}

fn open_card(model: HelpersModel, id: HelperId) -> (HelpersModel, Vec<HelpersCommand>) {
    if model.open_id() == Some(&id) {
        return (model, Vec::new());
    }
    (
        HelpersModel {
            open: Open::Reading(id.clone()),
            models: ModelList::Closed,
            automatic: Automatic::Off,
            notice: None,
            ..model
        },
        vec![HelpersCommand::ReadCard(id)],
    )
}

/// `model` with its draft changed by `change`; nothing when no card is being filled in.
fn edited(model: HelpersModel, change: impl FnOnce(Draft) -> Draft) -> HelpersModel {
    let open = match model.open {
        Open::Card(card) => {
            let OpenCard { view, draft } = *card;
            Open::Card(Box::new(OpenCard {
                view,
                draft: change(draft),
            }))
        }
        Open::New(draft) => Open::New(change(draft)),
        other @ (Open::Nothing | Open::Reading(_)) => other,
    };
    HelpersModel {
        open,
        notice: None,
        ..model
    }
}

/// `draft` with `app` ticked if it was not, unticked if it was. A card whose apps could not be
/// read starts afresh from the one the person ticks.
fn toggled(draft: Draft, app: AppName) -> Draft {
    let reach = match draft.reach {
        Reach::Chosen { mut apps, ceiling } => {
            if !apps.remove(&app) {
                apps.insert(app);
            }
            Reach::Chosen { apps, ceiling }
        }
        Reach::Unreadable => Reach::Chosen {
            apps: [app].into_iter().collect(),
            ceiling: Ceiling::Look,
        },
    };
    Draft { reach, ..draft }
}

/// `draft` limited to `ceiling`; a card whose apps could not be read starts with no apps.
fn limited(draft: Draft, ceiling: Ceiling) -> Draft {
    let apps = match draft.reach {
        Reach::Chosen { apps, .. } => apps,
        Reach::Unreadable => Default::default(),
    };
    Draft {
        reach: Reach::Chosen { apps, ceiling },
        ..draft
    }
}

/// Save the card: what is missing is said, and nothing is sent.
fn save(model: HelpersModel) -> (HelpersModel, Vec<HelpersCommand>) {
    let Some(draft) = model.draft() else {
        return (model, Vec::new());
    };
    let form = match draft.form() {
        Ok(form) => Box::new(form),
        Err(fault) => {
            let sentence = fault.to_string();
            return (said(model, &sentence), Vec::new());
        }
    };
    let helper = match &model.open {
        Open::Card(card) => Some(card.view.line.id.clone()),
        Open::New(_) | Open::Nothing | Open::Reading(_) => None,
    };
    let command = match helper {
        Some(helper) => HelpersCommand::Edit { helper, form },
        None => HelpersCommand::Create(form),
    };
    (
        HelpersModel {
            notice: None,
            ..model
        },
        vec![command],
    )
}

/// Remove a helper the person was asked about; a removal nobody asked about is not done.
fn remove(model: HelpersModel, id: HelperId) -> (HelpersModel, Vec<HelpersCommand>) {
    if model.removal != Removal::Asked(id.clone()) {
        return (model, Vec::new());
    }
    let open_removed = model.open_id() == Some(&id);
    let open = if open_removed {
        Open::Nothing
    } else {
        model.open
    };
    (
        HelpersModel {
            open,
            removal: Removal::Unasked,
            ..model
        },
        vec![HelpersCommand::Remove(id)],
    )
}

/// Give the open helper a job in the person's words.
fn give_job(model: HelpersModel, goal: &str) -> (HelpersModel, Vec<HelpersCommand>) {
    let Open::Card(card) = &model.open else {
        return (model, Vec::new());
    };
    let goal = goal.trim();
    if goal.is_empty() {
        return (said(model, words::NO_JOB_WORDS), Vec::new());
    }
    let command = HelpersCommand::PostJob {
        helper: card.view.line.id.clone(),
        goal: goal.to_owned(),
    };
    (
        HelpersModel {
            notice: None,
            ..model
        },
        vec![command],
    )
}
