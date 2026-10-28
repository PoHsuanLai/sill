//! Font rendering for apps that are not sill's, from the displays' density: greyscale
//! antialiasing always (subpixel colour fringes on a panel whose subpixel order nobody knows
//! are worse than none), and slight hinting only where pixels are still big enough to snap to.
//! Pure: the density in, the fontconfig snippet out, plus the check for a person's own
//! conflicting configuration.

use serde::{Deserialize, Serialize};

use super::ppi::Ppi;

/// How glyph outlines are fitted to the pixel grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hinting {
    /// None: at high density the outline's own shape reads best (what macOS does everywhere).
    None,
    /// Vertical-only snapping: crisper stems at low density without distorting the design.
    Slight,
}

/// The rendering defaults: glyph edges are always smoothed in greyscale, so only the hinting
/// varies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FontRendering {
    pub hinting: Hinting,
}

/// At or above this density, hinting does more harm than good.
pub const HIGH_DENSITY: Ppi = Ppi(150);

/// The defaults for the densest display (the one whose text a person reads closest).
pub fn font_rendering_for(max: Ppi) -> FontRendering {
    FontRendering {
        hinting: if max >= HIGH_DENSITY {
            Hinting::None
        } else {
            Hinting::Slight
        },
    }
}

/// The file's name inside `fontconfig/conf.d`. `50-` so it sits after the distribution's
/// defaults and before a person's own `9x-` files.
pub const FILE_NAME: &str = "50-sill-rendering.conf";

/// The comment every file sill writes starts with: what marks a file as sill's to remove.
pub const HEADER: &str = "<!-- Written by sill (display.font_rendering = auto).";

/// The fontconfig snippet for `rendering`.
pub fn snippet(rendering: FontRendering) -> String {
    let (hinting, hintstyle) = match rendering.hinting {
        Hinting::None => ("false", "hintnone"),
        Hinting::Slight => ("true", "hintslight"),
    };
    format!(
        r#"<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
{HEADER}
     Set display.font_rendering = "off" in ~/.config/sill/settings.toml and sill removes this
     file; a file of your own in conf.d that sets antialias, hinting, hintstyle or rgba makes
     sill stand aside. -->
<fontconfig>
  <match target="font">
    <edit name="antialias" mode="assign"><bool>true</bool></edit>
    <edit name="rgba" mode="assign"><const>none</const></edit>
    <edit name="lcdfilter" mode="assign"><const>lcdnone</const></edit>
    <edit name="hinting" mode="assign"><bool>{hinting}</bool></edit>
    <edit name="hintstyle" mode="assign"><const>{hintstyle}</const></edit>
  </match>
</fontconfig>
"#
    )
}

/// Whether `text` is a file sill wrote.
pub fn is_ours(text: &str) -> bool {
    text.lines()
        .any(|line| line.trim_start().starts_with(HEADER))
}

/// The properties whose setting by someone else means sill stands aside.
const CONTESTED: &[&str] = &["antialias", "hinting", "hintstyle", "rgba"];

/// Whether a person's fontconfig file sets one of the rendering properties: an `<edit>` whose
/// `name` attribute is exactly one of them (compared as a whole quoted value, so
/// `name="hintstyle_extra"` or a comment mentioning hinting is not a conflict).
pub fn conflicts(text: &str) -> bool {
    text.split("<edit").skip(1).any(|element| {
        let head = element.split('>').next().unwrap_or_default();
        attribute(head, "name").is_some_and(|name| CONTESTED.contains(&name))
    })
}

/// The value of `key="..."` (or single-quoted) in an element's attribute text.
fn attribute<'a>(head: &'a str, key: &str) -> Option<&'a str> {
    head.split_whitespace().find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == key).then_some(())?;
        let v = v.trim_end_matches('/');
        v.strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| v.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
    })
}

#[cfg(test)]
mod tests {
    use super::{Hinting, conflicts, font_rendering_for, is_ours, snippet};
    use crate::displays::ppi::Ppi;

    #[test]
    fn hinting_turns_off_at_high_density() {
        const CASES: &[(u16, Hinting)] = &[
            (92, Hinting::Slight),
            (109, Hinting::Slight),
            (149, Hinting::Slight),
            (150, Hinting::None),
            (163, Hinting::None),
            (227, Hinting::None),
        ];
        for &(ppi, want) in CASES {
            assert_eq!(font_rendering_for(Ppi(ppi)).hinting, want, "{ppi}");
        }
    }

    #[test]
    fn the_snippet_is_marked_as_ours_and_says_what_it_sets() {
        let text = snippet(font_rendering_for(Ppi(163)));
        assert!(is_ours(&text));
        assert!(text.contains("<const>hintnone</const>"), "{text}");
        assert!(text.contains("<const>none</const>"), "{text}");
        let low = snippet(font_rendering_for(Ppi(92)));
        assert!(low.contains("<const>hintslight</const>"), "{low}");
        // Our own file would read as a conflict: callers skip it by name.
        assert!(conflicts(&text));
        assert!(!is_ours("<fontconfig/>"));
    }

    #[test]
    fn only_a_rendering_edit_is_a_conflict() {
        let cases: &[(&str, &str, bool)] = &[
            (
                "hintstyle",
                r#"<edit name="hintstyle" mode="assign"><const>hintfull</const></edit>"#,
                true,
            ),
            (
                "rgba, single quotes",
                "<edit mode='assign' name='rgba'><const>rgb</const></edit>",
                true,
            ),
            (
                "antialias",
                r#"<edit name="antialias"><bool>false</bool></edit>"#,
                true,
            ),
            (
                "a family alias",
                r#"<alias><family>serif</family><prefer><family>Noto Serif</family></prefer></alias>"#,
                false,
            ),
            (
                "a longer name",
                r#"<edit name="hintstyle_extra"><int>1</int></edit>"#,
                false,
            ),
            (
                "a comment",
                "<!-- hinting is nice --><match></match>",
                false,
            ),
            (
                "dpi",
                r#"<edit name="dpi"><double>96</double></edit>"#,
                false,
            ),
        ];
        for &(label, text, want) in cases {
            assert_eq!(conflicts(text), want, "{label}");
        }
    }
}
