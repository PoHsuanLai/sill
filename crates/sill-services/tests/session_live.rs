//! Live check against this machine's real logind. `#[ignore]`d because it needs the system bus:
//! run with `cargo test -p sill-services -- --ignored`. Read-only, as CONVENTIONS and the M5
//! fill-in brief require: it only ever reads `session::spawn`'s receiver, never sends a
//! `SessionCommand` — so it never locks, logs out, suspends, restarts or shuts down this
//! machine (`SessionCommand::Run` is the only thing that would, and this test never sends one).

use std::time::Duration;

use sill_services::session::{self, Session};

#[test]
#[ignore = "needs the system bus"]
fn logind_offers_seed_within_one_second() {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let (mut receiver, _control) = session::spawn(runtime.handle());

    let seeded = runtime
        .block_on(async { tokio::time::timeout(Duration::from_secs(1), receiver.changed()).await });
    let session = *receiver.borrow();
    println!("session (logind's offers): {session:?}");

    assert!(
        seeded.is_ok() || session == Session::default(),
        "no update arrived within 1s and the receiver is not at its documented default: \
         {session:?}"
    );
}
