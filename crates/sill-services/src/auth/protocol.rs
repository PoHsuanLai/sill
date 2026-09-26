//! The words between [`super::PamHelper`] and its child, `sill auth-check`: pure, so both ends
//! are table-tested without a process.
//!
//! The request is the child's whole stdin: the PAM service, the user and the secret, each
//! followed by one NUL byte (a secret may hold any byte but NUL; `unix_chkpwd` reads the same
//! way). The child answers on stdout, one line each: `info TEXT` and `error TEXT` for a PAM
//! module's messages (newlines in TEXT become spaces), then exactly one of `accepted`,
//! `refused` or `fault WORD` (`WORD` is [`AuthFault`]'s snake_case name), then it exits.

use thiserror::Error;

use super::pam::PamService;
use super::types::{Attempt, AuthFault, Outcome, PamNote, Verdict};

/// What the parent writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperRequest {
    pub service: PamService,
    pub attempt: Attempt,
}

/// One line the child writes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HelperLine {
    Note(PamNote),
    Done(Outcome),
}

/// A request or a line that is not the protocol.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProtocolError {
    #[error("the request is not three NUL-terminated fields")]
    Fields,
    #[error("a field is not UTF-8")]
    Encoding,
    #[error("an unknown line: {0:?}")]
    Line(String),
}

/// The bytes of `request` for the child's stdin.
pub fn encode_request(request: &HelperRequest) -> Vec<u8> {
    todo!("M11 fill: a-lock ({request:?})")
}

/// The child's side of [`encode_request`]: exactly three NUL-terminated UTF-8 fields.
pub fn decode_request(bytes: &[u8]) -> Result<HelperRequest, ProtocolError> {
    todo!("M11 fill: a-lock ({} bytes)", bytes.len())
}

/// `line` as the child writes it, without the newline.
pub fn render_line(line: &HelperLine) -> String {
    todo!("M11 fill: a-lock ({line:?})")
}

/// One line of the child's stdout (its newline already stripped).
pub fn parse_line(line: &str) -> Result<HelperLine, ProtocolError> {
    todo!("M11 fill: a-lock ({line:?})")
}

/// The child's whole answer as one verdict: its notes in order, and its `Done`; no `Done` (or a
/// line that is not the protocol) is `Unavailable(HelperFailed)`.
pub fn verdict_of(lines: Vec<Result<HelperLine, ProtocolError>>) -> Verdict {
    todo!("M11 fill: a-lock ({} lines)", lines.len())
}

/// The fault's word on the wire.
pub fn fault_word(fault: AuthFault) -> &'static str {
    match fault {
        AuthFault::NoHelper => "no_helper",
        AuthFault::HelperFailed => "helper_failed",
        AuthFault::ServiceError => "service_error",
        AuthFault::TimedOut => "timed_out",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HelperLine, HelperRequest, ProtocolError, decode_request, encode_request, parse_line,
        render_line, verdict_of,
    };
    use crate::auth::pam::PamService;
    use crate::auth::types::{Attempt, AuthFault, Outcome, PamNote, Secret, UserName, Verdict};

    fn request(secret: &str) -> HelperRequest {
        HelperRequest {
            service: PamService("sill".to_owned()),
            attempt: Attempt {
                user: UserName("ada".to_owned()),
                secret: Secret::new(secret),
            },
        }
    }

    #[test]
    #[ignore = "M11 fill: a-lock"]
    fn requests_round_trip_and_bad_ones_are_refused() {
        assert_eq!(encode_request(&request("pw")), b"sill\0ada\0pw\0".to_vec());
        for secret in ["pw", "", "with space", "new\nline", "ünïcode"] {
            let bytes = encode_request(&request(secret));
            assert_eq!(decode_request(&bytes), Ok(request(secret)), "{secret:?}");
        }
        let bad: [(&[u8], ProtocolError); 3] = [
            (b"sill\0ada\0", ProtocolError::Fields),
            (b"sill\0ada\0pw", ProtocolError::Fields),
            (b"sill\0\xff\0pw\0", ProtocolError::Encoding),
        ];
        for (bytes, want) in bad {
            assert_eq!(decode_request(bytes), Err(want), "{bytes:?}");
        }
    }

    #[test]
    #[ignore = "M11 fill: a-lock"]
    fn lines_round_trip() {
        let cases = [
            (HelperLine::Done(Outcome::Accepted), "accepted"),
            (HelperLine::Done(Outcome::Refused), "refused"),
            (
                HelperLine::Done(Outcome::Unavailable(AuthFault::ServiceError)),
                "fault service_error",
            ),
            (
                HelperLine::Note(PamNote::Info("Place your finger".to_owned())),
                "info Place your finger",
            ),
            (
                HelperLine::Note(PamNote::Error("Account locked".to_owned())),
                "error Account locked",
            ),
        ];
        for (line, text) in cases {
            assert_eq!(render_line(&line), text);
            assert_eq!(parse_line(text), Ok(line), "{text}");
        }
        assert_eq!(
            render_line(&HelperLine::Note(PamNote::Info("two\nlines".to_owned()))),
            "info two lines"
        );
        assert!(parse_line("maybe").is_err());
    }

    #[test]
    #[ignore = "M11 fill: a-lock"]
    fn an_answer_folds_into_one_verdict() {
        let note = PamNote::Error("1 failed login".to_owned());
        let cases = [
            (
                vec![
                    Ok(HelperLine::Note(note.clone())),
                    Ok(HelperLine::Done(Outcome::Refused)),
                ],
                Verdict {
                    outcome: Outcome::Refused,
                    notes: vec![note.clone()],
                },
            ),
            (
                vec![Ok(HelperLine::Note(note.clone()))],
                Verdict {
                    outcome: Outcome::Unavailable(AuthFault::HelperFailed),
                    notes: vec![note],
                },
            ),
            (
                vec![Err(ProtocolError::Line("x".to_owned()))],
                Verdict::plain(Outcome::Unavailable(AuthFault::HelperFailed)),
            ),
            (
                vec![],
                Verdict::plain(Outcome::Unavailable(AuthFault::HelperFailed)),
            ),
        ];
        for (lines, want) in cases {
            assert_eq!(verdict_of(lines), want);
        }
    }
}
