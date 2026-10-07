//! The OSD card's presence on a real Blitz document: shown, it enters
//! (`data-presence="entering"`, `osd-in`) and comes to rest; hidden, it leaves (`osd-out`) and
//! `on_hidden` runs when `settle(OsdOut)` is up and not before, with the card no longer drawn;
//! shown again while it fades, the hide is taken back and `on_hidden` never runs for it.

use dioxus::prelude::*;
use ds::base::vocab::Muting;
use ds::components::content::level_glyph::vocab::LevelGlyph;
use ds::prelude::*;
use ds_harness::harness::settle_until;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Query, Viewport};
use sill_shell_kit::osd::OsdLevel;
use sill_shell_kit::prelude::*;
use std::time::Duration;

static SHOWN: GlobalSignal<Shown> = Signal::global(|| Shown::Visible);
static HIDDEN: GlobalSignal<u32> = Signal::global(|| 0);

const VIEW: Viewport = Viewport {
    width: 400,
    height: 160,
    scale_percent: 100,
};

#[allow(non_snake_case)]
fn Card() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Osd, chrome: Some(RootChrome::Transparent),
            Osd {
                shown: SHOWN(),
                label: "Sound",
                level: OsdLevel { value: Fraction(500), glyph: LevelGlyph::Volume(Muting::Audible) },
                on_hidden: move |()| *HIDDEN.write() += 1,
            }
        }
    }
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn presence(harness: &Harness) -> Option<String> {
    harness.attr(".ds-osd", "data-presence")
}

fn hidden(harness: &mut Harness) -> u32 {
    harness.within(|| *HIDDEN.peek())
}

fn show(harness: &mut Harness, shown: Shown) {
    harness.within(|| *SHOWN.write() = shown);
    harness.advance(ms(1));
}

#[test]
fn hidden_it_fades_and_on_hidden_runs_at_settle_and_not_before() {
    let mut harness = Harness::new(Card, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    assert_eq!(presence(&harness).as_deref(), Some("entering"));
    harness.advance(ms(300));
    assert_eq!(presence(&harness).as_deref(), Some("present"));
    let out = settle(Anim::OsdOut, MotionLevel::Standard);
    // Marked on the harness's own (virtual) clock, so the comparison against `out`
    // below is exact, not merely a true lower bound.
    let hiding = harness.now();
    show(&mut harness, Shown::Hidden);
    assert_eq!(presence(&harness).as_deref(), Some("leaving"));

    // Half the fade, not `out - 40ms` (fixed 2026-09-25, FINDINGS "Timing tests"): the old
    // margin was 40 ms of a ~250 ms window (84 % through it), so a loaded machine's overshoot
    // on `advance` (it guarantees *at least* the time asked for, never exactly it) could cross
    // the boundary before this read.
    harness.advance(out / 2);
    assert_eq!(
        hidden(&mut harness),
        0,
        "not well before settle(OsdOut) = {out:?}"
    );
    assert_eq!(presence(&harness).as_deref(), Some("leaving"));

    let gone = settle_until(&mut harness, |h| presence(h).is_none());
    assert_eq!(
        gone.duration_since(hiding),
        out,
        "the card left only once the full fade had run"
    );
    assert_eq!(
        hidden(&mut harness),
        1,
        "on_hidden ran once, at settle(OsdOut)"
    );
    assert_eq!(
        harness.attr(".ds-osd", "data-shown").as_deref(),
        Some("hidden")
    );
    // Shown again: it enters from its first frame.
    show(&mut harness, Shown::Visible);
    assert_eq!(presence(&harness).as_deref(), Some("entering"));
    assert_eq!(harness.attr(".ds-osd", "data-pulse").as_deref(), Some("a"));
}

#[test]
fn a_show_while_it_fades_takes_the_hide_back() {
    let mut harness = Harness::new(Card, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(ms(300));
    show(&mut harness, Shown::Hidden);
    harness.advance(ms(100));
    assert_eq!(presence(&harness).as_deref(), Some("leaving"));
    show(&mut harness, Shown::Visible);
    assert_eq!(presence(&harness).as_deref(), Some("present"));
    harness.advance(ms(500));
    assert_eq!(hidden(&mut harness), 0, "the hide was taken back");
    assert_eq!(presence(&harness).as_deref(), Some("present"));
    assert_eq!(
        harness.attr(".ds-osd", "data-shown").as_deref(),
        Some("visible")
    );
}
