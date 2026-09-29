//! The badges state's pure transition.

use super::{Badge, BadgeEvent, BadgeProp, Badges};
use crate::units::Permille;

/// `badges` after `event`. An app whose badge returns to showing nothing (hidden count, hidden
/// progress, calm) is dropped from the map.
pub fn step(badges: Badges, event: BadgeEvent) -> Badges {
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
