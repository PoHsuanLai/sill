//! `IdleCommand::Configure` against this session and this machine (blocking: `/proc` and a few
//! small files): who owns idle ([`resolve_mode`]) and, for `TimesChoice::Cosmic`, COSMIC's
//! times ([`cosmic_times`]) from the user's `$XDG_CONFIG_HOME/cosmic`, else
//! `/usr/share/cosmic`. The live loop repeats it every [`RESOLVE_EVERY_S`] seconds, so a
//! cosmic-idle started by hand, or a time changed on COSMIC Settings' Power page, is picked up
//! within that.

use std::path::{Path, PathBuf};

use crate::locker::{Host, SessionManager};
use crate::programs;

use super::cosmic::{
    COSMIC_IDLE_ID, COSMIC_IDLE_VERSION, CosmicIdle, CosmicIdleTimes, CosmicKey, cosmic_times,
};
use super::policy::{IdleConfig, IdlePolicy, TimesChoice, effective_times, resolve_mode};

/// How often a `Configure`d policy is resolved again.
pub(super) const RESOLVE_EVERY_S: u64 = 15;

/// `config` in this session.
pub(super) fn resolve(config: &IdleConfig, config_dir: &Path) -> IdlePolicy {
    let manager = programs::running("cosmic-session")
        .first()
        .map_or(SessionManager::Other, |_| SessionManager::CosmicSession);
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let cosmic_idle = programs::running("cosmic-idle")
        .first()
        .map_or(CosmicIdle::Absent, |_| CosmicIdle::Running);
    let cosmic = match config.times {
        TimesChoice::Cosmic => read_cosmic(config_dir),
        TimesChoice::Own(_) => cosmic_times(|_| None),
    };
    IdlePolicy {
        mode: resolve_mode(Host::of(manager, &desktop), cosmic_idle),
        times: effective_times(config.times, cosmic),
        dim: config.dim,
        lock: config.lock,
        locked_off: config.locked_off,
        grace: config.grace,
    }
}

/// COSMIC's three times from the user's directory under `config_dir`, else the system's.
fn read_cosmic(config_dir: &Path) -> CosmicIdleTimes {
    let dirs = [
        config_dir.join("cosmic"),
        PathBuf::from("/usr/share/cosmic"),
    ];
    cosmic_times(|key: CosmicKey| {
        dirs.iter().find_map(|dir| {
            let path = dir
                .join(COSMIC_IDLE_ID)
                .join(COSMIC_IDLE_VERSION)
                .join(key.file_name());
            std::fs::read_to_string(path).ok()
        })
    })
}
