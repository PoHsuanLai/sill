//! The checks every `*_ssr` module's specimens share: each is compared with its golden, linted
//! against the stylesheet, and every `ds-` class in it must be styled. A module lists its
//! specimens as [`Specimen`]s; `ssr_specimens` runs these checks over all of them at once, so a
//! failure names `<golden directory>/<specimen>`.

use ds_lint::{LintConfig, markup};

use super::golden;

/// Which of the lint checks a specimen takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checks {
    /// The markup lint and every `ds-` class styled.
    Both,
    /// The markup lint only.
    Lint,
    /// Every `ds-` class styled only.
    Styled,
}

/// One rendered specimen.
#[derive(Debug, Clone)]
pub struct Specimen {
    /// The golden's path under `tests/snapshots`, which also names the specimen.
    pub golden: String,
    /// What is compared with the golden (the markup, or its readable redaction).
    pub stored: String,
    /// The markup as rendered, which is linted.
    pub html: String,
    pub checks: Checks,
}

impl Specimen {
    /// The specimen whose golden is `golden` (`dir/name.html`), rendered as `html`.
    pub fn of(golden: &str, html: String) -> Specimen {
        Specimen {
            golden: golden.to_owned(),
            stored: html.clone(),
            html,
            checks: Checks::Both,
        }
    }

    /// The same specimen, with `stored` compared with the golden in place of the markup.
    pub fn stored_as(self, stored: String) -> Specimen {
        Specimen { stored, ..self }
    }

    /// The same specimen, taking only `checks`.
    pub fn checked(self, checks: Checks) -> Specimen {
        Specimen { checks, ..self }
    }
}

/// The goldens that differ from their specimens.
pub fn mismatches(set: &[Specimen]) -> Vec<String> {
    set.iter()
        .filter_map(|specimen| golden::check(&specimen.golden, &specimen.stored).err())
        .collect()
}

/// Every lint offence and every unstyled class in `set`.
pub fn offences(set: &[Specimen]) -> Vec<String> {
    let sheet = sill_shell_kit::stylesheet();
    let config = LintConfig::new(&sill_shell_kit::kits());
    let mut failures = Vec::new();
    for specimen in set {
        let name = &specimen.golden;
        if specimen.checks != Checks::Styled {
            for offence in markup(&specimen.html, sheet, &config) {
                failures.push(format!("{name}: {:?} {}", offence.rule, offence.text));
            }
        }
        if specimen.checks != Checks::Lint {
            for class in classes(&specimen.html).filter(|class| !styled(sheet, class)) {
                failures.push(format!("{name}: .{class} is not styled"));
            }
        }
    }
    failures.dedup();
    failures
}

/// Every `ds-` class `html` uses.
fn classes(html: &str) -> impl Iterator<Item = &str> {
    html.split("class=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .flat_map(str::split_whitespace)
        .filter(|class| class.starts_with("ds-"))
}

/// Whether `sheet` has a rule for exactly `.class` (not for a longer class it begins).
fn styled(sheet: &str, class: &str) -> bool {
    let needle = format!(".{class}");
    sheet.match_indices(&needle).any(|(at, _)| {
        !sheet[at + needle.len()..]
            .starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    })
}
