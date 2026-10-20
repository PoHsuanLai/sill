//! IBus's address file: where its own bus is.

/// The line IBus writes its bus address on.
const ADDRESS_KEY: &str = "IBUS_ADDRESS=";

/// The bus address in the text of one of IBus's address files (`IBUS_ADDRESS=unix:...`), when it
/// has one.
pub fn address_in(text: &str) -> Option<&str> {
    text.lines()
        .find_map(|line| line.strip_prefix(ADDRESS_KEY))
        .map(str::trim)
        .filter(|address| !address.is_empty())
}
