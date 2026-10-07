//! TrackPosition: the Now Playing position, a `ProgressIndicator` bar with its elapsed and
//! remaining times (design/26-DETAILS.md 5.2.10, design/30 section 2.10). A clock-paced report
//! steps: while playing, the bar and the times move once a second, on the second, from the last
//! position reported, and never between; paused or buffering, the bar holds at 0 frames. Mounted only while the panel is open, so it costs one
//! frame a second only while someone can see it.

use crate::now_playing::kind::Playback;
use crate::now_playing::position::{Report, TickIn, Ticker};
use dioxus::prelude::*;
use ds::base::vocab::Fraction;
use ds::components::content::label::{Label, LabelRole, LabelStyle};
use ds::components::controls::progress::model::{Progress, ProgressStyle};
use ds::components::controls::progress::view::ProgressIndicator;
use ds::motion::machine::use_machine;
use ds::root::common::Common;
use ds::style::tokens::control_size::ControlSize;
use std::time::Duration;

/// `m:ss`, or `h:mm:ss` past an hour.
fn clock_text(at: Duration) -> String {
    let seconds = at.as_secs();
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    match hours {
        0 => format!("{minutes}:{seconds:02}"),
        _ => format!("{hours}:{minutes:02}:{seconds:02}"),
    }
}

/// The track at `at` of `length` (the player's last report), advancing a second at a time while
/// `playback` is `Playing`. A new report re-anchors it; the same report again changes nothing.
#[component]
pub fn TrackPosition(
    at: Duration,
    length: Duration,
    #[props(default)] playback: Playback,
    #[props(default)] common: Common,
) -> Element {
    let report = Report {
        at: at.min(length),
        length,
        clock: playback.clock(),
    };
    let ticker = use_machine(|at| Ticker::arrived(report, at), (), || (), |(), _| {});
    if !ticker.state().peek().follows(report) {
        ticker.send_from_render(TickIn::Report(report));
    }
    // Reading the state subscribes: each whole second repaints.
    let shown = ticker.state().read().position(ticker.now());
    let whole = Duration::from_secs(shown.as_secs());
    let share = match length.as_millis() {
        0 => Fraction(0),
        total => Fraction((whole.as_millis() * 1000 / total).min(1000) as u16),
    };
    let left = length.saturating_sub(whole);
    let (elapsed, remaining) = (clock_text(whole), clock_text(left));
    let spoken = format!("Position, {elapsed} of {}", clock_text(length));
    let class = common.class("ds-track-position");
    let data = common.data_attributes();
    rsx! {
        div {
            class,
            id: common.id.clone(),
            onmounted: move |event| common.mounted(event),
            ..data,
            ProgressIndicator {
                style: ProgressStyle::Bar,
                progress: Progress::Known(share),
                size: ControlSize::Mini,
                common: Common { aria_label: Some(spoken), ..Common::default() },
            }
            span { class: "ds-track-times",
                Label { text: elapsed, role: LabelRole::Tertiary, style: LabelStyle::Footnote }
                Label { text: format!("-{remaining}"), role: LabelRole::Tertiary, style: LabelStyle::Footnote }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::clock_text;
    use std::time::Duration;

    #[test]
    fn it_writes_clock_time() {
        let cases = [
            (0, "0:00"),
            (61, "1:01"),
            (600, "10:00"),
            (3_725, "1:02:05"),
        ];
        for (seconds, want) in cases {
            assert_eq!(clock_text(Duration::from_secs(seconds)), want);
        }
    }
}
