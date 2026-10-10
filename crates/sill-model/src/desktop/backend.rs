//! Which desktop arm runs, chosen from what the compositor offers and nothing else.

use std::collections::BTreeMap;

/// `quire_desktop_manager_v1`: the compositor that tells the shell apps, windows and Spaces
/// itself.
const QUIRE_DESKTOP: &str = "quire_desktop_manager_v1";

/// `zwlr_foreign_toplevel_manager_v1`: toplevel state and actions on a wlroots-style
/// compositor.
const WLR_TOPLEVELS: &str = "zwlr_foreign_toplevel_manager_v1";

/// The arm of the desktop client the shell runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DesktopBackend {
    /// The desktop's own compositor and its `quire_desktop_v1` protocol.
    Quire,
    /// Standard protocols: ext toplevel list, wlr toplevel management, ext workspace.
    Wlr,
    /// COSMIC's toplevel and workspace protocols.
    Cosmic,
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

    /// The version it lists `interface` at.
    pub fn version(&self, interface: &str) -> Option<u32> {
        self.0.get(interface).copied()
    }
}

/// The arm for `offer`: Quire when the compositor offers its desktop manager, else Wlr when it
/// offers wlr toplevel management, else Cosmic (which degrades to an empty desktop on a
/// compositor with none of the three). Never chosen from a compositor's name or the
/// environment.
pub fn backend_for(offer: &GlobalOffer) -> DesktopBackend {
    if offer.offers(QUIRE_DESKTOP) {
        DesktopBackend::Quire
    } else if offer.offers(WLR_TOPLEVELS) {
        DesktopBackend::Wlr
    } else {
        DesktopBackend::Cosmic
    }
}
