//! `sill run`'s options and the [`Env`] they make. A build without the `debug` feature takes
//! none and always runs on the process's own seams ([`Env::from_process`]); a `debug` build's
//! dev scripts swap single seams for stand-ins with flags, never with environment variables.

use sill_services::env::Env;

/// `sill run`'s options.
#[derive(Debug, Clone, PartialEq, Eq, Default, clap::Args)]
pub struct RunArgs {
    #[cfg(feature = "debug")]
    #[command(flatten)]
    pub debug: DebugSeams,
}

/// The stand-ins a `debug` build's dev scripts can ask for.
#[cfg(feature = "debug")]
#[derive(Debug, Clone, PartialEq, Eq, Default, clap::Args)]
pub struct DebugSeams {
    /// Talk DDC/CI to the made-up monitors in this JSON file instead of `/dev/i2c-*`
    /// (`sill_services::env::ddc::FileBus`), so no real monitor is touched.
    #[arg(long, value_name = "FILE")]
    pub fake_ddc: Option<std::path::PathBuf>,
    /// Idle's screen-off and suspend times in seconds for both power sources (`<screen_off_s>,
    /// <suspend_s>`, 0 never) instead of the settings' minutes.
    #[arg(long, value_name = "OFF_S,SUSPEND_S", value_parser = idle_times)]
    pub idle_times: Option<sill_model::idle::IdleTimes>,
    /// Register the polkit agent with a stand-in Authority on this private bus instead of
    /// polkitd on the system bus. Its helper is then `--polkit-helper` (none without it: every
    /// request fails, never polkit's own helper) and its session `--polkit-session`; logind is
    /// never asked.
    #[arg(long, value_name = "ADDRESS")]
    pub polkit_bus: Option<String>,
    /// The stand-in polkit helper program (with `--polkit-bus`).
    #[arg(long, value_name = "PROGRAM", requires = "polkit_bus")]
    pub polkit_helper: Option<std::path::PathBuf>,
    /// The session id the agent registers for (with `--polkit-bus`; default `sill-test`).
    #[arg(long, value_name = "ID", requires = "polkit_bus")]
    pub polkit_session: Option<String>,
    /// Carry out no session action (power menu, launcher rows): each chosen one is only printed
    /// (`session.dry_run would=<Action>`) and appended to RECORD.
    #[arg(long, value_name = "RECORD")]
    pub session_dry_run: Option<std::path::PathBuf>,
    /// What the dry run offers: every action, or logind's answers (read only).
    #[arg(long, value_enum, default_value_t = DryRunOffersArg::Everything, requires = "session_dry_run")]
    pub session_dry_run_offers: DryRunOffersArg,
    /// Callers named in this file (the person's own, mode 600 or 644; peephole's `[[peer]]` rows)
    /// are let in as the services they stand for, at a standing below every real proof: a stand-in
    /// built in a checkout (`accountd_standin`, docket's `accept-intentd`) is the accounts service
    /// or the router for this daemon only. The desktop's table is read as usual and wins.
    #[arg(long, value_name = "FILE")]
    pub dev_callers: Option<std::path::PathBuf>,
    /// Name callers by this stand-in `/proc` (`<DIR>/<pid>/cgroup`, `<DIR>/units/<unit>` for a
    /// unit's main process) instead of the system's and its systemd manager: a jail has neither,
    /// so a test names intentd here as docket's own harness does (`INTENTD_PROC_ROOT`).
    #[arg(long, value_name = "DIR")]
    pub proc_root: Option<std::path::PathBuf>,
}

/// `--session-dry-run-offers`' words.
#[cfg(feature = "debug")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum DryRunOffersArg {
    #[default]
    Everything,
    Logind,
}

/// `--idle-times`' value.
#[cfg(feature = "debug")]
fn idle_times(text: &str) -> Result<sill_model::idle::IdleTimes, String> {
    crate::policy::idle::test_times(text)
        .ok_or_else(|| format!("{text:?} is not <screen_off_s>,<suspend_s>"))
}

/// The desktop's callers with the developer rows in `path` added after them. A file that is not
/// the person's own plain file, or does not parse, adds nothing (one line says why): the stand-ins
/// are then refused like any stranger.
#[cfg(feature = "debug")]
fn dev_callers(path: &std::path::Path) -> std::sync::Arc<dyn sill_services::callers::CallerSource> {
    use sill_services::callers::DesktopCallers;

    let shipped = DesktopCallers::from_system();
    match shipped.clone().with_dev_rows(path) {
        Ok(callers) => {
            eprintln!(
                "sill: callers: stand-ins named in {} are let in below every real proof (--dev-callers)",
                path.display()
            );
            std::sync::Arc::new(callers)
        }
        Err(error) => {
            eprintln!(
                "sill: callers: --dev-callers {} adds nothing ({error})",
                path.display()
            );
            std::sync::Arc::new(shipped)
        }
    }
}

impl RunArgs {
    /// The daemon's seams: the process's own, with this build's stand-ins in place.
    pub fn env(&self) -> Env {
        #[cfg(feature = "debug")]
        {
            self.debug.apply(Env::from_process())
        }
        #[cfg(not(feature = "debug"))]
        {
            Env::from_process()
        }
    }
}

#[cfg(feature = "debug")]
impl DebugSeams {
    /// `env` with every stand-in asked for.
    fn apply(&self, env: Env) -> Env {
        use std::sync::Arc;

        use sill_model::polkit_agent::HelperRoute;
        use sill_services::callers::CallerSource;
        use sill_services::env::ddc::FileBus;
        use sill_services::env::{
            Bus, BusRoute, DryRunActions, DryRunOffers, IdleTimesSource, SessionActions,
        };
        use sill_services::polkit_agent::{self, HelperChoice, SessionChoice};

        let ddc: Arc<dyn sill_services::env::ddc::DdcBus> = match &self.fake_ddc {
            Some(path) => {
                eprintln!(
                    "sill: displays: DDC is the fake in {} (--fake-ddc): no real monitor is touched",
                    path.display()
                );
                Arc::new(FileBus::new(path.clone()))
            }
            None => env.ddc,
        };
        let idle_times = match self.idle_times {
            Some(times) => IdleTimesSource::Fixed(times),
            None => env.idle_times,
        };
        let polkit = match &self.polkit_bus {
            Some(address) => polkit_agent::PolkitRoute {
                bus: Bus::system(BusRoute::Address(address.clone())),
                helper: HelperChoice::Use(
                    self.polkit_helper
                        .clone()
                        .map_or(HelperRoute::Missing, HelperRoute::Setuid),
                ),
                session: SessionChoice::Id(
                    self.polkit_session
                        .clone()
                        .unwrap_or_else(|| "sill-test".to_owned()),
                ),
            },
            None => env.polkit,
        };
        let session_actions: Arc<dyn SessionActions> = match &self.session_dry_run {
            Some(record) => {
                let offers = match self.session_dry_run_offers {
                    DryRunOffersArg::Everything => DryRunOffers::Everything,
                    DryRunOffersArg::Logind => DryRunOffers::Logind,
                };
                eprintln!(
                    "sill: session: dry run, recording to {}, offering {offers:?}",
                    record.display()
                );
                Arc::new(DryRunActions::new(offers, Some(record.clone())))
            }
            None => env.session_actions,
        };
        let callers: Arc<dyn CallerSource> = match &self.dev_callers {
            Some(path) => dev_callers(path),
            None => env.callers,
        };
        let proc_root = match &self.proc_root {
            Some(dir) => {
                eprintln!(
                    "sill: callers: named by the stand-in proc root {} (--proc-root)",
                    dir.display()
                );
                dir.clone()
            }
            None => env.proc_root,
        };
        Env {
            ddc,
            idle_times,
            polkit,
            session_actions,
            callers,
            proc_root,
            ..env
        }
    }
}

/// A release-shaped build has no `--dev-callers` at all, nor the stand-in flags beside it.
#[cfg(all(test, not(feature = "debug")))]
mod release_tests {
    use clap::Parser;

    use crate::cli::Cli;

    #[test]
    fn run_has_no_dev_callers_flag() {
        assert!(Cli::try_parse_from(["sill", "run"]).is_ok());
        assert!(Cli::try_parse_from(["sill", "run", "--dev-callers", "/tmp/rows.toml"]).is_err());
    }
}

#[cfg(all(test, feature = "debug"))]
mod tests {
    use clap::Parser;

    use crate::cli::{Cli, Command};

    #[test]
    fn run_takes_idle_times_in_seconds_and_refuses_anything_else() {
        let parse = |value: &str| Cli::try_parse_from(["sill", "run", "--idle-times", value]);
        let Ok(Cli {
            command: Some(Command::Run(args)),
            ..
        }) = parse("6,10")
        else {
            panic!("6,10 parses");
        };
        assert_eq!(
            args.debug.idle_times,
            crate::policy::idle::test_times("6,10")
        );
        assert!(parse("6").is_err());
    }

    /// Only a named bus leaves polkitd, and never for polkit's own helper.
    #[test]
    fn a_polkit_bus_brings_a_stand_in_helper_or_none() {
        use sill_model::polkit_agent::HelperRoute;
        use sill_services::env::{Bus, BusRoute};
        use sill_services::polkit_agent::{HelperChoice, PolkitRoute, SessionChoice};

        let setup = |args: &[&str]| {
            let cli = Cli::try_parse_from([&["sill", "run"], args].concat())
                .unwrap_or_else(|e| panic!("{e}"));
            let Some(Command::Run(run)) = cli.command else {
                panic!("not run");
            };
            run.env().polkit
        };
        let bus = || Bus::system(BusRoute::Address("unix:path=/tmp/b".to_owned()));
        let cases = [
            (vec![], PolkitRoute::system(Bus::system(BusRoute::Usual))),
            (
                vec!["--polkit-bus", "unix:path=/tmp/b"],
                PolkitRoute {
                    bus: bus(),
                    helper: HelperChoice::Use(HelperRoute::Missing),
                    session: SessionChoice::Id("sill-test".to_owned()),
                },
            ),
            (
                vec![
                    "--polkit-bus",
                    "unix:path=/tmp/b",
                    "--polkit-helper",
                    "/x/helper",
                    "--polkit-session",
                    "c7",
                ],
                PolkitRoute {
                    bus: bus(),
                    helper: HelperChoice::Use(HelperRoute::Setuid("/x/helper".into())),
                    session: SessionChoice::Id("c7".to_owned()),
                },
            ),
        ];
        for (args, want) in cases {
            assert_eq!(setup(&args), want, "{args:?}");
        }
        assert!(Cli::try_parse_from(["sill", "run", "--polkit-helper", "/x/helper"]).is_err());
    }

    /// The dry run's offers only make sense with the dry run.
    #[test]
    fn dry_run_offers_need_a_session_dry_run() {
        let parse = |args: &[&str]| Cli::try_parse_from([&["sill", "run"], args].concat());
        assert!(parse(&["--session-dry-run-offers", "logind"]).is_err());
        let Ok(Cli {
            command: Some(Command::Run(args)),
            ..
        }) = parse(&[
            "--session-dry-run",
            "/tmp/record",
            "--session-dry-run-offers",
            "logind",
        ])
        else {
            panic!("a dry run with offers parses");
        };
        assert_eq!(
            args.env().session_actions.reach(),
            sill_services::env::ActionReach::DryRun
        );
    }

    #[test]
    fn run_takes_its_stand_in_files() {
        type Pick = fn(&super::DebugSeams) -> Option<&std::path::Path>;
        let cases: [(&str, &str, Pick); 3] = [
            ("--dev-callers", "/tmp/rows.toml", |d| {
                d.dev_callers.as_deref()
            }),
            ("--fake-ddc", "/tmp/ddc.json", |d| d.fake_ddc.as_deref()),
            ("--proc-root", "/tmp/proc", |d| d.proc_root.as_deref()),
        ];
        for (flag, value, pick) in cases {
            let cli = Cli::try_parse_from(["sill", "run", flag, value])
                .unwrap_or_else(|e| panic!("{flag}: {e}"));
            let Some(Command::Run(args)) = cli.command else {
                panic!("{flag}: not run: {:?}", cli.command);
            };
            assert_eq!(
                pick(&args.debug),
                Some(std::path::Path::new(value)),
                "{flag}"
            );
        }
    }
}
