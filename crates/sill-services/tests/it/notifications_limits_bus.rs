//! What apps may do to the notifications server over a private bus: oversize and flooding
//! `Notify` calls are cut and refused at their ceilings, a caller replaces or closes only what it
//! posted, and the bus name cannot be taken from the server. Without a `dbus-daemon` they say so
//! and skip.

use std::collections::HashMap;

use sill_model::notifications::{
    ACTIONS_MAX, BANNERS_PER_APP_MAX, BODY_MAX_BYTES, HINTS_MAX, IconRef, NoteImage, NotesState,
    Notification, NotificationId, SENDER_BURST, SUMMARY_MAX_BYTES,
};
use sill_services::notifications::BUS_NAME;
use sill_testkit::PrivateBus;
use tokio::sync::watch;
use zbus::fdo::RequestNameFlags;
use zbus::zvariant::{OwnedValue, Value};

use crate::notifications_bus::{SETTLE, call, connect, notify, owning, wait_for};

type Hints = HashMap<String, OwnedValue>;

fn owned(value: Value<'_>) -> OwnedValue {
    value.try_into_owned().unwrap_or_else(|e| panic!("{e}"))
}

/// A `Notify` from `app` with the given pieces, answering the call's result.
async fn notify_with(
    connection: &zbus::Connection,
    app: &str,
    replaces: u32,
    text: (&str, &str),
    actions: Vec<String>,
    hints: Hints,
) -> zbus::Result<u32> {
    let body = (app, replaces, "", text.0, text.1, actions, hints, -1i32);
    let reply = call(connection, "Notify", &body).await?;
    reply.body().deserialize()
}

#[test]
fn oversize_fields_are_cut_and_a_flood_of_hints_is_refused() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = sill_testkit::TempDir::new("notes-oversize");
    let (mut notes, _control) = owning(&runtime, &bus, &scratch);
    runtime.block_on(async {
        let connection = connect(&bus).await;
        let huge = "x".repeat(2 << 20);
        let actions: Vec<String> = (0..200).map(|n| format!("key-{n}")).collect();
        let mut hints = Hints::new();
        hints.insert(
            "desktop-entry".to_owned(),
            owned(Value::from("sill-accounts")),
        );
        // 512 x 512 RGBA, far past what a card draws.
        let side = 512i32;
        let image = (
            side,
            side,
            side * 4,
            true,
            8i32,
            4i32,
            vec![9u8; 512 * 512 * 4],
        );
        hints.insert("image-data".to_owned(), owned(Value::from(image)));
        notify_with(
            &connection,
            "big",
            0,
            (huge.as_str(), huge.as_str()),
            actions,
            hints,
        )
        .await
        .unwrap_or_else(|e| panic!("Notify: {e}"));
        assert!(wait_for(&mut notes, SETTLE, |n| n.live.banners.len() == 1).await);
        let held = notes.borrow().live.banners[0].note.clone();
        assert!(
            held.summary.len() <= SUMMARY_MAX_BYTES + 3,
            "{}",
            held.summary.len()
        );
        assert!(held.body.text().len() <= BODY_MAX_BYTES);
        assert_eq!(held.actions.len(), ACTIONS_MAX);
        assert_eq!(
            held.hints.image,
            NoteImage::None,
            "the oversize image is dropped"
        );
        assert_eq!(
            held.app.desktop_entry, None,
            "the shell's namespace is not an app's"
        );

        let mut many = Hints::new();
        for n in 0..=HINTS_MAX {
            many.insert(format!("x-{n}"), owned(Value::from(1u32)));
        }
        let refused = notify_with(&connection, "big", 0, ("s", "b"), Vec::new(), many).await;
        assert!(refused.is_err(), "a flood of hints is refused: {refused:?}");
    });
}

/// A `Notify` from `app` with `app_icon` and an `image-path` hint.
async fn notify_with_pictures(
    connection: &zbus::Connection,
    app: &str,
    app_icon: &str,
    image_path: &str,
) -> zbus::Result<u32> {
    let mut hints = Hints::new();
    hints.insert("image-path".to_owned(), owned(Value::from(image_path)));
    let body = (
        app,
        0u32,
        app_icon,
        "s",
        "b",
        Vec::<String>::new(),
        hints,
        -1i32,
    );
    call(connection, "Notify", &body)
        .await?
        .body()
        .deserialize()
}

/// The live banner `app` posted.
fn banner_of(notes: &watch::Receiver<NotesState>, app: &str) -> Notification {
    notes
        .borrow()
        .live
        .banners
        .iter()
        .find(|banner| banner.note.app.name == app)
        .map(|banner| banner.note.clone())
        .unwrap_or_else(|| panic!("no banner from {app}"))
}

#[test]
fn a_picture_a_caller_names_is_drawn_only_from_the_icon_folders() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = sill_testkit::TempDir::new("notes-pictures");
    let (mut notes, _control) = owning(&runtime, &bus, &scratch);
    let themed = scratch.path().join("data/icons/hicolor/48x48/apps/a.png");
    let themed = themed.to_string_lossy().into_owned();
    runtime.block_on(async {
        let connection = connect(&bus).await;
        // A file outside every folder is no picture, whichever field named it.
        notify_with_pictures(&connection, "outside", "/etc/passwd", "file:///etc/shadow")
            .await
            .unwrap_or_else(|e| panic!("Notify: {e}"));
        assert!(wait_for(&mut notes, SETTLE, |n| n.live.banners.len() == 1).await);
        let held = banner_of(&notes, "outside");
        assert_eq!(held.app.icon, IconRef::None, "app_icon outside the folders");
        assert_eq!(held.hints.image, NoteImage::None, "image-path outside");

        // A file in the icon folders, and a theme name, are kept.
        notify_with_pictures(&connection, "inside", "mail-unread", &themed)
            .await
            .unwrap_or_else(|e| panic!("Notify: {e}"));
        assert!(wait_for(&mut notes, SETTLE, |n| n.live.banners.len() == 2).await);
        let held = banner_of(&notes, "inside");
        assert_eq!(held.app.icon, IconRef::Name("mail-unread".to_owned()));
        assert_eq!(
            held.hints.image,
            NoteImage::Icon(IconRef::File(themed.clone().into()))
        );
    });
}

#[test]
fn a_sender_that_floods_is_refused_and_its_banners_are_capped() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = sill_testkit::TempDir::new("notes-flood");
    let (mut notes, _control) = owning(&runtime, &bus, &scratch);
    runtime.block_on(async {
        let flooder = connect(&bus).await;
        let calls = usize::try_from(SENDER_BURST).unwrap_or(0) * 3;
        let mut refused = 0usize;
        for _ in 0..calls {
            let sent = notify_with(&flooder, "loop", 0, ("again", ""), Vec::new(), Hints::new());
            if sent.await.is_err() {
                refused += 1;
            }
        }
        assert!(refused > 0, "{calls} calls in a burst, none refused");
        assert!(
            wait_for(&mut notes, SETTLE, |n| n.live.banners.len()
                == BANNERS_PER_APP_MAX)
            .await,
            "the flood holds {} banners",
            notes.borrow().live.banners.len()
        );
        // Another caller is not held back by the flooder's spent allowance.
        let calm = connect(&bus).await;
        let id = notify(&calm, "calm", Vec::new()).await;
        assert_ne!(id, 0);
    });
}

#[test]
fn only_the_sender_or_the_same_app_may_replace_and_only_the_sender_may_close() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = sill_testkit::TempDir::new("notes-owner");
    let (mut notes, _control) = owning(&runtime, &bus, &scratch);
    runtime.block_on(async {
        let (owner, other) = (connect(&bus).await, connect(&bus).await);
        let first = notify(&owner, "owner", Vec::new()).await;
        assert!(wait_for(&mut notes, SETTLE, |n| n.live.banners.len() == 1).await);

        let taken = notify_with(
            &other,
            "other",
            first,
            ("mine now", ""),
            Vec::new(),
            Hints::new(),
        )
        .await
        .unwrap_or_else(|e| panic!("Notify: {e}"));
        assert_ne!(
            taken, first,
            "another sender's replacement is a new notification"
        );
        assert!(wait_for(&mut notes, SETTLE, |n| n.live.banners.len() == 2).await);
        let kept = notes
            .borrow()
            .live
            .banner(NotificationId(first))
            .map(|b| b.note.summary.clone());
        assert_eq!(kept.as_deref(), Some("Hello"), "the original is untouched");

        let closed = call(&other, "CloseNotification", &(first,)).await;
        assert!(closed.is_err(), "another sender closed it: {closed:?}");
        assert!(notes.borrow().live.banner(NotificationId(first)).is_some());

        let replaced = notify_with(
            &owner,
            "owner",
            first,
            ("edited", ""),
            Vec::new(),
            Hints::new(),
        )
        .await
        .unwrap_or_else(|e| panic!("Notify: {e}"));
        assert_eq!(replaced, first, "its own sender replaces it");
        call(&owner, "CloseNotification", &(first,))
            .await
            .unwrap_or_else(|e| panic!("CloseNotification: {e}"));
        assert!(
            wait_for(&mut notes, SETTLE, |n| n
                .live
                .banner(NotificationId(first))
                .is_none())
            .await
        );

        // A later run of the same script (a new connection naming the same app) replaces its
        // own notice, as `notify-send -r` does; closing stays with the connection that posted.
        let second = notify(&owner, "owner", Vec::new()).await;
        let script = connect(&bus).await;
        let rerun = notify_with(
            &script,
            "owner",
            second,
            ("volume 40%", ""),
            Vec::new(),
            Hints::new(),
        )
        .await
        .unwrap_or_else(|e| panic!("Notify: {e}"));
        assert_eq!(rerun, second, "the same app replaces across connections");
    });
}

#[test]
fn the_name_cannot_be_taken_from_the_server() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = sill_testkit::TempDir::new("notes-name");
    let (_notes, _control) = owning(&runtime, &bus, &scratch);
    runtime.block_on(async {
        let thief = connect(&bus).await;
        let flags = RequestNameFlags::ReplaceExisting | RequestNameFlags::DoNotQueue;
        let taken = thief.request_name_with_flags(BUS_NAME, flags).await;
        assert!(taken.is_err(), "the name was handed over: {taken:?}");
        let info = call(&thief, "GetServerInformation", &()).await;
        let info: (String, String, String, String) = info
            .unwrap_or_else(|e| panic!("{e}"))
            .body()
            .deserialize()
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(info.0, "sill", "sill still answers");
    });
}
