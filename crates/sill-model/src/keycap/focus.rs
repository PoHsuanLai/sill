//! What the service tells keycap: the focus when it changes, and what a new connection must
//! hear at once.

use keycap_protocol::{MAX_PACKET, ProfileName, ToDaemon, encode};

/// `next` when keycap has not been told it yet: keycap hears a `Focus` only when the profile
/// changes, never for a window change that leaves it the same.
pub fn focus_to_send(previous: Option<&ProfileName>, next: ProfileName) -> Option<ProfileName> {
    (previous != Some(&next)).then_some(next)
}

/// The `UserConfig` carrying `toml`, unless it would not fit in one packet (keycap would not
/// read half a config).
pub fn user_config(toml: String) -> Option<ToDaemon> {
    let message = ToDaemon::UserConfig { toml };
    (encode(&message).len() < MAX_PACKET).then_some(message)
}

/// What keycap should be running with: the last focus and the last user config sent. A new
/// connection starts from nothing, so it is told all of it again.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Wants {
    pub focus: Option<ProfileName>,
    pub config: Option<String>,
}

impl Wants {
    /// These wants after `message` has been sent.
    pub fn after(self, message: &ToDaemon) -> Wants {
        match message {
            ToDaemon::Focus { profile } => Wants {
                focus: Some(profile.clone()),
                ..self
            },
            ToDaemon::UserConfig { toml } => Wants {
                config: Some(toml.clone()),
                ..self
            },
            ToDaemon::Resume => self,
        }
    }

    /// The messages that bring a new connection up to date: the focus, then the config.
    pub fn greeting(&self) -> Vec<ToDaemon> {
        let focus = self.focus.iter().map(|profile| ToDaemon::Focus {
            profile: profile.clone(),
        });
        let config = self
            .config
            .iter()
            .map(|toml| ToDaemon::UserConfig { toml: toml.clone() });
        focus.chain(config).collect()
    }
}
