//! The wallpaper's source: COSMIC's own background config, since design/22 has no settings key
//! for it (FINDINGS.md "M2 wallpaper"; W0 F19's last bullet). `cosmic-bg` (the COSMIC session's
//! own background daemon; `/usr/share/cosmic/com.system76.CosmicBackground/v1/all` on a system
//! with it installed) writes one RON record per output plus an `all` fallback that applies to
//! every output without one of its own; this reader only follows `all`
//! (`$XDG_CONFIG_HOME/cosmic/com.system76.CosmicBackground/v1/all`), a record shaped like:
//!
//! ```text
//! (
//!     output: "all",
//!     source: Path("/usr/share/backgrounds/default-dark.jxl"),
//!     filter_by_theme: true,
//!     rotation_frequency: 3600,
//!     filter_method: Lanczos,
//!     scaling_mode: Zoom,
//!     sampling_method: Alphanumeric,
//! )
//! ```
//!
//! `ron` is not in the pinned dependency block, so [`parse_source_path`] reads the one field it
//! needs by hand rather than deserialising the record: the `source: Path("...")` value, RON's
//! own string-escaping unquoted. A `source: Color(...)` (or any shape this does not recognise)
//! answers `None`, which [`resolve_source`] turns into [`crate::wallpaper::WallpaperSource::None`].

use std::path::PathBuf;

use crate::wallpaper::WallpaperSource;

/// The file this reads: `all`, cosmic-bg's fallback record for every output.
const RELATIVE_PATH: &str = "cosmic/com.system76.CosmicBackground/v1/all";

/// Where COSMIC keeps its `all` background record, when `$XDG_CONFIG_HOME` (or `$HOME`)
/// resolves at all.
pub fn config_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join(RELATIVE_PATH))
}

/// The `all` record's `source: Path("...")` field. `None` for a `source: Color(...)`, a record
/// this does not recognise, or text with no `source:` field at all.
pub fn parse_source_path(ron: &str) -> Option<PathBuf> {
    let after_key = ron.split("source:").nth(1)?;
    let after_variant = after_key.trim_start().strip_prefix("Path(")?;
    let (literal, _rest) = ron_string(after_variant)?;
    Some(PathBuf::from(literal))
}

/// A RON string literal at the start of `text` (already past its opening `"`... no: `text`
/// itself starts at or before the opening quote, leading whitespace allowed), and what follows
/// its closing `"`. RON's escapes are the same as Rust's string escapes for the characters a
/// path can contain (`\\`, `\"`, `\n`, `\t`, `\r`); anything else after a backslash passes
/// through unescaped rather than failing the whole parse over a wallpaper path.
fn ron_string(text: &str) -> Option<(String, &str)> {
    let rest = text.trim_start().strip_prefix('"')?;
    let mut out = String::with_capacity(rest.len());
    let mut chars = rest.char_indices();
    while let Some((index, ch)) = chars.next() {
        match ch {
            '"' => return Some((out, &rest[index + 1..])),
            '\\' => {
                let (_, escaped) = chars.next()?;
                out.push(match escaped {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    other => other,
                });
            }
            other => out.push(other),
        }
    }
    None
}

/// The wallpaper source right now: the path COSMIC's `all` record names, when the file exists
/// on disk — otherwise [`WallpaperSource::None`], which the surface paints as `--paper`. A path
/// [`image`] cannot decode (COSMIC's own default backgrounds are `.jxl`; this crate's `image`
/// build carries only `png`/`jpeg`, FINDINGS.md "M2 wallpaper") is not distinguished here: it
/// reaches the same fallback once the resize fails (`resize::ensure_cached`'s `Err`).
pub fn resolve_source() -> WallpaperSource {
    let path = config_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|ron| parse_source_path(&ron))
        .filter(|path| path.is_file());
    match path {
        Some(path) => WallpaperSource::File(path),
        None => WallpaperSource::None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::parse_source_path;

    /// The exact shape `cosmic-bg` 1.8 writes (`/usr/share/cosmic/com.system76.CosmicBackground/
    /// v1/all` on a system with it installed), field order and all.
    const COSMIC_BG_ALL: &str = "(
    output: \"all\",
    source: Path(\"/usr/share/backgrounds/default-dark.jxl\"),
    filter_by_theme: true,
    rotation_frequency: 3600,
    filter_method: Lanczos,
    scaling_mode: Zoom,
    sampling_method: Alphanumeric,
)
";

    #[test]
    fn every_case_extracts_or_refuses_the_path() {
        let cases: &[(&str, &str, Option<&str>)] = &[
            (
                "cosmic-bg's own default record",
                COSMIC_BG_ALL,
                Some("/usr/share/backgrounds/default-dark.jxl"),
            ),
            (
                "a solid colour source has no path",
                "(output: \"all\", source: Color((r: 1.0, g: 0.0, b: 0.0)))",
                None,
            ),
            ("no source field at all", "(output: \"all\")", None),
            ("empty text", "", None),
            (
                "a backslash-escaped path",
                r#"(source: Path("C:\\Users\\pop\\bg.png"))"#,
                Some(r"C:\Users\pop\bg.png"),
            ),
        ];
        for (name, ron, want) in cases {
            let got = parse_source_path(ron);
            assert_eq!(got, want.map(PathBuf::from), "{name}");
        }
    }
}
