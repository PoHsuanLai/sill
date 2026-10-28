//! Where a summon goes (design/32 section 4.1): the launcher when it is open or the app has no
//! prompt of its own, the focused app's prompt when it serves the companion and answers within
//! `app_wait`. A pure machine; the service carries each [`SummonOut`] out.

use docket_wire::{SummonAnswer, SummonSerial};
use ds_core::machine::{Elapsed, Machine};
use ds_core::time::stamp::Stamp;
use ds_core::word::Word;
use prov::AppName;
use serde::{Deserialize, Serialize};
use sill_launcher::{LauncherMode, WindowKey};

use crate::units::Ms;

/// What a summon finds when it arrives.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FocusFacts {
    /// The focused window.
    pub window: Option<WindowKey>,
    /// Its app, when it has a bus name.
    pub app: Option<AppName>,
    /// The panel's mode while it is showing; `None` when it is not.
    pub launcher: Option<LauncherMode>,
    /// True when the app owns a companion client on the bus (it answers
    /// `IntentProvider1.Summon`).
    pub serves_companion: bool,
}

/// Where the launcher's companion mode came from, which decides how Escape leaves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LauncherOrigin {
    /// Opened for the summon, over this window.
    Fresh {
        /// The window it was summoned over.
        window: Option<WindowKey>,
    },
    /// The search palette switched in place; leaving restores the search.
    FromSearch,
}

/// What an unanswered ask falls back to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fallback {
    /// Open the launcher in companion mode.
    Launcher,
    /// Do nothing: the ask only wanted the app to put its prompt away.
    Rest,
}

/// Where a summon is.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Summon {
    /// Nothing summoned.
    #[default]
    Rest,
    /// The app was asked; its answer or `until` decides.
    AskingApp {
        /// The app asked.
        app: AppName,
        /// Its window.
        window: Option<WindowKey>,
        /// Which ask this is.
        serial: SummonSerial,
        /// When to stop waiting.
        until: Stamp,
        /// What the silence means.
        fallback: Fallback,
    },
    /// The app's own field is the prompt.
    InApp {
        /// The app.
        app: AppName,
        /// The ask it took.
        serial: SummonSerial,
        /// Its window.
        window: Option<WindowKey>,
    },
    /// The launcher is the prompt.
    InLauncher {
        /// How it came to be.
        origin: LauncherOrigin,
    },
}

/// What moves the machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SummonIn {
    /// A summon, with what it found.
    Summon(FocusFacts),
    /// The app answered an ask.
    AppAnswered(SummonSerial, SummonAnswer),
    /// The wait passed.
    Elapsed,
    /// The panel closed.
    LauncherClosed,
    /// Put the companion away.
    Dismiss,
}

impl From<Elapsed> for SummonIn {
    fn from(_: Elapsed) -> SummonIn {
        SummonIn::Elapsed
    }
}

/// What the machine asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SummonOut {
    /// Call `IntentProvider1.Summon(serial, origin)` on the app.
    AskApp {
        /// The app.
        app: AppName,
        /// The serial.
        serial: SummonSerial,
    },
    /// Show the launcher in `mode`, over `window`.
    OpenLauncher {
        /// The mode.
        mode: LauncherMode,
        /// The window it opens over.
        window: Option<WindowKey>,
    },
    /// Change the open launcher's mode in place.
    SwitchLauncher(LauncherMode),
    /// Hide the launcher.
    CloseLauncher,
}

/// Whether a summon goes to the field of the focused app or to the launcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum InField {
    /// The field becomes the prompt.
    #[default]
    Prompt,
    /// The launcher takes every summon.
    Launcher,
}

/// How long to wait for an app, and where a summon in a field goes
/// (`companion.in_field`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SummonParams {
    /// How long the app has to answer.
    pub app_wait: Ms,
    /// Where a summon in a field goes.
    pub in_field: InField,
}

impl Default for SummonParams {
    fn default() -> SummonParams {
        SummonParams {
            app_wait: Ms(80),
            in_field: InField::Prompt,
        }
    }
}

/// What the machine reads each step: the serial the next ask carries (the service counts them, so
/// a late answer to an old ask is never taken for the newest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SummonCx {
    /// The serial for an ask made now.
    pub next: SummonSerial,
}

impl Default for SummonCx {
    fn default() -> SummonCx {
        SummonCx {
            next: SummonSerial(1),
        }
    }
}

impl Machine for Summon {
    type In = SummonIn;
    type Out = SummonOut;
    type Params = SummonParams;
    type Ctx = SummonCx;

    fn step(
        self,
        input: SummonIn,
        at: Stamp,
        params: &SummonParams,
        cx: &SummonCx,
    ) -> (Summon, Vec<SummonOut>) {
        match (self, input) {
            (Summon::Rest, SummonIn::Summon(facts)) => summoned(facts, at, *params, *cx),
            (Summon::InLauncher { origin }, SummonIn::Summon(_) | SummonIn::Dismiss) => {
                (Summon::Rest, leave(origin))
            }
            (Summon::InLauncher { .. }, SummonIn::LauncherClosed) => (Summon::Rest, Vec::new()),
            (Summon::InApp { app, window, .. }, SummonIn::Summon(_) | SummonIn::Dismiss) => {
                ask(app, window, at, *params, *cx, Fallback::Rest)
            }
            (
                Summon::AskingApp {
                    serial: asked,
                    window,
                    app,
                    fallback,
                    ..
                },
                SummonIn::AppAnswered(serial, answer),
            ) if serial == asked => match answer {
                SummonAnswer::TookField | SummonAnswer::TookAnchored => (
                    Summon::InApp {
                        app,
                        serial,
                        window,
                    },
                    Vec::new(),
                ),
                SummonAnswer::Restored => (Summon::Rest, Vec::new()),
                SummonAnswer::Declined => fall_back(fallback, window),
            },
            (
                Summon::AskingApp {
                    until,
                    window,
                    fallback,
                    ..
                },
                SummonIn::Elapsed,
            ) if at >= until => fall_back(fallback, window),
            // A late answer to an older ask, or a wake before the wait is over.
            (state, _) => (state, Vec::new()),
        }
    }

    fn wake(&self) -> Option<Stamp> {
        match self {
            Summon::AskingApp { until, .. } => Some(*until),
            Summon::Rest | Summon::InApp { .. } | Summon::InLauncher { .. } => None,
        }
    }
}

/// A summon arriving at rest: the open launcher takes it, else the focused app's field when it
/// serves the companion and the settings send summons there, else a fresh launcher.
fn summoned(
    facts: FocusFacts,
    at: Stamp,
    params: SummonParams,
    cx: SummonCx,
) -> (Summon, Vec<SummonOut>) {
    let FocusFacts {
        window,
        app,
        launcher,
        serves_companion,
    } = facts;
    match (launcher, app, serves_companion, params.in_field) {
        (Some(LauncherMode::Search), _, _, _) => (
            Summon::InLauncher {
                origin: LauncherOrigin::FromSearch,
            },
            vec![SummonOut::SwitchLauncher(LauncherMode::Companion)],
        ),
        // The launcher is already in companion mode and nothing here opened it: close it.
        (Some(LauncherMode::Companion), _, _, _) => (Summon::Rest, vec![SummonOut::CloseLauncher]),
        (None, Some(app), true, InField::Prompt) => {
            ask(app, window, at, params, cx, Fallback::Launcher)
        }
        (None, _, _, _) => (
            Summon::InLauncher {
                origin: LauncherOrigin::Fresh { window },
            },
            vec![SummonOut::OpenLauncher {
                mode: LauncherMode::Companion,
                window,
            }],
        ),
    }
}

/// Ask `app` to take the summon, waiting `app_wait` for its answer.
fn ask(
    app: AppName,
    window: Option<WindowKey>,
    at: Stamp,
    params: SummonParams,
    cx: SummonCx,
    fallback: Fallback,
) -> (Summon, Vec<SummonOut>) {
    (
        Summon::AskingApp {
            app: app.clone(),
            window,
            serial: cx.next,
            until: at.after(params.app_wait.span()),
            fallback,
        },
        vec![SummonOut::AskApp {
            app,
            serial: cx.next,
        }],
    )
}

/// What an ask nobody took leads to.
fn fall_back(fallback: Fallback, window: Option<WindowKey>) -> (Summon, Vec<SummonOut>) {
    match fallback {
        Fallback::Launcher => (
            Summon::InLauncher {
                origin: LauncherOrigin::Fresh { window },
            },
            vec![SummonOut::OpenLauncher {
                mode: LauncherMode::Companion,
                window,
            }],
        ),
        Fallback::Rest => (Summon::Rest, Vec::new()),
    }
}

/// Leaving the launcher's companion mode: back to the search it came from, or closed.
fn leave(origin: LauncherOrigin) -> Vec<SummonOut> {
    match origin {
        LauncherOrigin::FromSearch => vec![SummonOut::SwitchLauncher(LauncherMode::Search)],
        LauncherOrigin::Fresh { .. } => vec![SummonOut::CloseLauncher],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str) -> AppName {
        AppName::parse(name).unwrap_or_else(|e| panic!("{e}"))
    }

    fn facts(launcher: Option<LauncherMode>, app_name: Option<&str>, serves: bool) -> FocusFacts {
        FocusFacts {
            window: Some(WindowKey(7)),
            app: app_name.map(app),
            launcher,
            serves_companion: serves,
        }
    }

    const WINDOW: Option<WindowKey> = Some(WindowKey(7));

    fn asking(fallback: Fallback) -> Summon {
        Summon::AskingApp {
            app: app("org.quire.Mail"),
            window: WINDOW,
            serial: SummonSerial(1),
            until: Stamp(180),
            fallback,
        }
    }

    fn in_app() -> Summon {
        Summon::InApp {
            app: app("org.quire.Mail"),
            serial: SummonSerial(1),
            window: WINDOW,
        }
    }

    fn fresh() -> Summon {
        Summon::InLauncher {
            origin: LauncherOrigin::Fresh { window: WINDOW },
        }
    }

    fn open_companion() -> SummonOut {
        SummonOut::OpenLauncher {
            mode: LauncherMode::Companion,
            window: WINDOW,
        }
    }

    type Case = (&'static str, Summon, SummonIn, u64, Summon, Vec<SummonOut>);

    /// Every row of design/32 section 4.1, and the rows its table leaves implicit.
    #[test]
    fn summon_table() {
        let mail = Some("org.quire.Mail");
        let from_search = Summon::InLauncher {
            origin: LauncherOrigin::FromSearch,
        };
        let ask_mail = SummonOut::AskApp {
            app: app("org.quire.Mail"),
            serial: SummonSerial(1),
        };
        let cases: Vec<Case> = vec![
            (
                "rest, search open: the palette becomes the prompt",
                Summon::Rest,
                SummonIn::Summon(facts(Some(LauncherMode::Search), mail, true)),
                100,
                from_search.clone(),
                vec![SummonOut::SwitchLauncher(LauncherMode::Companion)],
            ),
            (
                "rest, companion open and not ours: close it",
                Summon::Rest,
                SummonIn::Summon(facts(Some(LauncherMode::Companion), None, false)),
                100,
                Summon::Rest,
                vec![SummonOut::CloseLauncher],
            ),
            (
                "rest, a serving app in a field: ask it",
                Summon::Rest,
                SummonIn::Summon(facts(None, mail, true)),
                100,
                asking(Fallback::Launcher),
                vec![ask_mail.clone()],
            ),
            (
                "rest, an app that does not serve: the launcher",
                Summon::Rest,
                SummonIn::Summon(facts(None, mail, false)),
                100,
                fresh(),
                vec![open_companion()],
            ),
            (
                "rest, nothing focused: the launcher",
                Summon::Rest,
                SummonIn::Summon(facts(None, None, false)),
                100,
                fresh(),
                vec![open_companion()],
            ),
            (
                "asking, took the field",
                asking(Fallback::Launcher),
                SummonIn::AppAnswered(SummonSerial(1), SummonAnswer::TookField),
                120,
                in_app(),
                vec![],
            ),
            (
                "asking, took an anchored prompt",
                asking(Fallback::Launcher),
                SummonIn::AppAnswered(SummonSerial(1), SummonAnswer::TookAnchored),
                120,
                in_app(),
                vec![],
            ),
            (
                "asking, restored its prompt",
                asking(Fallback::Launcher),
                SummonIn::AppAnswered(SummonSerial(1), SummonAnswer::Restored),
                120,
                Summon::Rest,
                vec![],
            ),
            (
                "asking, declined: the launcher",
                asking(Fallback::Launcher),
                SummonIn::AppAnswered(SummonSerial(1), SummonAnswer::Declined),
                120,
                fresh(),
                vec![open_companion()],
            ),
            (
                "asking, an answer to an older ask is ignored",
                asking(Fallback::Launcher),
                SummonIn::AppAnswered(SummonSerial(0), SummonAnswer::TookField),
                120,
                asking(Fallback::Launcher),
                vec![],
            ),
            (
                "asking, the wait passed: the launcher",
                asking(Fallback::Launcher),
                SummonIn::Elapsed,
                180,
                fresh(),
                vec![open_companion()],
            ),
            (
                "asking, a wake before the wait is over",
                asking(Fallback::Launcher),
                SummonIn::Elapsed,
                179,
                asking(Fallback::Launcher),
                vec![],
            ),
            (
                "asking to put the prompt away, silence is rest",
                asking(Fallback::Rest),
                SummonIn::Elapsed,
                180,
                Summon::Rest,
                vec![],
            ),
            (
                "in the app, summoned again: ask it to restore",
                in_app(),
                SummonIn::Summon(facts(None, mail, true)),
                500,
                Summon::AskingApp {
                    app: app("org.quire.Mail"),
                    window: WINDOW,
                    serial: SummonSerial(1),
                    until: Stamp(580),
                    fallback: Fallback::Rest,
                },
                vec![ask_mail],
            ),
            (
                "in the launcher from search, summoned again: back to search",
                from_search.clone(),
                SummonIn::Summon(FocusFacts::default()),
                100,
                Summon::Rest,
                vec![SummonOut::SwitchLauncher(LauncherMode::Search)],
            ),
            (
                "in a fresh launcher, dismissed: closed",
                fresh(),
                SummonIn::Dismiss,
                100,
                Summon::Rest,
                vec![SummonOut::CloseLauncher],
            ),
            (
                "in the launcher, closed by someone else: at rest",
                fresh(),
                SummonIn::LauncherClosed,
                100,
                Summon::Rest,
                vec![],
            ),
            (
                "at rest, dismissed: nothing",
                Summon::Rest,
                SummonIn::Dismiss,
                100,
                Summon::Rest,
                vec![],
            ),
        ];
        for (name, from, input, at, next, outs) in cases {
            let (state, out) = from.step(
                input,
                Stamp(at),
                &SummonParams::default(),
                &SummonCx::default(),
            );
            assert_eq!(state, next, "{name}: state");
            assert_eq!(out, outs, "{name}: outputs");
        }
    }

    #[test]
    fn a_launcher_only_setting_sends_every_summon_to_the_launcher() {
        let params = SummonParams {
            in_field: InField::Launcher,
            ..SummonParams::default()
        };
        let (state, out) = Summon::Rest.step(
            SummonIn::Summon(facts(None, Some("org.quire.Mail"), true)),
            Stamp(0),
            &params,
            &SummonCx::default(),
        );
        assert_eq!(state, fresh());
        assert_eq!(out, vec![open_companion()]);
    }

    #[test]
    fn only_an_asking_summon_wakes() {
        assert_eq!(asking(Fallback::Launcher).wake(), Some(Stamp(180)));
        for state in [Summon::Rest, in_app(), fresh()] {
            assert_eq!(state.wake(), None, "{state:?}");
        }
    }
}
