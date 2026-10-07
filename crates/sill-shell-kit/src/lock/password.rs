//! The password a lock screen or a polkit prompt asks for, as the prompt holds it
//! (design/30 section 2.10): the text of a `TextField { Secure }`, kept out of the markup and
//! out of any signal a render reads, emptied by remounting the field under a new key. Enter hands
//! the text to `onsubmit`, Escape empties it, and arriving at `Wrong` plays `use_shake` once and
//! empties the field when the shake has settled ("Errors shake once and hold still",
//! design/05 principle 7).

use crate::lock::vocab::PromptState;
use dioxus::core::queue_effect;
use dioxus::prelude::*;
use ds::base::word::Word;
use ds::motion::anim::Anim;
use ds::motion::detail::{once::use_shake, touch::Touch, use_detail::use_detail};
use ds::motion::pulse_key::PulseKey;
use ds::motion::timer::{MotionTimer, use_motion_timer};

/// Whether the field holds anything: the enter button shows only once it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
pub(crate) enum Filled {
    /// Nothing typed.
    #[default]
    Empty,
    /// Something typed.
    Typed,
}

impl Filled {
    fn of(text: &str) -> Self {
        if text.is_empty() {
            Filled::Empty
        } else {
            Filled::Typed
        }
    }
}

/// Whether the prompt was in its failed state when last rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failed {
    Wrong,
    Other,
}

/// The prompt's hold on its password.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Password {
    /// The text typed, kept out of the markup and out of any signal a render reads.
    typed: CopyValue<String>,
    /// The field's key: a new one remounts the `Secure` field empty.
    generation: Signal<u32>,
    filled: Signal<Filled>,
    /// The shake, at rest once it has played.
    pub(crate) shake: PulseKey,
    oninput: EventHandler<String>,
}

impl Password {
    /// The key to mount the field under.
    pub(crate) fn key(&self) -> u32 {
        (self.generation)()
    }

    /// Whether anything is typed.
    pub(crate) fn filled(&self) -> Filled {
        (self.filled)()
    }

    /// The field reported `text`.
    pub(crate) fn input(&self, text: String) {
        let mut filled = self.filled;
        let now = Filled::of(&text);
        if *filled.peek() != now {
            filled.set(now);
        }
        let mut typed = self.typed;
        typed.set(text.clone());
        self.oninput.call(text);
    }

    /// Empty the field: remount it, forget the text, and tell the caller.
    pub(crate) fn clear(&self) {
        clear(self.typed, self.generation, self.filled, self.oninput);
    }

    /// Hand the text to `onsubmit`, when there is any.
    pub(crate) fn submit(&self, onsubmit: EventHandler<String>) {
        let text = self.typed.peek().clone();
        if !text.is_empty() {
            onsubmit.call(text);
        }
    }
}

fn clear(
    typed: CopyValue<String>,
    generation: Signal<u32>,
    filled: Signal<Filled>,
    oninput: EventHandler<String>,
) {
    let mut typed = typed;
    typed.set(String::new());
    let _ = ds::style::task::try_set(filled, Filled::Empty);
    if let Ok(round) = ds::style::task::try_get(generation) {
        let _ = ds::style::task::try_set(generation, round.wrapping_add(1));
    }
    oninput.call(String::new());
}

/// The password for a prompt in `state`. Each time `state` becomes `Wrong` the shake plays and,
/// `settle(ShakeX)` later, the field empties. Mounted in `Wrong`, it shakes as it arrives.
pub(crate) fn use_password(state: &PromptState, oninput: EventHandler<String>) -> Password {
    let typed = use_hook(|| CopyValue::new(String::new()));
    let generation = use_signal(|| 0u32);
    let filled = use_signal(Filled::default);
    let shake = use_shake(use_detail(state.clone(), Touch::Remote).cue());
    let emptying = use_motion_timer(Anim::ShakeX);
    let mut seen = use_hook(|| CopyValue::new(Failed::Other));
    let wrong = if state.is_wrong() {
        Failed::Wrong
    } else {
        Failed::Other
    };
    if *seen.peek() != wrong {
        seen.set(wrong);
        if wrong == Failed::Wrong {
            empty_after_shake(
                emptying,
                EventHandler::new(move |()| clear(typed, generation, filled, oninput)),
            );
        }
    }
    Password {
        typed,
        generation,
        filled,
        shake,
        oninput,
    }
}

/// Once the shake has played, `empty` the field. Started from an effect: the timer belongs to
/// the prompt, and a newer arrival restarts it rather than being cut short by an older one.
fn empty_after_shake(timer: MotionTimer, empty: EventHandler<()>) {
    queue_effect(move || timer.start(empty));
}
