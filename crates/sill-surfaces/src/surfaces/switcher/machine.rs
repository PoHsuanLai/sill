//! The switcher's machine (design/13 §13.4): a quick tap switches with no UI; held past the
//! show delay the panel appears; release activates the selection.
//!
//! The quick tap without the modifiers on focus (G180): a release before the surface has the
//! keyboard is never a key event, so what arrives inside the show delay decides. A `commit`
//! (`ModifierReleased`) switches to the armed selection with no UI; so does a second chord (a
//! second tap whose first release went unseen: the first tap wins, the second is spent). A key
//! the surface itself sees while armed (Tab, `` ` ``, the arrows) proves the chord is held and
//! steps as it would shown. Past the delay a chord steps. On a host surface the modifiers on
//! focus (shell-host's `use_modifiers`) settle the rest outside the machine (`hold.rs`): a
//! release they report is `ModifierReleased`, a chord they report held steps while armed, and
//! an opening that reaches the delay without reading the chord held gets `ModifierReleased`
//! instead of its `Tick`.

use sill_launcher::AppId;
use sill_services::Stamp;
use sill_settings::SwitcherSettings;

/// The switcher's phase. `sel` indexes the MRU app list passed to every [`step`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Switcher {
    #[default]
    Hidden,
    /// Chord seen, the panel not shown yet.
    Armed {
        since: Stamp,
        sel: usize,
    },
    Shown {
        sel: usize,
    },
}

/// Which way a chord or key moves the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dir {
    Next,
    Prev,
}

/// Keys while the switcher is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwKey {
    Tab,
    ShiftTab,
    /// ` (grave): previous.
    Grave,
    Left,
    Right,
    Q,
    H,
    Escape,
}

/// One input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwIn {
    /// `sill switcher next|prev` from the COSMIC shortcut.
    Chord(Dir),
    ModifierReleased,
    Key(SwKey),
    Hover(usize),
    Click(usize),
    Tick,
}

/// One thing the surface must do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwOut {
    Show,
    Hide,
    Select(usize),
    Activate(AppId),
    Quit(AppId),
    HideApp(AppId),
    RequestTick(Stamp),
}

/// `switcher.show_delay_ms`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitcherParams {
    pub show_delay_ms: u64,
}

impl SwitcherParams {
    pub fn new(settings: &SwitcherSettings) -> SwitcherParams {
        SwitcherParams {
            show_delay_ms: u64::from(settings.show_delay_ms.0),
        }
    }
}

/// `s` after `input` at `now`, over the MRU list `apps` (the current app first).
pub fn step(
    s: Switcher,
    input: SwIn,
    now: Stamp,
    apps: &[AppId],
    p: &SwitcherParams,
) -> (Switcher, Vec<SwOut>) {
    let n = apps.len();
    if n == 0 {
        let out = if matches!(s, Switcher::Shown { .. }) {
            vec![SwOut::Hide]
        } else {
            Vec::new()
        };
        return (Switcher::Hidden, out);
    }
    let app = |i: usize| apps[i.min(n - 1)].clone();
    let moved = |sel: usize, dir: Dir| match dir {
        Dir::Next => (sel + 1) % n,
        Dir::Prev => (sel + n - 1) % n,
    };
    match (s, input) {
        (Switcher::Hidden, SwIn::Chord(dir)) => {
            let sel = moved(0, dir);
            (
                Switcher::Armed { since: now, sel },
                vec![SwOut::RequestTick(now.after(p.show_delay_ms))],
            )
        }
        (Switcher::Armed { sel, .. }, SwIn::ModifierReleased) => {
            (Switcher::Hidden, vec![SwOut::Activate(app(sel))])
        }
        (Switcher::Armed { .. }, SwIn::Key(SwKey::Escape)) => (Switcher::Hidden, Vec::new()),
        // A second chord inside the show delay is a second quick tap whose release was never
        // seen (G180: no modifiers on focus): the first tap switches, with no UI.
        (Switcher::Armed { since, sel }, SwIn::Chord(_)) if now.since(since) < p.show_delay_ms => {
            (Switcher::Hidden, vec![SwOut::Activate(app(sel))])
        }
        // A key reaching the surface proves the chord is held: it steps as it would shown.
        (Switcher::Armed { since, sel }, SwIn::Key(key)) => (
            Switcher::Armed {
                since,
                sel: step_of(key).map_or(sel, |dir| moved(sel, dir)),
            },
            Vec::new(),
        ),
        (Switcher::Armed { since, sel }, SwIn::Chord(dir)) => (
            Switcher::Armed {
                since,
                sel: moved(sel, dir),
            },
            Vec::new(),
        ),
        (Switcher::Armed { since, sel }, SwIn::Tick) if now.since(since) >= p.show_delay_ms => (
            Switcher::Shown { sel },
            vec![SwOut::Show, SwOut::Select(sel)],
        ),
        (Switcher::Shown { sel }, SwIn::ModifierReleased) => (
            Switcher::Hidden,
            vec![SwOut::Hide, SwOut::Activate(app(sel))],
        ),
        (Switcher::Shown { .. }, SwIn::Key(SwKey::Escape)) => (Switcher::Hidden, vec![SwOut::Hide]),
        (Switcher::Shown { sel }, SwIn::Chord(dir)) => select(moved(sel, dir)),
        (Switcher::Shown { sel }, SwIn::Key(key)) if step_of(key).is_some() => {
            select(step_of(key).map_or(sel, |dir| moved(sel, dir)))
        }
        (Switcher::Shown { sel }, SwIn::Key(SwKey::Q)) => {
            (Switcher::Shown { sel }, vec![SwOut::Quit(app(sel))])
        }
        (Switcher::Shown { sel }, SwIn::Key(SwKey::H)) => {
            (Switcher::Shown { sel }, vec![SwOut::HideApp(app(sel))])
        }
        (Switcher::Shown { .. }, SwIn::Hover(i)) if i < n => select(i),
        (Switcher::Shown { .. }, SwIn::Click(i)) if i < n => {
            (Switcher::Hidden, vec![SwOut::Hide, SwOut::Activate(app(i))])
        }
        (Switcher::Shown { sel }, _) => (
            Switcher::Shown {
                sel: sel.min(n - 1),
            },
            Vec::new(),
        ),
        (other, _) => (other, Vec::new()),
    }
}

/// Which way a key steps the selection, if it steps it.
fn step_of(key: SwKey) -> Option<Dir> {
    match key {
        SwKey::Tab | SwKey::Right => Some(Dir::Next),
        SwKey::ShiftTab | SwKey::Grave | SwKey::Left => Some(Dir::Prev),
        SwKey::Q | SwKey::H | SwKey::Escape => None,
    }
}

fn select(sel: usize) -> (Switcher, Vec<SwOut>) {
    (Switcher::Shown { sel }, vec![SwOut::Select(sel)])
}

#[cfg(test)]
mod tests {
    use super::{Dir, SwIn, SwKey, SwOut, Switcher, SwitcherParams, step};
    use sill_launcher::AppId;
    use sill_services::Stamp;

    /// One scripted input sequence: `(ms, input)` pairs.
    type Script = Vec<(u64, SwIn)>;

    fn apps() -> Vec<AppId> {
        ["files", "firefox", "foot", "mail"]
            .map(|a| AppId(a.to_owned()))
            .to_vec()
    }

    fn id(a: &str) -> AppId {
        AppId(a.to_owned())
    }

    fn run(inputs: &[(u64, SwIn)]) -> (Switcher, Vec<SwOut>) {
        let p = SwitcherParams { show_delay_ms: 150 };
        let apps = apps();
        inputs.iter().fold(
            (Switcher::Hidden, Vec::new()),
            |(s, mut all), &(ms, input)| {
                let (s, out) = step(s, input, Stamp(ms), &apps, &p);
                all.extend(out);
                (s, all)
            },
        )
    }

    #[test]
    fn design_13_acceptance_8() {
        let chord = (0, SwIn::Chord(Dir::Next));
        let cases: Vec<(&str, Script, Switcher, Vec<SwOut>)> = vec![
            (
                "a quick tap activates the previous app with no UI",
                vec![chord, (80, SwIn::ModifierReleased)],
                Switcher::Hidden,
                vec![
                    SwOut::RequestTick(Stamp(150)),
                    SwOut::Activate(id("firefox")),
                ],
            ),
            (
                "held: the panel appears at 150 ms with index 1",
                vec![chord, (149, SwIn::Tick), (150, SwIn::Tick)],
                Switcher::Shown { sel: 1 },
                vec![
                    SwOut::RequestTick(Stamp(150)),
                    SwOut::Show,
                    SwOut::Select(1),
                ],
            ),
            (
                "Tab twice then release activates index 3",
                vec![
                    chord,
                    (150, SwIn::Tick),
                    (200, SwIn::Key(SwKey::Tab)),
                    (250, SwIn::Key(SwKey::Tab)),
                    (300, SwIn::ModifierReleased),
                ],
                Switcher::Hidden,
                vec![
                    SwOut::RequestTick(Stamp(150)),
                    SwOut::Show,
                    SwOut::Select(1),
                    SwOut::Select(2),
                    SwOut::Select(3),
                    SwOut::Hide,
                    SwOut::Activate(id("mail")),
                ],
            ),
            (
                "Escape activates nothing",
                vec![chord, (150, SwIn::Tick), (200, SwIn::Key(SwKey::Escape))],
                Switcher::Hidden,
                vec![
                    SwOut::RequestTick(Stamp(150)),
                    SwOut::Show,
                    SwOut::Select(1),
                    SwOut::Hide,
                ],
            ),
            (
                "Q quits the selection and stays",
                vec![chord, (150, SwIn::Tick), (200, SwIn::Key(SwKey::Q))],
                Switcher::Shown { sel: 1 },
                vec![
                    SwOut::RequestTick(Stamp(150)),
                    SwOut::Show,
                    SwOut::Select(1),
                    SwOut::Quit(id("firefox")),
                ],
            ),
            (
                "grave steps back and wraps",
                vec![
                    chord,
                    (150, SwIn::Tick),
                    (200, SwIn::Key(SwKey::Grave)),
                    (220, SwIn::Key(SwKey::Grave)),
                ],
                Switcher::Shown { sel: 3 },
                vec![
                    SwOut::RequestTick(Stamp(150)),
                    SwOut::Show,
                    SwOut::Select(1),
                    SwOut::Select(0),
                    SwOut::Select(3),
                ],
            ),
            (
                "a click activates the clicked app",
                vec![chord, (150, SwIn::Tick), (200, SwIn::Click(2))],
                Switcher::Hidden,
                vec![
                    SwOut::RequestTick(Stamp(150)),
                    SwOut::Show,
                    SwOut::Select(1),
                    SwOut::Hide,
                    SwOut::Activate(id("foot")),
                ],
            ),
            (
                "prev from hidden starts at the last app",
                vec![(0, SwIn::Chord(Dir::Prev)), (50, SwIn::ModifierReleased)],
                Switcher::Hidden,
                vec![SwOut::RequestTick(Stamp(150)), SwOut::Activate(id("mail"))],
            ),
        ];
        for (name, inputs, want_state, want_out) in cases {
            assert_eq!(run(&inputs), (want_state, want_out), "{name}");
        }
    }

    /// The quick-tap rule without the modifiers on focus (G180): what arrives inside the show
    /// delay decides.
    #[test]
    fn quick_taps_whose_release_is_not_seen() {
        let chord = (0, SwIn::Chord(Dir::Next));
        let cases: Vec<(&str, Script, Switcher, Vec<SwOut>)> = vec![
            (
                "a second chord inside the delay switches to the previous app, no UI",
                vec![chord, (90, SwIn::Chord(Dir::Next))],
                Switcher::Hidden,
                vec![
                    SwOut::RequestTick(Stamp(150)),
                    SwOut::Activate(id("firefox")),
                ],
            ),
            (
                "a second prev chord inside the delay switches to what the first chose",
                vec![(0, SwIn::Chord(Dir::Prev)), (90, SwIn::Chord(Dir::Prev))],
                Switcher::Hidden,
                vec![SwOut::RequestTick(Stamp(150)), SwOut::Activate(id("mail"))],
            ),
            (
                "a second chord after the delay, before its tick, steps",
                vec![chord, (170, SwIn::Chord(Dir::Next)), (175, SwIn::Tick)],
                Switcher::Shown { sel: 2 },
                vec![
                    SwOut::RequestTick(Stamp(150)),
                    SwOut::Show,
                    SwOut::Select(2),
                ],
            ),
            (
                "a Tab the surface sees while armed steps (the chord is held)",
                vec![
                    chord,
                    (60, SwIn::Key(SwKey::Tab)),
                    (80, SwIn::ModifierReleased),
                ],
                Switcher::Hidden,
                vec![SwOut::RequestTick(Stamp(150)), SwOut::Activate(id("foot"))],
            ),
            (
                "Q while armed does nothing",
                vec![chord, (60, SwIn::Key(SwKey::Q)), (150, SwIn::Tick)],
                Switcher::Shown { sel: 1 },
                vec![
                    SwOut::RequestTick(Stamp(150)),
                    SwOut::Show,
                    SwOut::Select(1),
                ],
            ),
            (
                "Escape while armed cancels with nothing shown",
                vec![chord, (60, SwIn::Key(SwKey::Escape)), (150, SwIn::Tick)],
                Switcher::Hidden,
                vec![SwOut::RequestTick(Stamp(150))],
            ),
            (
                "H hides the selection and stays; hover selects",
                vec![
                    chord,
                    (150, SwIn::Tick),
                    (200, SwIn::Key(SwKey::H)),
                    (220, SwIn::Hover(3)),
                    (230, SwIn::Hover(9)),
                ],
                Switcher::Shown { sel: 3 },
                vec![
                    SwOut::RequestTick(Stamp(150)),
                    SwOut::Show,
                    SwOut::Select(1),
                    SwOut::HideApp(id("firefox")),
                    SwOut::Select(3),
                ],
            ),
        ];
        for (name, inputs, want_state, want_out) in cases {
            assert_eq!(run(&inputs), (want_state, want_out), "{name}");
        }
    }

    #[test]
    fn no_apps_is_no_switcher() {
        let p = SwitcherParams { show_delay_ms: 150 };
        assert_eq!(
            step(Switcher::Hidden, SwIn::Chord(Dir::Next), Stamp(0), &[], &p),
            (Switcher::Hidden, vec![])
        );
        assert_eq!(
            step(Switcher::Shown { sel: 2 }, SwIn::Tick, Stamp(0), &[], &p),
            (Switcher::Hidden, vec![SwOut::Hide])
        );
    }
}
