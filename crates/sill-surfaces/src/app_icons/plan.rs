//! What an app icon needs before it can be shown: a file as it is, or a re-tinted raster from
//! the cache. Pure.
//!
//! | style | third-party icons |
//! | --- | --- |
//! | Colour | as they come |
//! | Muted | retinted, Muted |
//! | Monochrome | retinted, Monochrome |
//!
//! Symbolic icons never come here: they take the text colour.

use std::path::{Path, PathBuf};

use ds::icon::IconStyle;

use crate::app_icons::key::{IconKey, TintKey};
use crate::app_icons::look::IconLook;
use crate::app_icons::size::AssetPx;

/// An icon as the view gets it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Art {
    /// Show this file as it is.
    Plain(PathBuf),
    /// Show the cached raster `key` once it is made from `source`; `source` itself until then.
    Styled { key: IconKey, source: PathBuf },
}

impl Art {
    /// The file to show while nothing better is cached.
    pub fn untinted(&self) -> &Path {
        match self {
            Art::Plain(path) => path,
            Art::Styled { source, .. } => source,
        }
    }
}

/// A theme (or an entry's own) raster or SVG icon at `px` in `look`.
pub fn theme_art(path: &Path, px: AssetPx, look: IconLook) -> Art {
    match look.style {
        IconStyle::Colour => Art::Plain(path.to_owned()),
        IconStyle::Muted | IconStyle::Monochrome => Art::Styled {
            key: IconKey {
                path: path.to_owned(),
                px,
                style: look.style,
                tint: TintKey::of(look.tint),
            },
            source: path.to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{Art, theme_art};
    use crate::app_icons::look::IconLook;
    use crate::app_icons::size::AssetPx;
    use ds::icon::{IconStyle, Tint};
    use std::path::Path;

    fn look(style: IconStyle) -> IconLook {
        IconLook {
            style,
            tint: Tint {
                hue: 268.0,
                chroma: 0.06,
            },
        }
    }

    #[test]
    fn third_party_icons_are_retinted_unless_colour() {
        let path = Path::new("/usr/share/icons/hicolor/scalable/apps/firefox.svg");
        let at = |style| theme_art(path, AssetPx(48), look(style));
        assert_eq!(at(IconStyle::Colour), Art::Plain(path.to_owned()));
        for style in [IconStyle::Muted, IconStyle::Monochrome] {
            let art = at(style);
            assert!(matches!(art, Art::Styled { .. }), "{style:?}");
            assert_eq!(
                art.untinted(),
                path,
                "{style:?}: the first render's fallback"
            );
        }
    }
}
