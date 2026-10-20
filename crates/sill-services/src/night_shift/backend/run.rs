//! The loop: load the two files, then feed the pure step the person's commands, casement's file
//! changing under us, and the clock reaching the end of a manual turn-on; what the step asks
//! (a write of either file) is carried out here. Beside it, casement's `skip_outputs` is kept to
//! the monitors sill warms (`sill_model::night_shift::skip_outputs`) as the displays come and go.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use sill_model::displays::Displays;
use sill_model::ids::DdcId;
use sill_model::night_shift::{
    MinuteOfDay, NightShift, NightShiftCommand, NightShiftEffect, NightShiftEvent, Now, Reach,
    UnixSeconds, skip_outputs, step, unmatched,
};
use tokio::sync::watch;

use super::files::{SETTINGS, read_light, read_manual, write_light, write_manual, write_skip};
use crate::debounced::{self, Debounced};
use crate::env::WallClock;
use crate::night_shift::NightShiftService;
use crate::service::ServiceIo;

/// How often a manual turn-on's end is looked at again: the sleep is the monotonic clock's, which
/// stops while the machine sleeps, so the wall clock is asked at least this often.
const RECHECK: Duration = Duration::from_secs(60);

pub(crate) async fn run(
    casement: PathBuf,
    state: PathBuf,
    clock: Arc<dyn WallClock>,
    displays: watch::Receiver<Displays>,
    io: ServiceIo<NightShiftService>,
) {
    let ServiceIo {
        state: tx,
        mut commands,
    } = io;
    let mut touched = match Debounced::file(&casement, SETTINGS) {
        Ok(touched) => Some(touched),
        Err(error) => {
            eprintln!(
                "sill: night shift: watching {}: {error}",
                casement.display()
            );
            None
        }
    };
    let mut moving = Some(displays.clone());
    let mut live = Live {
        current: NightShift::default(),
        heard: Heard::Not,
        said: BTreeSet::new(),
        tx,
        casement,
        state,
        clock,
        displays,
    };
    live.reload();
    live.feed(NightShiftEvent::Tick(live.now()));
    loop {
        let due = async {
            match live.current.manual {
                Some(manual) => {
                    let remaining = manual.until.0 - live.now().unix.0;
                    let seconds = u64::try_from(remaining).unwrap_or(0);
                    tokio::time::sleep(Duration::from_secs(seconds).min(RECHECK)).await;
                }
                None => std::future::pending().await,
            }
        };
        let monitors = async {
            match &mut moving {
                Some(moving) => moving.changed().await.is_ok(),
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            command = commands.recv() => {
                let Some(command) = command else { return };
                live.tell(command);
            }
            followed = monitors => {
                if !followed {
                    moving = None;
                    continue;
                }
                live.sync_skip();
            }
            settled = debounced::settled(&mut touched) => {
                if settled.is_none() {
                    touched = None;
                    continue;
                }
                live.reload();
            }
            () = due => {
                let now = live.now();
                live.feed(NightShiftEvent::Tick(now));
            }
        }
    }
}

/// Whether the shell has said who warms the monitors. Until it has, casement's `skip_outputs`
/// is left as it is, so a start does not undo the last run's list before the policy speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Heard {
    Not,
    Yes,
}

struct Live {
    current: NightShift,
    heard: Heard,
    /// The buses already reported as unmatched, so each is said once.
    said: BTreeSet<DdcId>,
    tx: watch::Sender<NightShift>,
    /// casement's config directory.
    casement: PathBuf,
    /// sill's state directory.
    state: PathBuf,
    clock: Arc<dyn WallClock>,
    /// The displays service's monitors.
    displays: watch::Receiver<Displays>,
}

impl Live {
    fn now(&self) -> Now {
        let zoned = self.clock.now();
        let unix = zoned.timestamp().as_second();
        let local = unix + i64::from(zoned.offset().seconds());
        let minute = u32::try_from(local.rem_euclid(86_400) / 60).unwrap_or(0);
        Now {
            unix: UnixSeconds(unix),
            minute: MinuteOfDay::wrapping(minute),
        }
    }

    /// Read the two files and feed what they say.
    fn reload(&mut self) {
        let event = match read_light(&self.casement) {
            Ok(light) => NightShiftEvent::Loaded {
                light,
                manual: read_manual(&self.state),
            },
            Err(error) => {
                eprintln!("sill: night shift: casement's settings: {error}");
                NightShiftEvent::Unreachable
            }
        };
        self.feed(event);
        self.sync_skip();
    }

    /// Step on what the person or the shell told.
    fn tell(&mut self, command: NightShiftCommand) {
        if matches!(command, NightShiftCommand::Backend(_)) {
            self.heard = Heard::Yes;
        }
        let now = self.now();
        self.feed(NightShiftEvent::Told(command, now));
        self.sync_skip();
    }

    /// Make casement's `skip_outputs` list the monitors sill warms, once the shell has said who
    /// warms them and casement's settings are readable.
    fn sync_skip(&mut self) {
        if self.heard == Heard::Not || self.current.reach != Reach::Ready {
            return;
        }
        let (skip, lost) = {
            let displays = self.displays.borrow();
            (
                skip_outputs(self.current.backend, &displays.ddc),
                unmatched(self.current.backend, &displays.ddc),
            )
        };
        for bus in lost {
            if self.said.insert(bus) {
                eprintln!(
                    "sill: night shift: DDC monitor {} has no known connector; the software light warms it too",
                    bus.0
                );
            }
        }
        if let Err(error) = write_skip(&self.casement, &skip) {
            eprintln!("sill: night shift: casement's skip list: {error}");
        }
    }

    /// Step on `event`, carry out the writes it asks for, then publish: whoever sees the new
    /// state can read the files it describes.
    fn feed(&mut self, event: NightShiftEvent) {
        let (next, effects) = step(self.current, event);
        self.current = next;
        for effect in effects {
            if let Err(error) = self.carry(effect) {
                eprintln!("sill: night shift: {error}");
                let (lost, _) = step(self.current, NightShiftEvent::Unreachable);
                self.current = lost;
            }
        }
        let published = self.current;
        self.tx.send_if_modified(|shown| {
            let changed = *shown != published;
            *shown = published;
            changed
        });
    }

    fn carry(&self, effect: NightShiftEffect) -> Result<(), super::files::FileError> {
        match effect {
            NightShiftEffect::WriteLight(light) => write_light(&self.casement, &light),
            NightShiftEffect::WriteManual(manual) => write_manual(&self.state, manual.as_ref()),
        }
    }
}
