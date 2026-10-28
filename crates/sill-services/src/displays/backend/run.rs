//! The displays service task: commands in, [`Displays`] out. The DDC worker runs on a thread
//! of its own; output passes (an output-management round trip and a few sysfs reads) run on the
//! blocking pool; everything else here is quick.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, mpsc as std_mpsc};
use std::time::Duration;

use sill_model::displays::brightness::Backlight;
use sill_model::displays::ddc::{DdcDisplay, Feature, Write};
use sill_model::displays::font::font_rendering_for;
use sill_model::displays::hw_volume::follow;
use sill_model::displays::output::{DisplayOutput, describe, max_density};
use sill_model::displays::plan::{
    BrightnessAction, Originals, brightness_actions, probe_needed, set_brightness_action,
    warmth_writes,
};
use sill_model::displays::policy::DisplayPolicy;
use sill_model::displays::randr::Enablement;
use sill_model::displays::{
    Displays, DisplaysCommand, DisplaysEvent, FOLLOW_EVERY_MS, Following, step,
};
use sill_model::volume::Volume;
use tokio::sync::{mpsc, watch};

use super::ddc::worker;
use super::ddc::worker::DdcJob;
use super::outputs_route::OutputsRoute;
use super::{backlight, fontconfig_file};
use crate::env::ddc::{DdcBus, connected};
use crate::env::{Bus, Dirs, WaylandRoute};

/// Where the service looks and writes.
#[derive(Debug, Clone)]
pub struct Paths {
    /// `/sys/class/drm`.
    pub drm: PathBuf,
    /// `/sys/class/backlight`.
    pub backlight: PathBuf,
    /// `~/.config/fontconfig`; `None` without a config directory (no snippet).
    pub fontconfig: Option<PathBuf>,
    /// The DDC buses: the real ones, or (a debug build's dev scripts, tests) made-up monitors.
    pub ddc: Arc<dyn DdcBus>,
    /// logind, which sets a backlight as this session.
    pub system: Bus,
    /// The compositor whose outputs are read.
    pub wayland: WaylandRoute,
}

impl Paths {
    /// The kernel's usual places, the files in `dirs`, logind on `system`, DDC on `ddc`.
    pub fn of(system: Bus, dirs: &Dirs, ddc: Arc<dyn DdcBus>, wayland: WaylandRoute) -> Self {
        Paths {
            drm: PathBuf::from("/sys/class/drm"),
            backlight: PathBuf::from("/sys/class/backlight"),
            fontconfig: Some(fontconfig_file::dir_in(dirs)),
            ddc,
            system,
            wayland,
        }
    }
}

/// The DDC worker's two ends, once started.
struct Ddc {
    jobs: std_mpsc::Sender<DdcJob>,
    displays: watch::Receiver<Vec<DdcDisplay>>,
}

struct Service {
    tx: watch::Sender<Displays>,
    paths: Paths,
    policy: Option<DisplayPolicy>,
    ddc: Option<Ddc>,
    originals: Originals,
    volume: Volume,
    warned: BTreeSet<String>,
    /// How many views follow the brightness now ([`DisplaysCommand::Follow`]).
    followers: usize,
}

/// Serve until every `Control` is dropped.
pub async fn run(
    tx: watch::Sender<Displays>,
    mut commands: mpsc::UnboundedReceiver<DisplaysCommand>,
    mut volume: watch::Receiver<Volume>,
    paths: Paths,
) {
    let mut volume_alive = true;
    let mut service = Service {
        tx,
        paths,
        policy: None,
        ddc: None,
        originals: Originals::new(),
        volume: volume.borrow().clone(),
        warned: BTreeSet::new(),
        followers: 0,
    };
    let mut follow = tokio::time::interval(Duration::from_millis(FOLLOW_EVERY_MS));
    follow.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        let ddc_changed = async {
            match service.ddc.as_mut() {
                Some(ddc) => ddc.displays.changed().await.is_ok(),
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => service.command(command).await,
                None => break,
            },
            changed = volume.changed(), if volume_alive => match changed {
                Ok(()) => {
                    let now = volume.borrow_and_update().clone();
                    service.volume_changed(now);
                }
                Err(_) => volume_alive = false,
            },
            _ = follow.tick(), if service.followers > 0 => service.reread(),
            alive = ddc_changed => {
                if alive {
                    service.ddc_changed();
                } else {
                    service.ddc = None;
                }
            }
        }
    }
}

impl Service {
    async fn command(&mut self, command: DisplaysCommand) {
        match command {
            DisplaysCommand::Policy(policy) => self.policy_changed(policy).await,
            DisplaysCommand::Rescan => self.rescan().await,
            DisplaysCommand::Brightness { at, change } => {
                let Some(policy) = &self.policy else { return };
                let actions =
                    brightness_actions(&self.tx.borrow(), policy.keys_target, at.as_ref(), change);
                for action in actions {
                    self.brightness(action).await;
                }
            }
            DisplaysCommand::SetBrightness { output, level } => {
                if self.policy.is_none() {
                    return;
                }
                let action = set_brightness_action(&self.tx.borrow(), &output, level);
                if let Some(action) = action {
                    self.brightness(action).await;
                }
            }
            DisplaysCommand::Follow(following) => self.follow(following),
            DisplaysCommand::SetFeature {
                display,
                feature,
                value,
            } => self.write(Write {
                id: display,
                feature,
                value,
            }),
        }
    }

    async fn policy_changed(&mut self, policy: DisplayPolicy) {
        let first = self.policy.is_none();
        let outputs_differ = self
            .policy
            .as_ref()
            .is_none_or(|old| old.font_rendering != policy.font_rendering);
        self.policy = Some(policy);
        if first {
            self.start_ddc();
        }
        if outputs_differ {
            self.rescan_outputs().await;
        }
        self.apply_warmth();
    }

    fn start_ddc(&mut self) {
        let (jobs, rx) = std_mpsc::channel();
        let (tx, displays) = watch::channel(Vec::new());
        let bus = self.paths.ddc.clone();
        tokio::task::spawn_blocking(move || worker::run(bus, rx, tx));
        let _ = jobs.send(DdcJob::Probe);
        self.ddc = Some(Ddc { jobs, displays });
    }

    async fn rescan(&mut self) {
        if self.policy.is_none() {
            return;
        }
        let names = |displays: &Displays| -> Vec<_> {
            displays.outputs.iter().map(|o| o.name.clone()).collect()
        };
        let before = names(&self.tx.borrow());
        self.rescan_outputs().await;
        // A mode or scale change (which the compositor also reports as an output change) leaves
        // the monitors where they were: only a different set of outputs is worth a bus probe.
        if names(&self.tx.borrow()) != before
            && let Some(ddc) = &self.ddc
        {
            let _ = ddc.jobs.send(DdcJob::Probe);
        }
    }

    /// The outputs, the fontconfig snippet and the backlights, on the blocking pool.
    async fn rescan_outputs(&mut self) {
        let Some(policy) = self.policy.clone() else {
            return;
        };
        let paths = self.paths.clone();
        let done = tokio::task::spawn_blocking(move || outputs_pass(&paths, &policy)).await;
        let Ok((outputs, backlights, failures)) = done else {
            return;
        };
        for failure in failures {
            self.warn(failure);
        }
        self.publish(DisplaysEvent::Outputs(outputs));
        self.publish(DisplaysEvent::Backlights(backlights));
    }

    async fn brightness(&mut self, action: BrightnessAction) {
        match action {
            BrightnessAction::Ddc(write) => self.write(write),
            BrightnessAction::Backlight { name, value } => {
                if let Err(error) = backlight::set(&self.paths.system, &name, value).await {
                    self.warn(format!("backlight {}: {error}", name.0));
                }
                let backlights = backlight::read_all(&self.paths.backlight);
                self.publish(DisplaysEvent::Backlights(backlights));
            }
        }
    }

    /// A view opened or closed; the first to open reads the brightness at once.
    fn follow(&mut self, following: Following) {
        match following {
            Following::Start => {
                self.followers += 1;
                if self.followers == 1 {
                    self.reread();
                }
            }
            Following::Stop => self.followers = self.followers.saturating_sub(1),
        }
    }

    /// Read every monitor's brightness and the backlights again (a change made elsewhere); probe
    /// the monitors again instead while an external output has none answering (a monitor the
    /// startup probe missed would otherwise leave its capsule disabled for the whole session).
    fn reread(&mut self) {
        if self.policy.is_none() {
            return;
        }
        if let Some(ddc) = &self.ddc {
            let job = match probe_needed(&self.tx.borrow()) {
                true => DdcJob::Probe,
                false => DdcJob::Reread(Feature::Brightness),
            };
            let _ = ddc.jobs.send(job);
        }
        let backlights = backlight::read_all(&self.paths.backlight);
        if backlights != self.tx.borrow().backlights {
            self.publish(DisplaysEvent::Backlights(backlights));
        }
    }

    fn volume_changed(&mut self, now: Volume) {
        let before = std::mem::replace(&mut self.volume, now);
        let Some(policy) = &self.policy else { return };
        let writes = follow(
            policy.hardware_volume,
            &before,
            &self.volume,
            &self.tx.borrow().ddc,
        );
        for write in writes {
            self.write(write);
        }
    }

    fn ddc_changed(&mut self) {
        let displays = match &mut self.ddc {
            Some(ddc) => ddc.displays.borrow_and_update().clone(),
            None => return,
        };
        self.publish(DisplaysEvent::Ddc(displays));
        self.apply_warmth();
    }

    fn apply_warmth(&mut self) {
        let Some(policy) = &self.policy else { return };
        let (originals, writes) =
            warmth_writes(&self.tx.borrow(), &self.originals, policy.night_warmth);
        self.originals = originals;
        for write in writes {
            self.write(write);
        }
    }

    fn write(&self, write: Write) {
        if let Some(ddc) = &self.ddc {
            let _ = ddc.jobs.send(DdcJob::Write(write));
        }
    }

    fn publish(&self, event: DisplaysEvent) {
        self.tx.send_modify(|displays| {
            *displays = step(std::mem::take(displays), event);
        });
    }

    /// Log `message` the first time it is seen.
    fn warn(&mut self, message: String) {
        if self.warned.insert(message.clone()) {
            eprintln!("sill: displays: {message}");
        }
    }
}

/// One outputs pass: describe the compositor's enabled outputs, keep the fontconfig snippet in
/// step with the densest display, and read the backlights.
fn outputs_pass(
    paths: &Paths,
    policy: &DisplayPolicy,
) -> (Vec<DisplayOutput>, Vec<Backlight>, Vec<String>) {
    let connectors = connected(&paths.drm);
    let mut failures = Vec::new();
    let heads = OutputsRoute::pick(&paths.wayland)
        .heads()
        .unwrap_or_else(|error| {
            failures.push(format!("outputs: {error}"));
            Vec::new()
        });
    let outputs: Vec<DisplayOutput> = heads
        .iter()
        .filter(|head| head.enablement == Enablement::Enabled)
        .map(|head| describe(head, connectors.iter().find(|c| c.name == head.name)))
        .collect();
    if let Some(dir) = &paths.fontconfig {
        let rendering = max_density(&outputs).map(font_rendering_for);
        if let Err(error) = fontconfig_file::sync(dir, policy.font_rendering, rendering) {
            failures.push(format!("fontconfig: {error}"));
        }
    }
    (outputs, backlight::read_all(&paths.backlight), failures)
}
