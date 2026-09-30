//! `use_machine` on the virtual clock: the machine steps on the inputs sent to it, is woken by
//! the clock alone at the time it asked for, hands every output to the surface's handler once,
//! and runs no timer while it is at rest.

use std::cell::{Cell, RefCell};
use std::time::Duration;

use dioxus::prelude::*;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Input, Query, Viewport};
use sill_model::units::Stamp;
use sill_ui_kit::machine::{Elapsed, Machine, use_machine};

/// A hold that lasts `hold_ms` from the last press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Hold {
    #[default]
    Idle,
    Held {
        until: Stamp,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoldIn {
    Press,
    Tick,
}

impl From<Elapsed> for HoldIn {
    fn from(_: Elapsed) -> Self {
        HoldIn::Tick
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoldOut {
    Pressed,
    Released,
    /// Woken before the hold was due: a wake that should have been dropped.
    Early,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HoldParams {
    hold_ms: u64,
}

impl Machine for Hold {
    type In = HoldIn;
    type Out = HoldOut;
    type Params = HoldParams;

    fn step(self, input: HoldIn, at: Stamp, params: &HoldParams) -> (Hold, Vec<HoldOut>) {
        match (self, input) {
            (_, HoldIn::Press) => (
                Hold::Held {
                    until: at.after(params.hold_ms),
                },
                vec![HoldOut::Pressed],
            ),
            (Hold::Held { until }, HoldIn::Tick) if at >= until => {
                (Hold::Idle, vec![HoldOut::Released])
            }
            (state, HoldIn::Tick) => (state, vec![HoldOut::Early]),
        }
    }

    fn wake(&self) -> Option<Stamp> {
        match self {
            Hold::Idle => None,
            Hold::Held { until } => Some(*until),
        }
    }
}

thread_local! {
    static OUTS: RefCell<Vec<HoldOut>> = const { RefCell::new(Vec::new()) };
    static HOLD_MS: Cell<u64> = const { Cell::new(500) };
}

fn app() -> Element {
    let machine = use_machine::<Hold>(
        HoldParams {
            hold_ms: HOLD_MS.with(Cell::get),
        },
        |out| OUTS.with(|outs| outs.borrow_mut().push(out)),
    );
    let word = match machine.state()() {
        Hold::Idle => "idle",
        Hold::Held { .. } => "held",
    };
    rsx! {
        div { id: "state", style: "width:60px;height:20px", "{word}" }
        div {
            id: "press",
            style: "width:60px;height:20px",
            onpointerdown: move |_| machine.send(HoldIn::Press),
        }
    }
}

fn running(hold_ms: u64) -> Harness {
    OUTS.with(|outs| outs.borrow_mut().clear());
    HOLD_MS.with(|ms| ms.set(hold_ms));
    let viewport = Viewport {
        width: 100,
        height: 40,
        scale_percent: 100,
    };
    let mut harness = Harness::new(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(20));
    harness
}

fn press(harness: &mut Harness) {
    let at = harness.centre("#press").expect("the press target");
    harness.send(Input::click(at));
}

fn word(harness: &Harness) -> Option<String> {
    harness.text_of("#state")
}

fn outs() -> Vec<HoldOut> {
    OUTS.with(|outs| outs.borrow().clone())
}

#[test]
fn the_clock_wakes_the_machine_at_the_time_it_asked_for() {
    let mut harness = running(500);
    assert_eq!(word(&harness).as_deref(), Some("idle"));
    press(&mut harness);
    harness.advance(Duration::from_millis(20));
    assert_eq!(word(&harness).as_deref(), Some("held"));
    assert_eq!(outs(), vec![HoldOut::Pressed]);
    harness.advance(Duration::from_millis(400));
    assert_eq!(
        word(&harness).as_deref(),
        Some("held"),
        "420 ms in: still held"
    );
    harness.advance(Duration::from_millis(140));
    assert_eq!(
        word(&harness).as_deref(),
        Some("idle"),
        "560 ms in: released"
    );
    assert_eq!(outs(), vec![HoldOut::Pressed, HoldOut::Released]);
}

#[test]
fn a_press_while_held_restarts_the_hold_and_the_earlier_wake_never_fires() {
    let mut harness = running(500);
    press(&mut harness);
    harness.advance(Duration::from_millis(300));
    press(&mut harness);
    harness.advance(Duration::from_millis(300));
    assert_eq!(
        word(&harness).as_deref(),
        Some("held"),
        "600 ms after the first press, 300 after the second"
    );
    assert_eq!(outs(), vec![HoldOut::Pressed, HoldOut::Pressed]);
    harness.advance(Duration::from_millis(300));
    assert_eq!(word(&harness).as_deref(), Some("idle"));
    assert_eq!(
        outs(),
        vec![HoldOut::Pressed, HoldOut::Pressed, HoldOut::Released],
        "one release, from the second press"
    );
}

#[test]
fn a_machine_at_rest_runs_no_timer() {
    let mut harness = running(500);
    press(&mut harness);
    harness.advance(Duration::from_millis(600));
    assert_eq!(word(&harness).as_deref(), Some("idle"));
    harness.advance(Duration::from_millis(5000));
    assert_eq!(outs().len(), 2, "nothing more after the release");
}
