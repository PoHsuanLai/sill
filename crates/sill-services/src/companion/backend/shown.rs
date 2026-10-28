//! A handle's text for the screen, marked by who wrote it. intentd labels each handle; the text
//! of what the person wrote shows as it is, and the text of anyone else's words (mail, the web, a
//! file) shows quoted, so the card never reads as the companion's own voice. A router that does
//! not know the labelled reading, or does not give this caller it, is read the old way and
//! everything it shows is quoted, since nothing says whose words they are. Pure.

use docket_client::{ClientError, TransportError};
use docket_wire::{Displayed, WireRefusal};
use prov::Integrity;

/// `shown`'s text: plain when the label says it is the person's own, quoted when it is not.
pub(super) fn text_of(shown: Displayed) -> String {
    match shown.label.integrity {
        Integrity::Trusted => shown.text,
        Integrity::Untrusted => quoted(&shown.text),
    }
}

/// `text` between curly quotes.
pub(super) fn quoted(text: &str) -> String {
    format!("\u{201c}{text}\u{201d}")
}

/// Whether `error` says the router cannot give the labelled reading: this caller is not allowed
/// it, or the router is older and has no such method.
pub(super) fn lacks_labels(error: &ClientError) -> bool {
    match error {
        ClientError::Refused(WireRefusal::NotAllowed) => true,
        ClientError::Transport(TransportError::Bus(why)) => why.contains("UnknownMethod"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use docket_client::{ClientError, TransportError};
    use docket_wire::{Displayed, WireRefusal};
    use prov::{DataClass, Label, Source, SpaceId};

    use super::{lacks_labels, text_of};

    fn displayed(label: Label) -> Displayed {
        Displayed {
            text: "pay today".to_owned(),
            label,
        }
    }

    #[test]
    fn the_persons_own_words_show_plain_and_anyone_elses_quoted() {
        let mail = Label::untrusted(Source::Mail, DataClass::Mail, SpaceId::desktop());
        let cases = [
            ("typed by the person", Label::trusted_user(), "pay today"),
            ("mail", mail, "\u{201c}pay today\u{201d}"),
        ];
        for (name, label, want) in cases {
            assert_eq!(text_of(displayed(label)), want, "{name}");
        }
    }

    #[test]
    fn only_a_missing_or_forbidden_labelled_reading_falls_back() {
        let bus = |why: &str| ClientError::Transport(TransportError::Bus(why.to_owned()));
        let cases = [
            (
                "not allowed",
                ClientError::Refused(WireRefusal::NotAllowed),
                true,
            ),
            (
                "an older router",
                bus("org.freedesktop.DBus.Error.UnknownMethod: no DisplayLabelled"),
                true,
            ),
            (
                "the session is gone",
                ClientError::Refused(WireRefusal::NoSuchSession),
                false,
            ),
            (
                "the bus failed",
                bus("org.freedesktop.DBus.Error.Failed"),
                false,
            ),
            (
                "no router",
                ClientError::Transport(TransportError::Closed),
                false,
            ),
            ("a reply of another kind", ClientError::Unexpected, false),
        ];
        for (name, error, want) in cases {
            assert_eq!(lacks_labels(&error), want, "{name}");
        }
    }
}
