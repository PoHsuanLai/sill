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

/// One toplevel, as cctk's `ToplevelInfo` describes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Toplevel {
    pub key: WindowKey,
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
    use crate::cosmic_wl::Activity;
    use sill_launcher::WindowKey;

    fn window(key: u64, activity: Activity) -> Toplevel {
        Toplevel {
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
}
