//! The system provider's availability probe against this machine's real logind
//! (CONVENTIONS "run it the way its user would"). Only lists: it exercises `query`, which reads
//! logind's read-only `CanSuspend`/`CanReboot`/`CanPowerOff` properties — it never calls
//! `Suspend`, `Reboot`, `PowerOff` or `TerminateSession` (`activate` cannot: it is pure, and
//! this test never calls it), so running it does not lock, suspend, restart or shut down this
//! machine. `query` runs inside `spawn_blocking`, the same seam `results::spawn` (M4
//! launcher-core) queries every provider through.

use sill_launcher::{Cancel, Provider, Query, VecSink};
use sill_services::providers::SystemProvider;

#[test]
#[ignore = "needs logind"]
fn lists_session_commands_against_real_logind() {
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let items = rt.block_on(async {
        tokio::task::spawn_blocking(|| {
            let provider = SystemProvider::new();
            let cancel = Cancel::new();
            let query = Query::begin("", &cancel);
            let mut sink = VecSink::default();
            provider.query(&query, &mut sink);
            sink.0
        })
        .await
        .unwrap_or_else(|e| panic!("{e}"))
    });

    let titles: Vec<String> = items
        .iter()
        .flat_map(|batch| batch.items.iter())
        .map(|item| item.title.clone())
        .collect();
    eprintln!("sill: system provider lists on this machine: {titles:?}");

    // Lock, Log Out and Reload need nothing from logind beyond it existing at all, so they list
    // whenever the probe could reach the bus; this machine (`cargo test` runs under systemd on
    // every dev box this workspace targets) always can.
    for always in ["Lock", "Log Out", "Reload"] {
        assert!(
            titles.contains(&always.to_owned()),
            "{always} missing from {titles:?}"
        );
    }
}
