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
        Env { ddc, ..env }
    }
}

#[cfg(all(test, feature = "debug"))]
mod tests {
    use clap::Parser;

    use crate::cli::{Cli, Command};

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
