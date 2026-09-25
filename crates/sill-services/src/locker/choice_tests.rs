use std::path::PathBuf;

use super::{
    Host, KnownLocker, LockerChoice, LockerMode, LockerPlan, Readiness, SessionManager, resolve,
};
use crate::locker::machine::Triggers;

#[test]
fn setting_words() {
    let cases = [
        ("", LockerChoice::Auto),
        ("auto", LockerChoice::Auto),
        (" auto ", LockerChoice::Auto),
        ("off", LockerChoice::Off),
        ("none", LockerChoice::Off),
        (
            "cosmic-greeter",
            LockerChoice::Known(KnownLocker::CosmicGreeter),
        ),
        ("swaylock", LockerChoice::Known(KnownLocker::Swaylock)),
        ("hyprlock", LockerChoice::Known(KnownLocker::Hyprlock)),
        (
            "swaylock -c 000000",
            LockerChoice::Command("swaylock -c 000000".to_owned()),
        ),
    ];
    for (text, want) in cases {
        assert_eq!(LockerChoice::parse(text), want, "{text:?}");
    }
}

#[test]
fn kinds_of_session() {
    let cases = [
        (SessionManager::CosmicSession, "COSMIC", Host::CosmicSession),
        (SessionManager::CosmicSession, "KDE", Host::CosmicSession),
        (SessionManager::Other, "COSMIC", Host::SillSession),
        (SessionManager::Other, "sill:COSMIC", Host::SillSession),
        (SessionManager::Other, "cosmic", Host::SillSession),
        (SessionManager::Other, "KDE", Host::Guest),
        (SessionManager::Other, "", Host::Guest),
    ];
    for (manager, desktop, want) in cases {
        assert_eq!(Host::of(manager, desktop), want, "{manager:?} {desktop:?}");
    }
}

/// A `find` over a fixed set of installed program names, each at `/usr/bin/<name>`.
fn installed(names: &'static [&'static str]) -> impl Fn(&str) -> Option<PathBuf> {
    move |name| {
        names
            .contains(&name)
            .then(|| PathBuf::from(format!("/usr/bin/{name}")))
    }
}

fn run(known: KnownLocker, triggers: Triggers) -> LockerMode {
    LockerMode::Run {
        plan: known.plan(PathBuf::from(format!("/usr/bin/{}", known.program()))),
        triggers,
    }
}

#[test]
fn resolution_table() {
    use KnownLocker::{CosmicGreeter, Hyprlock, Swaylock};
    use Triggers::{FollowLogind, IgnoreLogind};
    const ALL: &[&str] = &["cosmic-greeter", "swaylock", "hyprlock"];
    const SWAY_HYPR: &[&str] = &["swaylock", "hyprlock"];
    const HYPR: &[&str] = &["hyprlock"];
    const NONE: &[&str] = &[];
    let cases: Vec<(LockerChoice, Host, &'static [&'static str], LockerMode)> = vec![
        (
            LockerChoice::Auto,
            Host::SillSession,
            ALL,
            run(CosmicGreeter, FollowLogind),
        ),
        (
            LockerChoice::Auto,
            Host::SillSession,
            SWAY_HYPR,
            run(Swaylock, FollowLogind),
        ),
        (
            LockerChoice::Auto,
            Host::SillSession,
            HYPR,
            run(Hyprlock, FollowLogind),
        ),
        (LockerChoice::Auto, Host::SillSession, NONE, LockerMode::Off),
        (
            LockerChoice::Auto,
            Host::Guest,
            ALL,
            run(CosmicGreeter, IgnoreLogind),
        ),
        (
            LockerChoice::Auto,
            Host::CosmicSession,
            ALL,
            LockerMode::Delegate,
        ),
        (
            LockerChoice::Known(CosmicGreeter),
            Host::CosmicSession,
            ALL,
            LockerMode::Delegate,
        ),
        (LockerChoice::Off, Host::SillSession, ALL, LockerMode::Off),
        (
            LockerChoice::Known(Swaylock),
            Host::SillSession,
            ALL,
            run(Swaylock, FollowLogind),
        ),
        (
            LockerChoice::Known(Swaylock),
            Host::SillSession,
            HYPR,
            LockerMode::Off,
        ),
        (
            LockerChoice::Known(Hyprlock),
            Host::CosmicSession,
            ALL,
            run(Hyprlock, IgnoreLogind),
        ),
        (
            LockerChoice::Command("mylock --now".to_owned()),
            Host::SillSession,
            NONE,
            LockerMode::Run {
                plan: LockerPlan::shell("mylock --now"),
                triggers: FollowLogind,
            },
        ),
    ];
    for (choice, host, names, want) in cases {
        assert_eq!(
            resolve(&choice, host, installed(names)),
            want,
            "{choice:?} {host:?} {names:?}"
        );
    }
}

#[test]
fn each_known_locker_has_a_ready_signal() {
    let cases = [
        (
            KnownLocker::CosmicGreeter,
            Readiness::Journal("session locked".to_owned()),
        ),
        (KnownLocker::Swaylock, Readiness::StdoutLine),
        (KnownLocker::Hyprlock, Readiness::Grace(super::GRACE_MS)),
    ];
    for (known, want) in cases {
        let plan = known.plan(PathBuf::from("/x"));
        assert_eq!(plan.readiness, want, "{known:?}");
        assert_eq!(plan.process_name.as_deref(), Some(known.program()));
    }
    let greeter = KnownLocker::CosmicGreeter.plan(PathBuf::from("/x"));
    assert!(
        greeter
            .env
            .iter()
            .any(|(k, v)| k == "DBUS_SYSTEM_BUS_ADDRESS" && v == super::NO_SYSTEM_BUS),
        "cosmic-greeter runs one-shot only without a system bus"
    );
}
