//! The pins' state and their one reducer.

use serde::{Deserialize, Serialize};
use sill_launcher::AppId;

/// `dock.json`: the pinned apps, left to right.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockPins {
    /// 1; nothing reads it yet but its absence.
    #[serde(default = "version_one")]
    pub version: u16,
    #[serde(default)]
    pub pinned: Vec<AppId>,
}

fn version_one() -> u16 {
    1
}

impl Default for DockPins {
    fn default() -> Self {
        DockPins {
            version: 1,
            pinned: Vec::new(),
        }
    }
}

impl DockPins {
    /// The file's contents; a damaged file is no pins (the dock still shows running apps).
    pub fn from_json(text: &str) -> DockPins {
        serde_json::from_str(text).unwrap_or_default()
    }

    /// The file's contents to write.
    pub fn to_json(&self) -> String {
        // Strings and a number: nothing here can fail to encode.
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Where a new pin goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum PinAt {
    /// After the last pin (`sill dock pin`, Keep in Dock).
    End,
    /// At this slot (a drop between tiles), clamped to the end.
    Index(u32),
}

/// One change to the pins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum PinEvent {
    /// Pin `app` at `at`; an app already pinned moves there.
    Pin { app: AppId, at: PinAt },
    /// Unpin `app`; nothing if it is not pinned.
    Unpin(AppId),
    /// Move a pinned `app` to slot `to` (drag reorder), clamped; nothing if not pinned.
    Move { app: AppId, to: u32 },
    /// The file changed on disk.
    Replaced(DockPins),
}

/// `pins` after `event`.
pub fn reduce_pins(pins: DockPins, event: PinEvent) -> DockPins {
    let DockPins {
        version,
        mut pinned,
    } = pins;
    match event {
        PinEvent::Pin { app, at } => {
            pinned.retain(|a| a != &app);
            let slot = match at {
                PinAt::End => pinned.len(),
                PinAt::Index(i) => clamp(i, pinned.len()),
            };
            pinned.insert(slot, app);
        }
        PinEvent::Unpin(app) => pinned.retain(|a| a != &app),
        PinEvent::Move { app, to } => {
            if let Some(from) = pinned.iter().position(|a| a == &app) {
                let moved = pinned.remove(from);
                pinned.insert(clamp(to, pinned.len()), moved);
            }
        }
        PinEvent::Replaced(all) => return all,
    }
    DockPins { version, pinned }
}

fn clamp(index: u32, len: usize) -> usize {
    usize::try_from(index).unwrap_or(usize::MAX).min(len)
}

#[cfg(test)]
mod tests {
    use super::{DockPins, PinAt, PinEvent, reduce_pins};
    use sill_launcher::AppId;

    fn pins(ids: &[&str]) -> DockPins {
        DockPins {
            version: 1,
            pinned: ids.iter().map(|s| AppId((*s).to_owned())).collect(),
        }
    }

    fn id(s: &str) -> AppId {
        AppId(s.to_owned())
    }

    #[test]
    fn events_reduce_to_the_expected_pins() {
        let cases: Vec<(&str, DockPins, PinEvent, DockPins)> = vec![
            (
                "pin at the end",
                pins(&["a", "b"]),
                PinEvent::Pin {
                    app: id("c"),
                    at: PinAt::End,
                },
                pins(&["a", "b", "c"]),
            ),
            (
                "pin at a slot",
                pins(&["a", "b"]),
                PinEvent::Pin {
                    app: id("c"),
                    at: PinAt::Index(1),
                },
                pins(&["a", "c", "b"]),
            ),
            (
                "pin past the end clamps",
                pins(&["a"]),
                PinEvent::Pin {
                    app: id("c"),
                    at: PinAt::Index(9),
                },
                pins(&["a", "c"]),
            ),
            (
                "pinning a pinned app moves it",
                pins(&["a", "b", "c"]),
                PinEvent::Pin {
                    app: id("a"),
                    at: PinAt::End,
                },
                pins(&["b", "c", "a"]),
            ),
            (
                "unpin",
                pins(&["a", "b"]),
                PinEvent::Unpin(id("a")),
                pins(&["b"]),
            ),
            (
                "unpin an unpinned app",
                pins(&["a"]),
                PinEvent::Unpin(id("z")),
                pins(&["a"]),
            ),
            (
                "move right",
                pins(&["a", "b", "c"]),
                PinEvent::Move {
                    app: id("a"),
                    to: 2,
                },
                pins(&["b", "c", "a"]),
            ),
            (
                "move left",
                pins(&["a", "b", "c"]),
                PinEvent::Move {
                    app: id("c"),
                    to: 0,
                },
                pins(&["c", "a", "b"]),
            ),
            (
                "move past the end clamps",
                pins(&["a", "b"]),
                PinEvent::Move {
                    app: id("a"),
                    to: 7,
                },
                pins(&["b", "a"]),
            ),
            (
                "moving an unpinned app does nothing",
                pins(&["a", "b"]),
                PinEvent::Move {
                    app: id("z"),
                    to: 0,
                },
                pins(&["a", "b"]),
            ),
            (
                "replaced",
                pins(&["a"]),
                PinEvent::Replaced(pins(&["x", "y"])),
                pins(&["x", "y"]),
            ),
        ];
        for (name, before, event, want) in cases {
            assert_eq!(reduce_pins(before, event), want, "{name}");
        }
    }

    #[test]
    fn the_file_round_trips_and_damage_is_no_pins() {
        let p = pins(&["org.mozilla.firefox", "foot"]);
        assert_eq!(DockPins::from_json(&p.to_json()), p);
        assert_eq!(
            DockPins::from_json(r#"{"pinned":["foot"]}"#),
            pins(&["foot"]),
            "version defaults to 1"
        );
        for text in ["", "[]", "{\"pinned\": 3}", "garbage"] {
            assert_eq!(DockPins::from_json(text), DockPins::default(), "{text:?}");
        }
    }
}
