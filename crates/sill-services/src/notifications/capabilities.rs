//! `GetCapabilities` and `GetServerInformation`: what sill tells a sender it can do.
//!
//! Advertised only what the banners and the center draw: actions, a body with the markup subset
//! and links, a static icon, and persistence (the history). Not `body-images` (an `<img>` shows
//! its alt text), `action-icons`, `icon-multi` or `sound` (the sound service is not real yet;
//! lane a adds `sound` once a hint actually plays).

/// The capability words, in the spec's spelling.
pub const CAPABILITIES: &[&str] = &[
    "actions",
    "body",
    "body-hyperlinks",
    "body-markup",
    "icon-static",
    "persistence",
];

/// The spec version sill implements (1.2 added `ActivationToken`).
pub const SPEC_VERSION: &str = "1.2";

/// `GetServerInformation`'s four strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerInformation {
    pub name: &'static str,
    pub vendor: &'static str,
    pub version: &'static str,
    pub spec_version: &'static str,
}

/// sill's answer.
pub const SERVER_INFORMATION: ServerInformation = ServerInformation {
    name: "sill",
    vendor: "sill",
    version: env!("CARGO_PKG_VERSION"),
    spec_version: SPEC_VERSION,
};

#[cfg(test)]
mod tests {
    use super::{CAPABILITIES, SERVER_INFORMATION};

    #[test]
    fn the_capabilities_are_spec_words_once_each() {
        let mut words = CAPABILITIES.to_vec();
        words.sort_unstable();
        words.dedup();
        assert_eq!(words.len(), CAPABILITIES.len());
        assert!(CAPABILITIES.contains(&"actions"));
        assert!(!CAPABILITIES.contains(&"body-images"));
        assert_eq!(SERVER_INFORMATION.spec_version, "1.2");
    }
}
