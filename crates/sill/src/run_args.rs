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
    /// Feed each line written to this FIFO (`right`, `enter`, ...) to every power menu as a key
    /// (only with `--session-dry-run`, so a scripted Enter never reaches logind).
    #[arg(long, value_name = "FIFO", requires = "session_dry_run")]
    pub power_menu_keys: Option<std::path::PathBuf>,
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

impl RunArgs {
    /// The scripted keys for every power menu, when this debug build was given a FIFO.
    pub fn power_menu_keys(&self) -> Option<sill_surfaces::surfaces::power_menu::PowerMenuScript> {
        #[cfg(feature = "debug")]
        {
            self.debug
                .power_menu_keys
                .clone()
                .map(crate::power_menu::key_script)
        }
        #[cfg(not(feature = "debug"))]
        {
            None
        }
    }

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
        Env {
            ddc,
            idle_times,
            polkit,
            session_actions,
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

    /// Scripted keys only ever drive a dry run.
    #[test]
    fn power_menu_keys_need_a_session_dry_run() {
        let parse = |args: &[&str]| Cli::try_parse_from([&["sill", "run"], args].concat());
        assert!(parse(&["--power-menu-keys", "/tmp/keys"]).is_err());
        assert!(parse(&["--session-dry-run-offers", "logind"]).is_err());
        let Ok(Cli {
            command: Some(Command::Run(args)),
            ..
        }) = parse(&[
            "--session-dry-run",
            "/tmp/record",
            "--session-dry-run-offers",
            "logind",
            "--power-menu-keys",
            "/tmp/keys",
        ])
        else {
            panic!("a dry run with keys parses");
        };
        assert_eq!(
            args.env().session_actions.reach(),
            sill_services::env::ActionReach::DryRun
        );
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
