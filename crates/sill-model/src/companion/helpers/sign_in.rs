//! Which app a helper runs and whether the person is signed in to it: the plain words for the
//! card, and the buttons it offers. Sign-in is the app's own; the buttons carry the app's names
//! for its ways and send the way's id back.

use companion_wire::{HelperCardView, LoginShown};
use docket_core::helper::HelperKind;

/// Said when the helper runs an app whose name is not known.
const THE_APP: &str = "the app it uses";

/// One button that signs in: what it says, and the way it sends back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignInButton {
    /// The button's label.
    pub label: String,
    /// The id of the way, as the card lists it.
    pub way: String,
}

/// "Claude Code" from `claude-code`: words joined by dashes, each capitalised.
pub fn program_name(slug: &str) -> String {
    slug.split('-')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The app a helper runs, by name; nothing for a helper that plans for itself.
pub fn app_of(kind: &HelperKind) -> Option<String> {
    kind.program().map(|program| program_name(program.as_str()))
}

/// What the card says about how the helper works: "Uses Claude Code" for a helper that runs an
/// app, nothing for one that plans for itself.
pub fn kind_line(kind: &HelperKind) -> Option<String> {
    app_of(kind).map(|app| format!("Uses {app}"))
}

/// Where the sign-in to the app stands, for a helper that runs one.
pub fn login_line(view: &HelperCardView) -> Option<&'static str> {
    view.kind.program()?;
    match view.login {
        LoginShown::SignedIn => Some("Signed in"),
        LoginShown::NeedsSignIn => Some("Waiting for you to sign in"),
        LoginShown::Unknown => None,
    }
}

/// The sentence under a job that waits for the person to sign in.
pub fn needs_sign_in_line(kind: &HelperKind) -> String {
    let app = app_of(kind).unwrap_or_else(|| THE_APP.to_owned());
    format!("Sign in to {app} so this job can start.")
}

/// The buttons that sign in, for a helper that runs an app the person is not signed in to: one
/// "Sign In" when the app has one way, else one button per way by the app's own name.
pub fn sign_in_buttons(view: &HelperCardView) -> Vec<SignInButton> {
    if view.kind.program().is_none() || view.login == LoginShown::SignedIn {
        return Vec::new();
    }
    match view.sign_in.as_slice() {
        [only] => vec![SignInButton {
            label: "Sign In".to_owned(),
            way: only.id.clone(),
        }],
        ways => ways
            .iter()
            .map(|way| SignInButton {
                label: way.name.clone(),
                way: way.id.clone(),
            })
            .collect(),
    }
}

/// Whether the card offers Sign Out: the person is signed in to the app the helper runs.
pub fn offers_sign_out(view: &HelperCardView) -> bool {
    view.kind.program().is_some() && view.login == LoginShown::SignedIn
}
