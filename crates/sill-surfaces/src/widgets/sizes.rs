//! `widgets.sizes`: the sizes the person picked in Edit Widgets, as `;`-separated
//! `host.kind=size` choices (`desktop.calendar=medium; center.up_next=large`). A widget with no
//! choice, or a choice its kind does not draw, takes its host's usual size ([`super::size_of`]).
//! Only a size other than the usual one is written, so the key stays empty until someone picks
//! one. Text, as `widgets.world_clocks` is, until `layout.toml` carries each placement's size
//! (design/28 section 5.6). Pure.

use super::{WidgetHost, WidgetKind, WidgetSize, offered, size_of};

/// One picked size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SizeChoice {
    pub host: WidgetHost,
    pub kind: WidgetKind,
    pub size: WidgetSize,
}

/// Every picked size, in the key's order.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Sizes(pub Vec<SizeChoice>);

impl Sizes {
    /// `widgets.sizes`; an entry it cannot read is skipped, a later one for the same widget wins.
    pub fn parse(text: &str) -> Sizes {
        text.split(';')
            .filter_map(choice_of)
            .fold(Sizes::default(), |sizes, choice| {
                sizes.with(choice.kind, choice.host, choice.size)
            })
    }

    /// The key's text.
    pub fn text(&self) -> String {
        self.0
            .iter()
            .map(|c| {
                let (host, kind, size) = (host_word(c.host), kind_word(c.kind), c.size.slug());
                format!("{host}.{kind}={size}")
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// The size `kind` is drawn at in `host`.
    pub fn of(&self, kind: WidgetKind, host: WidgetHost) -> WidgetSize {
        self.0
            .iter()
            .find(|c| c.kind == kind && c.host == host)
            .map(|c| c.size)
            .filter(|size| offered(kind).contains(size))
            .unwrap_or_else(|| size_of(kind, host))
    }

    /// These sizes with `kind` in `host` at `size`: the usual size is written as no choice.
    pub fn with(self, kind: WidgetKind, host: WidgetHost, size: WidgetSize) -> Sizes {
        let others = self
            .0
            .into_iter()
            .filter(|c| (c.kind, c.host) != (kind, host));
        let picked = (size != size_of(kind, host)).then_some(SizeChoice { host, kind, size });
        Sizes(others.chain(picked).collect())
    }
}

fn choice_of(entry: &str) -> Option<SizeChoice> {
    let (place, size) = entry.trim().split_once('=')?;
    let (host, kind) = place.trim().split_once('.')?;
    Some(SizeChoice {
        host: [WidgetHost::Desktop, WidgetHost::Tile]
            .into_iter()
            .find(|h| host_word(*h) == host.trim())?,
        kind: super::ALL_KINDS
            .into_iter()
            .find(|k| kind_word(*k) == kind.trim())?,
        size: [WidgetSize::Small, WidgetSize::Medium, WidgetSize::Large]
            .into_iter()
            .find(|s| s.slug() == size.trim())?,
    })
}

/// The host as the key spells it: the notification center is `center`, as in `widgets.center`.
fn host_word(host: WidgetHost) -> &'static str {
    match host {
        WidgetHost::Desktop => "desktop",
        WidgetHost::Tile => "center",
    }
}

/// The kind as the settings spell it (`widgets.center`'s words).
fn kind_word(kind: WidgetKind) -> &'static str {
    match kind {
        WidgetKind::Calendar => "calendar",
        WidgetKind::UpNext => "up_next",
        WidgetKind::Battery => "battery",
        WidgetKind::NowPlaying => "now_playing",
        WidgetKind::WorldClock => "world_clock",
    }
}

#[cfg(test)]
mod tests {
    use super::{SizeChoice, Sizes, WidgetHost, WidgetKind, WidgetSize};

    #[test]
    fn the_key_reads_choices_and_skips_what_it_cannot() {
        let choice = |host, kind, size| SizeChoice { host, kind, size };
        let cases = [
            ("", vec![]),
            (
                "desktop.calendar=medium",
                vec![choice(
                    WidgetHost::Desktop,
                    WidgetKind::Calendar,
                    WidgetSize::Medium,
                )],
            ),
            (
                " center.up_next = large ; desktop.nothing=small; desk.battery=small; x",
                vec![choice(
                    WidgetHost::Tile,
                    WidgetKind::UpNext,
                    WidgetSize::Large,
                )],
            ),
            (
                "desktop.battery=medium; desktop.battery=large",
                vec![choice(
                    WidgetHost::Desktop,
                    WidgetKind::Battery,
                    WidgetSize::Large,
                )],
            ),
            ("desktop.battery=small", vec![]),
        ];
        for (text, want) in cases {
            assert_eq!(Sizes::parse(text).0, want, "{text:?}");
        }
    }

    #[test]
    fn a_widget_takes_its_pick_when_its_kind_draws_it_else_its_usual_size() {
        let sizes =
            Sizes::parse("desktop.calendar=large; center.battery=small; desktop.now_playing=small");
        let cases = [
            (WidgetKind::Calendar, WidgetHost::Desktop, WidgetSize::Large),
            (WidgetKind::Calendar, WidgetHost::Tile, WidgetSize::Large),
            (WidgetKind::Battery, WidgetHost::Tile, WidgetSize::Small),
            (WidgetKind::Battery, WidgetHost::Desktop, WidgetSize::Small),
            (
                WidgetKind::NowPlaying,
                WidgetHost::Desktop,
                WidgetSize::Medium,
            ),
            (WidgetKind::UpNext, WidgetHost::Desktop, WidgetSize::Medium),
        ];
        for (kind, host, want) in cases {
            assert_eq!(sizes.of(kind, host), want, "{kind:?} {host:?}");
        }
    }

    #[test]
    fn a_pick_writes_only_what_differs_from_the_usual() {
        let sizes = Sizes::default()
            .with(
                WidgetKind::Calendar,
                WidgetHost::Desktop,
                WidgetSize::Medium,
            )
            .with(WidgetKind::UpNext, WidgetHost::Tile, WidgetSize::Large);
        assert_eq!(
            sizes.text(),
            "desktop.calendar=medium; center.up_next=large"
        );
        let back = sizes.with(WidgetKind::Calendar, WidgetHost::Desktop, WidgetSize::Small);
        assert_eq!(back.text(), "center.up_next=large");
        assert_eq!(Sizes::parse(&back.text()), back);
    }
}
