//! The moment tables of every component state that implements `Detailed` in design/26's first
//! wave, as data (design/26-DETAILS.md section 4.2; CHECKLIST 5b): every transition, including
//! the ones that must not move.

use ds::motion::detail::detailed::first_table;
use ds::motion::detail::detailed::moment_table;
use ds::prelude::*;
use sill_shell_kit::lock::vocab::PromptState;

#[test]
fn a_lock_prompts_moments() {
    let out = || PromptState::LockedOut {
        until: "9:52".into(),
    };
    moment_table(&[
        (PromptState::Idle, PromptState::Checking, Moment::Pending),
        (PromptState::Wrong, PromptState::Checking, Moment::Pending),
        (PromptState::Checking, PromptState::Wrong, Moment::Failure),
        (PromptState::Checking, PromptState::Idle, Moment::Change),
        (PromptState::Checking, out(), Moment::Unavailable),
        (out(), PromptState::Idle, Moment::Change),
        (PromptState::Wrong, PromptState::Idle, Moment::Change),
    ]);
    first_table(&[
        (PromptState::Idle, Moment::Rest),
        (PromptState::Checking, Moment::Pending),
        (PromptState::Wrong, Moment::Rest),
        (out(), Moment::Rest),
    ]);
}
