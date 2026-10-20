use std::collections::BTreeSet;

use super::Services;

/// Two lines for one service, or one field twice, would shadow a spawn: names are unique.
#[test]
fn every_declared_service_has_its_own_field_and_name() {
    let fields: BTreeSet<_> = Services::DECLARED.iter().map(|(field, _)| field).collect();
    let names: BTreeSet<_> = Services::DECLARED.iter().map(|(_, name)| name).collect();
    assert_eq!(fields.len(), Services::DECLARED.len(), "a field twice");
    assert_eq!(names.len(), Services::DECLARED.len(), "a service twice");
}

/// The fake holds every declared service, and every half of its channels is claimed or kept.
#[test]
fn the_fake_holds_every_declared_service() {
    let (services, _ios) = Services::fake();
    assert_eq!(services.held(), Services::DECLARED);
}
