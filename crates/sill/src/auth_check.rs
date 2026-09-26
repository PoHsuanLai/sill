//! `sill auth-check`: the lock screen's PAM helper (FINDINGS "M11 freeze" F540). The auth
//! service runs it once per attempt (`sill_services::auth::PamHelper`): it reads the whole
//! request from stdin (`protocol::decode_request`), runs PAM (`pam::check`), writes the notes
//! and the outcome on stdout (`protocol::render_line`), and exits 0 (an unreadable request is
//! `fault helper_failed` and exit 2). It never prints the secret, and holds it only as a
//! `Secret` (zeroed on drop).

use std::io::{Read, Write};
use std::process::ExitCode;

use sill_services::auth::protocol::{self, HelperLine};
use sill_services::auth::{AuthFault, Outcome, pam};

/// Run the helper over this process's stdin and stdout.
pub fn main() -> ExitCode {
    let mut request = Vec::new();
    let read = std::io::stdin().lock().read_to_end(&mut request);
    let decoded = read
        .ok()
        .and_then(|_| protocol::decode_request(&request).ok());
    request.iter_mut().for_each(|byte| *byte = 0);
    let Some(request) = decoded else {
        let line = HelperLine::Done(Outcome::Unavailable(AuthFault::HelperFailed));
        let _ = writeln!(std::io::stdout(), "{}", protocol::render_line(&line));
        return ExitCode::from(2);
    };
    let verdict = pam::check(&request.service, &request.attempt);
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
