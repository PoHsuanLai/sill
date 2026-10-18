//! `sill auth-check`: the lock screen's PAM helper. The auth service runs it once per attempt
//! (`sill_services::auth::PamHelper`): it reads the whole request from stdin
//! (`protocol::decode_request`), runs PAM (`pam::check`), writes the notes and the outcome on
//! stdout (`protocol::render_line`), and exits 0 (an unreadable request is `fault helper_failed`
//! and exit 2). It never prints the secret, and holds it only as a `Secret` (zeroed on drop). The
//! request is read into one buffer sized up front and never grown past [`MAX_REQUEST`], so no copy
//! of the secret is left behind by a reallocation; the buffer is zeroed as soon as it is decoded.

use std::io::{Read, Write};
use std::process::ExitCode;

use sill_model::auth::protocol::HelperLine;
use sill_model::auth::{AuthFault, Outcome, protocol};
use sill_services::auth::pam;

/// The longest request read: a service, a login name and a secret, with room to spare.
const MAX_REQUEST: usize = 8 * 1024;

/// Run the helper over this process's stdin and stdout.
pub fn main() -> ExitCode {
    let mut request = Vec::with_capacity(MAX_REQUEST + 1);
    let read = std::io::stdin()
        .lock()
        .take(u64::try_from(MAX_REQUEST).unwrap_or(u64::MAX))
        .read_to_end(&mut request);
    let decoded = read
        .ok()
        .and_then(|_| protocol::decode_request(&request).ok());
    request.fill(0);
    std::hint::black_box(&request);
    let Some(request) = decoded else {
        let line = HelperLine::Done(Outcome::Unavailable(AuthFault::HelperFailed));
        let _ = writeln!(std::io::stdout(), "{}", protocol::render_line(&line));
        return ExitCode::from(2);
    };
    let verdict = pam::check(&request.service, &request.attempt);
    drop(request);
    let mut out = std::io::stdout().lock();
    for note in verdict.notes {
        let _ = writeln!(out, "{}", protocol::render_line(&HelperLine::Note(note)));
    }
    let _ = writeln!(
        out,
        "{}",
        protocol::render_line(&HelperLine::Done(verdict.outcome))
    );
    ExitCode::SUCCESS
}
