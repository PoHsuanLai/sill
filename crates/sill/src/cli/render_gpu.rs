//! `sill session detect-gpu`: which GPU cosmic-comp should render on, for `dist/sill-session`
//! to hand it as `COSMIC_RENDER_DEVICE`.
//!
//! On a machine with several GPUs cosmic-comp renders on the boot GPU unless told otherwise, and
//! copies every frame to the GPU that scans the monitor out. Rendering on the GPU the monitor is
//! plugged into avoids that copy. This reads the kernel's DRM tree (`/sys/class/drm`) and answers
//! only when it is sure: more than one GPU can render, and exactly one of them has a connected
//! output. Otherwise it prints nothing and cosmic-comp chooses as it always did. The answer is
//! written the way cosmic-comp parses it: `pci-<address>` (which survives two identical cards and
//! renderD renumbering), or the render node's name for a GPU that is not on PCI.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Subcommand;

use super::{import_input, stdout};

/// `sill session ...`: helpers for the login script (`dist/sill-session`).
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum SessionAction {
    /// Print the device to render on, if one is clearly right (nothing otherwise); why goes to
    /// stderr.
    DetectGpu {
        /// The sysfs root to read (tests point it at a made-up tree).
        #[arg(long, default_value = "/sys")]
        sysfs: PathBuf,
    },
    /// Copy the per-device mouse and touchpad settings from Plasma's `kcminputrc` into COSMIC's
    /// `input_devices` (`~/.config/cosmic/com.system76.CosmicComp/v1/input_devices`), keyed by
    /// device name. Writes only when that file does not exist; otherwise it prints what a merge
    /// would add and exits 2. Natural scroll, scroll factor and pointer acceleration are carried.
    /// Note: cosmic-comp does not apply `scroll_factor` yet, so a scroll factor is saved but has
    /// no effect until it does.
    ImportInput(import_input::Args),
}

/// The value cosmic-comp's `COSMIC_RENDER_DEVICE` takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceName(pub String);

/// One GPU that can render, and how many outputs it has connected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gpu {
    /// `card1`.
    pub card: String,
    pub device: DeviceName,
    pub connected: usize,
}

/// What the scan comes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Render on this one: the only GPU with a connected output.
    Pin(Gpu),
    /// Fewer than two GPUs can render: nothing to choose.
    Alone,
    /// No GPU has a connected output yet.
    NoDisplay,
    /// Outputs are connected to more than one GPU (the cards named).
    Split(Vec<String>),
}

impl Verdict {
    /// The line for the journal.
    pub fn reason(&self) -> String {
        match self {
            Verdict::Pin(gpu) => format!(
                "render device {} ({}: the only GPU with a connected output)",
                gpu.device.0, gpu.card
            ),
            Verdict::Alone => "render device left to cosmic-comp (fewer than two GPUs)".to_owned(),
            Verdict::NoDisplay => {
                "render device left to cosmic-comp (no connected output seen)".to_owned()
            }
            Verdict::Split(cards) => format!(
                "render device left to cosmic-comp (outputs on several GPUs: {})",
                cards.join(", ")
            ),
        }
    }

    /// What to print on stdout, if anything.
    pub fn device(&self) -> Option<&DeviceName> {
        match self {
            Verdict::Pin(gpu) => Some(&gpu.device),
            Verdict::Alone | Verdict::NoDisplay | Verdict::Split(_) => None,
        }
    }
}

/// Pure: the verdict over the GPUs found.
pub fn decide(gpus: &[Gpu]) -> Verdict {
    if gpus.len() < 2 {
        return Verdict::Alone;
    }
    let mut lit = gpus.iter().filter(|gpu| gpu.connected > 0);
    match (lit.next(), lit.next()) {
        (None, _) => Verdict::NoDisplay,
        (Some(only), None) => Verdict::Pin(only.clone()),
        (Some(first), Some(second)) => Verdict::Split(
            [first, second]
                .into_iter()
                .chain(lit)
                .map(|gpu| gpu.card.clone())
                .collect(),
        ),
    }
}

/// The GPUs under `sysfs/class/drm`: every `cardN` with a render node, in name order.
pub fn scan(sysfs: &Path) -> Vec<Gpu> {
    let drm = sysfs.join("class/drm");
    let mut names: Vec<String> = fs::read_dir(&drm)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .collect();
    names.sort();
    names
        .iter()
        .filter(|name| is_card(name))
        .filter_map(|card| gpu_of(&drm, card, &names))
        .collect()
}

/// `card` followed by digits only: a connector (`card1-HDMI-A-1`) is not one.
fn is_card(name: &str) -> bool {
    name.strip_prefix("card")
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
}

fn gpu_of(drm: &Path, card: &str, names: &[String]) -> Option<Gpu> {
    let device_dir = drm.join(card).join("device");
    let render = fs::read_dir(device_dir.join("drm"))
        .ok()?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .find(|name| name.starts_with("renderD"))?;
    let address = fs::canonicalize(&device_dir)
        .ok()
        .and_then(|path| path.file_name().map(|n| n.to_string_lossy().into_owned()))
        .filter(|name| is_pci_address(name));
    let device = DeviceName(address.map_or(render, |address| format!("pci-{address}")));
    let prefix = format!("{card}-");
    let connected = names
        .iter()
        .filter(|name| name.starts_with(&prefix))
        .filter(|name| is_connected(&drm.join(name.as_str()).join("status")))
        .count();
    Some(Gpu {
        card: card.to_owned(),
        device,
        connected,
    })
}

fn is_connected(status: &Path) -> bool {
    fs::read_to_string(status).is_ok_and(|text| text.trim() == "connected")
}

/// `0000:10:00.0`: domain, bus, device and function in hex.
fn is_pci_address(name: &str) -> bool {
    let hex =
        |part: &str, len: usize| part.len() == len && part.chars().all(|c| c.is_ascii_hexdigit());
    let mut parts = name.split([':', '.']);
    let fields = [parts.next(), parts.next(), parts.next(), parts.next()];
    let lens = [4, 2, 2, 1];
    parts.next().is_none()
        && fields
            .iter()
            .zip(lens)
            .all(|(part, len)| part.is_some_and(|p| hex(p, len)))
}

/// Run `action`: the device on stdout (nothing when there is no clear choice), the reason on
/// stderr. Exits 0 whatever the verdict, and when the reader of stdout is gone: the session
/// starts either way.
pub fn main(action: &SessionAction) -> ExitCode {
    match action {
        SessionAction::ImportInput(args) => import_input::main(args),
        SessionAction::DetectGpu { sysfs } => {
            let verdict = decide(&scan(sysfs));
            eprintln!("sill: {}", verdict.reason());
            match verdict.device() {
                Some(device) => stdout::print(&format!("{}\n", device.0)).exit_code(),
                None => ExitCode::SUCCESS,
            }
        }
    }
}

#[cfg(test)]
#[path = "render_gpu_tests.rs"]
mod tests;
