//! The card's jobs: what the helper was given lately, what it last told the person, and the
//! field that gives it another.

use companion_wire::{HelperCardView, JobLine, JobShown};
use dioxus::prelude::*;
use ds::components::content::label::{LabelRole, LabelStyle};
use ds::prelude::*;
use sill_model::companion::{
    HelpersModel, HelpersMsg, Open, job_answer, job_prompt, job_words, needs_sign_in_line,
};

use super::agent::sign_in_row;
use super::say::{Say, use_say};

/// How many jobs and told sentences the card lists.
const SHOWN: usize = 4;

/// The open card's jobs, newest first.
#[component]
pub(super) fn Jobs(model: HelpersModel) -> Element {
    let say = use_say();
    let Open::Card(card) = &model.open else {
        return rsx! {};
    };
    if card.view.jobs.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "sill-helper-jobs",
            Label { text: "Jobs", style: LabelStyle::Headline }
            for job in card.view.jobs.iter().rev().take(SHOWN) {
                {job_row(job, &card.view, &say)}
            }
        }
    }
}

fn job_row(job: &JobLine, view: &HelperCardView, say: &Say) -> Element {
    let (goal, detail) = job_words(job);
    let answer = job_answer(job);
    let sign_in = match job.state {
        JobShown::NeedsSignIn => rsx! {
            Label { text: needs_sign_in_line(view), role: LabelRole::Secondary }
            div { class: "sill-helper-buttons",
                {sign_in_row(view, "helper-job-sign-in", say)}
            }
        },
        JobShown::Waiting | JobShown::Working | JobShown::Done | JobShown::NeedsModel => {
            rsx! {}
        }
    };
    rsx! {
        div { key: "{job.id}", class: "sill-helper-job",
            Label { text: goal }
            Label { text: detail, role: LabelRole::Secondary, style: LabelStyle::Footnote }
            if let Some(answer) = answer {
                Label { text: answer, role: LabelRole::Secondary }
            }
            {sign_in}
        }
    }
}

/// The newest sentences the open card's helper told the person, newest first.
#[component]
pub(super) fn Latest(model: HelpersModel) -> Element {
    let Open::Card(card) = &model.open else {
        return rsx! {};
    };
    let id = &card.view.line.id;
    let said: Vec<&str> = model
        .told
        .iter()
        .rev()
        .filter(|told| told.helper == *id)
        .take(SHOWN)
        .map(|told| told.text.as_str())
        .collect();
    if said.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "sill-helper-latest",
            Label { text: "Latest", style: LabelStyle::Headline }
            for (at , text) in said.into_iter().enumerate() {
                Label { key: "{at}", text: text.to_owned(), role: LabelRole::Secondary }
            }
        }
    }
}

/// "Give Research a job…": the words go to the helper when the person presses Return.
#[component]
pub(super) fn GiveJob(model: HelpersModel) -> Element {
    let say = use_say();
    let mut goal = use_signal(String::new);
    let Open::Card(card) = &model.open else {
        return rsx! {};
    };
    let prompt = job_prompt(card.view.line.name.as_str());
    rsx! {
        TextField {
            label: prompt.clone(),
            placeholder: prompt,
            value: goal(),
            common: Common { id: Some("helper-job".to_owned()), ..Common::default() },
            oninput: move |next: String| goal.set(next),
            onsubmit: move |text: String| {
                if !text.trim().is_empty() {
                    goal.set(String::new());
                }
                say.msg(HelpersMsg::GiveJob(text));
            },
        }
    }
}
