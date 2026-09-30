//! What the notification center draws: nothing while closed; open, quire's `Panel`
//! `notifications.center_width_px` wide at the right edge and the height of the surface, the
//! title and Clear All, the history grouped by app (quire's `GroupHeader` over the newest card,
//! stacked over the rest), the footer when another daemon holds the name, the widgets slot
//! last; and what its presses send.

use super::{
    HEIGHT, WIDTH, advance, commands, commands_until, fake, opened, settle, start, two_apps, until,
};
use ds::{Anim, MotionLevel, settle as settle_time};
use ds_harness::{Driver, Input, Query};
use sill_model::notifications::{
    AppKey, Clear, History, NameHolder, NameOwner, Notifications, NotificationsCommand, ServerMode,
    ServerStatus,
};
use sill_notify_ui::notification_center::PANEL_ID;
use sill_settings::ShellFile;
use sill_ui_kit::links::CenterShown;
use sill_ui_kit::panel::PANEL_PADDING;

/// A group's toggle ("N more", "Show less") and Clear, in its header.
const TOGGLE: &str = ".ds-group-header-actions .ds-button:first-child";
const CLEAR: &str = ".ds-group-header-actions .ds-button:last-child";

#[test]
fn closed_it_draws_nothing() {
    let mut running = start(ShellFile::default(), two_apps());
    advance(&mut running, 100);
    assert_eq!(running.harness.count(".ds-side-panel"), 0);
    assert_eq!(running.harness.count(".sill-nc-catcher"), 0);
}

#[test]
fn open_it_is_quires_full_height_panel_at_the_right_edge() {
    let running = opened(two_apps());
    let h = &running.harness;
    let panel = h.rect(".ds-side-panel").expect("the panel is laid out");
    let width = f32::from(ShellFile::default().notifications.center_width_px.0);
    assert!((panel.size.width.0 - width).abs() < 0.5, "{panel:?}");
    let right = panel.origin.x.0 + panel.size.width.0;
    assert!(
        (WIDTH - right - 8.0).abs() < 0.5,
        "8 px from the right edge: {panel:?}"
    );
    assert!((panel.origin.y.0 - 8.0).abs() < 0.5, "{panel:?}");
    assert!(
        (panel.size.height.0 - (HEIGHT - 16.0)).abs() < 0.5,
        "full height: {panel:?}"
    );
    assert_eq!(
        h.attr(".ds-side-panel-scope", "data-material").as_deref(),
        Some("popover")
    );
    assert_eq!(
        h.attr(".ds-side-panel", "data-presence").as_deref(),
        Some("present"),
        "quire's entrance has played"
    );
    assert_eq!(h.count(".sill-nc-catcher"), 1);
    let column = h.rect(&format!("#{PANEL_ID}")).expect("the column");
    let pad = PANEL_PADDING as f32;
    assert!(
        (column.size.height.0 + 2.0 * pad - panel.size.height.0).abs() < 0.5
            && (column.size.width.0 + 2.0 * pad - panel.size.width.0).abs() < 0.5,
        "the blur element grown by the padding is the panel: {column:?} {panel:?}"
    );
    // The widgets follow the history in the one scrolling column, as macOS's Notification
    // Center does, rather than being pinned to the bottom.
    let widgets = h.rect(".sill-nc-widgets").expect("the widgets slot");
    let list = h.rect(".sill-nc-list").expect("the history");
    assert!(
        widgets.origin.y.0 >= list.origin.y.0 + list.size.height.0 - 0.5,
        "the widgets slot follows the history: {widgets:?} {list:?}"
    );
    assert_eq!(
        h.attr(".sill-nc-column", "style"),
        None,
        "the column's own stylesheet makes it scroll"
    );
}

#[test]
fn the_history_is_grouped_by_app_collapsed_to_the_newest() {
    let running = opened(two_apps());
    let h = &running.harness;
    assert_eq!(
        h.text_of(".sill-nc-title").as_deref(),
        Some("Notifications")
    );
    assert_eq!(
        h.text_of(".sill-nc-head .ds-button").as_deref(),
        Some("Clear All")
    );
    assert_eq!(h.count(".sill-nc-group-head"), 2);
    assert_eq!(h.count(".sill-nc-group-head .ds-group-header"), 2);
    assert_eq!(
        h.text_of(".sill-nc-item[*|data-group=mail] .ds-group-header-name")
            .as_deref(),
        Some("Mail")
    );
    assert_eq!(
        h.text_of(".sill-nc-item[*|data-group=mail] .ds-group-header-count")
            .as_deref(),
        Some("3")
    );
    assert_eq!(
        h.text_of(".sill-nc-item[*|data-group=chat] .ds-group-header-name")
            .as_deref(),
        Some("Chat")
    );
    assert_eq!(
        h.count(".sill-nc-row"),
        2,
        "one card per group while collapsed"
    );
    assert_eq!(
        h.attr(
            ".sill-nc-item[*|data-group=mail] .sill-nc-row",
            "data-stack"
        )
        .as_deref(),
        Some("3"),
        "Mail's newest stands for three"
    );
    assert_eq!(
        h.count(".sill-nc-item[*|data-group=mail] .ds-notification-layer"),
        2,
        "stacked over the rest"
    );
    assert_eq!(
        h.attr(".sill-nc-row .ds-notification", "data-swipe"),
        None,
        "no swipe in the center"
    );
    assert_eq!(
        h.text_of(&format!(".sill-nc-item[*|data-group=mail] {TOGGLE}"))
            .as_deref(),
        Some("2 more")
    );
    assert_eq!(h.count(".sill-nc-widgets"), 1);
    assert_eq!(h.count(".sill-nc-empty"), 0);
}

#[test]
fn n_more_opens_the_group_and_show_less_closes_it() {
    let mut running = opened(two_apps());
    let toggle = format!(".sill-nc-item[*|data-group=mail] {TOGGLE}");
    let more = running.harness.centre(&toggle).expect("N more");
    running.harness.send(Input::click(more));
    until(&mut running, |h| h.count(".sill-nc-row") == 4);
    let h = &running.harness;
    assert_eq!(h.count(".sill-nc-row"), 4, "Mail's three and Chat's one");
    assert_eq!(h.text_of(&toggle).as_deref(), Some("Show less"));
    let less = h.centre(&toggle).expect("Show less");
    running.harness.send(Input::click(less));
    until(&mut running, |h| h.count(".sill-nc-row") == 2);
    assert_eq!(running.harness.count(".sill-nc-row"), 2);
    assert!(
        commands(&mut running).is_empty(),
        "opening a group sends nothing"
    );
}

#[test]
fn clear_and_clear_all_go_to_the_service() {
    let mut running = opened(two_apps());
    let clear = running
        .harness
        .centre(&format!(".sill-nc-item[*|data-group=chat] {CLEAR}"))
        .expect("Chat's Clear");
    running.harness.send(Input::click(clear));
    let mut sent = commands_until(&mut running, 1);
    let all = running
        .harness
        .centre(".sill-nc-head .ds-button")
        .expect("Clear All");
    running.harness.send(Input::click(all));
    sent.extend(commands_until(&mut running, 1));
    assert_eq!(
        sent,
        vec![
            NotificationsCommand::ClearHistory(Clear::App(AppKey("chat".to_owned()))),
            NotificationsCommand::ClearHistory(Clear::All),
        ]
    );
}

#[test]
fn a_press_on_a_stack_opens_it_and_a_lone_cards_close_dismisses_it() {
    let mut running = opened(two_apps());
    let stack = running
        .harness
        .centre(".sill-nc-item[*|data-group=mail] .ds-notification-plate")
        .expect("Mail's stack");
    running.harness.send(Input::click(stack));
    until(&mut running, |h| h.count(".sill-nc-row") == 4);
    assert_eq!(running.harness.count(".sill-nc-row"), 4, "the stack opened");
    let chat = running
        .harness
        .centre(".sill-nc-item[*|data-group=chat] .ds-notification-plate")
        .expect("Chat's card");
    running.harness.send(Input::pointer_move(chat));
    until(&mut running, |h| {
        h.attr(
            ".sill-nc-item[*|data-group=chat] .ds-notification",
            "data-hover",
        )
        .as_deref()
            == Some("on")
    });
    advance(&mut running, 300);
    let close = running
        .harness
        .centre(".sill-nc-item[*|data-group=chat] .ds-notification-close")
        .expect("its close button");
    running.harness.send(Input::click(close));
    assert_eq!(
        commands_until(&mut running, 1),
        vec![NotificationsCommand::Dismiss(
            sill_model::notifications::NotificationId(4)
        )]
    );
}

#[test]
fn an_empty_history_says_so_and_offers_no_clear_all() {
    let running = opened(History::default());
    let h = &running.harness;
    assert_eq!(
        h.text_of(".sill-nc-empty").as_deref(),
        Some("No Notifications")
    );
    assert_eq!(h.count(".sill-nc-head .ds-button"), 0);
    assert_eq!(h.count(".sill-nc-group-head"), 0);
}

#[test]
fn the_footer_names_the_daemon_holding_the_name() {
    let mut running = opened(two_apps());
    assert_eq!(
        running.harness.count(".sill-nc-foot"),
        1,
        "passive until the daemon's policy"
    );
    running.feeds.notifications.send_modify(|n| {
        n.live = Notifications {
            server: ServerStatus {
                mode: ServerMode::Own,
                holder: NameHolder::Other(NameOwner {
                    unique: ":1.7".to_owned(),
                    pid: Some(7),
                    command: Some("plasmashell".to_owned()),
                }),
            },
            banners: Vec::new(),
        }
    });
    advance(&mut running, 40);
    assert_eq!(
        running.harness.text_of(".sill-nc-foot").as_deref(),
        Some("Notifications are shown by plasmashell")
    );
    running.feeds.notifications.send_modify(|n| {
        n.live = Notifications {
            server: ServerStatus {
                mode: ServerMode::Own,
                holder: NameHolder::Sill,
            },
            banners: Vec::new(),
        }
    });
    advance(&mut running, 40);
    assert_eq!(running.harness.count(".sill-nc-foot"), 0);
}

#[test]
fn a_press_outside_the_panel_closes_it_after_quires_exit() {
    let mut running = opened(two_apps());
    running.harness.send(Input::click(ds::Point {
        x: ds::Px(100.0),
        y: ds::Px(400.0),
    }));
    until(&mut running, |h| {
        h.attr(".ds-side-panel", "data-presence").as_deref() != Some("present")
    });
    assert_eq!(running.link.now(), CenterShown::Closed);
    assert_eq!(
        running
            .harness
            .attr(".ds-side-panel", "data-presence")
            .as_deref(),
        Some("leaving"),
        "still drawn while quire's exit plays"
    );
    let exit = settle_time(Anim::PanelOut, MotionLevel::Standard);
    settle(&mut running.harness, |h| h.count(".ds-side-panel") == 0);
    assert_eq!(
        running.harness.count(".ds-side-panel"),
        0,
        "gone after {exit:?}"
    );
    assert!(commands(&mut running).is_empty());
}

#[test]
fn opened_again_while_leaving_it_stays() {
    let mut running = opened(two_apps());
    running.link.close();
    advance(&mut running, 60);
    running.link.open(fake::output().id);
    advance(&mut running, 600);
    assert_eq!(running.harness.count(".ds-side-panel"), 1);
    assert_eq!(
        running
            .harness
            .attr(".ds-side-panel", "data-presence")
            .as_deref(),
        Some("present")
    );
}

#[test]
fn a_press_inside_the_panel_keeps_it_open() {
    let mut running = opened(two_apps());
    let title = running.harness.centre(".sill-nc-title").expect("the title");
    running.harness.send(Input::click(title));
    // Nothing should change, so nothing to wait on: a margin past the panel's exit time.
    advance(&mut running, 400);
    assert_eq!(running.link.now(), CenterShown::Open(fake::output().id));
    assert_eq!(running.harness.count(".ds-side-panel"), 1);
}
