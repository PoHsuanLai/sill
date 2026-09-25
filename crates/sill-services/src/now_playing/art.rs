//! Album art: where an `mpris:artUrl` points, and the cache file a remote one is kept in
//! (`$XDG_CACHE_HOME/sill/art/<name>`). Pure; the service does the download.

use std::path::PathBuf;

/// Where an `mpris:artUrl` points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtSource {
    /// `file:///...`, percent-decoded: used as it is.
    File(PathBuf),
    /// `http://` or `https://`: fetched once into the cache under [`art_cache_name`].
    Remote(String),
    /// Empty, or a scheme nothing here loads (`data:` art is left to a later pass).
    Unusable,
}

/// Classify `url`. The scheme is compared whole and case-insensitively (RFC 3986 §3.1).
pub fn art_source(url: &str) -> ArtSource {
    let Some((scheme, rest)) = url.split_once("://") else {
        return ArtSource::Unusable;
    };
    match scheme.to_ascii_lowercase().as_str() {
        "file" => ArtSource::File(PathBuf::from(percent_decode(rest))),
        "http" | "https" => ArtSource::Remote(url.to_owned()),
        _ => ArtSource::Unusable,
    }
}

/// The cache file name for remote art at `url`: a stable hash of the whole URL (so a new cover
/// is a new file) with the URL's own extension when it has a known image one, else none.
pub fn art_cache_name(url: &str) -> String {
    let hash = fnv1a(url.as_bytes());
    match image_extension(url) {
        Some(ext) => format!("{hash:016x}.{ext}"),
        None => format!("{hash:016x}"),
    }
}

/// Known image extensions kept on a cache file name, so a browser or an image loader can still
/// guess the type from the name alone.
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "avif"];

/// `url`'s last path segment's extension, lower-cased, when it is one of [`IMAGE_EXTENSIONS`].
fn image_extension(url: &str) -> Option<String> {
    let last_segment = url.rsplit('/').next().unwrap_or(url);
    let last_segment = last_segment
        .split(['?', '#'])
        .next()
        .unwrap_or(last_segment);
    let (_, ext) = last_segment.rsplit_once('.')?;
    let ext = ext.to_ascii_lowercase();
    IMAGE_EXTENSIONS.contains(&ext.as_str()).then_some(ext)
}

/// FNV-1a, 64-bit: small, dependency-free and stable across builds and processes (unlike
/// `std::collections::hash_map::DefaultHasher`, which is not guaranteed to be).
fn fnv1a(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// Percent-decodes `%XX` escapes as UTF-8, lossily; a malformed escape passes through as-is.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2]))
        {
            out.push(hi * 16 + lo);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{ArtSource, art_cache_name, art_source};
    use std::path::PathBuf;

    #[test]
    fn art_urls_are_classified_by_their_scheme() {
        let cases: Vec<(&str, ArtSource)> = vec![
            (
                "file:///home/me/.cache/cover.png",
                ArtSource::File(PathBuf::from("/home/me/.cache/cover.png")),
            ),
            (
                "file:///tmp/My%20Cover.jpg",
                ArtSource::File(PathBuf::from("/tmp/My Cover.jpg")),
            ),
            (
                "https://i.scdn.co/image/ab67616d0000b273",
                ArtSource::Remote("https://i.scdn.co/image/ab67616d0000b273".to_owned()),
            ),
            (
                "HTTP://example.com/a.png",
                ArtSource::Remote("HTTP://example.com/a.png".to_owned()),
            ),
            ("data:image/png;base64,AAAA", ArtSource::Unusable),
            ("filesystem:///x", ArtSource::Unusable),
            ("", ArtSource::Unusable),
        ];
        for (url, want) in cases {
            assert_eq!(art_source(url), want, "{url:?}");
        }
    }

    #[test]
    fn cache_names_are_stable_distinct_and_keep_image_extensions() {
        let a = art_cache_name("https://example.com/a.png");
        assert_eq!(a, art_cache_name("https://example.com/a.png"), "stable");
        assert_ne!(a, art_cache_name("https://example.com/b.png"), "distinct");
        assert!(a.ends_with(".png"), "{a}");
        let bare = art_cache_name("https://i.scdn.co/image/ab67616d");
        assert!(!bare.contains('.'), "{bare}");
        assert!(!bare.contains('/'), "a file name, not a path: {bare}");
    }
}
