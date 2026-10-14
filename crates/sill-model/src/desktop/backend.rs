//! Which desktop arm runs, and the globals a compositor offers. The choice itself is
//! `sill-services`' (`desktop_wl::backend_of`): it asks `desktop-client` what the offer carries.

use std::collections::BTreeMap;

/// The arm of the desktop client the shell runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DesktopBackend {
    /// The desktop's own compositor and its `quire_desktop_v1` protocol.
    Quire,
    /// Standard protocols: ext toplevel list, wlr toplevel management, ext workspace.
    Wlr,
    /// No window protocol at all: no window list, the empty desktop.
    Bare,
}

/// The globals a compositor's registry listed: each interface name with its version.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GlobalOffer(BTreeMap<String, u32>);

impl GlobalOffer {
    /// The offer made of these `(interface, version)` pairs; a repeated interface keeps the
    /// highest version.
    pub fn from_globals<'a>(globals: impl IntoIterator<Item = (&'a str, u32)>) -> GlobalOffer {
        let mut offer = BTreeMap::new();
        for (interface, version) in globals {
            let seen = offer.entry(interface.to_owned()).or_insert(version);
            *seen = (*seen).max(version);
        }
        GlobalOffer(offer)
    }

    /// Whether the compositor lists `interface`.
    pub fn offers(&self, interface: &str) -> bool {
        self.0.contains_key(interface)
    }

    /// Every interface the compositor lists, in name order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    /// The version it lists `interface` at.
    pub fn version(&self, interface: &str) -> Option<u32> {
        self.0.get(interface).copied()
    }
}
