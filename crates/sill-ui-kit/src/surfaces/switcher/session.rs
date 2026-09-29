//! One opening of the switcher, from its chord to its close: the apps as they were at the chord
//! (macOS's switcher keeps its row while it is up), whose windows its verbs touch, and the tiles
//! leaving after a Q. Pure: `io.rs` holds it and carries out what it says.

use ds::TilePresence;
use sill_launcher::AppId;
use sill_model::cosmic_wl::WlCommand;
use sill_services::switcher::{AppShown, SwitcherApp, activate, hide, quit};

use super::step::SwOut;
use crate::links::AskSerial;

/// Whose windows a session's verbs touch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Windows {
    /// The compositor's: from `sill switcher next|prev`.
    Live,
    /// None: `sill debug switcher`'s made-up apps.
    MadeUp,
}

/// One opening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// The ask that opened it: the key element remounts (and takes the focus) per opening.
    pub serial: AskSerial,
    pub apps: Vec<SwitcherApp>,
    pub windows: Windows,
    /// Quit with Q, folding until they are dropped.
    pub leaving: Vec<AppId>,
}

impl Session {
    /// An opening on the compositor's apps.
    pub fn live(serial: AskSerial, apps: Vec<SwitcherApp>) -> Session {
        Session {
            serial,
            apps,
            windows: Windows::Live,
            leaving: Vec::new(),
        }
    }

    /// An opening on made-up apps, which have no windows.
    pub fn made_up(serial: AskSerial, ids: Vec<AppId>) -> Session {
        let apps = ids
            .into_iter()
            .map(|app| SwitcherApp {
                app,
                windows: Vec::new(),
                shown: AppShown::Visible,
            })
            .collect();
        Session {
            serial,
            apps,
            windows: Windows::MadeUp,
            leaving: Vec::new(),
        }
    }

    /// The machine's list.
    pub fn ids(&self) -> Vec<AppId> {
        self.apps.iter().map(|app| app.app.clone()).collect()
    }

    /// Whether `app`'s tile is folding.
    pub fn presence(&self, app: &AppId) -> TilePresence {
        if self.leaving.contains(app) {
            TilePresence::Leaving
        } else {
            TilePresence::Present
        }
    }

    /// `app`'s tile starts folding.
    pub fn leave(mut self, app: &AppId) -> Session {
        if !self.leaving.contains(app) {
            self.leaving.push(app.clone());
        }
        self
    }

    /// `app`'s fold has settled: its tile goes.
    pub fn gone(mut self, app: &AppId) -> Session {
        self.apps.retain(|known| known.app != *app);
        self.leaving.retain(|known| known != app);
        self
    }

    /// The commands `out` becomes: over the app as `live` has it now (a window may have come or
    /// gone since the chord), else as the session saw it; none for made-up apps.
    pub fn commands(&self, out: &SwOut, live: &[SwitcherApp]) -> Vec<WlCommand> {
        let (id, verb): (&AppId, fn(&SwitcherApp) -> Vec<WlCommand>) = match out {
            SwOut::Activate(id) => (id, activate),
            SwOut::Quit(id) => (id, quit),
            SwOut::HideApp(id) => (id, hide),
            SwOut::Show | SwOut::Hide | SwOut::Select(_) | SwOut::RequestTick(_) => {
                return Vec::new();
            }
        };
        match self.windows {
            Windows::MadeUp => Vec::new(),
            Windows::Live => live
                .iter()
                .chain(&self.apps)
                .find(|app| app.app == *id)
                .map(verb)
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use ds::TilePresence;
    use sill_launcher::{AppId, WindowKey};
    use sill_model::cosmic_wl::WlCommand;
    use sill_services::switcher::{AppShown, SwitcherApp};

    use super::Session;
    use crate::links::AskSerial;
    use crate::surfaces::switcher::step::SwOut;

    fn app(id: &str, windows: &[u64], shown: AppShown) -> SwitcherApp {
        SwitcherApp {
            app: AppId(id.to_owned()),
            windows: windows.iter().map(|&k| WindowKey(k)).collect(),
            shown,
        }
    }

    fn id(a: &str) -> AppId {
        AppId(a.to_owned())
    }

    #[test]
    fn verbs_act_on_the_app_as_it_is_now() {
        let session = Session::live(
            AskSerial(1),
            vec![
                app("term", &[1], AppShown::Visible),
                app("calc", &[2], AppShown::Visible),
            ],
        );
        // Since the chord, calc was minimized and got a second window.
        let live = [app("calc", &[3, 2], AppShown::AllMinimized)];
        let cases = [
            (
                SwOut::Activate(id("calc")),
                vec![
                    WlCommand::Unminimize(WindowKey(3)),
                    WlCommand::Activate(WindowKey(3)),
                ],
            ),
            (
                SwOut::Quit(id("calc")),
                vec![
                    WlCommand::Close(WindowKey(3)),
                    WlCommand::Close(WindowKey(2)),
                ],
            ),
            (
                SwOut::HideApp(id("term")),
                vec![WlCommand::Minimize(WindowKey(1))],
            ),
            (SwOut::Activate(id("gone")), vec![]),
            (SwOut::Hide, vec![]),
        ];
        for (out, want) in cases {
            assert_eq!(session.commands(&out, &live), want, "{out:?}");
        }
    }

    #[test]
    fn made_up_apps_touch_no_window() {
        let session = Session::made_up(AskSerial(1), vec![id("a"), id("b")]);
        let live = [app("a", &[1], AppShown::Visible)];
        assert_eq!(session.ids(), vec![id("a"), id("b")]);
        assert_eq!(session.commands(&SwOut::Quit(id("a")), &live), vec![]);
        assert_eq!(session.commands(&SwOut::Activate(id("a")), &live), vec![]);
    }

    #[test]
    fn a_quit_tile_folds_then_goes() {
        let session = Session::made_up(AskSerial(1), vec![id("a"), id("b")]).leave(&id("b"));
        assert_eq!(session.presence(&id("b")), TilePresence::Leaving);
        assert_eq!(session.presence(&id("a")), TilePresence::Present);
        let session = session.gone(&id("b"));
        assert_eq!(session.ids(), vec![id("a")]);
        assert_eq!(session.leaving, Vec::<AppId>::new());
    }
}
