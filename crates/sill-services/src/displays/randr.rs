//! The compositor's own view of its outputs, as `cosmic-randr list --kdl` prints it (cosmic-comp
//! 1.8 answers it over `zwlr_output_manager_v1`): names, the current mode, the physical size it
//! was told, and the scale. Pure: text in, [`Head`]s out. Only the lines sill needs are read;
//! anything else in a block is skipped.

use serde::{Deserialize, Serialize};

use super::edid::{Millimetres, Mode, Pixels};
use super::scale::DisplayScale;
use sill_model::ids::OutputName;

/// Whether the compositor is driving an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Enablement {
    #[default]
    Enabled,
    Disabled,
}

/// One output as the compositor reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Head {
    /// The connector name (`HDMI-A-2`; `WINIT-0` for a nested compositor's window).
    pub name: OutputName,
    pub enablement: Enablement,
    pub make: String,
    pub model: String,
    /// `None` when the compositor reports 0 x 0.
    pub physical: Option<Millimetres>,
    /// The mode marked `current`.
    pub current: Option<Mode>,
    pub scale: DisplayScale,
}

impl Default for Head {
    /// A nameless, enabled head at scale 1 with nothing known: what a block starts as.
    fn default() -> Self {
        Head {
            name: OutputName(String::new()),
            enablement: Enablement::Enabled,
            make: String::new(),
            model: String::new(),
            physical: None,
            current: None,
            scale: DisplayScale::ONE,
        }
    }
}

/// Every `output` block in `kdl`.
pub fn parse_heads(kdl: &str) -> Vec<Head> {
    let mut heads = Vec::new();
    let mut open: Option<Head> = None;
    for line in kdl.lines().map(str::trim) {
        let words = tokens(line);
        match (words.first().map(String::as_str), open.as_mut()) {
            (Some("output"), _) => {
                heads.extend(open.take());
                open = Some(head_line(&words));
            }
            (Some(_), Some(head)) => field_line(head, &words),
            _ => {}
        }
    }
    heads.extend(open);
    heads
}

/// `output "HDMI-A-2" enabled=#true {`.
fn head_line(words: &[String]) -> Head {
    Head {
        name: OutputName(words.get(1).cloned().unwrap_or_default()),
        enablement: match property(words, "enabled").as_deref() {
            Some("#false") => Enablement::Disabled,
            _ => Enablement::Enabled,
        },
        ..Head::default()
    }
}

fn field_line(head: &mut Head, words: &[String]) {
    let number = |i: usize| words.get(i).and_then(|w| w.parse::<u32>().ok());
    match words.first().map(String::as_str) {
        Some("description") => {
            head.make = property(words, "make").unwrap_or_default();
            head.model = property(words, "model").unwrap_or_default();
        }
        Some("physical") => {
            head.physical = match (number(1), number(2)) {
                (Some(width), Some(height)) if width > 0 && height > 0 => {
                    Some(Millimetres { width, height })
                }
                _ => None,
            };
        }
        Some("scale") => {
            if let Some(scale) = words.get(1).and_then(|w| DisplayScale::parse(w)) {
                head.scale = scale;
            }
        }
        Some("mode") if property(words, "current").as_deref() == Some("#true") => {
            if let (Some(width), Some(height)) = (number(1), number(2)) {
                head.current = Some(Mode {
                    pixels: Pixels { width, height },
                    refresh_mhz: number(3).unwrap_or(0),
                });
            }
        }
        _ => {}
    }
}

/// `key=value` among `words`, with a quoted value unquoted.
fn property(words: &[String], key: &str) -> Option<String> {
    words.iter().find_map(|w| {
        let (k, v) = w.split_once('=')?;
        (k == key).then(|| v.to_owned())
    })
}

/// Whitespace-separated words, where a double-quoted run (with `\"` escapes) is one word and
/// loses its quotes, also after `key=`.
fn tokens(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', _) => quoted = !quoted,
            ('\\', true) => word.extend(chars.next()),
            (c, false) if c.is_whitespace() => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            }
            (c, _) => word.push(c),
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::{Enablement, Head, parse_heads};
    use crate::displays::edid::{Millimetres, Mode, Pixels};
    use crate::displays::scale::DisplayScale;
    use sill_model::ids::OutputName;

    /// What the nested cosmic-comp 1.8 printed (dev/nested.sh), plus a second, real-looking head.
    const LIST: &str = r#"output "WINIT-0" enabled=#true {
  description make="COSMIC" model="WINIT-0"
  physical 0 0
  position 0 0
  scale 1.00
  transform "flipped180"
  adaptive_sync_support #false
  adaptive_sync "automatic"
  xwayland_primary #true
  modes {
    mode 2176 1360 60000 current=#true preferred=#true
  }
}
output "HDMI-A-2" enabled=#false {
  description make="BenQ" model="BenQ \"MA270U\""
  physical 597 336
  scale 1.50
  modes {
    mode 3840 2160 59997 preferred=#true
    mode 2560 1440 59951 current=#true
  }
}
"#;

    #[test]
    fn the_nested_and_a_real_head_parse() {
        let mode = |width, height, refresh_mhz| Mode {
            pixels: Pixels { width, height },
            refresh_mhz,
        };
        let want = vec![
            Head {
                name: OutputName("WINIT-0".to_owned()),
                enablement: Enablement::Enabled,
                make: "COSMIC".to_owned(),
                model: "WINIT-0".to_owned(),
                physical: None,
                current: Some(mode(2176, 1360, 60000)),
                scale: DisplayScale(100),
            },
            Head {
                name: OutputName("HDMI-A-2".to_owned()),
                enablement: Enablement::Disabled,
                make: "BenQ".to_owned(),
                model: "BenQ \"MA270U\"".to_owned(),
                physical: Some(Millimetres {
                    width: 597,
                    height: 336,
                }),
                current: Some(mode(2560, 1440, 59951)),
                scale: DisplayScale(150),
            },
        ];
        assert_eq!(parse_heads(LIST), want);
    }

    #[test]
    fn nothing_parses_to_nothing() {
        assert_eq!(parse_heads(""), Vec::<Head>::new());
        assert_eq!(parse_heads("error: no compositor\n"), Vec::<Head>::new());
    }
}
