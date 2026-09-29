//! The machine events a change in the services means (design/10 §10.3.5): a tile's app mapping
//! its first window or becoming active ends a launch or critical bounce; LauncherEntry
//! `urgent` starts a critical bounce and its clearing ends one. Pure over two snapshots.

use sill_model::badges::Badges;
use sill_model::units::Urgency;

use crate::surfaces::dock::items::DockItem;
use crate::surfaces::dock::machine::{AppChange, AttentionKind, DockEvent, TileApp};

/// What the services said about the dock's tiles.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Snapshot {
    pub items: Vec<DockItem>,
    pub badges: Badges,
}

/// The events that turn `before` into `after`, per tile in `after`'s order.
pub fn changes(before: &Snapshot, after: &Snapshot) -> Vec<DockEvent> {
    after
        .items
        .iter()
        .flat_map(|item| {
            let was = before.items.iter().find(|b| b.tile == item.tile);
            let apps = was.map_or(TileApp::NotRunning, |b| b.app);
            let urgency = |snapshot: &Snapshot| {
                item.kind
                    .app_id()
                    .and_then(|id| snapshot.badges.apps.get(id))
                    .map_or(Urgency::Calm, |badge| badge.urgency)
            };
            let tile = item.tile;
            let app = |change| DockEvent::AppState { tile, change };
            let mapped = (apps == TileApp::NotRunning && item.app != TileApp::NotRunning)
                .then(|| app(AppChange::Mapped));
            let activated = (matches!(item.app, TileApp::Active { .. })
                && !matches!(apps, TileApp::Active { .. }))
            .then(|| app(AppChange::Activated));
            let urgent = match (urgency(before), urgency(after)) {
                (Urgency::Calm, Urgency::Urgent) => Some(DockEvent::Attention {
                    tile,
                    kind: AttentionKind::Critical,
                }),
                (Urgency::Urgent, Urgency::Calm) => Some(app(AppChange::UrgencyCleared)),
                _ => None,
            };
            [mapped, activated, urgent].into_iter().flatten()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Snapshot, changes};
    use crate::surfaces::dock::items::{DockItem, ItemKind};
    use crate::surfaces::dock::machine::{AppChange, AttentionKind, DockEvent, TileApp, TileId};
    use sill_launcher::AppId;
    use sill_model::badges::{Badge, Badges};
    use sill_model::units::Urgency;

    const T: TileId = TileId(4);

    fn snap(app: TileApp, urgency: Urgency) -> Snapshot {
        let id = AppId("term".to_owned());
        let mut badges = Badges::default();
        badges.apps.insert(
            id.clone(),
            Badge {
                urgency,
                ..Badge::default()
            },
        );
        Snapshot {
            items: vec![DockItem {
                tile: T,
                kind: ItemKind::Pinned(id),
                app,
            }],
            badges,
        }
    }

    #[test]
    fn each_change_is_its_event() {
        use TileApp::{Active, Inactive, NotRunning};
        use Urgency::{Calm, Urgent};
        let state = |change| DockEvent::AppState { tile: T, change };
        let cases = [
            (
                "nothing",
                snap(NotRunning, Calm),
                snap(NotRunning, Calm),
                vec![],
            ),
            (
                "a first window",
                snap(NotRunning, Calm),
                snap(Inactive, Calm),
                vec![state(AppChange::Mapped)],
            ),
            (
                "mapped and active at once",
                snap(NotRunning, Calm),
                snap(Active { windows: 1 }, Calm),
                vec![state(AppChange::Mapped), state(AppChange::Activated)],
            ),
            (
                "activated",
                snap(Inactive, Calm),
                snap(Active { windows: 1 }, Calm),
                vec![state(AppChange::Activated)],
            ),
            (
                "urgent",
                snap(Inactive, Calm),
                snap(Inactive, Urgent),
                vec![DockEvent::Attention {
                    tile: T,
                    kind: AttentionKind::Critical,
                }],
            ),
            (
                "urgency cleared",
                snap(Inactive, Urgent),
                snap(Inactive, Calm),
                vec![state(AppChange::UrgencyCleared)],
            ),
        ];
        for (name, before, after, want) in cases {
            assert_eq!(changes(&before, &after), want, "{name}");
        }
    }
}
