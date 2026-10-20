//! The session lookup order: by pid, then `XDG_SESSION_ID`, then the user's display. A fake
//! logind answers only the steps it is given, and records which steps were asked.

use std::sync::Mutex;

use sill_services::login_session::{SessionLookup, find_session};
use zbus::zvariant::OwnedObjectPath;

fn path(text: &str) -> OwnedObjectPath {
    OwnedObjectPath::try_from(text).expect("a valid object path")
}

/// Which steps know a session, and which steps were asked.
#[derive(Default)]
struct Fake {
    pid: Option<&'static str>,
    id: Option<&'static str>,
    display: Option<&'static str>,
    asked: Mutex<Vec<&'static str>>,
}

impl Fake {
    fn ask(&self, step: &'static str) {
        self.asked.lock().expect("lock").push(step);
    }
}

impl SessionLookup for Fake {
    async fn by_pid(&self) -> Option<OwnedObjectPath> {
        self.ask("pid");
        self.pid.map(path)
    }

    async fn by_id(&self, _id: &str) -> Option<OwnedObjectPath> {
        self.ask("id");
        self.id.map(path)
    }

    async fn display(&self) -> Option<OwnedObjectPath> {
        self.ask("display");
        self.display.map(path)
    }
}

#[tokio::test]
async fn the_pid_wins_then_the_id_then_the_display() {
    let all = Fake {
        pid: Some("/s/pid"),
        id: Some("/s/id"),
        display: Some("/s/display"),
        ..Fake::default()
    };
    assert_eq!(find_session(&all, Some("3")).await, Some(path("/s/pid")));
    assert_eq!(*all.asked.lock().expect("lock"), ["pid"]);

    let no_pid = Fake {
        id: Some("/s/id"),
        display: Some("/s/display"),
        ..Fake::default()
    };
    assert_eq!(find_session(&no_pid, Some("3")).await, Some(path("/s/id")));
    assert_eq!(*no_pid.asked.lock().expect("lock"), ["pid", "id"]);

    let display_only = Fake {
        display: Some("/s/display"),
        ..Fake::default()
    };
    assert_eq!(
        find_session(&display_only, Some("3")).await,
        Some(path("/s/display"))
    );
    assert_eq!(
        *display_only.asked.lock().expect("lock"),
        ["pid", "id", "display"]
    );
}

#[tokio::test]
async fn without_an_id_the_id_step_is_skipped() {
    let fake = Fake {
        id: Some("/s/id"),
        display: Some("/s/display"),
        ..Fake::default()
    };
    assert_eq!(find_session(&fake, None).await, Some(path("/s/display")));
    assert_eq!(*fake.asked.lock().expect("lock"), ["pid", "display"]);
}

#[tokio::test]
async fn nothing_known_is_none() {
    assert_eq!(find_session(&Fake::default(), Some("3")).await, None);
}
