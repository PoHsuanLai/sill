//! The hand-written parse behind [`super::markup::parse_body`]: a scan over the body that keeps
//! how deep each style is open, and turns text between tags into runs (FINDINGS F260: five tags
//! and the entities, no XML crate).

use super::markup::{Body, BodyRun, Href, RunLink, RunStyle, Slant, Underline, Weight};

/// One piece the scan meets.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    Text(String),
    Open(Tag),
    Close(TagName),
    /// `<br>` or `<img alt>`: text that stands where the tag was.
    Stands(String),
    /// A tag sill does not draw: its text is kept, the tag dropped.
    Dropped,
}

/// The tags that style text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagName {
    Bold,
    Italic,
    Underline,
    Link,
}

/// An opening styling tag, with the link's target.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Tag {
    Styling(TagName),
    Link(Href),
}

/// How deep each style is open at a point of the scan.
#[derive(Debug, Clone, Default)]
struct Open {
    bold: u32,
    italic: u32,
    underline: u32,
    links: Vec<Href>,
}

impl Open {
    fn style(&self) -> RunStyle {
        RunStyle {
            weight: if self.bold > 0 {
                Weight::Bold
            } else {
                Weight::Regular
            },
            slant: if self.italic > 0 {
                Slant::Italic
            } else {
                Slant::Upright
            },
            underline: if self.underline > 0 {
                Underline::Single
            } else {
                Underline::None
            },
            link: self
                .links
                .last()
                .map_or(RunLink::Plain, |href| RunLink::To(href.clone())),
        }
    }

    fn enter(mut self, tag: Tag) -> Open {
        match tag {
            Tag::Styling(TagName::Bold) => self.bold += 1,
            Tag::Styling(TagName::Italic) => self.italic += 1,
            Tag::Styling(TagName::Underline) => self.underline += 1,
            Tag::Styling(TagName::Link) => self.links.push(Href(String::new())),
            Tag::Link(href) => self.links.push(href),
        }
        self
    }

    /// A close of a tag that is not open changes nothing.
    fn leave(mut self, name: TagName) -> Open {
        match name {
            TagName::Bold => self.bold = self.bold.saturating_sub(1),
            TagName::Italic => self.italic = self.italic.saturating_sub(1),
            TagName::Underline => self.underline = self.underline.saturating_sub(1),
            TagName::Link => {
                self.links.pop();
            }
        }
        self
    }
}

/// `raw` as runs.
pub(super) fn parse(raw: &str) -> Body {
    let mut runs: Vec<BodyRun> = Vec::new();
    let mut open = Open::default();
    for piece in pieces(raw) {
        match piece {
            Piece::Text(text) | Piece::Stands(text) => push(&mut runs, text, open.style()),
            Piece::Open(tag) => open = open.enter(tag),
            Piece::Close(name) => open = open.leave(name),
            Piece::Dropped => {}
        }
    }
    Body { runs }
}

/// Append `text` in `style`, joining the last run when it has the same style.
fn push(runs: &mut Vec<BodyRun>, text: String, style: RunStyle) {
    if text.is_empty() {
        return;
    }
    match runs.last_mut() {
        Some(last) if last.style == style => last.text.push_str(&text),
        _ => runs.push(BodyRun { text, style }),
    }
}

/// The body cut into text and tags, entities decoded in the text.
fn pieces(raw: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut text = String::new();
    let mut rest = raw;
    while let Some(c) = rest.chars().next() {
        let tag = (c == '<').then(|| tag_at(rest)).flatten();
        match tag {
            Some((piece, used)) => {
                if !text.is_empty() {
                    out.push(Piece::Text(std::mem::take(&mut text)));
                }
                out.push(piece);
                rest = &rest[used..];
            }
            None => {
                let (decoded, used) = if c == '&' {
                    entity_at(rest).unwrap_or(('&', 1))
                } else {
                    (c, c.len_utf8())
                };
                text.push(decoded);
                rest = &rest[used..];
            }
        }
    }
    if !text.is_empty() {
        out.push(Piece::Text(text));
    }
    out
}

/// The tag starting at `at` (which starts with `<`) and how many bytes it spans, when `<` opens
/// one: an optional `/`, a letter, then anything up to the next `>` with no `<` before it.
fn tag_at(at: &str) -> Option<(Piece, usize)> {
    let inner_start = 1;
    let end = at[inner_start..].find(['>', '<'])? + inner_start;
    if at.as_bytes().get(end) != Some(&b'>') {
        return None;
    }
    let inner = &at[inner_start..end];
    let (closing, inner) = match inner.strip_prefix('/') {
        Some(rest) => (Closing::Close, rest),
        None => (Closing::Open, inner),
    };
    if !inner
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
    {
        return None;
    }
    let inner = inner.strip_suffix('/').unwrap_or(inner);
    let name_len = inner
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(inner.len());
    let name = inner[..name_len].to_ascii_lowercase();
    let attributes = &inner[name_len..];
    Some((piece_of(&name, attributes, closing), end + 1))
}

/// Whether a tag opens or closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Closing {
    Open,
    Close,
}

fn piece_of(name: &str, attributes: &str, closing: Closing) -> Piece {
    let styling = match name {
        "b" => Some(TagName::Bold),
        "i" => Some(TagName::Italic),
        "u" => Some(TagName::Underline),
        "a" => Some(TagName::Link),
        _ => None,
    };
    match (styling, closing, name) {
        (Some(tag), Closing::Close, _) => Piece::Close(tag),
        (Some(TagName::Link), Closing::Open, _) => {
            Piece::Open(match attribute(attributes, "href") {
                Some(href) => Tag::Link(Href(href)),
                None => Tag::Styling(TagName::Link),
            })
        }
        (Some(tag), Closing::Open, _) => Piece::Open(Tag::Styling(tag)),
        (None, Closing::Open, "br") => Piece::Stands("\n".to_owned()),
        (None, Closing::Open, "img") => {
            Piece::Stands(attribute(attributes, "alt").unwrap_or_default())
        }
        (None, _, _) => Piece::Dropped,
    }
}

/// `key="value"` (or single-quoted) in `attributes`, entities decoded.
fn attribute(attributes: &str, key: &str) -> Option<String> {
    let mut rest = attributes;
    loop {
        rest = rest.trim_start();
        let eq = rest.find('=')?;
        let name = rest[..eq].trim();
        let after = rest[eq + 1..].trim_start();
        let quote = after.chars().next().filter(|q| *q == '"' || *q == '\'')?;
        let close = after[1..].find(quote)? + 1;
        let value = &after[1..close];
        if name.eq_ignore_ascii_case(key) {
            return Some(decode(value));
        }
        rest = &after[close + 1..];
    }
}

/// `text` with its entities decoded (an unknown one kept as written).
fn decode(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        let (decoded, used) = if c == '&' {
            entity_at(rest).unwrap_or(('&', 1))
        } else {
            (c, c.len_utf8())
        };
        out.push(decoded);
        rest = &rest[used..];
    }
    out
}

/// The entity starting at `at` (which starts with `&`) and how many bytes it spans.
fn entity_at(at: &str) -> Option<(char, usize)> {
    let (semi, _) = at.char_indices().take(12).find(|(_, c)| *c == ';')?;
    let name = &at[1..semi];
    let decoded = match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => numeric(name),
    }?;
    Some((decoded, semi + 1))
}

/// `#65` or `#x42`.
fn numeric(name: &str) -> Option<char> {
    let number = name.strip_prefix('#')?;
    let code = match number.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => number.parse::<u32>().ok()?,
    };
    char::from_u32(code)
}
