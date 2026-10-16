//! Whose menu the bar shows: the owner of `org.quire.AppMenu.<id>` must be the program the
//! focused window belongs to (QUIRE-12). Any process may claim the name, so the name proves
//! nothing; the system's account of the owner (peephole's standing: an image the system owns, a
//! sandbox it attests, a desktop service) is compared with the window's client.
//!
//! Pure. `sill-services::callers` turns a standing into a [`MenuOwner`]; the services ask
//! [`menu_trust`] before they read a menu, show it or click it for an agent.
//!
//! **The seam.** The window's client is known by the app key the desktop names it by. The
//! compositor reports the client's own *subject* (the program's image path or the sandboxed
//! app's id, `quire_desktop_app_v1.subject`) once it has established one; it reaches here as
//! [`MenuClient::reported`]. With a subject the subject is compared, which is the stronger test
//! (a launcher's scope name is only a claim). Without one (a program the compositor cannot name,
//! a compositor older than protocol version 2, or an app id two programs declare) the key is
//! compared with what the owner's standing says of its app.

use super::model::MenuApp;

/// The owner of a menu's bus name, as the system shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuOwner {
    /// A sandboxed app, by its app id.
    Sandboxed { app: String },
    /// A program whose image the system owns: the image's path, and the app the launcher's
    /// scope names (a claim, never an identity by itself).
    Program {
        exe: String,
        scope_app: Option<String>,
    },
    /// One of the desktop's own services, by the app its table row names.
    Service { app: String },
    /// Nothing that can be remembered: an image the person can write, one that cannot be read,
    /// or a process that left.
    Unknown,
}

/// The program a window belongs to, as the desktop says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuClient {
    /// The app key the desktop names the window's client by.
    pub key: String,
    /// The client's own subject from the compositor (the program's image path or the sandboxed
    /// app's id), when the compositor has established one; `None` otherwise.
    pub reported: Option<String>,
}

impl MenuClient {
    /// The client of a window of `app`, with no subject reported.
    pub fn unreported(app: &MenuApp) -> MenuClient {
        MenuClient::new(app, None)
    }

    /// The client of a window of `app`, whose program the compositor reports as `reported`
    /// (`None` when it reported none).
    pub fn new(app: &MenuApp, reported: Option<&str>) -> MenuClient {
        MenuClient {
            key: app.0.clone(),
            reported: reported.map(str::to_owned),
        }
    }
}

/// Whether the owner's menu is the window's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuTrust {
    /// The owner is the window's program: its menu is read and shown.
    Shown,
    /// It is not, or cannot be shown to be: the app has no menu in the bar, and an agent is
    /// refused it.
    Hidden,
}

/// Whether `owner` is the program of `client`.
pub fn menu_trust(owner: &MenuOwner, client: &MenuClient) -> MenuTrust {
    let same = match (&client.reported, owner) {
        (_, MenuOwner::Unknown) => false,
        // A service's standing carries no image, so it is known by its app key either way.
        (_, MenuOwner::Service { app }) => *app == client.key,
        (Some(subject), MenuOwner::Sandboxed { app }) => app == subject,
        (Some(subject), MenuOwner::Program { exe, .. }) => exe == subject,
        (None, MenuOwner::Sandboxed { app }) => *app == client.key,
        (None, MenuOwner::Program { scope_app, .. }) => scope_app.as_ref() == Some(&client.key),
    };
    match same {
        true => MenuTrust::Shown,
        false => MenuTrust::Hidden,
    }
}
