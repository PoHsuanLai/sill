//! The bar's reveal as table rows: mode x events (with what holds the bar) -> phase, and the
//! effects that matter. The shared machine's own timing is the dock's (`dock/machine/reveal.rs`).

use super::{
    BarReveal, RevealEffect, RevealEvent, mode_on, rest_phase, shown_blur, shown_input, step,
    strip_input, timing,
};
use ds::base::time::stamp::Stamp;
use ds::prelude::Motion;
use shell_host::BlurSpec;
use sill_model::desktop::Shown;
use sill_settings::MenuBarHiding;
use sill_ui_kit::autohide::{Hold, Phase};

/// One scripted input: when, what, and whether a popup is open at that moment.
type Script = Vec<(u64, RevealEvent, Hold)>;

use Hold::{Free, Held};
use RevealEvent::{Edge, Enter, Frame, Leave, MenuClosed, MenuOpened, Tick};

fn run(mode: MenuBarHiding, motion: Motion, script: &Script) -> (BarReveal, Vec<RevealEffect>) {
    let t = timing(motion);
    let mut effects = Vec::new();
    let mut state = BarReveal::at_rest(mode);
    for &(ms, event, hold) in script {
        let (next, out) = step(state, event, Stamp(ms), hold, &t);
        state = next;
        effects = out;
    }
    (state, effects)
}

fn phase(mode: MenuBarHiding, script: Script) -> Phase {
    run(mode, Motion::Standard, &script).0.phase
}

#[test]
fn each_mode_rests_as_its_place_asks_on_the_desktop_and_in_full_screen() {
    // (mode, on the desktop, over a full-screen Space)
    let cases = [
        (MenuBarHiding::Always, Phase::Hidden, Phase::Hidden),
        (MenuBarHiding::DesktopOnly, Phase::Hidden, Phase::Fixed),
        (MenuBarHiding::FullScreenOnly, Phase::Fixed, Phase::Fixed),
        (MenuBarHiding::Never, Phase::Fixed, Phase::Fixed),
    ];
    for (mode, desktop, full_screen) in cases {
        assert_eq!(
            rest_phase(mode_on(mode, Shown::Hidden)),
            desktop,
            "{mode:?}"
        );
        assert_eq!(
            rest_phase(mode_on(mode, Shown::Shown)),
            full_screen,
            "{mode:?}"
        );
    }
}

#[test]
fn a_hiding_bar_follows_the_pointer_the_popups_and_the_timer() {
    let hiding = [MenuBarHiding::Always, MenuBarHiding::DesktopOnly];
    let cases: Vec<(&str, Script, Phase)> = vec![
        ("rest: hidden", vec![], Phase::Hidden),
        (
            "the pointer at the strip arms",
            vec![(0, Enter, Free)],
            Phase::Armed { since: Stamp(0) },
        ),
        (
            "190 ms: still hidden",
            vec![(0, Enter, Free), (190, Tick, Free)],
            Phase::Armed { since: Stamp(0) },
        ),
        (
            "200 ms: the slide starts",
            vec![(0, Enter, Free), (200, Tick, Free)],
            Phase::Showing { since: Stamp(200) },
        ),
        (
            "leaving the strip disarms",
            vec![(0, Enter, Free), (100, Leave, Free)],
            Phase::Hidden,
        ),
        (
            "the slide completes in 500 ms",
            vec![(0, Enter, Free), (200, Tick, Free), (700, Frame, Free)],
            Phase::Shown,
        ),
        (
            "leaving the bar slides it away",
            vec![
                (0, Enter, Free),
                (200, Tick, Free),
                (700, Frame, Free),
                (900, Leave, Free),
            ],
            Phase::Hiding { since: Stamp(900) },
        ),
        (
            "an open popup keeps it when the pointer leaves",
            vec![
                (0, Enter, Free),
                (200, Tick, Free),
                (700, Frame, Free),
                (800, MenuOpened, Held),
                (900, Leave, Held),
            ],
            Phase::Shown,
        ),
        (
            "closing the last popup with the pointer off slides it away",
            vec![
                (0, Enter, Free),
                (200, Tick, Free),
                (700, Frame, Free),
                (800, MenuOpened, Held),
                (900, Leave, Held),
                (1500, MenuClosed, Free),
            ],
            Phase::Hiding { since: Stamp(1500) },
        ),
        (
            "closing a popup with the pointer on the bar keeps it",
            vec![
                (0, Enter, Free),
                (200, Tick, Free),
                (700, Frame, Free),
                (800, MenuOpened, Held),
                (1500, MenuClosed, Free),
            ],
            Phase::Shown,
        ),
        (
            "the slide out ends hidden",
            vec![
                (0, Enter, Free),
                (200, Tick, Free),
                (700, Frame, Free),
                (900, Leave, Free),
                (1400, Frame, Free),
            ],
            Phase::Hidden,
        ),
        (
            "coming back mid-hide reverses",
            vec![
                (0, Enter, Free),
                (200, Tick, Free),
                (700, Frame, Free),
                (900, Leave, Free),
                (1000, Enter, Free),
            ],
            Phase::Showing { since: Stamp(600) },
        ),
        (
            "a popup opened over IPC shows the hidden bar at once",
            vec![(0, MenuOpened, Held)],
            Phase::Shown,
        ),
        (
            "full screen: the compositor's reveal shows it with no slide",
            vec![(0, Edge(Shown::Shown), Free)],
            Phase::Shown,
        ),
        (
            "full screen: the compositor's hide puts it back",
            vec![
                (0, Edge(Shown::Shown), Free),
                (500, Edge(Shown::Hidden), Free),
            ],
            Phase::Hidden,
        ),
        (
            "full screen: a popup holds it against the hide",
            vec![
                (0, Edge(Shown::Shown), Free),
                (100, MenuOpened, Held),
                (500, Edge(Shown::Hidden), Held),
            ],
            Phase::Shown,
        ),
    ];
    for mode in hiding {
        for (name, script, want) in &cases {
            assert_eq!(phase(mode, script.clone()), *want, "{mode:?}: {name}");
        }
    }
}

#[test]
fn a_fixed_bar_ignores_all_of_it() {
    let script: Script = vec![
        (0, Enter, Free),
        (200, Tick, Free),
        (700, Frame, Free),
        (800, Edge(Shown::Shown), Free),
        (900, Edge(Shown::Hidden), Free),
        (1000, Leave, Free),
    ];
    for mode in [MenuBarHiding::FullScreenOnly, MenuBarHiding::Never] {
        let (state, effects) = run(mode, Motion::Standard, &script);
        assert_eq!(state.phase, Phase::Fixed, "{mode:?}");
        assert!(
            effects
                .iter()
                .all(|e| matches!(e, RevealEffect::StopFrames)),
            "{mode:?}: {effects:?}"
        );
    }
}

#[test]
fn reduced_motion_slides_in_no_time() {
    let script: Script = vec![(0, Enter, Free), (200, Tick, Free), (200, Frame, Free)];
    let (state, _) = run(MenuBarHiding::Always, Motion::Reduced, &script);
    assert_eq!(state.phase, Phase::Shown);
}

#[test]
fn the_regions_follow_the_phase() {
    let go = |script: Script| run(MenuBarHiding::Always, Motion::Standard, &script).1;
    // The slide starts: the bar's content takes input and blur, and frames are asked for.
    assert_eq!(
        go(vec![(0, Enter, Free), (200, Tick, Free)]),
        vec![
            RevealEffect::SetInput(shown_input()),
            RevealEffect::SetBlur(shown_blur()),
            RevealEffect::RequestFrames,
        ]
    );
    // The compositor's reveal: the same regions, no frames.
    assert_eq!(
        go(vec![(0, Edge(Shown::Shown), Free)]),
        vec![
            RevealEffect::SetInput(shown_input()),
            RevealEffect::SetBlur(shown_blur()),
        ]
    );
    // Hidden again: the strip takes input, nothing blurs.
    assert_eq!(
        go(vec![
            (0, Edge(Shown::Shown), Free),
            (1, Edge(Shown::Hidden), Free)
        ]),
        vec![
            RevealEffect::SetInput(strip_input()),
            RevealEffect::SetBlur(BlurSpec::None),
        ]
    );
    // Arming asks for the timer at the dock's delay.
    assert_eq!(
        go(vec![(10, Enter, Free)]),
        vec![RevealEffect::WakeAt(Stamp(210))]
    );
}
