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
    let _ = url;
    todo!("M5 fill: svc-media-wifi-focus (b)")
}

/// The cache file name for remote art at `url`: a stable hash of the whole URL (so a new cover
/// is a new file) with the URL's own extension when it has a known image one, else none.
pub fn art_cache_name(url: &str) -> String {
    let _ = url;
    todo!("M5 fill: svc-media-wifi-focus (b)")
}

#[cfg(test)]
mod tests {
    use super::{ArtSource, art_cache_name, art_source};
    use std::path::PathBuf;

    #[test]
    #[ignore = "M5 fill: svc-media-wifi-focus (b)"]
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
    #[ignore = "M5 fill: svc-media-wifi-focus (b)"]
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
