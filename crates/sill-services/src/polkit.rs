//! `session.polkit_agent`: which polkit authentication agent a sill session starts. Until M11
//! sill had no agent of its own (design/20 §1.10: borrowed at M7); `sill polkit-agent` resolves
//! the setting with [`resolve`] and execs the result, so `dist/sill-session` can supervise it.
//! `sill` ([`AgentChoice::Own`], M11) is the daemon's own agent (`crate::polkit_agent`): there
//! is nothing to exec, and the daemon registers it itself.
//!
//! `auto` prefers a dedicated agent and takes cosmic-osd last: cosmic-osd is COSMIC's agent, but
//! it is also COSMIC's volume and brightness OSD (it shows its own indicator, with a sound, on
//! every sink volume change) and its logout dialogs, and sill's OSD replaces the first, so
//! running it only for polkit doubles every volume popup (sill FINDINGS "M7").

use std::path::PathBuf;

/// What `session.polkit_agent` says.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentChoice {
    Auto,
    Off,
    /// sill's own agent, in the daemon (`sill`).
    Own,
    Known(KnownAgent),
    /// A command line for `/bin/sh -c`.
    Command(String),
}

impl AgentChoice {
    /// The setting's text: `auto` (also empty), `off`, a known agent's name, or a command.
    pub fn parse(text: &str) -> AgentChoice {
        match text.trim() {
            "" | "auto" => AgentChoice::Auto,
            "off" | "none" => AgentChoice::Off,
            "sill" => AgentChoice::Own,
            other => KnownAgent::AUTO_ORDER
                .into_iter()
                .find(|known| known.name() == other)
                .map_or_else(
                    || AgentChoice::Command(other.to_owned()),
                    AgentChoice::Known,
                ),
        }
    }
}

/// An agent sill knows where to find.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnownAgent {
    PolkitKde,
    PolkitGnome,
    LxqtPolicykit,
    PolkitMate,
    CosmicOsd,
}

/// Where an agent may be installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Place {
    /// An absolute path (agents in libexec are not on `PATH`).
    At(&'static str),
    /// A program name looked up on `PATH`.
    OnPath(&'static str),
}

impl KnownAgent {
    /// `auto`'s order.
    pub const AUTO_ORDER: [KnownAgent; 5] = [
        KnownAgent::PolkitKde,
        KnownAgent::PolkitGnome,
        KnownAgent::LxqtPolicykit,
        KnownAgent::PolkitMate,
        KnownAgent::CosmicOsd,
    ];

    /// The setting's word.
    pub fn name(self) -> &'static str {
        match self {
            KnownAgent::PolkitKde => "polkit-kde-authentication-agent-1",
            KnownAgent::PolkitGnome => "polkit-gnome-authentication-agent-1",
            KnownAgent::LxqtPolicykit => "lxqt-policykit-agent",
            KnownAgent::PolkitMate => "polkit-mate-authentication-agent-1",
            KnownAgent::CosmicOsd => "cosmic-osd",
        }
    }

    /// Where distributions install it (Fedora, Debian/Ubuntu, Arch), first found wins.
    pub fn places(self) -> &'static [Place] {
        match self {
            KnownAgent::PolkitKde => &[
                Place::At("/usr/libexec/kf6/polkit-kde-authentication-agent-1"),
                Place::At("/usr/libexec/polkit-kde-authentication-agent-1"),
                Place::At("/usr/lib/x86_64-linux-gnu/libexec/polkit-kde-authentication-agent-1"),
                Place::At("/usr/lib/polkit-kde-authentication-agent-1"),
            ],
            KnownAgent::PolkitGnome => &[
                Place::At("/usr/libexec/polkit-gnome-authentication-agent-1"),
                Place::At("/usr/lib/polkit-gnome/polkit-gnome-authentication-agent-1"),
            ],
            KnownAgent::LxqtPolicykit => &[Place::OnPath("lxqt-policykit-agent")],
            KnownAgent::PolkitMate => &[
                Place::At("/usr/libexec/polkit-mate-authentication-agent-1"),
                Place::At("/usr/lib/mate-polkit/polkit-mate-authentication-agent-1"),
            ],
            KnownAgent::CosmicOsd => &[Place::OnPath("cosmic-osd")],
        }
    }
}

/// What to run.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentMode {
    Off,
    Run {
        program: PathBuf,
        args: Vec<String>,
    },
    /// The daemon's own agent: `sill polkit-agent` has nothing to start.
    Own,
}

/// `choice`, where `find` says whether (and where) a place holds a program.
pub fn resolve(choice: &AgentChoice, find: impl Fn(Place) -> Option<PathBuf>) -> AgentMode {
    let found = |known: KnownAgent| {
        known
            .places()
            .iter()
            .find_map(|place| find(*place))
            .map(|program| AgentMode::Run {
                program,
                args: Vec::new(),
            })
    };
    match choice {
        AgentChoice::Off => AgentMode::Off,
        AgentChoice::Own => AgentMode::Own,
        AgentChoice::Auto => KnownAgent::AUTO_ORDER
            .into_iter()
            .find_map(found)
            .unwrap_or(AgentMode::Off),
        AgentChoice::Known(known) => found(*known).unwrap_or(AgentMode::Off),
        AgentChoice::Command(command) => AgentMode::Run {
            program: PathBuf::from("/bin/sh"),
            args: vec!["-c".to_owned(), command.clone()],
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{AgentChoice, AgentMode, KnownAgent, Place, resolve};

    #[test]
    fn setting_words() {
        let cases = [
            ("", AgentChoice::Auto),
            ("auto", AgentChoice::Auto),
            ("off", AgentChoice::Off),
            ("sill", AgentChoice::Own),
            ("cosmic-osd", AgentChoice::Known(KnownAgent::CosmicOsd)),
            (
                "lxqt-policykit-agent",
                AgentChoice::Known(KnownAgent::LxqtPolicykit),
            ),
            (
                "polkit-kde-authentication-agent-1",
                AgentChoice::Known(KnownAgent::PolkitKde),
            ),
            (
                "/opt/agent --flag",
                AgentChoice::Command("/opt/agent --flag".to_owned()),
            ),
        ];
        for (text, want) in cases {
            assert_eq!(AgentChoice::parse(text), want, "{text:?}");
        }
    }

    const KDE_FEDORA: &str = "/usr/libexec/kf6/polkit-kde-authentication-agent-1";
    const GNOME_ARCH: &str = "/usr/lib/polkit-gnome/polkit-gnome-authentication-agent-1";

    /// A `find` over installed places: absolute paths as themselves, names at `/usr/bin`.
    fn installed(places: &'static [Place]) -> impl Fn(Place) -> Option<PathBuf> {
        move |place| {
            places.contains(&place).then(|| match place {
                Place::At(path) => PathBuf::from(path),
                Place::OnPath(name) => PathBuf::from(format!("/usr/bin/{name}")),
            })
        }
    }

    fn runs(path: &str) -> AgentMode {
        AgentMode::Run {
            program: PathBuf::from(path),
            args: Vec::new(),
        }
    }

    #[test]
    fn resolution_table() {
        const THIS_MACHINE: &[Place] = &[Place::At(KDE_FEDORA), Place::OnPath("cosmic-osd")];
        const GNOME_AND_OSD: &[Place] = &[Place::At(GNOME_ARCH), Place::OnPath("cosmic-osd")];
        const OSD_ONLY: &[Place] = &[Place::OnPath("cosmic-osd")];
        const LXQT: &[Place] = &[Place::OnPath("lxqt-policykit-agent")];
        const NOTHING: &[Place] = &[];
        let cases: Vec<(AgentChoice, &'static [Place], AgentMode)> = vec![
            (AgentChoice::Auto, THIS_MACHINE, runs(KDE_FEDORA)),
            (AgentChoice::Auto, GNOME_AND_OSD, runs(GNOME_ARCH)),
            (
                AgentChoice::Auto,
                LXQT,
                runs("/usr/bin/lxqt-policykit-agent"),
            ),
            (AgentChoice::Auto, OSD_ONLY, runs("/usr/bin/cosmic-osd")),
            (AgentChoice::Auto, NOTHING, AgentMode::Off),
            (AgentChoice::Off, THIS_MACHINE, AgentMode::Off),
            (AgentChoice::Own, NOTHING, AgentMode::Own),
            (
                AgentChoice::Known(KnownAgent::CosmicOsd),
                THIS_MACHINE,
                runs("/usr/bin/cosmic-osd"),
            ),
            (
                AgentChoice::Known(KnownAgent::PolkitGnome),
                THIS_MACHINE,
                AgentMode::Off,
            ),
            (
                AgentChoice::Command("my-agent".to_owned()),
                NOTHING,
                AgentMode::Run {
                    program: PathBuf::from("/bin/sh"),
                    args: vec!["-c".to_owned(), "my-agent".to_owned()],
                },
            ),
        ];
        for (choice, places, want) in cases {
            assert_eq!(
                resolve(&choice, installed(places)),
                want,
                "{choice:?} {places:?}"
            );
        }
    }
}
