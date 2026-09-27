//! The displays service task: commands in, [`Displays`] out. The DDC worker runs on a thread
//! of its own; output passes (a `cosmic-randr` round trip and a few sysfs reads) run on the
//! blocking pool; everything else here is quick.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;

use tokio::sync::{mpsc, watch};

use super::backlight;
use super::brightness::Backlight;
use super::ddc::{DdcDisplay, DdcJob, I2cBuses, Write, worker};
use super::drm;
use super::font::font_rendering_for;
use super::fontconfig_file;
use super::hw_volume::follow;
use super::kscreen::KscreenDoctor;
use super::model::{Displays, DisplaysCommand, DisplaysEvent, reduce_displays};
use super::output::{DisplayOutput, max_density};
use super::plan::{BrightnessAction, Originals, brightness_actions, warmth_writes};
use super::policy::DisplayPolicy;
use super::scale_pass::{HeadSource, scale_pass};
use super::seen;
use super::sink::CosmicRandr;
use crate::volume::Volume;

/// Where the service looks and writes.
#[derive(Debug, Clone)]
pub struct Paths {
    /// `/sys/class/drm`.
    pub drm: PathBuf,
    /// `/sys/class/backlight`.
    pub backlight: PathBuf,
    /// `displays.json`; `None` without a state directory (nothing is remembered).
    pub seen: Option<PathBuf>,
    /// `~/.config/fontconfig`; `None` without a config directory (no snippet).
    pub fontconfig: Option<PathBuf>,
}

impl Default for Paths {
    fn default() -> Self {
        Paths {
            drm: PathBuf::from("/sys/class/drm"),
            backlight: PathBuf::from("/sys/class/backlight"),
            seen: seen::default_path(),
            fontconfig: fontconfig_file::default_dir(),
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
    };
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
        let outputs_differ = self.policy.as_ref().is_none_or(|old| {
            (
                old.scale,
                &old.scale_overrides,
                old.density_target,
                old.font_rendering,
            ) != (
                policy.scale,
                &policy.scale_overrides,
                policy.density_target,
                policy.font_rendering,
            )
        });
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
        tokio::task::spawn_blocking(move || worker::run(I2cBuses::default(), rx, tx));
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

    /// The scale pass, the fontconfig snippet and the backlights, on the blocking pool.
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
                if let Err(error) = backlight::set(&name, value).await {
                    self.warn(format!("backlight {}: {error}", name.0));
                }
                let backlights = backlight::read_all(&self.paths.backlight);
                self.publish(DisplaysEvent::Backlights(backlights));
            }
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
            *displays = reduce_displays(std::mem::take(displays), event);
        });
    }

    /// Log `message` the first time it is seen.
    fn warn(&mut self, message: String) {
        if self.warned.insert(message.clone()) {
            eprintln!("sill: displays: {message}");
        }
    }
}

/// One outputs pass: describe, decide and set scales, remember them, keep the fontconfig
/// snippet in step with the densest display, and read the backlights. Outputs read from
/// `kscreen-doctor` (KWin) are only described: no scale, no record, and no fontconfig snippet,
/// since KDE's own scaling and font settings rule there (FINDINGS F980).
fn outputs_pass(
    paths: &Paths,
    policy: &DisplayPolicy,
) -> (Vec<DisplayOutput>, Vec<Backlight>, Vec<String>) {
    let connectors = drm::connected(&paths.drm);
    let before = paths.seen.as_deref().map(seen::load).unwrap_or_default();
    let pass = scale_pass(
        &mut CosmicRandr,
        &mut KscreenDoctor,
        &connectors,
        before.clone(),
        policy,
    );
    let mut failures: Vec<String> = pass
        .failures
        .iter()
        .map(|e| format!("scale: {e}"))
        .collect();
    let fontconfig = match pass.source {
        HeadSource::Reader => {
            failures.push(
                "outputs read from kscreen-doctor (no cosmic-randr answer): read-only, \
                 no scale policy, no fontconfig snippet"
                    .to_owned(),
            );
            None
        }
        HeadSource::Sink | HeadSource::Neither => paths.fontconfig.as_ref(),
    };
    if pass.seen != before
        && let Some(path) = &paths.seen
        && let Err(error) = seen::save(path, &pass.seen)
    {
        failures.push(format!("remembering displays: {error}"));
    }
    if let Some(dir) = fontconfig {
        let rendering = max_density(&pass.outputs).map(font_rendering_for);
        if let Err(error) = fontconfig_file::sync(dir, policy.font_rendering, rendering) {
            failures.push(format!("fontconfig: {error}"));
        }
    }
    (
        pass.outputs,
        backlight::read_all(&paths.backlight),
        failures,
    )
}
