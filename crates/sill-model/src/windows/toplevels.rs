//! Open windows in most-recently-activated order, and their one reducer.

use serde::{Deserialize, Serialize};
use sill_launcher::WindowKey;

use super::Activity;
use crate::ids::{OutputName, WorkspaceKey};

/// Every toplevel, most recently activated first (the dock's cycle order, the switcher's
/// order, the context menu's window list).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Toplevels {
    pub windows: Vec<Toplevel>,
}

impl Toplevels {
    /// The first output of the active window: where a key press acts when the pointer's output is
    /// not known. `None` without an active window or when it names no output.
    pub fn focused_output(&self) -> Option<OutputName> {
        self.active()?.outputs.first().cloned()
    }

    /// The window that has focus: the first with `Activity::Active`. `None` while nothing is
    /// focused; callers that act on focus (keys, profiles, the companion) use this.
    pub fn active(&self) -> Option<&Toplevel> {
        self.windows.iter().find(|w| w.activity == Activity::Active)
    }

    /// The window a person would call frontmost on `output` (any output for `None`): the
    /// active window when it is there, else the most recently activated one there. For
    /// showing "the front app"; use [`Self::active`] to act on focus.
    pub fn front(&self, output: Option<&OutputName>) -> Option<&Toplevel> {
        let there = |w: &&Toplevel| output.is_none_or(|o| w.outputs.contains(o));
        self.windows
            .iter()
            .filter(there)
            .find(|w| w.activity == Activity::Active)
            .or_else(|| self.windows.iter().find(there))
    }
}

/// One toplevel, as the protocol describes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Toplevel {
    pub key: WindowKey,
    /// The compositor's stable name for the window: the one `DesktopView` quotes it by, so a
    /// window can be named in either model. Made up from the key where the compositor gives none.
    pub identifier: String,
    /// The Wayland app id (or X11 `WM_CLASS` through XWayland); `match_app` maps it to an entry.
    pub app_id: String,
    pub title: String,
    pub activity: Activity,
    pub window: WindowState,
    pub sizing: Sizing,
    pub workspaces: Vec<WorkspaceKey>,
    pub outputs: Vec<OutputName>,
}

/// Minimized or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowState {
    #[default]
    Normal,
    Minimized,
}

/// How the toplevel fills its output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sizing {
    #[default]
    Floating,
    Maximized,
    Fullscreen,
}

/// One change the compositor reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ToplevelEvent {
    Opened(Toplevel),
    /// A known toplevel's whole new description.
    Changed(Toplevel),
    Closed(WindowKey),
}

/// `toplevels` after `event`. A toplevel that is (or becomes) active moves to the front; one
/// that opens inactive goes to the back. A change for an unknown key is treated as an open (the
/// thread may coalesce an open and a change into one `done`).
pub fn step(toplevels: Toplevels, event: ToplevelEvent) -> Toplevels {
    let Toplevels { mut windows } = toplevels;
    match event {
        ToplevelEvent::Opened(window) | ToplevelEvent::Changed(window) => {
            let at = windows.iter().position(|w| w.key == window.key);
            let was = at.map(|i| windows.remove(i));
            let becomes_active = window.activity == Activity::Active
                && was.as_ref().is_none_or(|w| w.activity != Activity::Active);
            match (at, becomes_active) {
                (_, true) => windows.insert(0, window),
                (Some(i), false) => windows.insert(i, window),
                (None, false) => windows.push(window),
            }
        }
        ToplevelEvent::Closed(key) => windows.retain(|w| w.key != key),
    }
    Toplevels { windows }
}

#[cfg(test)]
mod tests {
    use super::{Sizing, Toplevel, ToplevelEvent, Toplevels, WindowState, step};
    use crate::ids::OutputName;
    use crate::windows::Activity;
    use sill_launcher::WindowKey;

    fn window(key: u64, activity: Activity) -> Toplevel {
        Toplevel {
            identifier: String::new(),
            key: WindowKey(key),
            app_id: format!("app{key}"),
            title: format!("window {key}"),
            activity,
            window: WindowState::Normal,
            sizing: Sizing::Floating,
            workspaces: Vec::new(),
            outputs: Vec::new(),
        }
    }

    fn order(t: &Toplevels) -> Vec<u64> {
        t.windows.iter().map(|w| w.key.0).collect()
    }

    fn of(keys: &[u64]) -> Toplevels {
        Toplevels {
            windows: keys
                .iter()
                .map(|&k| window(k, Activity::Inactive))
                .collect(),
        }
    }

    #[test]
    fn events_keep_most_recently_activated_first() {
        let cases: Vec<(&str, Toplevels, ToplevelEvent, Vec<u64>)> = vec![
            (
                "an inactive open goes last",
                of(&[1, 2]),
                ToplevelEvent::Opened(window(3, Activity::Inactive)),
                vec![1, 2, 3],
            ),
            (
                "an active open goes first",
                of(&[1, 2]),
                ToplevelEvent::Opened(window(3, Activity::Active)),
                vec![3, 1, 2],
            ),
            (
                "activation moves to the front",
                of(&[1, 2, 3]),
                ToplevelEvent::Changed(window(3, Activity::Active)),
                vec![3, 1, 2],
            ),
            (
                "a title change keeps the place",
                of(&[1, 2, 3]),
                ToplevelEvent::Changed(Toplevel {
                    title: "renamed".to_owned(),
                    ..window(2, Activity::Inactive)
                }),
                vec![1, 2, 3],
            ),
            (
                "deactivation keeps the place",
                Toplevels {
                    windows: vec![window(1, Activity::Active), window(2, Activity::Inactive)],
                },
                ToplevelEvent::Changed(window(1, Activity::Inactive)),
                vec![1, 2],
            ),
            (
                "a change for an unknown key is an open",
                of(&[1]),
                ToplevelEvent::Changed(window(9, Activity::Inactive)),
                vec![1, 9],
            ),
            (
                "close removes",
                of(&[1, 2, 3]),
                ToplevelEvent::Closed(WindowKey(2)),
                vec![1, 3],
            ),
            (
                "closing an unknown key changes nothing",
                of(&[1]),
                ToplevelEvent::Closed(WindowKey(7)),
                vec![1],
            ),
        ];
        for (name, before, event, want) in cases {
            assert_eq!(order(&step(before, event)), want, "{name}");
        }
    }

    #[test]
    fn a_still_active_window_changing_title_does_not_jump() {
        let before = Toplevels {
            windows: vec![window(2, Activity::Inactive), window(1, Activity::Active)],
        };
        let after = step(
            before,
            ToplevelEvent::Changed(Toplevel {
                title: "new".to_owned(),
                ..window(1, Activity::Active)
            }),
        );
        assert_eq!(order(&after), vec![2, 1]);
        assert_eq!(after.windows[1].title, "new");
    }

    fn named_window(key: u64, activity: Activity, outputs: &[&str]) -> Toplevel {
        Toplevel {
            identifier: String::new(),
            key: sill_launcher::WindowKey(key),
            app_id: "foot".to_owned(),
            title: String::new(),
            activity,
            window: Default::default(),
            sizing: Default::default(),
            workspaces: Vec::new(),
            outputs: outputs
                .iter()
                .map(|o| OutputName((*o).to_owned()))
                .collect(),
        }
    }

    #[test]
    fn the_front_window_prefers_active_then_most_recent_on_the_output() {
        let windows = vec![
            named_window(1, Activity::Inactive, &["DP-1"]),
            named_window(2, Activity::Inactive, &["DP-2"]),
            named_window(3, Activity::Active, &["DP-2"]),
        ];
        let on = |o: &str| Some(OutputName(o.to_owned()));
        let cases = [
            ("any output: the active one", None, Some(3)),
            ("the active window's output", on("DP-2"), Some(3)),
            (
                "an output without it: the most recent there",
                on("DP-1"),
                Some(1),
            ),
            ("an unknown output", on("HDMI-1"), None),
        ];
        for (name, output, want) in cases {
            let all = Toplevels {
                windows: windows.clone(),
            };
            let got = all.front(output.as_ref()).map(|w| w.key.0);
            assert_eq!(got, want, "{name}");
        }
        let none_active = of(&[4, 5]);
        assert_eq!(none_active.active(), None);
        assert_eq!(none_active.front(None).map(|w| w.key.0), Some(4));
    }

    #[test]
    fn the_focused_output_is_the_active_windows_first() {
        let cases = [
            ("no windows", vec![], None),
            (
                "the active window's first output",
                vec![
                    named_window(1, Activity::Inactive, &["eDP-1"]),
                    named_window(2, Activity::Active, &["HDMI-A-2", "eDP-1"]),
                ],
                Some("HDMI-A-2"),
            ),
            (
                "an active window on no output",
                vec![named_window(1, Activity::Active, &[])],
                None,
            ),
        ];
        for (name, windows, want) in cases {
            assert_eq!(
                Toplevels { windows }.focused_output(),
                want.map(|w| OutputName(w.to_owned())),
                "{name}"
            );
        }
    }
}
