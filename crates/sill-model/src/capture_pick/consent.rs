//! The compositor's question as a sheet: who the program is called, and the words of the question
//! ([`consent_ask`]). The sheet is the one `AskPermission` draws, so a question from the portal
//! and one from the compositor look alike. Pure.
//!
//! A program is named from the desktop entries only when the compositor vouches for it: a root-held
//! image (its entry's `Exec` is that image) or a sandboxed app (its id). A program it cannot vouch
//! for is called by its file name: a user-writable program could otherwise write an entry that
//! names it "Firefox".

use std::path::Path;

use casement_protocols::vocab::SubjectKind;

use super::consent_words::{DONT_ALLOW, UNNAMED_PROGRAM, body, grant_words, headline};
use super::permission::PermissionAsk;
use crate::apps::{App, AppIndex, ExecArgs, expand_exec};
use crate::desktop::{Consent, ConsentSubject};
use crate::untrusted::clip_chars;

/// The longest program name put in a headline, in characters: the headline has a ceiling of its
/// own and a name must leave room for the rest of the sentence.
const NAME_MAX_CHARS: usize = 48;

/// The installed app `subject` is, when the compositor vouches for the subject.
fn app_of<'a>(subject: &ConsentSubject, apps: &'a AppIndex) -> Option<&'a App> {
    match subject.kind {
        SubjectKind::SandboxedApp => apps
            .apps
            .iter()
            .find(|app| is_sandboxed(app, &subject.text)),
        SubjectKind::Program => apps.apps.iter().find(|app| runs_image(app, &subject.text)),
        SubjectKind::Unrecognised => None,
    }
}

/// Whether `app` is the Flatpak app `id`.
fn is_sandboxed(app: &App, id: &str) -> bool {
    let entry = app.id.0.strip_suffix(".desktop").unwrap_or(&app.id.0);
    app.flatpak.as_deref() == Some(id) || entry == id
}

/// Whether `app`'s `Exec` starts the program at `path`: by the same path, or by a bare name that
/// is the image's file name (the entry leaves the search path to find it).
fn runs_image(app: &App, path: &str) -> bool {
    let Ok(argv) = expand_exec(&app.exec, &ExecArgs::default()) else {
        return false;
    };
    let Some(program) = argv.first() else {
        return false;
    };
    program == path || (!program.contains('/') && file_name(program) == file_name(path))
}

/// The last part of `path`, when there is one.
fn file_name(path: &str) -> Option<&str> {
    Path::new(path).file_name().and_then(|name| name.to_str())
}

/// What the person reads as the program's name: its entry's name, else the file name (or the
/// app's id for a sandboxed app), else a plain word.
pub(super) fn subject_name(subject: &ConsentSubject, apps: &AppIndex) -> String {
    let named = app_of(subject, apps).map(|app| app.name.clone());
    let fallback = match subject.kind {
        SubjectKind::SandboxedApp => Some(subject.text.as_str()).filter(|id| !id.is_empty()),
        SubjectKind::Program | SubjectKind::Unrecognised => file_name(&subject.text),
    };
    let name = named
        .filter(|name| !name.is_empty())
        .or_else(|| fallback.map(str::to_owned))
        .unwrap_or_else(|| UNNAMED_PROGRAM.to_owned());
    clip_chars(&name, NAME_MAX_CHARS)
}

/// The sheet for `consent`: the headline and explanation of its power, the buttons for whether
/// the answer is kept. `apps` names the program.
pub fn consent_ask(consent: &Consent, apps: &AppIndex) -> PermissionAsk {
    let name = subject_name(&consent.subject, apps);
    PermissionAsk::from_wire(
        &name,
        &headline(consent.power, &name),
        "",
        &body(consent.power, consent.memory),
        grant_words(consent.memory),
        DONT_ALLOW,
    )
}
