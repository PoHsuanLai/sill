//! Badges' state and their one reducer.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sill_launcher::AppId;

use crate::cosmic_wl::Urgency;
use crate::units::{Permille, Visibility};

/// Every app with something to show, by desktop id.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Badges {
    pub apps: BTreeMap<AppId, Badge>,
}

/// One app's LauncherEntry properties. The count and progress are kept even while hidden, as
/// the protocol sends value and visibility separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Badge {
    pub count: u64,
    pub count_visibility: Visibility,
    /// `progress` 0..1 as thousandths.
    pub progress: Permille,
    pub progress_visibility: Visibility,
    /// `urgent`: starts a Critical bounce (design/10 §10.3.2).
    pub urgency: Urgency,
}

impl Badge {
    /// The count to draw: visible and above zero.
    pub fn shown_count(&self) -> Option<u64> {
        (self.count_visibility == Visibility::Shown && self.count > 0).then_some(self.count)
    }

    /// The progress to draw, when visible.
    pub fn shown_progress(&self) -> Option<Permille> {
        (self.progress_visibility == Visibility::Shown).then_some(self.progress)
    }
}

/// One LauncherEntry property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum BadgeProp {
    /// `count` (x); negative clamps to 0.
    Count(i64),
    /// `count-visible` (b).
    CountVisibility(Visibility),
    /// `progress` (d) as thousandths, clamped to 0..=1000.
    Progress(Permille),
    /// `progress-visible` (b).
    ProgressVisibility(Visibility),
    /// `urgent` (b).
    Urgency(Urgency),
}

/// One change on the bus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum BadgeEvent {
    /// An `Update` signal for `app`, its properties in signal order.
    Update { app: AppId, props: Vec<BadgeProp> },
    /// The app's sender left the bus: nothing it said stands.
    Cleared(AppId),
    /// Forget everything.
    ClearedAll,
}

/// `badges` after `event`. An app whose badge returns to showing nothing (hidden count, hidden
/// progress, calm) is dropped from the map.
pub fn reduce_badges(badges: Badges, event: BadgeEvent) -> Badges {
    let Badges { mut apps } = badges;
    match event {
        BadgeEvent::Update { app, props } => {
            let badge = props
                .into_iter()
                .fold(apps.get(&app).copied().unwrap_or_default(), apply);
            if badge == Badge::default() {
                apps.remove(&app);
            } else {
                apps.insert(app, badge);
            }
        }
        BadgeEvent::Cleared(app) => {
            apps.remove(&app);
        }
        BadgeEvent::ClearedAll => apps.clear(),
    }
    Badges { apps }
}

fn apply(badge: Badge, prop: BadgeProp) -> Badge {
    match prop {
        BadgeProp::Count(count) => Badge {
            count: u64::try_from(count).unwrap_or(0),
            ..badge
        },
        BadgeProp::CountVisibility(count_visibility) => Badge {
            count_visibility,
            ..badge
        },
        BadgeProp::Progress(progress) => Badge {
            progress: Permille(progress.0.min(1000)),
            ..badge
        },
        BadgeProp::ProgressVisibility(progress_visibility) => Badge {
            progress_visibility,
            ..badge
        },
        BadgeProp::Urgency(urgency) => Badge { urgency, ..badge },
    }
}

/// The desktop id an `app_uri` names: `application://org.gnome.Nautilus.desktop` is
/// `org.gnome.Nautilus`. Another scheme, or an empty id, is `None`.
pub fn parse_app_uri(uri: &str) -> Option<AppId> {
    let rest = uri.strip_prefix("application://")?;
    let id = rest.strip_suffix(".desktop").unwrap_or(rest);
    (!id.is_empty() && !id.contains('/')).then(|| AppId(id.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::{Badge, BadgeEvent, BadgeProp, Badges, parse_app_uri, reduce_badges};
    use crate::cosmic_wl::Urgency;
    use crate::units::{Permille, Visibility};
    use sill_launcher::AppId;

    fn id(s: &str) -> AppId {
        AppId(s.to_owned())
    }

    fn update(app: &str, props: Vec<BadgeProp>) -> BadgeEvent {
        BadgeEvent::Update {
            app: id(app),
            props,
        }
    }

    fn one(app: &str, badge: Badge) -> Badges {
        Badges {
            apps: [(id(app), badge)].into_iter().collect(),
        }
    }

    fn counting(n: u64) -> Badge {
        Badge {
            count: n,
            count_visibility: Visibility::Shown,
            ..Badge::default()
        }
    }

    #[test]
    fn updates_merge_into_the_apps_badge() {
        let cases: Vec<(&str, Badges, BadgeEvent, Badges)> = vec![
            (
                "a visible count",
                Badges::default(),
                update(
                    "mail",
                    vec![
                        BadgeProp::Count(3),
                        BadgeProp::CountVisibility(Visibility::Shown),
                    ],
                ),
                one("mail", counting(3)),
            ),
            (
                "a count change keeps visibility",
                one("mail", counting(3)),
                update("mail", vec![BadgeProp::Count(7)]),
                one("mail", counting(7)),
            ),
            (
                "a negative count is zero",
                one("mail", counting(3)),
                update("mail", vec![BadgeProp::Count(-4)]),
                one("mail", counting(0)),
            ),
            (
                "progress is clamped",
                Badges::default(),
                update(
                    "dl",
                    vec![
                        BadgeProp::Progress(Permille(1400)),
                        BadgeProp::ProgressVisibility(Visibility::Shown),
                    ],
                ),
                one(
                    "dl",
                    Badge {
                        progress: Permille(1000),
                        progress_visibility: Visibility::Shown,
                        ..Badge::default()
                    },
                ),
            ),
            (
                "urgency alone",
                Badges::default(),
                update("chat", vec![BadgeProp::Urgency(Urgency::Urgent)]),
                one(
                    "chat",
                    Badge {
                        urgency: Urgency::Urgent,
                        ..Badge::default()
                    },
                ),
            ),
            (
                "hiding everything drops the app",
                one("mail", counting(3)),
                update(
                    "mail",
                    vec![
                        BadgeProp::Count(0),
                        BadgeProp::CountVisibility(Visibility::Hidden),
                    ],
                ),
                Badges::default(),
            ),
            (
                "a hidden count is kept while it does not show",
                Badges::default(),
                update("mail", vec![BadgeProp::Count(5)]),
                one(
                    "mail",
                    Badge {
                        count: 5,
                        ..Badge::default()
                    },
                ),
            ),
            (
                "cleared",
                one("mail", counting(3)),
                BadgeEvent::Cleared(id("mail")),
                Badges::default(),
            ),
            (
                "cleared all",
                one("mail", counting(3)),
                BadgeEvent::ClearedAll,
                Badges::default(),
            ),
        ];
        for (name, before, event, want) in cases {
            assert_eq!(reduce_badges(before, event), want, "{name}");
        }
    }

    #[test]
    fn shown_values_need_visibility() {
        assert_eq!(counting(3).shown_count(), Some(3));
        assert_eq!(counting(0).shown_count(), None);
        let hidden = Badge {
            count: 3,
            ..Badge::default()
        };
        assert_eq!(hidden.shown_count(), None);
        assert_eq!(hidden.shown_progress(), None);
    }

    #[test]
    fn app_uris_name_desktop_ids() {
        const CASES: &[(&str, Option<&str>)] = &[
            (
                "application://org.gnome.Nautilus.desktop",
                Some("org.gnome.Nautilus"),
            ),
            ("application://firefox.desktop", Some("firefox")),
            ("application://firefox", Some("firefox")),
            ("application://.desktop", None),
            ("application://", None),
            ("file:///usr/share/applications/firefox.desktop", None),
            ("application://../../etc/passwd", None),
            ("", None),
        ];
        for &(uri, want) in CASES {
            assert_eq!(parse_app_uri(uri), want.map(id), "{uri}");
        }
    }
}
