//! A re-tinted icon's identity in the cache: which file, at which raster size, in which style,
//! tint and scheme. The output scale is folded into the size (a 48 px tile at 1.5 and a 72 px one at 1
//! are the same raster). Pure.

use std::path::PathBuf;

use ds::style::icon::retint::IconStyle;
use ds::style::icon::retint::Tint;
use ds::prelude::*;

use crate::app_icons::size::AssetPx;

/// A tint in whole steps, so it can key a map: tenths of a degree of hue, ten-thousandths of
/// chroma (finer than any two Space presets differ, coarser than float noise).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TintKey {
    pub hue_tenths: u16,
    pub chroma_ten_thousandths: u16,
}

impl TintKey {
    /// `tint`, rounded to the key's steps.
    pub fn of(tint: Tint) -> TintKey {
        let hue = (f64::from(tint.hue).rem_euclid(360.0) * 10.0).round() as u16 % 3600;
        let chroma = (f64::from(tint.chroma).clamp(0.0, 1.0) * 10_000.0).round() as u16;
        TintKey {
            hue_tenths: hue,
            chroma_ten_thousandths: chroma,
        }
    }
}

/// One cached raster.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IconKey {
    /// The untinted source file (a theme, or an entry's own icon).
    pub path: PathBuf,
    pub px: AssetPx,
    pub style: IconStyle,
    pub tint: TintKey,
    /// The scheme it is re-coloured for (the dark tone band, design/29 F3).
    pub scheme: Scheme,
}

impl IconKey {
    /// The cache file's name: readable parts for whoever looks in the directory, then a hash
    /// of the whole key so two keys never share a file. Stable across runs and builds (FNV-1a,
    /// not std's hasher, whose output may change between releases).
    pub fn file_name(&self) -> String {
        let name = stem(&self.path);
        let style = style_word(self.style);
        format!(
            "{name}-{}-{style}-{:016x}.png",
            self.px.0,
            fnv1a(self.canonical().as_bytes())
        )
    }

    /// Every field of the key as text, one way only.
    fn canonical(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}",
            self.path.display(),
            self.px.0,
            style_word(self.style),
            self.tint.hue_tenths,
            self.tint.chroma_ten_thousandths,
            scheme_word(self.scheme)
        )
    }
}

fn scheme_word(scheme: Scheme) -> &'static str {
    match scheme {
        Scheme::Light => "light",
        Scheme::Dark => "dark",
    }
}

fn style_word(style: IconStyle) -> &'static str {
    match style {
        IconStyle::Colour => "colour",
        IconStyle::Muted => "muted",
        IconStyle::Monochrome => "monochrome",
    }
}

/// The file's stem, kept to ASCII letters, digits, `.` and `-` (at most 48 of them).
fn stem(path: &std::path::Path) -> String {
    let raw = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let kept: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(48)
        .collect();
    if kept.is_empty() {
        "icon".to_owned()
    } else {
        kept
    }
}

/// 64-bit FNV-1a.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::{IconKey, TintKey, fnv1a};
    use crate::app_icons::size::AssetPx;
    use ds::style::icon::retint::IconStyle;
    use ds::style::icon::retint::Tint;
    use ds::prelude::*;
    use std::collections::HashSet;
    use std::path::PathBuf;

    fn key(path: PathBuf, px: u16, style: IconStyle, hue: f32) -> IconKey {
        IconKey {
            path,
            px: AssetPx(px),
            style,
            tint: TintKey::of(Tint { hue, chroma: 0.06 }),
            scheme: Scheme::Light,
        }
    }

    #[test]
    fn fnv1a_matches_its_reference_values() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn file_names_are_readable_and_stable() {
        let term = key(
            PathBuf::from("/usr/share/icons/hicolor/scalable/apps/terminal.svg"),
            96,
            IconStyle::Monochrome,
            268.0,
        );
        let name = term.file_name();
        assert!(name.starts_with("terminal-96-monochrome-"), "{name}");
        assert!(name.ends_with(".png"), "{name}");
        assert_eq!(name, term.clone().file_name(), "same key, same name");
        let odd = key(
            PathBuf::from("/usr/share/icons/a b/Ü?.svg"),
            48,
            IconStyle::Muted,
            0.0,
        );
        assert!(
            odd.file_name().starts_with("__-48-muted-"),
            "{}",
            odd.file_name()
        );
        assert!(!odd.file_name().contains('/'));
    }

    /// Every field changes the file: the source path, size, style and tint (a Space switch).
    #[test]
    fn every_field_of_the_key_changes_the_file() {
        let base = || {
            key(
                PathBuf::from("/i/firefox.svg"),
                48,
                IconStyle::Monochrome,
                268.0,
            )
        };
        let variants = [
            base(),
            IconKey {
                path: PathBuf::from("/j/firefox.svg"),
                ..base()
            },
            IconKey {
                px: AssetPx(72),
                ..base()
            },
            IconKey {
                style: IconStyle::Muted,
                ..base()
            },
            key(
                PathBuf::from("/i/firefox.svg"),
                48,
                IconStyle::Monochrome,
                152.0,
            ),
        ];
        let names: HashSet<String> = variants.iter().map(IconKey::file_name).collect();
        assert_eq!(names.len(), variants.len(), "{names:?}");
    }

    #[test]
    fn tints_key_in_whole_steps() {
        const CASES: &[(f32, f32, u16, u16)] = &[
            (268.04, 0.061_23, 2680, 612),
            (-10.0, 0.0, 3500, 0),
            (359.99, 0.07, 0, 700),
        ];
        for &(hue, chroma, h, c) in CASES {
            assert_eq!(
                TintKey::of(Tint { hue, chroma }),
                TintKey {
                    hue_tenths: h,
                    chroma_ten_thousandths: c
                },
                "{hue} {chroma}"
            );
        }
    }
}
