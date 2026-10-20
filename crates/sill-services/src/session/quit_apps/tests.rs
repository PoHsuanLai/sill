use std::cell::{Cell, RefCell};
use std::time::Duration;

use sill_launcher::WindowKey;

use super::sequence::{POLL, QUIT_TIMEOUT};
use super::{AppsHost, OpenWindow, QuitOutcome, quit_apps};

/// How the fake's windows answer a close request.
#[derive(Clone, Copy)]
enum Answer {
    /// Each window is gone after this many polls.
    GoneAfter(u32),
    /// The first window never closes (it asks to save); the others go at once.
    FirstStays,
}

struct Fake {
    windows: RefCell<Vec<OpenWindow>>,
    answer: Answer,
    closed: RefCell<Vec<WindowKey>>,
    polls: Cell<u32>,
    scopes_stopped: Cell<u32>,
}

impl Fake {
    fn with(apps: &[&str], answer: Answer) -> Fake {
        let windows = apps
            .iter()
            .zip(1u64..)
            .map(|(app, key)| OpenWindow {
                key: WindowKey(key),
                app_id: (*app).to_owned(),
            })
            .collect();
        Fake {
            windows: RefCell::new(windows),
            answer,
            closed: RefCell::new(Vec::new()),
            polls: Cell::new(0),
            scopes_stopped: Cell::new(0),
        }
    }
}

impl AppsHost for Fake {
    fn open_windows(&self) -> Vec<OpenWindow> {
        self.windows.borrow().clone()
    }

    fn close(&self, key: WindowKey) {
        self.closed.borrow_mut().push(key);
        if matches!(self.answer, Answer::FirstStays) && key != WindowKey(1) {
            self.windows.borrow_mut().retain(|w| w.key != key);
        }
    }

    async fn wait(&self, span: Duration) {
        assert_eq!(span, POLL);
        self.polls.set(self.polls.get() + 1);
        if let Answer::GoneAfter(polls) = self.answer
            && self.polls.get() >= polls
        {
            self.windows.borrow_mut().clear();
        }
    }

    async fn stop_scopes(&self) {
        self.scopes_stopped.set(self.scopes_stopped.get() + 1);
    }
}

fn run(fake: &Fake) -> QuitOutcome {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a current-thread runtime builds");
    runtime.block_on(quit_apps(fake))
}

#[test]
fn windows_that_close_let_the_session_end_after_the_scopes_are_asked_to_stop() {
    let fake = Fake::with(&["zen", "kate"], Answer::GoneAfter(3));
    assert_eq!(run(&fake), QuitOutcome::Quit);
    assert_eq!(*fake.closed.borrow(), vec![WindowKey(1), WindowKey(2)]);
    assert_eq!(fake.polls.get(), 3);
    assert_eq!(fake.scopes_stopped.get(), 1);
}

#[test]
fn no_windows_ends_at_once_without_waiting() {
    let fake = Fake::with(&[], Answer::GoneAfter(1));
    assert_eq!(run(&fake), QuitOutcome::Quit);
    assert_eq!(fake.polls.get(), 0);
    assert_eq!(fake.scopes_stopped.get(), 1);
}

#[test]
fn a_window_that_stays_holds_the_session_after_the_timeout_and_names_it() {
    let fake = Fake::with(&["zen", "kate"], Answer::FirstStays);
    let held = OpenWindow {
        key: WindowKey(1),
        app_id: "zen".to_owned(),
    };
    assert_eq!(run(&fake), QuitOutcome::Held(vec![held]));
    let steps = QUIT_TIMEOUT.as_millis() / POLL.as_millis();
    assert_eq!(u128::from(fake.polls.get()), steps);
    assert_eq!(fake.scopes_stopped.get(), 0);
}
