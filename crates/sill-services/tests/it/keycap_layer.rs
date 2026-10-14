//! The keycap layer sill ships (`dist/keycap/keycap.toml`) carries a rule for each chord the
//! session binds with Command, so keycap's built-in mapping does not turn it into Ctrl and hand
//! it to the focused app.

const LAYER: &str = include_str!("../../../../dist/keycap/keycap.toml");

/// The default profile's rules: the text of each `{ ... }` line before the next profile.
fn default_rules() -> Vec<&'static str> {
    LAYER
        .split("[profile.")
        .find(|section| section.starts_with("default]"))
        .map(|section| {
            section
                .lines()
                .map(str::trim)
                .filter(|line| line.starts_with("{ when"))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn the_shell_chords_become_super_not_ctrl() {
    let rules = default_rules();
    let wanted = [
        r#"{ when = "command+tab", then = "super+tab", hold = "while_held" },"#,
        r#"{ when = "command+shift+tab", then = "super+shift+tab", hold = "while_held" },"#,
        r#"{ when = "command+shift+q", then = "super+shift+q" },"#,
        r#"{ when = "option+command+esc", then = "alt+super+esc" },"#,
        r#"{ when = "command+space", then = "super+space" },"#,
    ];
    let missing: Vec<&str> = wanted
        .into_iter()
        .filter(|rule| !rules.contains(rule))
        .collect();
    assert_eq!(missing, Vec::<&str>::new());
}
