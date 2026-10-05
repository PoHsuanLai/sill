//! A dbusmenu label as the person reads it.

/// `label` without its access-key marks: dbusmenu writes `__` for a literal underscore and any
/// other `_` marks the next character as the row's access key, which a menu drawn without
/// underlines leaves out.
pub fn plain_label(label: &str) -> String {
    let mut plain = String::with_capacity(label.len());
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            ('_', Some('_')) => {
                plain.push('_');
                chars.next();
            }
            ('_', _) => {}
            (c, _) => plain.push(c),
        }
    }
    plain
}

#[cfg(test)]
mod tests {
    use super::plain_label;

    #[test]
    fn access_keys_go_and_doubled_underscores_stay() {
        const CASES: &[(&str, &str)] = &[
            ("Quit", "Quit"),
            ("_Quit", "Quit"),
            ("Show _Window", "Show Window"),
            ("snake__case", "snake_case"),
            ("a___b", "a_b"),
            ("trailing_", "trailing"),
            ("", ""),
        ];
        for &(label, want) in CASES {
            assert_eq!(plain_label(label), want, "{label:?}");
        }
    }
}
