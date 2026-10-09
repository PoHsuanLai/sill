//! The words and marks the sheet's props take from porter's values: what a person reads for an
//! app, a provider, a kind of service and a reason a service is limited. Pure, table-tested in
//! `props.rs`'s tests.

use ds::base::text::clip::clip_chars;
use ds::components::content::mark_face::MarkFace;
use ds::components::content::provider_mark::MarkProvider;
use ds_shell::accounts::model::{Limitation, SignInFault};
use ds_shell::prelude::{Recovery, WorkingFor};
use porter_core::LimitReason;
use porter_core::consent::Usage;
use porter_core::sheet::SignInFault as PorterFault;
use porter_core::sheet::{ProviderKind, ProviderRow, RowKind};
use porter_core::{AbsentReason, AppId, CapabilityKind, DataClass, ProviderId, WebUrl};

/// What the person reads for an app. `resolved` is the label the app's own entry gives it (porter
/// will send one with the ask: this is where it plugs in); without one the last element of its
/// reverse-DNS name is shown as it is written (`org.quire.Photos` is "Photos",
/// `org.quire.Agent.claude-code` is "claude-code"), never lowercased or re-spelled.
pub(super) fn app_name(app: &AppId, resolved: Option<&str>) -> String {
    match resolved {
        Some(label) => label.to_owned(),
        None => app
            .name
            .as_str()
            .rsplit('.')
            .next()
            .unwrap_or_default()
            .to_owned(),
    }
}

/// What the steps call the provider: the label of its row, which porter sends with every view
/// after the list; the provider's id as written for a view that carries no row.
pub(super) fn provider_name(id: &ProviderId, row: Option<&ProviderRow>) -> String {
    row.map_or_else(|| id.as_str().to_owned(), |row| row.label.clone())
}

/// `text` cut to `limit` characters with an ellipsis at the end when it is longer: an address or
/// an app id that would run out of the window is shown short, never clipped on both edges.
pub(super) fn clipped(text: &str, limit: TextLimit) -> String {
    clip_chars(text, limit.0)
}

/// How many characters a line of the sheet may hold before it is cut short.
#[derive(Debug, Clone, Copy)]
pub(super) struct TextLimit(usize);

/// The app's name in the consent heading and the review's last button.
pub(super) const APP_NAME: TextLimit = TextLimit(28);
/// An account's address on a single line of the review and the consent line.
pub(super) const ACCOUNT_LABEL: TextLimit = TextLimit(48);

/// The page a browser step opened, short: its host, and an ellipsis when there is more to the
/// address. The whole address is for "Copy Link" and never printed.
pub(super) fn short_link(url: &WebUrl) -> String {
    let host = url.origin().host;
    match (url.path(), url.query()) {
        ("/", None) => host,
        _ => format!("{host}/\u{2026}"),
    }
}

/// The mark a provider row wears, from the provider file's `mark` word. Providers without a mark
/// of their own wear the neutral one.
pub(super) fn mark_of(mark: &str) -> MarkProvider {
    match mark {
        "google" | "gemini" | "google-ai" => MarkProvider::Google,
        "microsoft" => MarkProvider::Microsoft,
        "fastmail" => MarkProvider::Fastmail,
        "icloud" => MarkProvider::ICloud,
        "yahoo" => MarkProvider::Yahoo,
        "local" => MarkProvider::Local,
        _ => MarkProvider::Imap,
    }
}

/// The face the provider file gives `row`'s mark (a letter on a colour), in quire's type; `None`
/// when the file gives none, and the named mark is worn.
pub(super) fn face_of(row: &ProviderRow) -> Option<MarkFace> {
    row.mark_face
        .as_ref()
        .map(|face| MarkFace::new(face.letter.as_str(), face.colour.as_str()))
}

/// Whether the list's "Other…" row (the hand-typed mail server form, which also takes the newer
/// protocol from its pop-up) stands for `row`, so the row is not listed by name. A generic row
/// for anything else (calendar and contacts) is only reachable by its own name.
pub(super) fn reached_by_other(row: &ProviderRow) -> bool {
    row.kind == RowKind::Generic && matches!(row.id.as_str(), "generic-imap" | "generic-jmap")
}

/// What a sign-in is waiting on: the agent that logs itself in for a row whose provider file says
/// `agent_login`, the provider's own sign-in for every other (and for a view that carries no row).
pub(super) fn waiting_on(row: Option<&ProviderRow>) -> WorkingFor {
    match row {
        Some(row) if row.auth == ProviderKind::AgentLogin => WorkingFor::Agent(row.label.clone()),
        Some(_) | None => WorkingFor::Provider,
    }
}

/// How the consent title finishes "{app} wants to ...": by what it asks for, never by provider
/// (design/31 §4.5).
pub(super) fn request_phrase(kind: CapabilityKind, class: DataClass, usage: Usage) -> String {
    let what = match (kind, class) {
        (CapabilityKind::Storage, DataClass::AppOwn) => "keep its data in your files",
        (CapabilityKind::Photos, DataClass::AppOwn) => "keep its library in your photos",
        (CapabilityKind::Identity, _) => "know who you are",
        (CapabilityKind::Mail, _) => "use your email",
        (CapabilityKind::Calendar, _) => "use your calendars",
        (CapabilityKind::Contacts, _) => "use your contacts",
        (CapabilityKind::Tasks, _) => "use your tasks",
        (CapabilityKind::Notes, _) => "use your notes",
        (CapabilityKind::Storage, _) => "use your files",
        (CapabilityKind::Photos, _) => "use your photos",
        (
            CapabilityKind::Llm
            | CapabilityKind::Embeddings
            | CapabilityKind::Speech
            | CapabilityKind::ImageGen
            | CapabilityKind::Rerank,
            _,
        ) => "use an AI model",
        (CapabilityKind::ComputerUse, _) => "operate a window for you",
        (CapabilityKind::KeyValue, _) => "keep small settings in sync",
        (CapabilityKind::Push, _) => "send you notifications",
        (CapabilityKind::Agent, _) => "run an assistant with your account",
    };
    match usage {
        Usage::Interactive => what.to_owned(),
        Usage::Background => format!("{what} in the background"),
    }
}

/// The review's secondary line for a service the provider offers but limits.
pub(super) fn limit_of(reason: LimitReason) -> Limitation {
    match reason {
        LimitReason::AppendOnly => Limitation::AppendOnly,
        LimitReason::PickerOnly => Limitation::PickerOnly,
        LimitReason::AppFolderOnly => Limitation::AppFolderOnly,
        LimitReason::ProviderOffersNone => Limitation::ProviderOffersNone,
    }
}

/// Why a service is absent.
pub(super) fn absence_of(reason: AbsentReason) -> Limitation {
    match reason {
        AbsentReason::ProviderOffersNone => Limitation::ProviderOffersNone,
        AbsentReason::TenantConsent => Limitation::AdminConsent,
        AbsentReason::UnverifiedBuild => Limitation::UnverifiedBuild,
        AbsentReason::TurnedOff => Limitation::TurnedOff,
        AbsentReason::NotOnServer => Limitation::NotOnServer,
    }
}

/// Why a sign-in ended without an account.
pub(super) fn fault_of(fault: PorterFault) -> SignInFault {
    match fault {
        PorterFault::Refused => SignInFault::Refused,
        PorterFault::Unreachable => SignInFault::Unreachable,
        PorterFault::Unreadable => SignInFault::Unreadable,
        PorterFault::NeedsClientId => SignInFault::NeedsClientId,
        PorterFault::TimedOut => SignInFault::TimedOut,
        PorterFault::Cancelled => SignInFault::Cancelled,
        PorterFault::Forbidden => SignInFault::Forbidden,
        PorterFault::StoreFailed => SignInFault::StoreFailed,
        PorterFault::NoLauncher => SignInFault::NoLauncher,
        PorterFault::NotInstalled => SignInFault::NotInstalled,
        PorterFault::Expired => SignInFault::TimedOut,
        PorterFault::AlreadyAdded => SignInFault::AlreadyAdded,
        PorterFault::NotRunning => SignInFault::NotRunning,
        PorterFault::SignedOut => SignInFault::SignedOut,
        PorterFault::NotAllowed => SignInFault::NotAllowed,
    }
}

/// Whether trying the sign-in again could help, when the sheet has more to say than the fault's
/// own answer: not for a fault only a change outside the sheet fixes (a build without a client id,
/// a missing assistant or launcher). An account already added needs no word of ours: the fault
/// knows a retry changes nothing.
pub(super) fn recovery_of(fault: PorterFault) -> Option<Recovery> {
    match fault {
        PorterFault::AlreadyAdded => None,
        PorterFault::NeedsClientId
        | PorterFault::NotInstalled
        | PorterFault::NoLauncher
        | PorterFault::Forbidden => Some(Recovery::NoRetry),
        PorterFault::Refused
        | PorterFault::Unreachable
        | PorterFault::Unreadable
        | PorterFault::TimedOut
        | PorterFault::Cancelled
        | PorterFault::StoreFailed
        | PorterFault::Expired
        | PorterFault::NotRunning
        | PorterFault::SignedOut
        | PorterFault::NotAllowed => Some(Recovery::Retry),
    }
}

/// The key a review row goes by: the kind's slug on the wire.
pub(super) fn service_key(kind: CapabilityKind) -> String {
    match serde_json::to_value(kind) {
        Ok(serde_json::Value::String(slug)) => slug,
        _ => String::new(),
    }
}

/// The kind a review row's key names.
pub(super) fn service_of(key: &str) -> Option<CapabilityKind> {
    serde_json::from_value(serde_json::Value::String(key.to_owned())).ok()
}

/// What a choice's slug is called on a pop-up: the plain name of each protocol and security porter
/// offers, the protocol after it in brackets (the hand-typed server form is the one place it shows).
pub(super) fn choice_label(slug: &str) -> &str {
    match slug {
        "imap" => "Most servers (IMAP)",
        "pop3" => "Older servers (POP)",
        "jmap" => "Newer servers (JMAP)",
        "tls" => "Secure from the start (SSL/TLS)",
        "starttls" => "Secure after connecting (STARTTLS)",
        other => other,
    }
}

/// The port a server usually listens on, the hint in an empty port entry. porter keeps its own
/// table private (`usual_port` in `porter-core/src/sheet/manual.rs`); this one repeats it, and
/// a test pins the two together through `refit`'s prefill: outgoing 465 for TLS and 587 for
/// STARTTLS; POP3 995 for TLS and 110 for STARTTLS; IMAP (and JMAP, which has no port) 993 and 143.
pub(super) fn usual_port(protocol: &str, outgoing: bool, security: &str) -> u16 {
    match (outgoing, protocol, security) {
        (true, _, "tls") => 465,
        (true, _, _) => 587,
        (false, "pop3", "tls") => 995,
        (false, "pop3", _) => 110,
        (false, _, "tls") => 993,
        (false, _, _) => 143,
    }
}

/// Where the person makes the app password a provider asks for in place of the account's own,
/// said short enough for an empty entry. porter words the full steps; the entry only points.
pub(super) fn app_password_hint(provider: &ProviderId) -> Option<&'static str> {
    match provider.as_str() {
        "icloud" => Some("Made at account.apple.com, not your Apple Account password"),
        "fastmail" => Some("Made in Fastmail under Privacy & Security, not your password"),
        "yahoo" => Some("Made under Account security, not your Yahoo password"),
        _ => None,
    }
}
