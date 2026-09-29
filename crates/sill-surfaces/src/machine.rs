//! A timed pure state machine and the one timer that drives it. A [`Machine`] decides
//! `(state, input, time) -> (state, outputs)` and says when it wants to be stepped again with no
//! input; [`use_machine`] owns the state, stamps every step with the frame clock, sleeps with
//! quire's timer until the machine's [`Machine::wake`], and hands each output to the surface's
//! effect handler. Nothing here decides: the surface's `io.rs` feeds inputs and carries out
//! outputs.

use std::collections::VecDeque;
use std::time::Duration;

use dioxus::core::Task;
use dioxus::prelude::*;
use sill_model::units::Stamp;

use crate::clock::FrameClock;

/// What a machine is woken with when the time it asked for (its [`Machine::wake`]) has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Elapsed;

/// A state that changes over time or by input.
pub trait Machine: Clone + PartialEq + Default + 'static {
    /// What moves the machine; it can be woken by the clock alone.
    type In: From<Elapsed>;
    /// What the machine wants done.
    type Out: 'static;
    /// What the surface's settings say about timing; a change applies from the next step.
    type Params: Clone + PartialEq + 'static;

    /// The machine after `input` at `at`, and what it wants done.
    fn step(self, input: Self::In, at: Stamp, params: &Self::Params) -> (Self, Vec<Self::Out>);

    /// When to step again with no input; none when the machine is at rest, so an idle machine
    /// runs no timer at all.
    fn wake(&self) -> Option<Stamp>;
}

/// A running [`Machine`]: send it inputs, read its state. Copy: signals and cells only.
pub struct MachineRef<M: Machine> {
    state: Signal<M>,
    params: CopyValue<M::Params>,
    on_out: CopyValue<Box<dyn FnMut(M::Out)>>,
    pending: CopyValue<VecDeque<M::Out>>,
    draining: CopyValue<Drain>,
    waking: CopyValue<Option<Task>>,
    clock: FrameClock,
}

/// Whether outputs are being handed out, so an input sent from a handler queues its outputs
/// behind the ones being handled instead of running the handler again inside itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Drain {
    Idle,
    Running,
}

impl<M: Machine> Clone for MachineRef<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M: Machine> Copy for MachineRef<M> {}

impl<M: Machine> std::fmt::Debug for MachineRef<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MachineRef")
    }
}

/// Run a machine `M` on `params`, giving each output to `on_out`. The machine starts at its
/// default; it steps on [`MachineRef::send`] and whenever it asked to wake.
pub fn use_machine<M: Machine>(
    params: M::Params,
    on_out: impl FnMut(M::Out) + 'static,
) -> MachineRef<M> {
    let state = use_signal(M::default);
    let mut wanted = use_hook(|| CopyValue::new(params.clone()));
    if *wanted.peek() != params {
        wanted.set(params);
    }
    let mut handler =
        use_hook(|| CopyValue::new(Box::new(|_: M::Out| {}) as Box<dyn FnMut(M::Out)>));
    handler.set(Box::new(on_out));
    MachineRef {
        state,
        params: wanted,
        on_out: handler,
        pending: use_hook(|| CopyValue::new(VecDeque::new())),
        draining: use_hook(|| CopyValue::new(Drain::Idle)),
        waking: use_hook(|| CopyValue::new(None)),
        clock: use_hook(FrameClock::started),
    }
}

impl<M: Machine> MachineRef<M> {
    /// Step the machine with `input` now, and carry out what it says.
    pub fn send(&self, input: M::In) {
        self.advance(input);
    }

    /// The machine's state, live.
    pub fn state(&self) -> ReadSignal<M> {
        self.state.into()
    }

    fn advance(mut self, input: M::In) {
        let now = self.clock.now();
        let (next, outs) = self
            .state
            .peek()
            .clone()
            .step(input, now, &self.params.peek());
        let mut state = self.state;
        if *state.peek() != next {
            state.set(next);
        }
        self.pending.write().extend(outs);
        self.drain();
        self.rewake(now);
    }

    fn drain(mut self) {
        let mut draining = self.draining;
        if *draining.peek() == Drain::Running {
            return;
        }
        draining.set(Drain::Running);
        loop {
            let next = self.pending.write().pop_front();
            let Some(out) = next else { break };
            (self.on_out.write())(out);
        }
        draining.set(Drain::Idle);
    }

    /// Sleep until the machine's next wake, replacing any earlier sleep.
    fn rewake(self, now: Stamp) {
        let mut waking = self.waking;
        if let Some(task) = waking.take() {
            task.cancel();
        }
        if let Some(at) = self.state.peek().wake() {
            let wait = Duration::from_millis(at.since(now));
            waking.set(Some(spawn(async move {
                ds::sleep(wait).await;
                self.advance(M::In::from(Elapsed));
            })));
        }
    }
}
