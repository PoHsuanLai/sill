//! A hot corner's trigger machine: pure, table-tested. The surface feeds it the pointer's enter
//! and leave, the modifiers it knows, and quire's timers; it answers when to start a timer and
//! when to fire.
//!
//! - `Armed`: the pointer entering starts the dwell (`StartDwell`, a fresh token).
//! - `Dwelling`: the dwell running out fires (`Fire`) when the required modifier is held (or
//!   none is required) and moves to `Spent` with the re-arm running (`StartRearm`); without the
//!   modifier it moves to `Blocked`. Leaving first goes back to `Armed` (the dwell's timer is
//!   stale by its token).
//! - `Blocked`: nothing fires until the pointer leaves (back to `Armed`).
//! - `Spent`: fires nothing; it is `Armed` again once the pointer has left **and** the re-arm
//!   has run out, in either order.
//!
//! Modifiers: a layer surface with keyboard `None` is never sent `wl_keyboard.modifiers`, so the
//! surface may not know them (FINDINGS "M10 freeze", G121); `Mods` is whatever it last learned,
//! all up by default, so a required modifier blocks until shell-host can report one.

use sill_settings::CornerModifier;

/// A timer's identity; an elapsed timer with another token is stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Token(pub u32);

/// A key's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum KeyState {
    #[default]
    Up,
    Down,
}

/// The modifiers held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Mods {
    pub super_key: KeyState,
    pub alt: KeyState,
    pub ctrl: KeyState,
    pub shift: KeyState,
}

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
    Blocked,
    Spent {
        pointer: Inside,
        rearm: Rearm,
    },
}

/// The machine: its phase, the modifiers last seen, and the next token to hand out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CornerMachine {
    pub phase: Phase,
    pub mods: Mods,
    pub next: Token,
}

/// The numbers, from `hot_corners.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CornerParams {
    pub dwell_ms: u32,
    pub rearm_ms: u32,
    pub modifier: CornerModifier,
}

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CornerIn {
    Enter,
    Leave,
    DwellElapsed(Token),
    RearmElapsed(Token),
    Mods(Mods),
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
    let _ = (machine, input, params);
    todo!("M10 fill: d-corners")
}

/// Whether `mods` satisfies `needed`.
pub fn held(mods: Mods, needed: CornerModifier) -> KeyState {
    match needed {
        CornerModifier::None => KeyState::Down,
        CornerModifier::Super => mods.super_key,
        CornerModifier::Alt => mods.alt,
        CornerModifier::Ctrl => mods.ctrl,
        CornerModifier::Shift => mods.shift,
    }
}

#[cfg(test)]
mod tests {
    use sill_settings::CornerModifier;

    use super::{
        CornerIn, CornerMachine, CornerOut, CornerParams, Inside, KeyState, Mods, Phase, Rearm,
        Token, held, step,
    };

    const PLAIN: CornerParams = CornerParams {
        dwell_ms: 150,
        rearm_ms: 500,
        modifier: CornerModifier::None,
    };
    const WITH_SUPER: CornerParams = CornerParams {
        modifier: CornerModifier::Super,
        ..PLAIN
    };

    fn at(phase: Phase, next: u32) -> CornerMachine {
        CornerMachine {
            phase,
            mods: Mods::default(),
            next: Token(next),
        }
    }

    #[test]
    fn no_modifier_is_always_held() {
        assert_eq!(held(Mods::default(), CornerModifier::None), KeyState::Down);
        assert_eq!(held(Mods::default(), CornerModifier::Super), KeyState::Up);
        let down = Mods {
            shift: KeyState::Down,
            ..Mods::default()
        };
        assert_eq!(held(down, CornerModifier::Shift), KeyState::Down);
    }

    #[test]
    #[ignore = "M10 fill: d-corners"]
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
            (
                at(Phase::Dwelling(Token(0)), 1),
                DwellElapsed(Token(0)),
                WITH_SUPER,
                at(Phase::Blocked, 1),
                vec![],
            ),
            (
                at(Phase::Blocked, 1),
                Leave,
                WITH_SUPER,
                at(Phase::Armed, 1),
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

    #[test]
    #[ignore = "M10 fill: d-corners"]
    fn a_held_modifier_lets_the_corner_fire() {
        let down = Mods {
            super_key: KeyState::Down,
            ..Mods::default()
        };
        let (with_mods, outs) = step(
            at(Phase::Dwelling(Token(0)), 1),
            CornerIn::Mods(down),
            WITH_SUPER,
        );
        assert_eq!(outs, vec![]);
        assert_eq!(with_mods.mods, down);
        let (fired, outs) = step(with_mods, CornerIn::DwellElapsed(Token(0)), WITH_SUPER);
        assert_eq!(
            outs,
            vec![CornerOut::Fire, CornerOut::StartRearm(Token(1), 500)]
        );
        assert_eq!(
            fired.phase,
            Phase::Spent {
                pointer: Inside::In,
                rearm: Rearm::Running(Token(1))
            }
        );
    }
}
