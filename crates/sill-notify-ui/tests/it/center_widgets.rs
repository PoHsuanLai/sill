//! The notification center's widget slot: it draws Calendar then Up Next under the history, each
//! at its one size (Medium), and nothing to edit them with: the real `NotificationCenter` on the
//! fake services.

#[test]
fn the_center_draws_calendar_then_up_next_under_the_history() {
    let running = center_support::opened(center_support::two_apps());
    let h = &running.harness;
    assert_eq!(h.count(".sill-nc-widgets > .ds-widget"), 2);
    let ids: Vec<_> = (1..=2)
        .map(|n| {
            h.attr(
                &format!(".sill-nc-widgets > .ds-widget:nth-child({n})"),
                "id",
            )
        })
        .collect();
    assert_eq!(
        ids,
        vec![
            Some("widget-calendar".to_owned()),
            Some("widget-up-next".to_owned()),
        ]
    );
    let list = h.rect(".sill-nc-list").expect("the history");
    let first = h.rect("#widget-calendar").expect("the first widget");
    assert!(
        first.origin.y.0 >= list.origin.y.0 + list.size.height.0 - 0.5,
        "under the history: {list:?} {first:?}"
    );
    // A medium tile (296: two 140 cells and a gap) fills the center's 320 px column after the panel's padding.
    let column = h.rect(".sill-nc-column").expect("the column");
    let tile = h.rect("#widget-up-next").expect("a medium tile");
    assert_eq!(tile.size.width.0.round(), 296.0);
    assert!(
        tile.origin.x.0 >= column.origin.x.0 - 0.5
            && tile.origin.x.0 + tile.size.width.0 <= column.origin.x.0 + column.size.width.0 + 0.5,
        "inside the column: {column:?} {tile:?}"
    );
}

/// The notification center opened over a history, for the widgets it draws under it.
mod center_support {
    use std::time::Duration;

    use ds_harness::Harness;
    use ds_harness::harness::settle_until;
    use sill_model::notifications::{
        AppGroup, AppKey, AppRef, Body, Expiry, Hints, History, IconRef, Notification,
        NotificationId, UnixMs,
    };
    use sill_notify_ui::NotificationCenter;
    use sill_ui_kit::harness::SurfaceHarness;
    use sill_ui_kit::links::CenterLink;

    pub use sill_ui_kit::harness::fake;

    fn note(id: u32, app: &str, summary: &str) -> Notification {
        Notification {
            id: NotificationId(id),
            arrived: UnixMs(1_000 * i64::from(id)),
            app: AppRef {
                name: app.to_owned(),
                icon: IconRef::None,
                desktop_entry: None,
            },
            summary: summary.to_owned(),
            body: Body::default(),
            actions: Vec::new(),
            hints: Hints::default(),
            expiry: Expiry::Default,
        }
    }

    /// Three from Mail (ids 5, 3, 1, newest first) and one from Chat (4).
    pub fn two_apps() -> History {
        History {
            groups: vec![
                AppGroup {
                    key: AppKey("mail".to_owned()),
                    notes: vec![
                        note(5, "Mail", "Lunch?"),
                        note(3, "Mail", "Re: plans"),
                        note(1, "Mail", "Hello"),
                    ],
                },
                AppGroup {
                    key: AppKey("chat".to_owned()),
                    notes: vec![note(4, "Chat", "ping")],
                },
            ],
        }
    }

    /// Advance until `done` holds, then one more frame's worth.
    pub fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
        settle_until(harness, done);
        harness.advance(Duration::from_millis(20));
    }

    /// The center opened over `history`, once its entrance has played.
    pub fn opened(history: History) -> SurfaceHarness<NotificationCenter> {
        let mut running = SurfaceHarness::<NotificationCenter>::setup(CenterLink::new(), ())
            .viewport(ds_harness::Viewport {
                width: 1280,
                height: 800,
                scale_percent: 100,
            })
            .start();
        running
            .feeds
            .notifications
            .send_modify(|n| n.history = history);
        running.harness.advance(Duration::from_millis(40));
        running.link.open(fake::output().id);
        settle(&mut running.harness, |h| h.count(".ds-side-panel") == 1);
        running.harness.advance(Duration::from_millis(600));
        running
    }
}
