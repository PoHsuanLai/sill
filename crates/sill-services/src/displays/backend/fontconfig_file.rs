//! Writing and removing sill's fontconfig snippet: `$XDG_CONFIG_HOME/fontconfig/conf.d/
//! 50-sill-rendering.conf`. The decision is [`plan`] (pure); [`sync`] carries it out.

use std::path::{Path, PathBuf};

use sill_model::displays::font::{FILE_NAME, FontRendering, conflicts, is_ours, snippet};
use sill_model::displays::policy::FontRenderingMode;

use crate::error::ServiceError;

/// What to do with the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontFileAction {
    /// Write this text (the file is missing or says something else).
    Write(String),
    /// Remove sill's file.
    Remove,
    /// Nothing to do.
    Keep,
}

/// What the directory holds now.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FontFiles {
    /// sill's file's text, if it exists.
    pub ours: Option<String>,
    /// Every other file's text: `conf.d/*.conf` and `fonts.conf`.
    pub others: Vec<String>,
}

/// The rule: `auto` with a known density and no conflicting file of the person's own writes
/// the snippet; `off`, or a conflict, removes sill's file if sill wrote it (a file of the same
/// name without sill's header is the person's, and stays). Without a density there is nothing
/// to base a default on, and the file is left as it is.
pub fn plan(
    mode: FontRenderingMode,
    rendering: Option<FontRendering>,
    files: &FontFiles,
) -> FontFileAction {
    let ours_is_sills = files.ours.as_deref().is_some_and(is_ours);
    let remove = || {
        if ours_is_sills {
            FontFileAction::Remove
        } else {
            FontFileAction::Keep
        }
    };
    let contested = files.others.iter().any(|text| conflicts(text));
    match (mode, rendering) {
        (FontRenderingMode::Off, _) => remove(),
        (FontRenderingMode::Auto, _) if contested => remove(),
        (FontRenderingMode::Auto, None) => FontFileAction::Keep,
        (FontRenderingMode::Auto, Some(_)) if files.ours.is_some() && !ours_is_sills => {
            FontFileAction::Keep
        }
        (FontRenderingMode::Auto, Some(rendering)) => {
            let text = snippet(rendering);
            if files.ours.as_deref() == Some(text.as_str()) {
                FontFileAction::Keep
            } else {
                FontFileAction::Write(text)
            }
        }
    }
}

/// `fontconfig` under `dirs`' config directory.
pub fn dir_in(dirs: &crate::env::Dirs) -> PathBuf {
    dirs.config.join("fontconfig")
}

/// Read `dir` (a `fontconfig` directory), [`plan`], and carry the plan out.
pub fn sync(
    dir: &Path,
    mode: FontRenderingMode,
    rendering: Option<FontRendering>,
) -> Result<FontFileAction, ServiceError> {
    let conf_d = dir.join("conf.d");
    let path = conf_d.join(FILE_NAME);
    let action = plan(mode, rendering, &read(dir, &conf_d, &path));
    let io = |at: &Path| {
        let path = at.to_path_buf();
        move |source| ServiceError::Io { path, source }
    };
    match &action {
        FontFileAction::Write(text) => {
            std::fs::create_dir_all(&conf_d).map_err(io(&conf_d))?;
            let tmp = path.with_extension("conf.part");
            std::fs::write(&tmp, text).map_err(io(&tmp))?;
            std::fs::rename(&tmp, &path).map_err(io(&path))?;
        }
        FontFileAction::Remove => std::fs::remove_file(&path).map_err(io(&path))?,
        FontFileAction::Keep => {}
    }
    Ok(action)
}

fn read(dir: &Path, conf_d: &Path, ours: &Path) -> FontFiles {
    let others = std::fs::read_dir(conf_d)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|p| p.as_path() != ours && p.extension().is_some_and(|e| e == "conf"))
        .chain(std::iter::once(dir.join("fonts.conf")))
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect();
    FontFiles {
        ours: std::fs::read_to_string(ours).ok(),
        others,
    }
}

#[cfg(test)]
mod tests {
    use super::{FontFileAction, FontFiles, plan, sync};
    use sill_model::displays::Ppi;
    use sill_model::displays::font::{FILE_NAME, font_rendering_for, snippet};
    use sill_model::displays::policy::FontRenderingMode;

    #[test]
    fn the_plan_follows_the_rule() {
        use FontRenderingMode::{Auto, Off};
        let dense = Some(font_rendering_for(Ppi(163)));
        let text = snippet(font_rendering_for(Ppi(163)));
        let older = snippet(font_rendering_for(Ppi(92)));
        let theirs_conflicting = r#"<edit name="hintstyle"><const>hintfull</const></edit>"#;
        let files = |ours: Option<&str>, others: &[&str]| FontFiles {
            ours: ours.map(str::to_owned),
            others: others.iter().map(|s| (*s).to_owned()).collect(),
        };
        let cases: Vec<(&str, FontRenderingMode, FontFiles, FontFileAction)> = vec![
            (
                "auto, nothing there",
                Auto,
                files(None, &[]),
                FontFileAction::Write(text.clone()),
            ),
            (
                "auto, already written",
                Auto,
                files(Some(&text), &[]),
                FontFileAction::Keep,
            ),
            (
                "auto, density changed",
                Auto,
                files(Some(&older), &[]),
                FontFileAction::Write(text.clone()),
            ),
            (
                "auto, a conflicting file",
                Auto,
                files(None, &[theirs_conflicting]),
                FontFileAction::Keep,
            ),
            (
                "auto, a conflict appears later",
                Auto,
                files(Some(&text), &[theirs_conflicting]),
                FontFileAction::Remove,
            ),
            (
                "auto, same name but not ours",
                Auto,
                files(Some("<fontconfig/>"), &[]),
                FontFileAction::Keep,
            ),
            (
                "off, ours there",
                Off,
                files(Some(&text), &[]),
                FontFileAction::Remove,
            ),
            (
                "off, nothing there",
                Off,
                files(None, &[]),
                FontFileAction::Keep,
            ),
            (
                "off, same name but not ours",
                Off,
                files(Some("<fontconfig/>"), &[]),
                FontFileAction::Keep,
            ),
        ];
        for (label, mode, files, want) in cases {
            assert_eq!(plan(mode, dense, &files), want, "{label}");
        }
        assert_eq!(
            plan(Auto, None, &files(None, &[])),
            FontFileAction::Keep,
            "no density"
        );
    }

    /// The whole round in a temp directory standing in for `~/.config/fontconfig`: never the
    /// real one.
    #[test]
    fn sync_writes_then_removes_cleanly() {
        let dir = std::env::temp_dir().join(format!("sill-fontconfig-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let rendering = Some(font_rendering_for(Ppi(163)));
        let path = dir.join("conf.d").join(FILE_NAME);

        let wrote =
            sync(&dir, FontRenderingMode::Auto, rendering).unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(wrote, FontFileAction::Write(_)));
        assert!(path.exists());
        let again =
            sync(&dir, FontRenderingMode::Auto, rendering).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(again, FontFileAction::Keep);

        let removed =
            sync(&dir, FontRenderingMode::Off, rendering).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(removed, FontFileAction::Remove);
        assert!(!path.exists());
        assert!(dir.join("conf.d").exists(), "the directory stays");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
