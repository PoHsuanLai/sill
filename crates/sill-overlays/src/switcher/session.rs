//! One opening of the switcher, from its chord to its close: the apps as they were at the chord
//! (macOS's switcher keeps its row while it is up), whose windows its verbs touch, and the tiles
//! leaving after a Q. Pure: `io.rs` holds it and carries out what it says.

use super::apps::{AppShown, SwitcherApp, activate, hide, quit};
use ds::prelude::*;
use ds_behaviour::switcher::SwOut;
use sill_launcher::AppId;
use sill_model::windows::WlCommand;

use sill_ui_kit::links::AskSerial;

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

    /// The app at `index` in the machine's list, if there is one.
    pub fn id_at(&self, index: usize) -> Option<&AppId> {
        self.apps.get(index).map(|app| &app.app)
    }

    /// Whether `app`'s tile is listed: `Hidden` while it folds (quire's `AppSwitcher` keeps a tile
    /// it is no longer given until its exit has settled).
    pub fn shown(&self, app: &AppId) -> Shown {
        if self.leaving.contains(app) {
            Shown::Hidden
        } else {
            Shown::Visible
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
        let (index, verb): (usize, fn(&SwitcherApp) -> Vec<WlCommand>) = match out {
            SwOut::Activate(index) => (*index, activate),
            SwOut::Quit(index) => (*index, quit),
            SwOut::HideApp(index) => (*index, hide),
            // `sw_key` maps no Up or Down, so the machine never asks for App Exposé.
            SwOut::Show | SwOut::Hide | SwOut::Select(_) | SwOut::Expose(_) => {
                return Vec::new();
            }
        };
        let Some(id) = self.id_at(index) else {
            return Vec::new();
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
    use crate::switcher::apps::{AppShown, SwitcherApp};
    use ds::prelude::*;
    use ds_behaviour::switcher::SwOut;
    use sill_launcher::{AppId, WindowKey};
    use sill_model::windows::WlCommand;

    use super::Session;
    use ds_behaviour::switcher::SwOut;
    use sill_ui_kit::links::AskSerial;

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
                SwOut::Activate(1),
                vec![
                    WlCommand::Unminimize(WindowKey(3)),
                    WlCommand::Activate(WindowKey(3)),
                ],
            ),
            (
                SwOut::Quit(1),
                vec![
                    WlCommand::Close(WindowKey(3)),
                    WlCommand::Close(WindowKey(2)),
                ],
            ),
            (SwOut::HideApp(0), vec![WlCommand::Minimize(WindowKey(1))]),
            (SwOut::Activate(9), vec![]),
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
        assert_eq!(session.id_at(1), Some(&id("b")));
        assert_eq!(session.commands(&SwOut::Quit(0), &live), vec![]);
        assert_eq!(session.commands(&SwOut::Activate(0), &live), vec![]);
    }

    #[test]
    fn a_quit_tile_folds_then_goes() {
        let session = Session::made_up(AskSerial(1), vec![id("a"), id("b")]).leave(&id("b"));
        assert_eq!(session.shown(&id("b")), Shown::Hidden);
        assert_eq!(session.shown(&id("a")), Shown::Visible);
        let session = session.gone(&id("b"));
        assert_eq!(session.id_at(0), Some(&id("a")));
        assert_eq!(session.id_at(1), None);
        assert_eq!(session.leaving, Vec::<AppId>::new());
    }
}
