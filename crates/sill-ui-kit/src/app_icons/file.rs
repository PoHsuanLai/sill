//! How an icon file is drawn: the one classification the launcher row, the dock tile and the
//! notification card share.

use std::path::{Path, PathBuf};

/// An icon file, by how it is drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconFile {
    /// A freedesktop `*-symbolic` file, recoloured like a glyph.
    Symbolic(PathBuf),
    /// Any other file, in its own colours (re-tinted under Muted and Monochrome).
    Themed(PathBuf),
}

impl IconFile {
    /// `path` classified; `None` for a relative one, which is a name and not a file.
    pub fn of(path: &Path) -> Option<IconFile> {
        if !path.is_absolute() {
            return None;
        }
        let symbolic = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem.ends_with("-symbolic"));
        Some(if symbolic {
            IconFile::Symbolic(path.to_owned())
        } else {
            IconFile::Themed(path.to_owned())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::IconFile;

    #[test]
    fn a_file_is_symbolic_by_its_stem_and_a_relative_path_is_no_file() {
        let of = |path: &str| IconFile::of(Path::new(path));
        assert_eq!(
            of("/i/x-symbolic.svg"),
            Some(IconFile::Symbolic("/i/x-symbolic.svg".into()))
        );
        assert_eq!(
            of("/i/term.svg"),
            Some(IconFile::Themed("/i/term.svg".into()))
        );
        assert_eq!(
            of("/i/symbolic-x.svg"),
            Some(IconFile::Themed("/i/symbolic-x.svg".into()))
        );
        assert_eq!(of("rel.svg"), None);
    }
}
