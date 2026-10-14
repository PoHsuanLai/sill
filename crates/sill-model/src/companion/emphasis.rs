//! The inline emphasis a model puts in a plain line (`**bold**`, `*slanted*`, `_slanted_`,
//! `` `code` ``), read once into spans so the row can say the words alone and the preview can set
//! them in their tones. Only these four marks are read; any other mark, and a mark with no
//! partner, stays as typed.

/// How one span of a line is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emphasis {
    /// The line's own tone.
    Plain,
    /// Stronger than the line.
    Strong,
    /// Slanted.
    Italic,
    /// Fixed pitch.
    Code,
}

/// A piece of a line in one emphasis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// The words, marks removed.
    pub text: String,
    /// How they are set.
    pub emphasis: Emphasis,
}

const MARKS: [(&str, Emphasis); 4] = [
    ("**", Emphasis::Strong),
    ("*", Emphasis::Italic),
    ("_", Emphasis::Italic),
    ("`", Emphasis::Code),
];

/// `line` as spans, in order, with no empty span.
pub fn spans(line: &str) -> Vec<Span> {
    let mut done = Vec::new();
    let mut plain = String::new();
    let mut rest = line;
    while let Some(next) = rest.chars().next() {
        match marked(rest, &plain) {
            Some(found) => {
                if !plain.is_empty() {
                    done.push(span(std::mem::take(&mut plain), Emphasis::Plain));
                }
                done.push(span(found.inner.to_owned(), found.emphasis));
                rest = &rest[found.used..];
            }
            None => {
                plain.push(next);
                rest = &rest[next.len_utf8()..];
            }
        }
    }
    if !plain.is_empty() {
        done.push(span(plain, Emphasis::Plain));
    }
    done
}

/// `line` as the words alone: every mark `spans` reads removed.
pub fn plain_of(line: &str) -> String {
    spans(line).into_iter().map(|span| span.text).collect()
}

fn span(text: String, emphasis: Emphasis) -> Span {
    Span { text, emphasis }
}

/// A marked stretch at the start of `rest`.
struct Marked<'a> {
    inner: &'a str,
    emphasis: Emphasis,
    /// Bytes of `rest` it takes, both marks included.
    used: usize,
}

/// The stretch `rest` opens with, if a mark opens it and the same mark closes it after words
/// that do not start or end in a space. An underscore inside a word (`snake_case`) is not one.
fn marked<'a>(rest: &'a str, before: &str) -> Option<Marked<'a>> {
    MARKS.iter().find_map(|(mark, emphasis)| {
        let body = rest.strip_prefix(mark)?;
        let word_inside = before.chars().last().is_some_and(char::is_alphanumeric);
        if *mark == "_" && word_inside {
            return None;
        }
        let end = body.find(mark)?;
        let inner = &body[..end];
        let spaced = inner.starts_with(char::is_whitespace) || inner.ends_with(char::is_whitespace);
        (!inner.is_empty() && !spaced).then_some(Marked {
            inner,
            emphasis: *emphasis,
            used: mark.len() * 2 + end,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{Emphasis, plain_of, spans};

    #[test]
    fn marks_are_read_and_stray_marks_stay() {
        const CASES: &[(&str, &str)] = &[
            ("The capital is **Taipei** now", "The capital is Taipei now"),
            (
                "a *slanted* and _slanted_ word",
                "a slanted and slanted word",
            ),
            ("run `ls -la` here", "run ls -la here"),
            ("snake_case_name stays", "snake_case_name stays"),
            ("2 * 3 * 4", "2 * 3 * 4"),
            ("open **never closed", "open **never closed"),
            ("empty **** pair", "empty **** pair"),
            ("plain", "plain"),
        ];
        for (line, want) in CASES {
            assert_eq!(plain_of(line), *want, "{line}");
        }
    }

    #[test]
    fn spans_carry_their_emphasis() {
        let got: Vec<(String, Emphasis)> = spans("is **Taipei**, `x`")
            .into_iter()
            .map(|span| (span.text, span.emphasis))
            .collect();
        let want = vec![
            ("is ".to_owned(), Emphasis::Plain),
            ("Taipei".to_owned(), Emphasis::Strong),
            (", ".to_owned(), Emphasis::Plain),
            ("x".to_owned(), Emphasis::Code),
        ];
        assert_eq!(got, want);
    }
}
