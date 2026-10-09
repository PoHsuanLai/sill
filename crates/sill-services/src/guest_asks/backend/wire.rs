//! inferd's `GuestAsks` signal and `Guests` rows, as questions; the words of the answer.

use porter_dbus::{
    Details, GUEST_ANSWER_ALLOW, GUEST_ANSWER_DENY, GUEST_KEY_NAME, GUEST_KEY_STATE,
};
use sill_model::guest_asks::{Ask, Computer, Verdict};
use zbus::zvariant::OwnedValue;

/// inferd's well-known name.
pub(crate) const INFERD: &str = porter_dbus::INFERENCE_BUS;

/// The state word of a computer still waiting for the person.
const ASKING: &str = "asking";

fn text(details: &Details, key: &str) -> Option<String> {
    details
        .get(key)
        .and_then(|value: &OwnedValue| String::try_from(value.clone()).ok())
}

/// The question a `GuestAsks` signal carries. A signal without a node or a name is not one the
/// person can answer, so it says nothing.
pub(crate) fn asked(node: &str, details: &Details) -> Option<Ask> {
    let name = text(details, GUEST_KEY_NAME).filter(|name| !name.is_empty())?;
    (!node.is_empty()).then(|| Ask {
        node: Computer::new(node),
        name,
    })
}

/// The questions waiting in a `Guests` listing: the rows in state `asking`.
pub(crate) fn waiting(rows: &[(String, Details)]) -> Vec<Ask> {
    rows.iter()
        .filter(|(_, details)| text(details, GUEST_KEY_STATE).as_deref() == Some(ASKING))
        .filter_map(|(node, details)| asked(node, details))
        .collect()
}

/// The word `AnswerGuest` takes for `verdict`.
pub(crate) fn word(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Allow => GUEST_ANSWER_ALLOW,
        Verdict::Deny => GUEST_ANSWER_DENY,
    }
}
