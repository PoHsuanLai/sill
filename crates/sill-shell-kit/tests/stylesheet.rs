//! The generated stylesheet: pinned by a golden file, free of the constructs Blitz cannot use,
//! with every attribute selector in the namespaced form Blitz matches, in the documented
//! cascade order.
//!
//! `DS_BLESS=1 cargo test -p sill-shell-kit --test stylesheet` rewrites the golden file.

use sill_shell_kit::stylesheet;
use std::path::PathBuf;

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/stylesheet.css")
}

#[test]
fn the_stylesheet_matches_its_golden_file() {
    let path = golden_path();
    if std::env::var_os("DS_BLESS").is_some() {
        std::fs::write(&path, stylesheet())
            .unwrap_or_else(|why| panic!("writing {}: {why}", path.display()));
        return;
    }
    let golden = std::fs::read_to_string(&path)
        .unwrap_or_else(|why| panic!("{}: {why}; run with DS_BLESS=1", path.display()));
    if golden != stylesheet() {
        let first = golden
            .lines()
            .zip(stylesheet().lines())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| golden.lines().count().min(stylesheet().lines().count()));
        panic!(
            "the stylesheet differs from {} from line {}; rerun with DS_BLESS=1 if intended\n\
             golden:    {:?}\ngenerated: {:?}",
            path.display(),
            first + 1,
            golden.lines().nth(first),
            stylesheet().lines().nth(first)
        );
    }
}

fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// Every declared property name, taken at declaration boundaries: after `{` or `;`, up to `:`.
/// Selectors and values are never mistaken for one, so `.ds-menu-filter` would not count.
fn properties(css: &str) -> Vec<String> {
    css.split(['{', ';', '}'])
        .filter_map(|piece| piece.split_once(':'))
        .map(|(name, _)| name.trim().to_ascii_lowercase())
        .filter(|name| name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        .collect()
}

#[test]
fn nothing_blitz_cannot_draw_or_the_lint_bans() {
    let css = strip_comments(stylesheet());
    for construct in [
        ":root",
        ":focus-visible",
        ":focus-within",
        "@media",
        "prefers-",
    ] {
        assert!(!css.contains(construct), "the stylesheet uses {construct}");
    }
    let properties = properties(&css);
    for banned in [
        "backdrop-filter",
        "text-overflow",
        "line-clamp",
        "-webkit-line-clamp",
    ] {
        assert!(
            !properties.iter().any(|p| p == banned),
            "the stylesheet declares {banned}"
        );
    }
    assert!(
        properties.iter().any(|p| p == "mask-image"),
        "the parser found no declarations"
    );
}

#[test]
fn every_attribute_selector_is_namespaced() {
    // `[` appears in this stylesheet only in selectors: no value uses brackets.
    let css = strip_comments(stylesheet());
    let unprefixed: Vec<String> = css
        .match_indices('[')
        .filter(|(index, _)| !css[index + 1..].starts_with("*|"))
        .map(|(index, _)| css[index..(index + 40).min(css.len())].to_owned())
        .collect();
    assert!(
        unprefixed.is_empty(),
        "attribute selectors without *|: {unprefixed:#?}"
    );
    assert!(
        css.contains("[*|data-theme=dark]"),
        "no attribute selector was checked"
    );
}

#[test]
fn the_cascade_is_in_the_documented_order() {
    let css = stylesheet();
    let order = [
        "/* == reset == */",
        "/* == tokens == */",
        "/* == accents == */",
        "/* == materials == */",
        "/* == motion == */",
        "/* == utilities == */",
        "/* == components == */",
    ];
    let positions: Vec<usize> = order
        .iter()
        .map(|marker| {
            css.find(marker)
                .unwrap_or_else(|| panic!("{marker} missing"))
        })
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "{positions:?}"
    );
    // Inside the tokens section: the light block, the dark block, then the levels.
    let tokens = [
        ".ds{--paper:",
        ".ds[*|data-theme=dark]{--paper:",
        ".ds[*|data-motion=reduced]{",
    ];
    let at: Vec<usize> = tokens
        .iter()
        .map(|needle| {
            css.find(needle)
                .unwrap_or_else(|| panic!("{needle} missing"))
        })
        .collect();
    assert!(at.windows(2).all(|pair| pair[0] < pair[1]), "{at:?}");
    // The components, in the fixed list's order, all of them.
    let components: Vec<usize> = sill_shell_kit::component_sheets()
        .iter()
        .map(|(name, _)| {
            let marker = format!("/* -- {name} -- */");
            css.find(&marker)
                .unwrap_or_else(|| panic!("{marker} missing"))
        })
        .collect();
    assert!(components.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(
        components
            .first()
            .is_some_and(|first| *first > positions[6])
    );
}

/// The declarations of the first rule whose selector is exactly `selector`.
fn rule_body(css: &str, selector: &str) -> Option<String> {
    css.split('}')
        .filter_map(|rule| rule.split_once('{'))
        .find(|(head, _)| head.trim() == selector)
        .map(|(_, body)| body.to_owned())
}

/// A banner slides like a toast and a side panel (design/30 section 1.3): its card enters with
/// `panel-in`, and its row leaves with `panel-out`.
#[test]
fn a_banner_slides_in_and_out_with_the_panel_keyframes() {
    let css = strip_comments(stylesheet());
    for (selector, keyframes) in [
        (".ds-banner-card", "animation:panel-in"),
        (".ds-banner[*|data-presence=leaving]", "animation:panel-out"),
    ] {
        let body = rule_body(&css, selector).unwrap_or_else(|| panic!("no rule for {selector}"));
        assert!(body.contains(keyframes), "{selector}: {body}");
    }
}
