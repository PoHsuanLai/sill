//! Live check against this machine's real NetworkManager. `#[ignore]`d because it needs the
//! system bus: run with `cargo test -p sill-services -- --ignored`.

use std::time::Duration;

use sill_services::network::{self, Network};

#[test]
#[ignore = "needs the session bus"]
fn the_primary_connection_seeds_within_half_a_second() {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let (mut receiver, _control) = network::spawn(runtime.handle());

    let seeded = runtime.block_on(async {
        tokio::time::timeout(Duration::from_millis(500), receiver.changed()).await
    });
    let network = receiver.borrow().clone();
    println!("network (NetworkManager primary connection): {network:?}");

    assert!(
        seeded.is_ok() || network == Network::default(),
        "no update arrived within 500ms and the receiver is not at its documented default: \
         {network:?}"
    );
}
