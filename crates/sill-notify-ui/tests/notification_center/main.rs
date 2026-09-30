//! The notification center on `SurfaceHarness`, in four files: what it draws (`render`), its keys
//! (`keys`), its list motion (`motion`) and the focus ring under the keyboard (`modality`). The
//! history is fed through the fake's `watch` channel and the commands the center sends are read
//! back from its queue: nothing here reaches a bus. On the virtual clock: every timer and every
//! frame's CSS move only with the test's `advance`.

#![allow(dead_code)] // Each file uses its own subset.

mod keys;
mod modality;
mod motion;
mod render;

pub use sill_ui_kit::harness::fake;

use std::time::Duration;

use ds_native::harness::settle_until;
use ds_native::{Harness, Viewport};
use sill_model::notifications::{
    AppGroup, AppKey, AppRef, Body, Expiry, Hints, History, IconRef, Notification, NotificationId,
    NotificationsCommand, UnixMs,
};
use sill_notify_ui::NotificationCenter;
use sill_settings::ShellFile;
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::links::CenterLink;

/// The viewport's size, as the rects report it.
pub const WIDTH: f32 = 1280.0;
pub const HEIGHT: f32 = 800.0;

pub const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

/// A running center and what it talks to.
pub type Running = SurfaceHarness<NotificationCenter>;

/// Every command the center has sent since the last call.
pub fn commands(running: &mut Running) -> Vec<NotificationsCommand> {
    std::iter::from_fn(|| running.feeds.notification_commands.try_recv().ok()).collect()
}

/// Let `time` pass.
pub fn advance(running: &mut Running, ms: u64) {
    running.harness.advance(Duration::from_millis(ms));
}

/// Advance until `done` holds ([`settle`]): what a click test waits on.
pub fn until(running: &mut Running, done: impl Fn(&Harness) -> bool) {
    settle(&mut running.harness, done);
}

/// The commands sent, advancing until `count` have come, at most [`STEPS`] frames.
pub fn commands_until(running: &mut Running, count: usize) -> Vec<NotificationsCommand> {
    let mut sent = commands(running);
    for _ in 0..STEPS {
        if sent.len() >= count {
            break;
        }
        running.harness.advance(Duration::from_millis(20));
        sent.extend(commands(running));
    }
    sent
}

/// The center on `file`'s settings over `history`, closed.
pub fn start(file: ShellFile, history: History) -> Running {
    let running = SurfaceHarness::<NotificationCenter>::setup(CenterLink::new(), ())
        .settings(file)
        .viewport(VIEWPORT)
        .start();
    running
        .feeds
        .notifications
        .send_modify(|n| n.history = history);
    running
}

/// The center opened over `history`, once its entrance has played.
pub fn opened(history: History) -> Running {
    let mut running = start(ShellFile::default(), history);
    advance(&mut running, 40);
    running.link.open(fake::output().id);
    settle(&mut running.harness, |h| h.count(".ds-panel") == 1);
    advance(&mut running, 600);
    running
}

/// Advance until `done` holds (`settle_until`, which fails with the document if it never does);
/// then one more frame's worth.
pub fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
    settle_until(harness, done);
    harness.advance(Duration::from_millis(20));
}

/// The most 20 ms frames a wait advances: four seconds of the harness's clock.
const STEPS: usize = 200;

/// A notification from `app`.
pub fn note(id: u32, app: &str, summary: &str) -> Notification {
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

/// Three from Mail (ids 5, 3, 1, newest first) and one from Chat (4), Mail's group first.
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

/// The one row (card or group header) that carries the keyboard's ring, as its card's stack
/// size or `head`; `None` without a ring.
pub fn ringed(harness: &Harness) -> Option<String> {
    match harness.count(".sill-nc-row[*|data-focus=ring]") {
        1 => harness.attr(".sill-nc-row[*|data-focus=ring]", "data-stack"),
        _ => (harness.count(".sill-nc-group-head[*|data-focus=ring]") == 1)
            .then(|| "head".to_owned()),
    }
}
