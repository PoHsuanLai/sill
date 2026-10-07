//! Playing an animated emoji's own animation (design/30 section 2.10, design/25-EMOJI.md): the
//! sheet's frames once through at the frame durations the asset carries, on mounting, on each new
//! [`WakeStamp`] and on each new pick, then at rest on frame 0. Nothing loops and nothing runs
//! between plays (the idle-frame rule). Under Reduced motion, or with
//! [`EmojiPlayback::Still`], only the rest frame is shown. The machine that plays it is owned by
//! the component's scope and dropped with it.

use super::disc::EmojiPlayback;
use super::id::EmojiId;
use super::sheet::durations;
use dioxus::prelude::*;
use ds::base::machine::{Elapsed, Machine};
use ds::base::time::stamp::Stamp;
use ds::motion::machine::use_machine;
use ds::motion::wake::WakeStamp;
use ds::style::appearance::motion::MotionLevel;
use ds::style::scope::use_scope_signal;
use std::time::Duration;

/// What a play is keyed on.
type Seen = (EmojiId, WakeStamp, EmojiPlayback);

/// One frame of the animation and how long it is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Held {
    pub(crate) frame: u16,
    pub(crate) hold: Duration,
}

/// The frames of `emoji`'s animation in order, each with the duration the asset gives it.
pub(crate) fn animation(emoji: EmojiId) -> Vec<Held> {
    durations(emoji)
        .into_iter()
        .zip(0u16..)
        .map(|(hold, frame)| Held { frame, hold })
        .collect()
}

/// The frame shown, and the frames still to come, nearest last.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Frames {
    shown: u16,
    upcoming: Vec<Held>,
    /// When the shown frame has been held long enough; none at rest.
    until: Option<Stamp>,
}

/// What moves the player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FramesIn {
    /// Play these frames from the first, now; none stands on the rest frame.
    Play(Vec<Held>),
    /// The shown frame's hold ended.
    Elapsed,
}

impl From<Elapsed> for FramesIn {
    fn from(_: Elapsed) -> Self {
        FramesIn::Elapsed
    }
}

impl Frames {
    /// The frame to draw: the playing one, or frame 0 at rest.
    pub(crate) fn shown(&self) -> u16 {
        self.shown
    }

    /// `frame` shown from `at` for its hold, with `upcoming` after it (nearest last).
    fn showing(frame: Held, upcoming: Vec<Held>, at: Stamp) -> Self {
        Frames {
            shown: frame.frame,
            upcoming,
            until: Some(at.after_span(frame.hold)),
        }
    }
}

impl Machine for Frames {
    type In = FramesIn;
    type Out = ();
    type Params = ();
    type Ctx = ();

    /// A play shows its first frame at once; each hold's end shows the next, and the last one's
    /// ends on frame 0. Nothing loops and nothing runs at rest. A wake before the hold ends
    /// changes nothing.
    fn step(self, input: FramesIn, at: Stamp, _: &(), _: &()) -> (Frames, Vec<()>) {
        let next = match (input, self.until) {
            (FramesIn::Play(frames), _) => {
                let mut upcoming = frames;
                upcoming.reverse();
                match upcoming.pop() {
                    Some(first) => Frames::showing(first, upcoming, at),
                    None => Frames::default(),
                }
            }
            (FramesIn::Elapsed, Some(until)) if at >= until => {
                let mut upcoming = self.upcoming;
                match upcoming.pop() {
                    Some(next) => Frames::showing(next, upcoming, at),
                    None => Frames::default(),
                }
            }
            (FramesIn::Elapsed, Some(_) | None) => self,
        };
        (next, Vec::new())
    }

    fn wake(&self) -> Option<Stamp> {
        self.until
    }
}

/// The frame to draw now for `emoji`: the animation plays through on mounting and on every new
/// `wake` or pick, and ends on the rest frame.
pub(crate) fn use_frame(emoji: EmojiId, wake: WakeStamp, playback: EmojiPlayback) -> u16 {
    let scope = use_scope_signal();
    let player = use_machine(|_| Frames::default(), (), || (), |(), _| {});
    // The last play's key lives in a plain value, not a signal written while rendering.
    let mut seen = use_hook(|| CopyValue::new(None::<Seen>));
    let key = (emoji, wake, playback);
    if *seen.peek() != Some(key) {
        seen.set(Some(key));
        let frames = match (scope.peek().resolved.motion, playback) {
            (MotionLevel::Reduced, _) | (_, EmojiPlayback::Still) => Vec::new(),
            (MotionLevel::Standard, EmojiPlayback::Once) => animation(emoji),
        };
        player.send_from_render(FramesIn::Play(frames));
    }
    player.state().read().shown()
}

#[cfg(test)]
mod tests {
    use super::{Frames, FramesIn, Held};
    use ds::base::machine::Machine;
    use ds::base::time::stamp::Stamp;
    use std::time::Duration;

    fn held(frame: u16, ms: u64) -> Held {
        Held {
            frame,
            hold: Duration::from_millis(ms),
        }
    }

    #[test]
    fn a_play_shows_each_frame_for_its_hold_and_ends_on_the_rest_frame() {
        let anim = vec![held(0, 100), held(1, 50), held(2, 200)];
        let play = |at| {
            Frames::default()
                .step(FramesIn::Play(anim.clone()), Stamp(at), &(), &())
                .0
        };
        /// Name, state before, input, time, frame shown after, next wake.
        type Case = (&'static str, Frames, FramesIn, u64, u16, Option<u64>);
        let cases: Vec<Case> = vec![
            (
                "a play shows the first frame at once",
                Frames::default(),
                FramesIn::Play(anim.clone()),
                1_000,
                0,
                Some(1_100),
            ),
            (
                "woken early: the same frame",
                play(1_000),
                FramesIn::Elapsed,
                1_099,
                0,
                Some(1_100),
            ),
            (
                "the first hold ends: the second frame",
                play(1_000),
                FramesIn::Elapsed,
                1_100,
                1,
                Some(1_150),
            ),
            (
                "a late wake holds the next from when it ran",
                play(1_000),
                FramesIn::Elapsed,
                1_130,
                1,
                Some(1_180),
            ),
            (
                "an empty play is the rest frame with no wake",
                play(1_000),
                FramesIn::Play(Vec::new()),
                1_020,
                0,
                None,
            ),
            (
                "a new play restarts from the first frame",
                play(1_000),
                FramesIn::Play(anim.clone()),
                1_120,
                0,
                Some(1_220),
            ),
            (
                "a wake at rest does nothing",
                Frames::default(),
                FramesIn::Elapsed,
                1_000,
                0,
                None,
            ),
        ];
        for (name, from, input, at, shown, wake) in cases {
            let (next, out) = from.step(input, Stamp(at), &(), &());
            assert_eq!(next.shown(), shown, "{name}: frame");
            assert!(out.is_empty(), "{name}: outputs");
            assert_eq!(next.wake(), wake.map(Stamp), "{name}: wake");
        }
    }

    #[test]
    fn a_play_run_to_rest_on_its_wakes_shows_every_frame_in_order() {
        let anim = vec![held(0, 100), held(1, 50), held(2, 200)];
        let mut state = Frames::default()
            .step(FramesIn::Play(anim), Stamp(0), &(), &())
            .0;
        let mut seen = vec![state.shown()];
        while let Some(due) = state.wake() {
            state = state.step(FramesIn::Elapsed, due, &(), &()).0;
            seen.push(state.shown());
        }
        assert_eq!(
            seen,
            vec![0, 1, 2, 0],
            "the last frame is followed by the rest frame"
        );
    }
}
