//! The keycap loop: the socket thread's events, the focused window, the config file's edits, the
//! person's commands and the unit check, stepping [`KeycapState`] and posting its notices.

use keycap_protocol::{ProfileName, ToDaemon};
use sill_model::apps::AppIndex;
use sill_model::cosmic_wl::CosmicWl;
use sill_model::keyboard::{CategoryTable, UserOverrides, wanted};
use sill_model::keycap::{
    Attention, KeycapCommand, KeycapEvent, KeycapState, NoticeChange, attention, focus_to_send,
    notice_for, step, user_config, watching_unit,
};
use sill_model::notifications::{NotesState, NotificationsCommand};
use tokio::runtime::Handle;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tokio::sync::watch;
use tokio::time::{Duration, Instant, sleep_until};

use super::links::{KeycapLinks, Reach};
use super::socket::{self, Link};
use super::{config, notice, systemd};
use crate::control::Control;
use crate::error::ServiceError;
use crate::file_touch::FileTouch;
use crate::keycap::KeycapService;
use crate::service::ServiceIo;

/// How soon the unit is read after a retry asked systemd to restart it.
const AFTER_RETRY: Duration = Duration::from_secs(1);

/// Everything the loop owns.
struct Live {
    reach: Reach,
    rt: Handle,
    link: Link,
    told: UnboundedSender<KeycapEvent>,
    tx: watch::Sender<KeycapState>,
    cosmic_wl: watch::Receiver<CosmicWl>,
    apps: watch::Receiver<AppIndex>,
    notes: watch::Receiver<NotesState>,
    notifications: Control<NotificationsCommand>,
    state: KeycapState,
    /// What the notification now says.
    shown: Option<Attention>,
    /// The profile keycap was last sent.
    chosen: Option<ProfileName>,
    overrides: UserOverrides,
    categories: CategoryTable,
    /// When the unit is read next; `None` while it is not worth reading.
    check_at: Option<Instant>,
}

pub(crate) async fn run(
    reach: Reach,
    links: KeycapLinks,
    io: ServiceIo<KeycapService>,
) -> Result<(), ServiceError> {
    let ServiceIo {
        state: tx,
        mut commands,
    } = io;
    let Some(socket) = reach.socket.clone() else {
        // Nothing to connect to: commands go nowhere, and the receivers keep reading `Unknown`.
        while commands.recv().await.is_some() {}
        return Ok(());
    };
    let (told, mut events) = unbounded_channel();
    let link = socket::start(socket, told.clone())?;
    let mut touch = config::watch(&reach.config_dir)
        .map_err(|error| eprintln!("sill: keycap: watching keycap.toml: {error}"))
        .ok();
    let (mut cosmic_wl, mut apps) = (links.cosmic_wl.clone(), links.apps.clone());
    let mut live = Live {
        reach,
        rt: Handle::current(),
        link,
        told,
        tx,
        cosmic_wl: links.cosmic_wl,
        apps: links.apps,
        notes: links.notes,
        notifications: links.notifications,
        state: KeycapState::default(),
        shown: None,
        chosen: None,
        overrides: UserOverrides::default(),
        categories: CategoryTable::standard(),
        check_at: None,
    };
    live.start();
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => live.command(command),
                None => break,
            },
            Some(event) = events.recv() => live.feed(event),
            Ok(()) = cosmic_wl.changed() => live.refocus(),
            Ok(()) = apps.changed() => live.refocus(),
            Some(()) = touched(&mut touch) => live.send_config(),
            () = due(live.check_at) => live.check_unit(),
        }
    }
    Ok(())
}

/// The next settled edit of the config file; never, without a watch.
async fn touched(touch: &mut Option<FileTouch>) -> Option<()> {
    match touch {
        Some(touch) => touch.settled().await,
        None => std::future::pending().await,
    }
}

/// Resolves at `at`; never, for `None`.
async fn due(at: Option<Instant>) {
    match at {
        Some(at) => sleep_until(at).await,
        None => std::future::pending().await,
    }
}

impl Live {
    /// Tell keycap what the window and the config file say now, and start watching the unit.
    fn start(&mut self) {
        self.refocus();
        if let Some(message) = config::read(&self.reach.config_dir).and_then(user_config) {
            self.link.send(message);
        }
        self.rearm();
    }

    fn command(&mut self, command: KeycapCommand) {
        match command {
            KeycapCommand::Resume => self.link.send(ToDaemon::Resume),
            KeycapCommand::Retry => self.retry(),
            KeycapCommand::Overrides(overrides) => {
                self.overrides = overrides;
                self.refocus();
            }
        }
    }

    /// Ask systemd to reset and restart the unit, and read it again soon after.
    fn retry(&mut self) {
        let Some(bus) = self.reach.system.clone() else {
            eprintln!("sill: keycap: there is no system bus to ask systemd on");
            return;
        };
        self.rt.spawn(async move {
            if let Err(error) = systemd::restart(&bus).await {
                eprintln!("sill: keycap: restarting keycap.service: {error}");
            }
        });
        self.check_at = Some(Instant::now() + AFTER_RETRY);
    }

    /// Send the profile the focused window wants, when it is not the one keycap has.
    fn refocus(&mut self) {
        let next = wanted(
            &self.overrides,
            &self.categories,
            &self.cosmic_wl.borrow().toplevels,
            &self.apps.borrow(),
        );
        if let Some(profile) = focus_to_send(self.chosen.as_ref(), next) {
            self.chosen = Some(profile.clone());
            self.link.send(ToDaemon::Focus { profile });
        }
    }

    /// Send the config file as it is now; a file that is gone sends an empty config, so keycap
    /// forgets the one it had.
    fn send_config(&mut self) {
        let text = config::read(&self.reach.config_dir).unwrap_or_default();
        match user_config(text) {
            Some(message) => self.link.send(message),
            None => eprintln!("sill: keycap: keycap.toml is too large to send"),
        }
    }

    /// Step the state by `event`, publish it, and tell the person what it newly needs them to know.
    fn feed(&mut self, event: KeycapEvent) {
        let next = step(self.state.clone(), event);
        if next != self.state {
            self.tx.send_replace(next.clone());
        }
        self.state = next;
        self.notify();
        self.rearm();
    }

    /// Post, replace or withdraw the notification as `attention` now says.
    fn notify(&mut self) {
        let next = attention(&self.state);
        let command = match notice_for(self.shown.as_ref(), next.as_ref()) {
            NoticeChange::Keep => None,
            NoticeChange::Show(attention) => {
                let replaces = notice::shown(&self.notes.borrow());
                Some(NotificationsCommand::Post(Box::new(notice::posting(
                    &attention, replaces,
                ))))
            }
            NoticeChange::Withdraw => {
                notice::shown(&self.notes.borrow()).map(NotificationsCommand::Dismiss)
            }
        };
        if let Some(command) = command {
            // A stopped notifications service has nobody to tell.
            let _ = self.notifications.send(command);
        }
        self.shown = next;
    }

    /// Start the unit check clock when the socket is down, and stop it when it is not worth running.
    fn rearm(&mut self) {
        let watching = self.reach.system.is_some() && watching_unit(&self.state);
        self.check_at = match (watching, self.check_at) {
            (true, None) => Some(Instant::now() + self.reach.unit_wait.grace),
            (true, due) => due,
            (false, _) => None,
        };
    }

    /// Read the unit now, off the loop, and again after the recheck wait.
    fn check_unit(&mut self) {
        self.check_at = Some(Instant::now() + self.reach.unit_wait.recheck);
        let Some(bus) = self.reach.system.clone() else {
            return;
        };
        let told = self.told.clone();
        self.rt.spawn(async move {
            if let Some(unit) = systemd::read_state(&bus).await {
                let _ = told.send(KeycapEvent::UnitState(unit));
            }
        });
    }
}
