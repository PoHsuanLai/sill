//! A notification's body, clamped to two lines at rest and six under the pointer (design/13 section
//! 13.3.6). Blitz has no `-webkit-line-clamp` (the risk table; CONSUMING
//! section 8), so the clamp is a `max-height` of whole lines (`calc(2 * 1.35em)`, `6 *` on
//! hover) with a `--t-move --e-out` transition, and the fade that marks cut text is decided in
//! Rust: the body measures its text and one line once laid out, counts its lines, and writes
//! `data-clip` so the fade is drawn only where text is actually cut. A body of two lines never
//! fades; one of four fades at rest and not on hover; one of nine fades on both.

use dioxus::prelude::*;
use ds::base::word::Word;
use ds::components::content::rich_text::{Rich, rich};
use ds::host::measure::use_rect;

/// The lines a body is clamped to at rest.
const REST_LINES: u8 = 2;
/// The lines a body opens to under the pointer.
const HOVER_LINES: u8 = 6;

/// Where a body of some number of lines is cut: `data-clip`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub(crate) enum Clip {
    /// It fits at rest: nothing is cut, nothing fades.
    Never,
    /// Cut at rest, whole under the pointer: it fades only at rest.
    Rest,
    /// Cut even when open: it fades at rest and under the pointer.
    Always,
}

impl Clip {
    /// The clip for a body of `lines` lines.
    pub(crate) fn of(lines: u8) -> Self {
        match lines {
            0..=REST_LINES => Clip::Never,
            3..=HOVER_LINES => Clip::Rest,
            _ => Clip::Always,
        }
    }

    /// The `data-clip` word, or nothing when nothing is cut.
    fn attr(self) -> Option<&'static str> {
        (self != Clip::Never).then(|| self.slug())
    }
}

/// How many lines text `height` tall makes at `line` per line, to the nearest whole line.
pub(crate) fn lines(height: f32, line: f32) -> u8 {
    if line <= 0.0 {
        return 0;
    }
    (height / line).round().clamp(0.0, f32::from(u8::MAX)) as u8
}

/// The body: its runs inside a clamped box, links reporting to `on_link`. Keyed by the caller
/// on its text, so new words are measured again.
#[component]
pub(crate) fn NotificationBody(body: Rich, on_link: Option<EventHandler<String>>) -> Element {
    let text = use_rect();
    let line = use_rect();
    let clip = match (text.rect(), line.rect()) {
        (Some(text), Some(line)) => Clip::of(lines(text.size.height.0, line.size.height.0)),
        _ => Clip::Never,
    };
    rsx! {
        div { class: "ds-notification-body", "data-clip": clip.attr(),
            div {
                class: "ds-notification-body-text",
                onmounted: move |event| text.on_mounted(event),
                {rich(&body, on_link)}
            }
            span {
                class: "ds-notification-body-line",
                "aria-hidden": "true",
                onmounted: move |event| line.on_mounted(event),
                "X"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Clip, lines};

    #[test]
    fn a_body_fades_only_where_it_is_cut() {
        const CASES: &[(u8, Clip)] = &[
            (0, Clip::Never),
            (1, Clip::Never),
            (2, Clip::Never),
            (3, Clip::Rest),
            (6, Clip::Rest),
            (7, Clip::Always),
            (40, Clip::Always),
        ];
        for &(count, want) in CASES {
            assert_eq!(Clip::of(count), want, "{count} lines");
        }
    }

    #[test]
    fn a_height_counts_whole_lines() {
        const CASES: &[(f32, f32, u8)] = &[
            (17.55, 17.55, 1),
            (35.1, 17.55, 2),
            (52.0, 17.55, 3),
            (70.9, 17.55, 4),
            (0.0, 17.55, 0),
            (40.0, 0.0, 0),
        ];
        for &(height, line, want) in CASES {
            assert_eq!(lines(height, line), want, "{height} / {line}");
        }
    }
}
