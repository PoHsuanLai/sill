//! The words and marks the sheet's props take from porter's values: what a person reads for an
//! app, a provider, a kind of service and a reason a service is limited. Pure, table-tested in
//! `props.rs`'s tests.

use ds::components::content::provider_mark::MarkProvider;
use ds_shell::accounts::model::{Limitation, SignInFault};
use ds_shell::prelude::WorkingFor;
use porter_core::LimitReason;
use porter_core::consent::Usage;
use porter_core::sheet::SignInFault as PorterFault;
use porter_core::sheet::{ProviderKind, ProviderRow};
use porter_core::{AbsentReason, AppId, CapabilityKind, DataClass, ProviderId};

/// What the person reads for an app: the last element of its reverse-DNS name (`org.quire.Photos`
/// is "Photos"; `com.example.my_app-2` is "my app 2").
pub(super) fn app_name(app: &AppId) -> String {
    let last = app.name.as_str().rsplit('.').next().unwrap_or_default();
    last.replace(['_', '-'], " ")
}

/// What a provider is called, for the titles of the steps after the list (the list carries its own
/// labels): the shipped provider files' labels, and the id itself for any other.
pub(super) fn provider_label(id: &ProviderId) -> String {
    match id.as_str() {
        "anthropic" => "Anthropic",
        "fastmail" => "Fastmail",
        "generic-dav" => "your calendar and contacts server",
        "generic-imap" => "your email server",
        "generic-jmap" => "your email server",
        "gmx" => "GMX",
        "google" => "Google",
        "google-ai" => "Google (Gemini)",
        "icloud" => "iCloud",
        "llama-cpp" => "llama.cpp",
        "lm-studio" => "LM Studio",
        "local" => "this computer",
        "microsoft" => "Microsoft",
        "moonshot" => "Moonshot (Kimi)",
        "nextcloud" => "Nextcloud",
        "ollama" => "Ollama",
        "openai" => "OpenAI",
        "openrouter" => "OpenRouter",
        "yahoo" => "Yahoo Mail",
        other => other,
    }
    .to_owned()
}

/// What the steps call the provider: the row's own label for a coding agent (its provider file's
/// display name), else `provider_label`.
pub(super) fn provider_name(id: &ProviderId, row: Option<&ProviderRow>) -> String {
    match row {
        Some(row) if row.auth == ProviderKind::AgentLogin => row.label.clone(),
        Some(_) | None => provider_label(id),
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

/// What a sign-in is waiting on: the agent that logs itself in for a row whose provider file says
/// `agent_login`, the provider's own sign-in for every other (and for a view that carries no row).
pub(super) fn waiting_on(row: Option<&ProviderRow>) -> WorkingFor {
    match row {
        Some(row) if row.auth == ProviderKind::AgentLogin => WorkingFor::Agent(row.label.clone()),
        Some(_) | None => WorkingFor::Provider,
    }
}

/// What a kind of service is called on the review.
pub(super) fn service_name(kind: CapabilityKind) -> &'static str {
    match kind {
        CapabilityKind::Identity => "Account details",
        CapabilityKind::Mail => "Mail",
        CapabilityKind::Calendar => "Calendar",
        CapabilityKind::Contacts => "Contacts",
        CapabilityKind::Tasks => "Tasks",
        CapabilityKind::Notes => "Notes",
        CapabilityKind::Storage => "Files",
        CapabilityKind::Photos => "Photos",
        CapabilityKind::Llm => "Language model",
        CapabilityKind::Embeddings => "Search by meaning",
        CapabilityKind::Speech => "Speech",
        CapabilityKind::ImageGen => "Image generation",
        CapabilityKind::Rerank => "Result ranking",
        CapabilityKind::ComputerUse => "Operating windows",
        CapabilityKind::KeyValue => "Small synced items",
        CapabilityKind::Push => "Notifications",
        CapabilityKind::Agent => "Coding agent",
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
        (CapabilityKind::Agent, _) => "run a coding agent with your account",
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

/// What a choice's slug is called on a pop-up: the protocols and securities porter offers.
pub(super) fn choice_label(slug: &str) -> &str {
    match slug {
        "imap" => "IMAP",
        "pop3" => "POP3",
        "jmap" => "JMAP",
        "tls" => "TLS",
        "starttls" => "STARTTLS",
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
