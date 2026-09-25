//! A hot corner's trigger machine: pure, table-tested. The surface feeds it the pointer's enter
//! and leave and quire's timers; it answers when to start a timer and when to fire.
//!
//! - `Armed`: the pointer entering starts the dwell (`StartDwell`, a fresh token).
//! - `Dwelling`: the dwell running out fires (`Fire`) and moves to `Spent` with the re-arm
//!   running (`StartRearm`). Leaving first goes back to `Armed` (the dwell's timer is stale by
//!   its token).
//! - `Spent`: fires nothing; it is `Armed` again once the pointer has left **and** the re-arm
//!   has run out, in either order.
//!
//! No modifier: `hot_corners.modifier` (macOS's "and hold a key" option) is retired. A layer
//! surface with keyboard `None` is never sent `wl_keyboard.modifiers` on Wayland, so the surface
//! cannot know one is held; shell-host's answer (FINDINGS G121, F61) is that no compositor gives
//! a keyboard-less surface that state portably (KWin happens to after an enter, cosmic-comp
//! likely does not) and a fix would need `OnDemand` keyboard, which a hot corner must never take
//! (it would steal focus). So the dwell fires unconditionally once it elapses.

/// A timer's identity; an elapsed timer with another token is stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Token(pub u32);

/// Where the pointer is, for `Spent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Inside {
    In,
    Out,
}

/// Whether `Spent`'s re-arm is still running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rearm {
    Running(Token),
    Over,
}

/// The phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Phase {
    #[default]
    Armed,
    Dwelling(Token),
    Spent {
        pointer: Inside,
        rearm: Rearm,
    },
}

/// The machine: its phase and the next token to hand out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CornerMachine {
    pub phase: Phase,
    pub next: Token,
}

/// The numbers, from `hot_corners.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CornerParams {
    pub dwell_ms: u32,
    pub rearm_ms: u32,
}

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CornerIn {
    Enter,
    Leave,
    DwellElapsed(Token),
    RearmElapsed(Token),
}

/// What to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CornerOut {
    /// Sleep this long with quire's timer, then feed `DwellElapsed(token)`.
    StartDwell(Token, u32),
    /// Sleep this long, then feed `RearmElapsed(token)`.
    StartRearm(Token, u32),
    /// The corner acts: `CornerLink::hit`.
    Fire,
}

/// One step.
pub fn step(
    machine: CornerMachine,
    input: CornerIn,
    params: CornerParams,
) -> (CornerMachine, Vec<CornerOut>) {
    match input {
        CornerIn::Enter => enter(machine, params),
        CornerIn::Leave => leave(machine),
        CornerIn::DwellElapsed(token) => dwell_elapsed(machine, token, params),
        CornerIn::RearmElapsed(token) => rearm_elapsed(machine, token),
    }
}

/// The pointer entered: `Armed` starts a dwell; `Spent` (still waiting on the re-arm) just notes
/// the pointer is back in; `Dwelling` already has it.
fn enter(machine: CornerMachine, params: CornerParams) -> (CornerMachine, Vec<CornerOut>) {
    match machine.phase {
        Phase::Armed => {
            let token = machine.next;
            let phase = Phase::Dwelling(token);
            let next = Token(token.0 + 1);
            (
                CornerMachine { phase, next },
                vec![CornerOut::StartDwell(token, params.dwell_ms)],
            )
        }
        Phase::Spent { rearm, .. } => (
            CornerMachine {
                phase: Phase::Spent {
                    pointer: Inside::In,
                    rearm,
                },
                ..machine
            },
            Vec::new(),
        ),
        Phase::Dwelling(_) => (machine, Vec::new()),
    }
}

/// The pointer left: `Dwelling` cancels back to `Armed`; `Spent` is done once both the pointer
/// has left and the re-arm is over (in either order — the leave itself completes it when the
/// re-arm already ran out), else it only notes the pointer is out.
fn leave(machine: CornerMachine) -> (CornerMachine, Vec<CornerOut>) {
    match machine.phase {
        Phase::Dwelling(_) => (
            CornerMachine {
                phase: Phase::Armed,
                ..machine
            },
            Vec::new(),
        ),
        Phase::Spent {
            rearm: Rearm::Over, ..
        } => (
            CornerMachine {
                phase: Phase::Armed,
                ..machine
            },
            Vec::new(),
        ),
        Phase::Spent { rearm, .. } => (
            CornerMachine {
                phase: Phase::Spent {
                    pointer: Inside::Out,
                    rearm,
                },
                ..machine
            },
            Vec::new(),
        ),
        Phase::Armed => (machine, Vec::new()),
    }
}

/// The dwell ran out: a stale token (the pointer already left, or a fresh dwell started) does
/// nothing; a live one fires.
fn dwell_elapsed(
    machine: CornerMachine,
    token: Token,
    params: CornerParams,
) -> (CornerMachine, Vec<CornerOut>) {
    let Phase::Dwelling(current) = machine.phase else {
        return (machine, Vec::new());
    };
    if current != token {
        return (machine, Vec::new());
    }
    let rearm = machine.next;
    let next = Token(rearm.0 + 1);
    let phase = Phase::Spent {
        pointer: Inside::In,
        rearm: Rearm::Running(rearm),
    };
    (
        CornerMachine { phase, next },
        vec![
            CornerOut::Fire,
            CornerOut::StartRearm(rearm, params.rearm_ms),
        ],
    )
}

/// The re-arm ran out: a stale token does nothing; a live one re-arms at once if the pointer has
/// already left, else only marks the re-arm over (the next leave finishes it).
fn rearm_elapsed(machine: CornerMachine, token: Token) -> (CornerMachine, Vec<CornerOut>) {
    let Phase::Spent {
        pointer,
        rearm: Rearm::Running(current),
    } = machine.phase
    else {
        return (machine, Vec::new());
    };
    if current != token {
        return (machine, Vec::new());
    }
    match pointer {
        Inside::Out => (
            CornerMachine {
                phase: Phase::Armed,
                ..machine
            },
            Vec::new(),
        ),
        Inside::In => (
            CornerMachine {
                phase: Phase::Spent {
                    pointer,
                    rearm: Rearm::Over,
                },
                ..machine
            },
            Vec::new(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CornerIn, CornerMachine, CornerOut, CornerParams, Inside, Phase, Rearm, Token, step,
    };

    const PLAIN: CornerParams = CornerParams {
        dwell_ms: 150,
        rearm_ms: 500,
    };

    fn at(phase: Phase, next: u32) -> CornerMachine {
        CornerMachine {
            phase,
            next: Token(next),
        }
    }

    #[test]
    fn the_corner_follows_its_table() {
        use CornerIn::*;
        let spent = |pointer, rearm| Phase::Spent { pointer, rearm };
        let cases: Vec<(
            CornerMachine,
            CornerIn,
            CornerParams,
            CornerMachine,
            Vec<CornerOut>,
        )> = vec![
            (
                at(Phase::Armed, 0),
                Enter,
                PLAIN,
                at(Phase::Dwelling(Token(0)), 1),
                vec![CornerOut::StartDwell(Token(0), 150)],
            ),
            (
                at(Phase::Dwelling(Token(0)), 1),
                Leave,
                PLAIN,
                at(Phase::Armed, 1),
                vec![],
            ),
            (
                at(Phase::Armed, 1),
                DwellElapsed(Token(0)),
                PLAIN,
                at(Phase::Armed, 1),
                vec![],
            ),
            (
                at(Phase::Dwelling(Token(0)), 1),
                DwellElapsed(Token(0)),
                PLAIN,
                at(spent(Inside::In, Rearm::Running(Token(1))), 2),
                vec![CornerOut::Fire, CornerOut::StartRearm(Token(1), 500)],
            ),
            (
                at(spent(Inside::In, Rearm::Running(Token(1))), 2),
                RearmElapsed(Token(1)),
                PLAIN,
                at(spent(Inside::In, Rearm::Over), 2),
                vec![],
            ),
            (
                at(spent(Inside::In, Rearm::Over), 2),
                Leave,
                PLAIN,
                at(Phase::Armed, 2),
                vec![],
            ),
            (
                at(spent(Inside::In, Rearm::Running(Token(1))), 2),
                Leave,
                PLAIN,
                at(spent(Inside::Out, Rearm::Running(Token(1))), 2),
                vec![],
            ),
            (
                at(spent(Inside::Out, Rearm::Running(Token(1))), 2),
                Enter,
                PLAIN,
                at(spent(Inside::In, Rearm::Running(Token(1))), 2),
                vec![],
            ),
            (
                at(spent(Inside::Out, Rearm::Running(Token(1))), 2),
                RearmElapsed(Token(1)),
                PLAIN,
                at(Phase::Armed, 2),
                vec![],
            ),
        ];
        for (from, input, params, want, outs) in cases {
            assert_eq!(
                step(from, input, params),
                (want, outs),
                "{from:?} + {input:?}"
            );
        }
    }
}
