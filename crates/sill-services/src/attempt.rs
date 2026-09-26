//! A service's attempts at one operation (a Wi-Fi join, a Bluetooth connect) and the stamp it
//! mints for each (quire design/26 R4, R6): the stamp stays the same for as long as one attempt
//! runs, so the bar's pending loop keeps its grace and cap, and every failure gets a new one, so
//! each failure shakes once and a repeated report of the same failure does not.

use serde::{Deserialize, Serialize};

/// One attempt's identity, minted by the service. Only [`Attempts::next`] mints one.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct OpStamp(pub u32);

/// Where the latest attempt stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Attempt {
    /// None is running and the last one did not fail (or there was none).
    #[default]
    Quiet,
    /// One is running, since this stamp was minted.
    Running(OpStamp),
    /// The last one failed; its failure's own stamp.
    Failed(OpStamp),
}

/// What the service saw happen to the operation, derived from its old and new state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttemptStep {
    /// An attempt is under way (a new one, or the same one reported again).
    Start,
    /// The operation ended without the result asked for.
    Fail,
    /// The operation's result is here (joined, connected), or it was dropped with the service.
    Land,
    /// Nothing about the operation changed.
    Hold,
}

/// The latest attempt and the last stamp minted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Attempts {
    pub now: Attempt,
    /// The last stamp minted, so a stamp is never reused while the service runs.
    pub minted: OpStamp,
}

impl Attempts {
    /// The attempts after `step`: a start while one runs keeps its stamp; a start otherwise, and
    /// every failure, mints a new one.
    pub fn next(self, step: AttemptStep) -> Attempts {
        let fresh = OpStamp(self.minted.0.wrapping_add(1));
        match (self.now, step) {
            (Attempt::Running(_), AttemptStep::Start) | (_, AttemptStep::Hold) => self,
            (Attempt::Quiet | Attempt::Failed(_), AttemptStep::Start) => Attempts {
                now: Attempt::Running(fresh),
                minted: fresh,
            },
            (_, AttemptStep::Fail) => Attempts {
                now: Attempt::Failed(fresh),
                minted: fresh,
            },
            (_, AttemptStep::Land) => Attempts {
                now: Attempt::Quiet,
                ..self
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Attempt, AttemptStep, Attempts, OpStamp};

    fn at(now: Attempt, minted: u32) -> Attempts {
        Attempts {
            now,
            minted: OpStamp(minted),
        }
    }

    #[test]
    fn each_attempt_and_each_failure_has_its_own_stamp() {
        use Attempt::{Failed, Quiet, Running};
        let s = OpStamp;
        // (name, before, step, after)
        let cases = [
            (
                "a start mints",
                at(Quiet, 0),
                AttemptStep::Start,
                at(Running(s(1)), 1),
            ),
            (
                "the same join keeps it",
                at(Running(s(1)), 1),
                AttemptStep::Start,
                at(Running(s(1)), 1),
            ),
            (
                "a failure mints",
                at(Running(s(1)), 1),
                AttemptStep::Fail,
                at(Failed(s(2)), 2),
            ),
            (
                "a retry mints",
                at(Failed(s(2)), 2),
                AttemptStep::Start,
                at(Running(s(3)), 3),
            ),
            (
                "a second failure mints",
                at(Failed(s(2)), 2),
                AttemptStep::Fail,
                at(Failed(s(3)), 3),
            ),
            (
                "landing quiets",
                at(Running(s(3)), 3),
                AttemptStep::Land,
                at(Quiet, 3),
            ),
            (
                "landing after a failure",
                at(Failed(s(3)), 3),
                AttemptStep::Land,
                at(Quiet, 3),
            ),
            (
                "hold keeps a failure",
                at(Failed(s(4)), 4),
                AttemptStep::Hold,
                at(Failed(s(4)), 4),
            ),
            (
                "hold keeps a join",
                at(Running(s(4)), 4),
                AttemptStep::Hold,
                at(Running(s(4)), 4),
            ),
            (
                "stamps go on after landing",
                at(Quiet, 3),
                AttemptStep::Start,
                at(Running(s(4)), 4),
            ),
        ];
        for (name, before, step, want) in cases {
            assert_eq!(before.next(step), want, "{name}");
        }
    }
}
