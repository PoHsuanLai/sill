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
    /// (`sill_services::displays::ddc::file_bus`), so no real monitor is touched.
    #[arg(long, value_name = "FILE")]
    pub fake_ddc: Option<std::path::PathBuf>,
    /// Idle's screen-off and suspend times in seconds for both power sources (`<screen_off_s>,
    /// <suspend_s>`, 0 never) instead of the settings' minutes.
    #[arg(long, value_name = "OFF_S,SUSPEND_S", value_parser = idle_times)]
    pub idle_times: Option<sill_services::idle::IdleTimes>,
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
}

/// `--idle-times`' value.
#[cfg(feature = "debug")]
fn idle_times(text: &str) -> Result<sill_services::idle::IdleTimes, String> {
    crate::idle::test_times(text)
        .ok_or_else(|| format!("{text:?} is not <screen_off_s>,<suspend_s>"))
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

        use sill_services::displays::ddc::file_bus::FileBus;
        use sill_services::env::{Bus, BusRoute, IdleTimesSource};
        use sill_services::polkit_agent::{self, HelperChoice, HelperRoute, SessionChoice};

        let ddc: Arc<dyn sill_services::displays::DdcBus> = match &self.fake_ddc {
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
            Some(address) => polkit_agent::Setup {
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
        Env {
            ddc,
            idle_times,
            polkit,
            ..env
        }
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
        assert_eq!(args.debug.idle_times, crate::idle::test_times("6,10"));
        assert!(parse("6").is_err());
    }

    /// Only a named bus leaves polkitd, and never for polkit's own helper.
    #[test]
    fn a_polkit_bus_brings_a_stand_in_helper_or_none() {
        use sill_services::env::{Bus, BusRoute};
        use sill_services::polkit_agent::{HelperChoice, HelperRoute, SessionChoice, Setup};

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
            (vec![], Setup::system(Bus::system(BusRoute::Usual))),
            (
                vec!["--polkit-bus", "unix:path=/tmp/b"],
                Setup {
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
                Setup {
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

    #[test]
    fn run_takes_a_fake_ddc_file() {
        let cli = Cli::try_parse_from(["sill", "run", "--fake-ddc", "/tmp/ddc.json"])
            .unwrap_or_else(|e| panic!("{e}"));
        let Some(Command::Run(args)) = cli.command else {
            panic!("not run: {:?}", cli.command);
        };
        assert_eq!(
            args.debug.fake_ddc.as_deref(),
            Some(std::path::Path::new("/tmp/ddc.json"))
        );
    }
}
