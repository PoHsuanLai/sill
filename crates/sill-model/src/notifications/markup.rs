//! A notification body: the spec's markup subset (`<b>`, `<i>`, `<u>`, `<a href>`, `<img src
//! alt>`, the five XML entities and numeric references) parsed once, at the D-Bus boundary, into
//! styled runs the surfaces draw with quire's text components. Nothing downstream sees markup.
//!
//! Parse at the boundary (CONVENTIONS §4): a body that is not well-formed still becomes runs. An
//! unknown tag is dropped and its text kept; a `<` that opens no tag is text; `<br>` (not in the
//! spec, but common from web senders) is a line break; an image becomes its `alt` text, since
//! sill does not advertise `body-images`.

use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

/// A link target as the sender wrote it (entities decoded), of a scheme a person may follow
/// ([`LINK_SCHEMES`]). Opened through the portal or `xdg-open` by the surface, never fetched by
/// the server. Shared, so every run of a link holds the one copy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Href(pub Arc<str>);

impl Href {
    /// `target` as a link target.
    pub fn new(target: &str) -> Href {
        Href(Arc::from(target))
    }

    /// No target: a link the sender gave no usable address.
    pub fn empty() -> Href {
        Href::new("")
    }
}

impl Serialize for Href {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Href {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Href, D::Error> {
        String::deserialize(deserializer).map(|target| Href::new(&target))
    }
}

/// The schemes a body link may have: the ones a person expects a mail or web link to open.
pub const LINK_SCHEMES: &[&str] = &["http", "https", "mailto"];

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
            link: RunLink::To(Href::new("https://example.org/?a=1&b=2")),
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

    fn link_of(run: &BodyRun) -> Option<&Href> {
        match &run.style.link {
            RunLink::To(href) => Some(href),
            RunLink::Plain => None,
        }
    }

    #[test]
    fn a_link_is_followed_only_for_a_known_scheme_and_a_sane_length() {
        let long = format!("<a href=\"https://example.org/{}\">x</a>", "a".repeat(3000));
        let cases: Vec<(&str, String, &str)> = vec![
            (
                "https",
                "<a href=\"https://example.org/\">x</a>".to_owned(),
                "https://example.org/",
            ),
            (
                "mailto",
                "<a href=\"MAILTO:ada@example.org\">x</a>".to_owned(),
                "MAILTO:ada@example.org",
            ),
            (
                "script",
                "<a href=\"javascript:alert(1)\">x</a>".to_owned(),
                "",
            ),
            (
                "custom handler",
                "<a href=\"steam://run/10\">x</a>".to_owned(),
                "",
            ),
            ("relative", "<a href=\"/etc/passwd\">x</a>".to_owned(), ""),
            (
                "padded scheme",
                "<a href=\" https://example.org/\">x</a>".to_owned(),
                "",
            ),
            ("too long", long, ""),
        ];
        for (name, raw, want) in cases {
            let body = parse_body(&raw);
            let href = body.runs.first().and_then(link_of);
            assert_eq!(href.map(|h| &*h.0), Some(want), "{name}");
        }
    }

    #[test]
    fn a_link_inside_a_link_keeps_the_outer_target() {
        let body = parse_body(
            "<a href=\"https://a.example/\">one <a href=\"https://b.example/\">two</a> three</a> four",
        );
        let shown: Vec<(&str, Option<&str>)> = body
            .runs
            .iter()
            .map(|run| (run.text.as_str(), link_of(run).map(|h| &*h.0)))
            .collect();
        assert_eq!(
            shown,
            vec![
                ("one two three", Some("https://a.example/")),
                (" four", None),
            ]
        );
    }

    #[test]
    fn many_runs_of_one_link_share_one_copy_of_it_and_are_capped() {
        let href = "h".repeat(1500);
        let raw = format!(
            "<a href=\"https://x/{href}\">{}</a>",
            "<b>a</b>b".repeat(400)
        );
        let body = parse_body(&raw);
        assert!(body.runs.len() <= 128, "{} runs", body.runs.len());
        assert!(body.runs.len() > 100, "the cap is reached, not lower");
        let first = body.runs.first().and_then(link_of).map(|h| h.0.clone());
        let first = first.unwrap_or_else(|| panic!("the first run links"));
        assert!(
            body.runs
                .iter()
                .filter_map(link_of)
                .all(|h| std::sync::Arc::ptr_eq(&h.0, &first)),
            "every run points at one copy"
        );
    }

    #[test]
    fn a_body_is_read_only_up_to_its_ceiling() {
        let body = parse_body(&"word ".repeat(5000));
        assert!(body.text().len() <= 4096, "{}", body.text().len());
        assert!(body.text().len() > 4000);
    }
}
