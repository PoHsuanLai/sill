//! The routine the person is filling in: raw words and choices, checked once when they confirm.
//! The time is checked by docket's own reading of a schedule, so a sheet never offers one the
//! companion would refuse.

use companion_wire::{RoutineForm, WhenForm};
use docket_core::routine::{Missed, Schedule};
use prov::HelperId;

/// The most steps one run takes when the person has not said.
const STEPS: u32 = 12;

/// How often a routine runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Repeat {
    /// Every day at a time.
    #[default]
    Daily,
    /// Monday to Friday at a time.
    Weekdays,
    /// Every few hours.
    EveryFewHours,
}

/// Every way to repeat, in the order a choice lists them.
pub const REPEATS: [Repeat; 3] = [Repeat::Daily, Repeat::Weekdays, Repeat::EveryFewHours];

/// Why a draft cannot be confirmed yet.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RoutineFault {
    /// No words for what it is to do.
    #[error("Say what it is to do.")]
    NoGoal,
    /// The time or the hours are not ones a schedule can hold; the sentence says how to write them.
    #[error("{0}")]
    When(String),
}

/// A routine being made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutineDraft {
    /// The helper that will do it.
    pub helper: HelperId,
    /// What it is to do, in the person's words.
    pub goal: String,
    /// How often.
    pub repeat: Repeat,
    /// The time of day, for a daily or weekday routine ("08:00").
    pub time: String,
    /// The hours between runs, for a routine every few hours ("2").
    pub hours: String,
}

impl RoutineDraft {
    /// A blank routine for `helper`: every day at 8:00.
    pub fn blank(helper: HelperId) -> RoutineDraft {
        RoutineDraft {
            helper,
            goal: String::new(),
            repeat: Repeat::Daily,
            time: "08:00".to_owned(),
            hours: "2".to_owned(),
        }
    }

    /// The schedule as docket reads it: "daily 08:00", "weekdays 08:00", "every 2 hours".
    fn words(&self) -> String {
        match self.repeat {
            Repeat::Daily => format!("daily {}", self.time.trim()),
            Repeat::Weekdays => format!("weekdays {}", self.time.trim()),
            Repeat::EveryFewHours => format!("every {} hours", self.hours.trim()),
        }
    }

    /// The form to send, or the first thing missing from it. A time that is missed is skipped.
    pub fn form(&self) -> Result<RoutineForm, RoutineFault> {
        let goal = self.goal.trim();
        if goal.is_empty() {
            return Err(RoutineFault::NoGoal);
        }
        let words = self.words();
        Schedule::parse(&words)
            .map_err(|fault| RoutineFault::When(sentence(&fault.to_string())))?;
        Ok(RoutineForm {
            helper: self.helper.clone(),
            goal: goal.to_owned(),
            when: WhenForm::At(words),
            missed: Missed::Skip,
            steps: STEPS,
        })
    }
}

/// `text` with its first letter capitalised and a full stop at the end.
fn sentence(text: &str) -> String {
    let mut chars = text.chars();
    let first: String = chars
        .next()
        .into_iter()
        .flat_map(char::to_uppercase)
        .collect();
    format!("{first}{}.", chars.as_str())
}
