//! The words the helpers screen wears. Plain ones: a helper has a name, a job, a model and a
//! pause; nothing here names how the machinery works.

use companion_wire::{HelperLine, JobLine, JobShown, model_name};
use docket_core::helper::{Ceiling, HelperModel, HelperPhase};
use prov::AppName;
use sill_launcher::HelperControl;

/// Said when a call to the companion could not be made.
pub const UNREACHED: &str = "The helpers could not be reached just now.";

/// Said when a helper is not there any more.
pub const GONE: &str = "That helper is gone.";

/// Said when a job has no words.
pub const NO_JOB_WORDS: &str = "Say what it is to do.";

/// The badge on a helper that needs the person.
pub const WAITING_BADGE: &str = "Waiting for you";

/// Under a helper that has no model.
pub const CHOOSE_MODEL: &str = "Choose a model so it can start";

/// The question before a helper goes.
pub fn remove_question(name: &str) -> String {
    format!("Remove {name}? Its finished jobs stay in your history.")
}

/// The placeholder of the field that gives a helper a job.
pub fn job_prompt(name: &str) -> String {
    format!("Give {name} a job\u{2026}")
}

/// A helper's model as the card names it.
pub fn model_line(model: &HelperModel) -> String {
    match model {
        HelperModel::Unset => "Not chosen yet".to_owned(),
        HelperModel::Named(model) => model_name(&model.to_string()),
    }
}

/// What the card says under a helper's model, when there is something to say.
pub fn model_hint(model: &HelperModel) -> Option<&'static str> {
    match model {
        HelperModel::Unset => Some(CHOOSE_MODEL),
        HelperModel::Named(_) => None,
    }
}

/// The badge a helper's row wears, when it needs the person.
pub fn badge_of(line: &HelperLine) -> Option<&'static str> {
    line.needs_you.map(|_| WAITING_BADGE)
}

/// How far a helper may go, as a choice reads.
pub fn ceiling_label(ceiling: Ceiling) -> &'static str {
    match ceiling {
        Ceiling::Look => "Look at things",
        Ceiling::Change => "Look and change (changes can be undone)",
        Ceiling::Send => "Look, change and send things out",
    }
}

/// Every limit, narrowest first.
pub const CEILINGS: [Ceiling; 3] = [Ceiling::Look, Ceiling::Change, Ceiling::Send];

/// Where a job stands, in a few words.
pub fn job_state_label(state: JobShown) -> &'static str {
    match state {
        JobShown::Waiting => "Waiting its turn",
        JobShown::Working => "Working on it",
        JobShown::Done => "Done",
        JobShown::NeedsModel => "Needs a model first",
    }
}

/// A job as a line of the card: what it is to do, then where it stands and why a try did not
/// finish.
pub fn job_words(job: &JobLine) -> (String, String) {
    let state = job_state_label(job.state);
    let detail = match &job.note {
        Some(note) => format!("{state}. {note}"),
        None => state.to_owned(),
    };
    (job.goal.clone(), detail)
}

/// What the person can do to a helper in this state, in the order a menu lists them.
pub fn controls_of(phase: HelperPhase) -> Vec<HelperControl> {
    match phase {
        HelperPhase::Paused => vec![HelperControl::Resume, HelperControl::Remove],
        HelperPhase::Idle | HelperPhase::Working | HelperPhase::Waiting => {
            vec![HelperControl::Pause, HelperControl::Remove]
        }
    }
}

/// An app as the card names it: the last word of its name (`Notes` for `org.quire.Notes`).
pub fn app_label(app: &AppName) -> String {
    let name = app.as_str();
    name.rsplit('.').next().unwrap_or(name).to_owned()
}

/// Said under a finished job whose answer is no longer kept here.
pub const ANSWER_IN_HISTORY: &str = "Its answer is in your history.";

/// What a finished job answered: its words while the companion keeps them, else where to find
/// them. Nothing for a job that is not done.
pub fn job_answer(job: &JobLine) -> Option<String> {
    match job.state {
        JobShown::Done => Some(
            job.answer
                .clone()
                .unwrap_or_else(|| ANSWER_IN_HISTORY.to_owned()),
        ),
        JobShown::Waiting | JobShown::Working | JobShown::NeedsModel => None,
    }
}
