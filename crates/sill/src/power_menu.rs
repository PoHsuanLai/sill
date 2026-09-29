//! The daemon's power menu half: one power menu surface per output, spawned warm and hidden (the
//! launcher's pattern), and the `power-menu` and `control-center` requests, which only write the
//! shared links the surfaces follow ([`crate::links`]).
//!
//! **Dry run.** A debug build (`--features debug`) started with `SILL_POWER_MENU_DRY_RUN` set
//! gives every power menu a [`DryRunSession`] instead of the session service: every button is
//! offered, and a chosen action is only printed (`power_menu.dry_run would=Restart`) and appended
//! to the file the variable names (none when it is empty), never sent to logind. Each surface
//! prints `power_menu.backend=dry-run` as it mounts, which `dev/accept-power-menu.sh` checks
//! before it presses Enter. With `SILL_POWER_MENU_DRY_RUN_OFFERS=logind` too, the dry run offers
//! what logind offers (the session service's `Can*` answers, read only) instead of everything,
//! so an action this machine refuses is drawn disabled; a choice is still only recorded. A
//! release build ignores both variables.
//!
//! **Scripted keys.** On the nested cosmic-comp the power menu gets no keyboard focus, and
//! shell-host injects no keys, so a debug build started with `SILL_POWER_MENU_KEYS` naming a FIFO
//! also feeds each line written to it (`right`, `enter`, ...) to every power menu as a key. Only
//! alongside the dry run: without it the variable is refused, so a scripted Enter can never reach
//! logind.

use shell_host::{HostCtx, OutputInfo, Root};
use sill_ipc::{ControlCenterRequest, PowerMenuRequest, Response};
use sill_services::Session;
use sill_surfaces::surfaces::power_menu::{
    PowerMenuBackend, PowerMenuProps, PowerMenuScript, Preselect, power_menu_layer,
};
use tokio::sync::watch;

use crate::links::Links;
use crate::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn a hidden power menu for every output whose row has none yet, each root built by `root`
/// (the daemon's `surface_root`, which adds the links). `offers` is the session service's, which
/// a dry run may follow.
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    offers: &watch::Receiver<Session>,
    root: impl Fn(Root) -> Root,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, surfaces)| surfaces.power_menu.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    for info in missing {
        let config = power_menu_layer(&info);
        let namespace = config.namespace.clone();
        let props = PowerMenuProps {
            output: info.clone(),
        };
        let surface = debug_contexts(
            root(Root::with_props(sill_surfaces::PowerMenu, props)),
            offers,
        );
        match ctx.spawn_layer(config, surface) {
            Ok(id) => {
                ctx.set_visibility(id, shell_host::Visibility::Hidden);
                let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
                surfaces.power_menu = Some(id);
                outputs.set(info, surfaces);
            }
            Err(error) => eprintln!("sill: spawning {namespace} on {:?}: {error}", info.name),
        }
    }
}

/// `surface` with the dry run and the scripted keys, when this debug build has them.
fn debug_contexts(surface: Root, offers: &watch::Receiver<Session>) -> Root {
    let surface = match dry_run(offers) {
        Some(backend) => surface.context(backend),
        None => surface,
    };
    match key_script(offers) {
        Some(script) => surface.context(script),
        None => surface,
    }
}

/// The environment variable that turns the dry run on (a debug build only).
pub const DRY_RUN_VAR: &str = "SILL_POWER_MENU_DRY_RUN";

/// The environment variable that makes the dry run offer logind's answers (`logind`) rather than
/// everything (a debug build, with the dry run).
pub const DRY_RUN_OFFERS_VAR: &str = "SILL_POWER_MENU_DRY_RUN_OFFERS";

/// The dry-run backend every power menu shares, when this debug build was started with
/// [`DRY_RUN_VAR`]; `offers` is the session service's, which it follows under
/// [`DRY_RUN_OFFERS_VAR`].
#[cfg(feature = "debug")]
fn dry_run(offers: &watch::Receiver<Session>) -> Option<PowerMenuBackend> {
    use sill_surfaces::surfaces::power_menu::{DryRunSession, everything_offered};
    static DRY_RUN: std::sync::OnceLock<Option<PowerMenuBackend>> = std::sync::OnceLock::new();
    DRY_RUN
        .get_or_init(|| {
            let value = std::env::var_os(DRY_RUN_VAR)?;
            let record = (!value.is_empty()).then(|| std::path::PathBuf::from(value));
            let from = DryRunOffers::from_env(std::env::var_os(DRY_RUN_OFFERS_VAR));
            eprintln!(
                "sill: power menu: dry run, recording to {}, offering {}",
                record
                    .as_ref()
                    .map_or("stderr only".to_owned(), |path| path.display().to_string()),
                from.name()
            );
            let session = match from {
                DryRunOffers::Logind => DryRunSession::following(offers.clone(), record),
                DryRunOffers::Everything => DryRunSession::new(everything_offered(), record),
            };
            Some(PowerMenuBackend::dry_run(session))
        })
        .clone()
}

/// What a dry run offers: every action, or logind's answers ([`DRY_RUN_OFFERS_VAR`]).
#[cfg(feature = "debug")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DryRunOffers {
    Everything,
    Logind,
}

#[cfg(feature = "debug")]
impl DryRunOffers {
    /// The variable's value: `logind`, else everything.
    fn from_env(value: Option<std::ffi::OsString>) -> DryRunOffers {
        match value {
            Some(value) if value == "logind" => DryRunOffers::Logind,
            _ => DryRunOffers::Everything,
        }
    }

    /// For the dry run's log line.
    fn name(self) -> &'static str {
        match self {
            DryRunOffers::Everything => "everything",
            DryRunOffers::Logind => "logind's answers",
        }
    }
}

/// Without the `debug` feature there is no dry run: the power menu speaks to logind.
#[cfg(not(feature = "debug"))]
fn dry_run(_offers: &watch::Receiver<Session>) -> Option<PowerMenuBackend> {
    None
}

/// The environment variable naming the scripted keys' FIFO (a debug build, with the dry run).
pub const KEYS_VAR: &str = "SILL_POWER_MENU_KEYS";

/// The scripted keys every power menu shares, when this debug build was started with
/// [`KEYS_VAR`] and the dry run.
#[cfg(feature = "debug")]
fn key_script(offers: &watch::Receiver<Session>) -> Option<PowerMenuScript> {
    use sill_surfaces::surfaces::power_menu::ScriptedKey;
    static SCRIPT: std::sync::OnceLock<Option<PowerMenuScript>> = std::sync::OnceLock::new();
    SCRIPT
        .get_or_init(|| {
            let path = std::path::PathBuf::from(std::env::var_os(KEYS_VAR)?);
            if dry_run(offers).is_none() {
                eprintln!(
                    "sill: power menu: {KEYS_VAR} refused without {DRY_RUN_VAR}: scripted keys \
                     only ever drive a dry run"
                );
                return None;
            }
            let (tx, rx) = tokio::sync::watch::channel(ScriptedKey::default());
            eprintln!("sill: power menu: scripted keys from {}", path.display());
            let spawned = std::thread::Builder::new()
                .name("power-menu-keys".to_owned())
                .spawn(move || read_keys(&path, &tx));
            if let Err(error) = spawned {
                eprintln!("sill: power menu: no key script: {error}");
            }
            Some(PowerMenuScript(rx))
        })
        .clone()
}

/// Read `path` a line at a time, forever (a FIFO ends at each writer's close and is opened
/// again), sending each key it names.
#[cfg(feature = "debug")]
fn read_keys(
    path: &std::path::Path,
    tx: &tokio::sync::watch::Sender<sill_surfaces::surfaces::power_menu::ScriptedKey>,
) {
    use sill_surfaces::surfaces::power_menu::script_key;
    use std::io::BufRead;
    loop {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) => {
                eprintln!("sill: power menu: key script {}: {error}", path.display());
                return;
            }
        };
        for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
            match script_key(&line) {
                Some(key) => tx.send_modify(|scripted| {
                    scripted.count += 1;
                    scripted.key = Some(key);
                }),
                None => eprintln!("sill: power menu: no key {line:?}"),
            }
        }
    }
}

/// Without the `debug` feature there are no scripted keys.
#[cfg(not(feature = "debug"))]
fn key_script(_offers: &watch::Receiver<Session>) -> Option<PowerMenuScript> {
    None
}

/// `sill power-menu open|close|toggle`: on the first output with a power menu (the output under
/// the pointer is not known to the daemon).
pub fn dispatch_power_menu(
    outputs: &Outputs,
    links: Option<&Links>,
    request: PowerMenuRequest,
) -> Response {
    let Some(links) = links else {
        return unsupported("the power menu's link is not installed");
    };
    let target = outputs
        .outputs
        .iter()
        .find(|(_, s)| s.power_menu.is_some())
        .map(|(info, _)| info.id);
    match (request, target) {
        (PowerMenuRequest::Close, _) => links.power_menu.close(),
        (PowerMenuRequest::Open, Some(output)) => links.power_menu.open(output, Preselect::Default),
        (PowerMenuRequest::Toggle, Some(output)) => links.power_menu.toggle(output),
        (PowerMenuRequest::Open | PowerMenuRequest::Toggle, None) => {
            return unsupported("no output has a power menu surface");
        }
    }
    Response::Done
}

/// `sill control-center toggle`: asks the first output's bar.
pub fn dispatch_control_center(
    outputs: &Outputs,
    links: Option<&Links>,
    request: ControlCenterRequest,
) -> Response {
    let Some(links) = links else {
        return unsupported("the control center's link is not installed");
    };
    let target = outputs
        .outputs
        .iter()
        .find(|(_, s)| s.bar.is_some())
        .map(|(info, _)| info.id);
    match (request, target) {
        (ControlCenterRequest::Toggle, Some(output)) => links.control_center.toggle(output),
        (ControlCenterRequest::Toggle, None) => return unsupported("no output has a bar"),
    }
    Response::Done
}

fn unsupported(message: &str) -> Response {
    Response::Failed(sill_ipc::Failure {
        kind: sill_ipc::FailureKind::Unsupported,
        message: message.to_owned(),
    })
}

#[cfg(all(test, feature = "debug"))]
mod tests {
    use super::DryRunOffers;
    use std::ffi::OsString;

    #[test]
    fn only_logind_makes_the_dry_run_follow_logind() {
        let cases = [
            (Some("logind"), DryRunOffers::Logind),
            (Some("everything"), DryRunOffers::Everything),
            (Some(""), DryRunOffers::Everything),
            (None, DryRunOffers::Everything),
        ];
        for (value, want) in cases {
            assert_eq!(
                DryRunOffers::from_env(value.map(OsString::from)),
                want,
                "{value:?}"
            );
        }
    }
}
