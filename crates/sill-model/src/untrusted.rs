//! Words that another program chose, made safe to hold, show and log: cut to a length, stripped of
//! what can disguise a line, or escaped for the journal. Every boundary that takes an app's text
//! (a notification, a tray label, a polkit detail, an inhibit reason) uses these, so no caller
//! invents its own cut.

/// The most characters of an app's text one journal line carries: a line is a hint to the
/// reader, not a copy of what the app sent.
pub const LOG_TEXT_MAX_CHARS: usize = 120;

/// What marks where a cut stopped.
const ELLIPSIS: char = '\u{2026}';

/// `text` cut to at most `max` bytes at a character boundary.
pub fn cut_bytes(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let end = (0..=max)
        .rev()
        .find(|at| text.is_char_boundary(*at))
        .unwrap_or(0);
    &text[..end]
}

/// `text` cut to at most `max` bytes at a character boundary, with an ellipsis where it was cut.
pub fn clip_bytes(text: &str, max: usize) -> String {
    let kept = cut_bytes(text, max);
    let mut clipped = kept.to_owned();
    if kept.len() < text.len() {
        clipped.push(ELLIPSIS);
    }
    clipped
}

/// `text` cut to at most `max` characters, with an ellipsis where it was cut.
pub fn clip_chars(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        None => text.to_owned(),
        Some((end, _)) => {
            let mut cut = text[..end].to_owned();
            cut.push(ELLIPSIS);
            cut
        }
    }
}

/// Whether `c` reorders or hides text around it (the bidirectional marks and embeddings), which
/// lets a row read as another row.
fn is_bidi_control(c: char) -> bool {
    matches!(
        c,
        '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
    )
}

/// `text` as one line the person reads: control and bidirectional marks dropped, cut to `max`
/// characters.
pub fn for_display(text: &str, max: usize) -> String {
    let plain: String = text
        .chars()
        .filter(|c| !c.is_control() && !is_bidi_control(*c))
        .take(max.saturating_add(1))
        .collect();
    clip_chars(&plain, max)
}

/// `text` for a journal line: escaped (a newline or an escape sequence cannot forge a line) and
/// cut to [`LOG_TEXT_MAX_CHARS`], between quotes.
pub fn for_log(text: &str) -> String {
    let mut shown = String::from('"');
    shown.extend(
        text.chars()
            .take(LOG_TEXT_MAX_CHARS)
            .flat_map(char::escape_debug),
    );
    if text.chars().nth(LOG_TEXT_MAX_CHARS).is_some() {
        shown.push(ELLIPSIS);
    }
    shown.push('"');
    shown
}

#[cfg(test)]
mod tests {
    use super::{clip_bytes, clip_chars, for_display, for_log};

    #[test]
    fn a_cut_lands_on_a_character_and_says_so() {
        const CASES: &[(&str, usize, &str)] = &[
            ("short", 10, "short"),
            ("exact", 5, "exact"),
            ("abcdef", 3, "abc\u{2026}"),
            ("h\u{e9}llo", 2, "h\u{2026}"),
            ("\u{1f600}\u{1f600}", 5, "\u{1f600}\u{2026}"),
            ("anything", 0, "\u{2026}"),
        ];
        for &(text, max, want) in CASES {
            assert_eq!(clip_bytes(text, max), want, "{text:?} at {max} bytes");
        }
        assert_eq!(clip_chars("h\u{e9}llo", 2), "h\u{e9}\u{2026}");
        assert_eq!(clip_chars("hi", 2), "hi");
    }

    #[test]
    fn displayed_text_loses_what_disguises_a_line() {
        const CASES: &[(&str, &str)] = &[
            ("plain", "plain"),
            ("two\nlines\r\n", "twolines"),
            ("pay \u{202E}gnp.exe", "pay gnp.exe"),
            ("\u{2066}hidden\u{2069}", "hidden"),
            ("tab\there", "tabhere"),
        ];
        for &(text, want) in CASES {
            assert_eq!(for_display(text, 100), want, "{text:?}");
        }
        assert_eq!(for_display("abcdef", 3), "abc\u{2026}");
    }

    #[test]
    fn a_log_line_cannot_be_forged_or_flooded() {
        assert_eq!(
            for_log("a\nsill: fake\u{1b}[2J"),
            r#""a\nsill: fake\u{1b}[2J""#
        );
        let long = "x".repeat(10_000);
        let shown = for_log(&long);
        assert_eq!(shown.chars().count(), 2 + 120 + 1, "{shown}");
        assert!(shown.ends_with("\u{2026}\""));
    }
}
