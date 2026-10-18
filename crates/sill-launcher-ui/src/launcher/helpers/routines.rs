//! A helper's routines, under its jobs: what it does by itself and when, to pause, resume or remove
//! each, and "Add Routine...", which opens a panel where the person confirms a new one before
//! anything is made. The words and the checks are the model's (`sill_model::companion`); this draws
//! them, and says wherever a time is shown that a missed run is skipped.

use companion_wire::{RoutineLine, RoutineShown};
use dioxus::prelude::*;
use ds::components::content::label::{LabelRole, LabelStyle};
use ds::components::controls::button_model::{Bezel, ButtonRole};
use ds::components::fields::field_row::{FieldGroup, FieldRow};
use ds::prelude::*;
use prov::HelperId;
use sill_model::companion::{
    HelpersModel, HelpersMsg, MISSED_SKIPPED, NO_ROUTINES, REPEATS, Repeat, RoutineDraft,
    RoutineRemoval, RoutinesMsg, Sheet, last_run_line, remove_routine_question, repeat_label,
    state_label,
};

use super::say::{Say, use_say};

/// The routines of `helper`, with the panel that adds one while it is open.
#[component]
pub(super) fn Routines(model: HelpersModel, helper: HelperId) -> Element {
    let say = use_say();
    let rows: Vec<&RoutineLine> = model.routines.of_helper(&helper).collect();
    let add_for = helper.clone();
    let add = say.clone();
    let panel = match &model.routines.sheet {
        Sheet::Closed => rsx! {
            Button {
                label: "Add Routine\u{2026}",
                bezel: Bezel::Inline,
                common: Common { id: Some("helper-routine-add".to_owned()), ..Common::default() },
                onclick: move |_| add.msg(routine(RoutinesMsg::OpenSheet(add_for.clone()))),
            }
        },
        Sheet::Adding(draft) => rsx! {
            RoutineSheet { draft: draft.clone(), notice: model.routines.notice.clone() }
        },
    };
    rsx! {
        div { class: "sill-helper-routines",
            Label { text: "Routines", style: LabelStyle::Headline }
            if rows.is_empty() {
                Label { text: NO_ROUTINES, role: LabelRole::Secondary }
            }
            for line in rows.iter() {
                {routine_row(line, &model, &say)}
            }
            if !rows.is_empty() {
                Label { text: MISSED_SKIPPED, role: LabelRole::Secondary, style: LabelStyle::Footnote }
            }
            {panel}
        }
    }
}

fn routine(msg: RoutinesMsg) -> HelpersMsg {
    HelpersMsg::Routines(msg)
}

/// One routine: what it does, when, whether it runs and how its last run came out, with the
/// buttons that move it. Remove asks here first.
fn routine_row(line: &RoutineLine, model: &HelpersModel, say: &Say) -> Element {
    let id = line.id;
    let asked = model.routines.removal == RoutineRemoval::Asked(id);
    let (toggle_label, toggle) = match line.state {
        RoutineShown::On => ("Pause", RoutinesMsg::Pause(id)),
        RoutineShown::Paused | RoutineShown::Stopped => ("Resume", RoutinesMsg::Resume(id)),
    };
    let (toggler, remover) = (say.clone(), say.clone());
    rsx! {
        div { key: "{id}", class: "sill-helper-routine",
            Label { text: line.goal.clone() }
            Label { text: line.when.clone(), role: LabelRole::Secondary, style: LabelStyle::Footnote }
            Label { text: state_label(line.state), role: LabelRole::Secondary, style: LabelStyle::Footnote }
            if let Some(last) = last_run_line(line) {
                Label { text: last, role: LabelRole::Secondary, style: LabelStyle::Footnote }
            }
            div { class: "sill-helper-buttons",
                Button {
                    label: toggle_label,
                    bezel: Bezel::Inline,
                    common: Common { id: Some(format!("helper-routine-toggle-{id}")), ..Common::default() },
                    onclick: move |_| toggler.msg(routine(toggle.clone())),
                }
                Button {
                    label: "Remove",
                    bezel: Bezel::Inline,
                    common: Common { id: Some(format!("helper-routine-remove-{id}")), ..Common::default() },
                    onclick: move |_| remover.msg(routine(RoutinesMsg::AskRemove(id))),
                }
            }
            if asked {
                {removal_question(line, say)}
            }
        }
    }
}

/// "Remove "Read the news"? Runs it already did stay in your history.", with its two answers.
fn removal_question(line: &RoutineLine, say: &Say) -> Element {
    let question = remove_routine_question(&line.goal);
    let (keep, remove, id) = (say.clone(), say.clone(), line.id);
    rsx! {
        div { class: "sill-helper-ask", role: "alert",
            p { "{question}" }
            div { class: "sill-helper-buttons",
                Button {
                    label: "Cancel",
                    common: Common { id: Some("helper-routine-keep".to_owned()), ..Common::default() },
                    onclick: move |_| keep.msg(routine(RoutinesMsg::CancelRemove)),
                }
                Button {
                    label: "Remove",
                    role: ButtonRole::Destructive,
                    common: Common { id: Some("helper-routine-remove-yes".to_owned()), ..Common::default() },
                    onclick: move |_| remove.msg(routine(RoutinesMsg::Remove(id))),
                }
            }
        }
    }
}

/// The panel where a routine is confirmed: what it is to do, how often, and a reminder that a
/// missed run is skipped. The words typed are held here until the person confirms.
#[component]
fn RoutineSheet(draft: RoutineDraft, notice: Option<String>) -> Element {
    let say = use_say();
    let mut goal = use_signal(|| draft.goal.clone());
    let mut time = use_signal(|| draft.time.clone());
    let mut hours = use_signal(|| draft.hours.clone());
    let choices: Vec<Choice<Repeat>> = REPEATS
        .iter()
        .map(|repeat| Choice::new(*repeat, repeat_label(*repeat)))
        .collect();
    let repeat = draft.repeat;
    let (repeat_say, cancel, confirm) = (say.clone(), say.clone(), say.clone());
    let when = match repeat {
        Repeat::Daily | Repeat::Weekdays => rsx! {
            FieldRow { label: "At",
                TextField {
                    label: "Time of day",
                    value: time(),
                    placeholder: "08:00",
                    oninput: move |next: String| time.set(next),
                }
            }
        },
        Repeat::EveryFewHours => rsx! {
            FieldRow { label: "Every",
                TextField {
                    label: "Hours between runs",
                    value: hours(),
                    placeholder: "2",
                    oninput: move |next: String| hours.set(next),
                }
            }
        },
    };
    rsx! {
        div { class: "sill-helper-ask",
            FieldGroup {
                FieldRow { label: "Do",
                    TextField {
                        label: "What it is to do",
                        kind: FieldKind::Multiline,
                        value: goal(),
                        placeholder: "Read the news and tell me what matters",
                        oninput: move |next: String| goal.set(next),
                    }
                }
                FieldRow { label: "Repeat",
                    RadioGroup {
                        label: "How often",
                        choices,
                        value: repeat,
                        onchange: move |chosen: Repeat| {
                            repeat_say.msg(routine(RoutinesMsg::SetRepeat(chosen)));
                        },
                    }
                }
                {when}
            }
            Label { text: MISSED_SKIPPED, role: LabelRole::Secondary, style: LabelStyle::Footnote }
            if let Some(sentence) = &notice {
                p { class: "sill-helper-notice", role: "status", "{sentence}" }
            }
            div { class: "sill-helper-buttons",
                Button {
                    label: "Cancel",
                    common: Common { id: Some("helper-routine-cancel".to_owned()), ..Common::default() },
                    onclick: move |_| cancel.msg(routine(RoutinesMsg::CloseSheet)),
                }
                Button {
                    label: "Add Routine",
                    common: Common { id: Some("helper-routine-confirm".to_owned()), ..Common::default() },
                    onclick: move |_| {
                        confirm.msg(routine(RoutinesMsg::EditGoal(goal())));
                        confirm.msg(routine(RoutinesMsg::EditTime(time())));
                        confirm.msg(routine(RoutinesMsg::EditHours(hours())));
                        confirm.msg(routine(RoutinesMsg::Confirm));
                    },
                }
            }
        }
    }
}
