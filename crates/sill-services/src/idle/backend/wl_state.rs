//! The idle thread's protocol state: the held notifications, the outputs and their power
//! controls, and what it does on each order.

use smithay_client_toolkit::reexports::protocols_wlr::output_power_management::v1::client::zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1;
use smithay_client_toolkit::reexports::protocols_wlr::output_power_management::v1::client::zwlr_output_power_v1::{Mode, ZwlrOutputPowerV1};
use wayland_client::{Proxy, QueueHandle};
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notification_v1::ExtIdleNotificationV1;
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notifier_v1::ExtIdleNotifierV1;
use sill_model::idle::{Threshold, Watch, DisplayPower};

use super::wl::{Offered, Protocols, Tell, WlEvent};

/// Whether a notification has said `idled` and not yet `resumed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Idleness {
    Waiting,
    Idled,
}

/// One held notification. Its user data is its `id`.
#[derive(Debug)]
struct Held {
    id: u64,
    watch: Watch,
    note: ExtIdleNotificationV1,
    idleness: Idleness,
}

/// One output and its power control (none without the manager, or after `failed`).
#[derive(Debug)]
pub(crate) struct Screen {
    pub(crate) global: u32,
    pub(crate) output: WlOutput,
    pub(crate) power: Option<ZwlrOutputPowerV1>,
}

/// Everything the loop's callbacks reach.
pub(crate) struct WlState {
    pub(crate) qh: QueueHandle<WlState>,
    pub(crate) tell: Tell,
    notifier: Option<ExtIdleNotifierV1>,
    seat: Option<WlSeat>,
    pub(crate) power: Option<ZwlrOutputPowerManagerV1>,
    pub(crate) screens: Vec<Screen>,
    held: Vec<Held>,
    /// Notifications no longer wanted that had idled: kept until they resume.
    retiring: Vec<Held>,
    next_id: u64,
    /// What the displays were last told.
    pub(crate) displays: DisplayPower,
}

impl WlState {
    /// Bind the notifier, the first seat, the output power manager and every output there is.
    pub(crate) fn bind(globals: &GlobalList, qh: QueueHandle<WlState>, tell: Tell) -> WlState {
        let notifier = globals.bind::<ExtIdleNotifierV1, _, _>(&qh, 1..=2, ()).ok();
        let seat = globals.bind::<WlSeat, _, _>(&qh, 1..=1, ()).ok();
        let power = globals
            .bind::<ZwlrOutputPowerManagerV1, _, _>(&qh, 1..=1, ())
            .ok();
        let mut state = WlState {
            qh,
            tell,
            notifier,
            seat,
            power,
            screens: Vec::new(),
            held: Vec::new(),
            retiring: Vec::new(),
            next_id: 0,
            displays: DisplayPower::On,
        };
        let outputs: Vec<(u32, u32)> = globals
            .contents()
            .clone_list()
            .into_iter()
            .filter(|global| global.interface == WlOutput::interface().name)
            .map(|global| (global.name, global.version))
            .collect();
        for (name, version) in outputs {
            let output =
                globals
                    .registry()
                    .bind::<WlOutput, _, _>(name, version.min(4), &state.qh, ());
            state.add_screen(name, output);
        }
        state
    }

    pub(crate) fn protocols(&self) -> Protocols {
        let offered = |yes: bool| if yes { Offered::Yes } else { Offered::No };
        Protocols {
            notifier: offered(self.notifier.is_some() && self.seat.is_some()),
            output_power: offered(self.power.is_some()),
        }
    }

    /// A new output gets a power control, set off at once if the displays are off.
    pub(crate) fn add_screen(&mut self, global: u32, output: WlOutput) {
        let power = self
            .power
            .as_ref()
            .map(|manager| manager.get_output_power(&output, &self.qh, ()));
        if let (Some(power), DisplayPower::Off) = (&power, self.displays) {
            power.set_mode(Mode::Off);
        }
        self.screens.push(Screen {
            global,
            output,
            power,
        });
    }

    /// An output went away: drop its power control and give it back.
    pub(crate) fn remove_screen(&mut self, global: u32) {
        let (gone, kept) = std::mem::take(&mut self.screens)
            .into_iter()
            .partition(|screen| screen.global == global);
        self.screens = kept;
        for screen in gone {
            if let Some(power) = screen.power {
                power.destroy();
            }
            if screen.output.version() >= 3 {
                screen.output.release();
            }
        }
    }

    /// Hold `watches`: unchanged ones are kept, the rest destroyed (or retired while idled)
    /// and the new ones created.
    pub(crate) fn watch(&mut self, watches: Vec<Watch>) {
        let (kept, dropped): (Vec<Held>, Vec<Held>) = std::mem::take(&mut self.held)
            .into_iter()
            .partition(|held| watches.contains(&held.watch));
        for held in dropped {
            match held.idleness {
                Idleness::Idled => self.retiring.push(held),
                Idleness::Waiting => held.note.destroy(),
            }
        }
        self.held = kept;
        let (Some(notifier), Some(seat)) = (&self.notifier, &self.seat) else {
            return;
        };
        for watch in watches {
            if self.held.iter().any(|held| held.watch == watch) {
                continue;
            }
            self.next_id += 1;
            let ms = watch.after.0.saturating_mul(1_000);
            let note = notifier.get_idle_notification(ms, seat, &self.qh, self.next_id);
            self.held.push(Held {
                id: self.next_id,
                watch,
                note,
                idleness: Idleness::Waiting,
            });
        }
    }

    /// Drop every notification and hold `watches` afresh.
    pub(crate) fn renew(&mut self, watches: Vec<Watch>) {
        for held in self.held.drain(..).chain(self.retiring.drain(..)) {
            held.note.destroy();
        }
        self.watch(watches);
    }

    /// Every output's power as `power` says.
    pub(crate) fn displays(&mut self, power: DisplayPower) {
        self.displays = power;
        let mode = match power {
            DisplayPower::Off => Mode::Off,
            DisplayPower::On => Mode::On,
        };
        for control in self.screens.iter().filter_map(|s| s.power.as_ref()) {
            control.set_mode(mode);
        }
    }

    /// A notification said `idled`.
    pub(crate) fn idled(&mut self, id: u64) {
        if let Some(held) = self.held.iter_mut().find(|held| held.id == id) {
            held.idleness = Idleness::Idled;
            let stage: Threshold = held.watch.stage;
            self.tell.say(WlEvent::Idled(stage));
        }
    }

    /// A notification said `resumed`: real input. A retired one is destroyed now.
    pub(crate) fn resumed(&mut self, id: u64) {
        if let Some(held) = self.held.iter_mut().find(|held| held.id == id) {
            held.idleness = Idleness::Waiting;
        }
        if let Some(at) = self.retiring.iter().position(|held| held.id == id) {
            self.retiring.remove(at).note.destroy();
        }
        self.tell.say(WlEvent::Resumed);
    }

    /// A power control failed (the output went, or it cannot be controlled): forget it.
    pub(crate) fn power_failed(&mut self, control: &ZwlrOutputPowerV1) {
        for screen in &mut self.screens {
            if screen.power.as_ref() == Some(control) {
                control.destroy();
                screen.power = None;
            }
        }
    }
}
