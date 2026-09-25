//! What an app icon needs before it can be shown: a file as it is, or a re-tinted raster from
//! the cache (quire CONSUMING's table). Pure: a shipped icon's file is found elsewhere
//! (`cache::IconResolver::shipped_path`, sill FINDINGS Q71, which touches the disk) and handed
//! in already resolved.
//!
//! | style | our icons | third-party icons |
//! | --- | --- | --- |
//! | Colour | `<app>/<px>.png` | as they come |
//! | Muted | `<app>/muted/<px>.png` | retinted, Muted |
//! | Monochrome | `<app>/monochrome/<px>.png`, retinted | retinted, Monochrome |
//!
//! Symbolic icons never come here: they take the text colour.

use std::path::{Path, PathBuf};

use ds::icon::IconStyle;

use crate::app_icons::key::{IconKey, Origin, TintKey};
use crate::app_icons::look::IconLook;
use crate::app_icons::shipped::ShippedApp;
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

/// Our icon for `app` at `px` in `look`, already found at `source`
/// (`cache::IconResolver::shipped_path`).
pub fn shipped_art(source: PathBuf, app: ShippedApp, px: AssetPx, look: IconLook) -> Art {
    match look.style {
        IconStyle::Colour | IconStyle::Muted => Art::Plain(source),
        IconStyle::Monochrome => Art::Styled {
            key: IconKey {
                origin: Origin::Shipped(app),
                px,
                style: look.style,
                tint: TintKey::of(look.tint),
            },
            source,
        },
    }
}

/// A third-party (theme) raster or SVG icon at `px` in `look`.
pub fn theme_art(path: &Path, px: AssetPx, look: IconLook) -> Art {
    match look.style {
        IconStyle::Colour => Art::Plain(path.to_owned()),
        IconStyle::Muted | IconStyle::Monochrome => Art::Styled {
            key: IconKey {
                origin: Origin::Theme(path.to_owned()),
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
    use super::{Art, shipped_art, theme_art};
    use crate::app_icons::look::IconLook;
    use crate::app_icons::shipped::ShippedApp;
    use crate::app_icons::size::AssetPx;
    use ds::icon::{IconStyle, Tint};
    use std::path::{Path, PathBuf};

    fn look(style: IconStyle) -> IconLook {
        IconLook {
            style,
            tint: Tint {
                hue: 268.0,
                chroma: 0.06,
            },
        }
    }

    fn word(art: &Art) -> String {
        match art {
            Art::Plain(path) => format!("plain {}", path.display()),
            Art::Styled { key, source } => {
                format!("styled {} from {}", key.file_name(), source.display())
            }
        }
    }

    #[test]
    fn our_icons_come_from_their_style_set() {
        let source = |style| PathBuf::from(format!("/q/apps/terminal/{style}72.png"));
        let at = |style: IconStyle, sub| {
            word(&shipped_art(
                source(sub),
                ShippedApp::Terminal,
                AssetPx(72),
                look(style),
            ))
        };
        assert_eq!(at(IconStyle::Colour, ""), "plain /q/apps/terminal/72.png");
        assert_eq!(
            at(IconStyle::Muted, "muted/"),
            "plain /q/apps/terminal/muted/72.png"
        );
        let mono = at(IconStyle::Monochrome, "monochrome/");
        assert!(
            mono.starts_with("styled quire-terminal-72-monochrome-")
                && mono.ends_with("from /q/apps/terminal/monochrome/72.png"),
            "{mono}"
        );
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
