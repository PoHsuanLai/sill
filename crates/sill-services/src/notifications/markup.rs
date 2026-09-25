//! A notification body: the spec's markup subset (`<b>`, `<i>`, `<u>`, `<a href>`, `<img src
//! alt>`, the five XML entities and numeric references) parsed once, at the D-Bus boundary, into
//! styled runs the surfaces draw with quire's text components. Nothing downstream sees markup.
//!
//! Parse, don't validate (CONVENTIONS §0): a body that is not well-formed still becomes runs. An
//! unknown tag is dropped and its text kept; a `<` that opens no tag is text; `<br>` (not in the
//! spec, but common from web senders) is a line break; an image becomes its `alt` text, since
//! sill does not advertise `body-images`.

use serde::{Deserialize, Serialize};

/// A parsed body, in reading order. Adjacent runs never share a style.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Body {
    pub runs: Vec<BodyRun>,
}

/// Text in one style.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BodyRun {
    pub text: String,
    pub style: RunStyle,
}

/// How a run is drawn.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RunStyle {
    pub weight: Weight,
    pub slant: Slant,
    pub underline: Underline,
    pub link: RunLink,
}

/// `<b>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weight {
    #[default]
    Regular,
    Bold,
}

/// `<i>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Slant {
    #[default]
    Upright,
    Italic,
}

/// `<u>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Underline {
    #[default]
    None,
    Single,
}

/// `<a href>`: where a run links to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum RunLink {
    #[default]
    Plain,
    To(Href),
}

/// A link target as the sender wrote it (entities decoded). Opened through the portal or
/// `xdg-open` by the surface, never fetched by the server.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Href(pub String);

impl Body {
    /// `text` as one unstyled run, markup and all (a sender that did not ask for markup, and
    /// `sill debug notify`); no runs for an empty text.
    pub fn plain(text: &str) -> Body {
        let runs = (!text.is_empty())
            .then(|| BodyRun {
                text: text.to_owned(),
                style: RunStyle::default(),
            })
            .into_iter()
            .collect();
        Body { runs }
    }

    /// Every run's text, joined: the accessible name, and what a search matches.
    pub fn text(&self) -> String {
        self.runs.iter().map(|run| run.text.as_str()).collect()
    }
}

/// `raw` (a `Notify` body) parsed per the module doc.
pub fn parse_body(raw: &str) -> Body {
    super::markup_parse::parse(raw)
}

#[cfg(test)]
mod tests {
    use super::{Body, BodyRun, Href, RunLink, RunStyle, Slant, Underline, Weight, parse_body};

    fn run(text: &str, style: RunStyle) -> BodyRun {
        BodyRun {
            text: text.to_owned(),
            style,
        }
    }

    fn plain(text: &str) -> BodyRun {
        run(text, RunStyle::default())
    }

    fn bold() -> RunStyle {
        RunStyle {
            weight: Weight::Bold,
            ..RunStyle::default()
        }
    }

    #[test]
    fn plain_bodies_are_one_run_and_empty_is_none() {
        assert_eq!(Body::plain(""), Body::default());
        assert_eq!(
            Body::plain("a <b>"),
            Body {
                runs: vec![plain("a <b>")]
            }
        );
        let body = Body {
            runs: vec![run("Hi", bold()), plain(" there")],
        };
        assert_eq!(body.text(), "Hi there");
    }

    #[test]
    fn the_spec_markup_subset_becomes_runs() {
        let italic_bold = RunStyle {
            weight: Weight::Bold,
            slant: Slant::Italic,
            ..RunStyle::default()
        };
        let underlined = RunStyle {
            underline: Underline::Single,
            ..RunStyle::default()
        };
        let linked = RunStyle {
            link: RunLink::To(Href("https://example.org/?a=1&b=2".to_owned())),
            ..RunStyle::default()
        };
        let cases: Vec<(&str, Vec<BodyRun>)> = vec![
            ("", vec![]),
            ("plain text", vec![plain("plain text")]),
            (
                "<b>bold</b> rest",
                vec![run("bold", bold()), plain(" rest")],
            ),
            ("<b><i>both</i></b>", vec![run("both", italic_bold)]),
            ("<u>under</u>", vec![run("under", underlined)]),
            (
                "see <a href=\"https://example.org/?a=1&amp;b=2\">the site</a>.",
                vec![plain("see "), run("the site", linked), plain(".")],
            ),
            (
                "a &amp; b &lt;c&gt; &quot;d&quot; &apos;e&apos;",
                vec![plain("a & b <c> \"d\" 'e'")],
            ),
            ("&#65;&#x42;", vec![plain("AB")]),
            (
                "<img src=\"/tmp/smile.png\" alt=\"smile\"/>",
                vec![plain("smile")],
            ),
            (
                "unknown <span>tag</span> kept",
                vec![plain("unknown tag kept")],
            ),
            ("line<br/>two<br>three", vec![plain("line\ntwo\nthree")]),
            ("5 < 6 and 7 > 3", vec![plain("5 < 6 and 7 > 3")]),
            ("<b>un</b><b>split</b>", vec![run("unsplit", bold())]),
            ("<b>never closed", vec![run("never closed", bold())]),
            ("stray </i> close", vec![plain("stray  close")]),
            ("&unknown; entity", vec![plain("&unknown; entity")]),
        ];
        for (raw, runs) in cases {
            assert_eq!(parse_body(raw), Body { runs }, "{raw:?}");
        }
    }
}
