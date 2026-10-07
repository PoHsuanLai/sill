//! The widget registry (design/23-WIDGETS.md section 9.4): every widget a host can place, by
//! kind, with what a picker needs to show it (its name, its sizes, its placeholder drawn on the
//! card) without knowing its type. A host builds one ([`WidgetRegistry::quire`] holds quire's
//! own), adds its own and the apps' widgets, and provides it with
//! [`provide_widget_registry`]; a picker reads it with [`use_widget_registry`].
//!
//! Each widget takes one size per host (the user's decision, 2026-09-28): its own
//! [`Widget::size_in`], which a host narrows per kind with [`WidgetRegistry::sized`] (sill's
//! `size_of`, `widgets.sizes`), so Edit Widgets previews and adds that one size.

use crate::widget::battery::BatteryWidget;
use crate::widget::calendar::MonthWidget;
use crate::widget::card::WidgetCard;
use crate::widget::clock::WorldClockWidget;
use crate::widget::contract::{Widget, WidgetKind, fit};
use crate::widget::kind::{Lift, WidgetHost, WidgetSize};
use crate::widget::timeline::Timeline;
use dioxus::prelude::*;
use ds::components::content::text_runs::TextLine;

/// One registered widget, its type erased.
#[derive(Clone)]
pub struct WidgetInfo {
    /// Its kind.
    pub kind: WidgetKind,
    /// Its name in a picker.
    pub name: TextLine,
    /// Its one line in a picker.
    pub description: TextLine,
    /// The sizes it draws.
    pub sizes: &'static [WidgetSize],
    hosts: HostSizes,
    preview: fn(WidgetSize, WidgetHost, Lift) -> Element,
}

/// The one size a widget takes in each host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct HostSizes {
    desktop: WidgetSize,
    tile: WidgetSize,
}

impl HostSizes {
    fn of<W: Widget>() -> Self {
        HostSizes {
            desktop: fit::<W>(W::size_in(WidgetHost::Desktop)),
            tile: fit::<W>(W::size_in(WidgetHost::Tile)),
        }
    }

    fn get(self, host: WidgetHost) -> WidgetSize {
        match host {
            WidgetHost::Desktop => self.desktop,
            WidgetHost::Tile => self.tile,
        }
    }

    fn with(self, host: WidgetHost, size: WidgetSize) -> Self {
        match host {
            WidgetHost::Desktop => HostSizes {
                desktop: size,
                ..self
            },
            WidgetHost::Tile => HostSizes { tile: size, ..self },
        }
    }
}

impl std::fmt::Debug for WidgetInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetInfo")
            .field("kind", &self.kind)
            .field("name", &self.name)
            .field("description", &self.description)
            .field("sizes", &self.sizes)
            .field("hosts", &self.hosts)
            .finish_non_exhaustive()
    }
}

impl PartialEq for WidgetInfo {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.name == other.name
            && self.description == other.description
            && self.sizes == other.sizes
            && self.hosts == other.hosts
    }
}

impl WidgetInfo {
    /// `W`'s entry in a registry.
    pub fn of<W: Widget>() -> Self {
        WidgetInfo {
            kind: W::kind(),
            name: W::name(),
            description: W::description(),
            sizes: W::sizes(),
            hosts: HostSizes::of::<W>(),
            preview: preview::<W>,
        }
    }

    /// The one size it takes in `host`: what a picker previews and adds.
    pub fn size_in(&self, host: WidgetHost) -> WidgetSize {
        self.hosts.get(host)
    }

    /// Its card at `size` in `host`, showing its preview entry, lifted or at rest: what a
    /// picker shows.
    pub fn preview(&self, size: WidgetSize, host: WidgetHost, lift: Lift) -> Element {
        (self.preview)(size, host, lift)
    }
}

/// `W`'s card showing its preview entry.
fn preview<W: Widget>(size: WidgetSize, host: WidgetHost, lift: Lift) -> Element {
    rsx! {
        WidgetCard::<W> { widget: W::default(), timeline: Timeline::now(W::preview(size)), size, host, lift }
    }
}

/// A kind registered twice.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the widget kind {0:?} is already registered")]
pub struct TakenKind(pub WidgetKind);

/// A size asked of a kind the registry does not hold, or that the kind does not draw.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the widget kind {kind:?} is not registered or does not draw {size:?}")]
pub struct UnsizedKind {
    /// The kind asked about.
    pub kind: WidgetKind,
    /// The size asked for.
    pub size: WidgetSize,
}

/// The widgets a host can place, in the order they were added.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WidgetRegistry {
    infos: Vec<WidgetInfo>,
}

impl WidgetRegistry {
    /// quire's own widgets: the batteries, the world clock and the month.
    pub fn quire() -> Self {
        WidgetRegistry::default()
            .with::<BatteryWidget>()
            .and_then(WidgetRegistry::with::<WorldClockWidget>)
            .and_then(WidgetRegistry::with::<MonthWidget>)
            .unwrap_or_default()
    }

    /// This registry with `W` added, or the kind it already holds.
    pub fn with<W: Widget>(self) -> Result<Self, TakenKind> {
        let info = WidgetInfo::of::<W>();
        if self.get(&info.kind).is_some() {
            return Err(TakenKind(info.kind));
        }
        let mut infos = self.infos;
        infos.push(info);
        Ok(WidgetRegistry { infos })
    }

    /// This registry with `kind` taking `size` in `host` (a host's own choice, as sill's
    /// `size_of` is), or the kind it does not hold or the size it does not draw.
    pub fn sized(
        self,
        kind: &WidgetKind,
        host: WidgetHost,
        size: WidgetSize,
    ) -> Result<Self, UnsizedKind> {
        let unsized_kind = || UnsizedKind {
            kind: kind.clone(),
            size,
        };
        let at = self
            .infos
            .iter()
            .position(|info| info.kind == *kind && info.sizes.contains(&size))
            .ok_or_else(unsized_kind)?;
        let mut infos = self.infos;
        infos[at].hosts = infos[at].hosts.with(host, size);
        Ok(WidgetRegistry { infos })
    }

    /// The widget of `kind`.
    pub fn get(&self, kind: &WidgetKind) -> Option<&WidgetInfo> {
        self.infos.iter().find(|info| info.kind == *kind)
    }

    /// Every widget, in the order added.
    pub fn all(&self) -> &[WidgetInfo] {
        &self.infos
    }
}

/// Provide `registry` to the subtree.
pub fn provide_widget_registry(registry: WidgetRegistry) -> WidgetRegistry {
    use_context_provider(|| registry)
}

/// The registry an ancestor provided, or quire's own when none did.
pub fn use_widget_registry() -> WidgetRegistry {
    try_use_context::<WidgetRegistry>().unwrap_or_else(WidgetRegistry::quire)
}

#[cfg(test)]
mod tests {
    use super::{TakenKind, UnsizedKind, WidgetRegistry};
    use crate::widget::battery::BatteryWidget;
    use crate::widget::contract::{Widget, WidgetKind};
    use crate::widget::kind::{WidgetHost, WidgetSize};

    #[test]
    fn quire_registers_its_three_widgets_once_each() {
        let registry = WidgetRegistry::quire();
        let kinds: Vec<&str> = registry
            .all()
            .iter()
            .map(|info| info.kind.as_str())
            .collect();
        assert_eq!(kinds, ["quire.battery", "quire.world-clock", "quire.month"]);
        assert_eq!(
            registry
                .get(&WidgetKind::fixed("quire.battery"))
                .map(|info| info.sizes),
            Some(BatteryWidget::sizes())
        );
        assert_eq!(
            registry.with::<BatteryWidget>(),
            Err(TakenKind(WidgetKind::fixed("quire.battery")))
        );
    }

    #[test]
    fn each_widget_takes_one_size_per_host_and_a_host_may_choose_it() {
        let registry = WidgetRegistry::quire();
        let size = |registry: &WidgetRegistry, kind: &'static str, host| {
            registry
                .get(&WidgetKind::fixed(kind))
                .map(|info| info.size_in(host))
        };
        const CASES: &[(&str, WidgetHost, WidgetSize)] = &[
            ("quire.battery", WidgetHost::Desktop, WidgetSize::Small),
            ("quire.battery", WidgetHost::Tile, WidgetSize::Medium),
            ("quire.world-clock", WidgetHost::Desktop, WidgetSize::Medium),
            ("quire.world-clock", WidgetHost::Tile, WidgetSize::Medium),
            ("quire.month", WidgetHost::Desktop, WidgetSize::Small),
            ("quire.month", WidgetHost::Tile, WidgetSize::Large),
        ];
        for &(kind, host, want) in CASES {
            assert_eq!(size(&registry, kind, host), Some(want), "{kind} {host:?}");
        }
        let month = WidgetKind::fixed("quire.month");
        let chosen = registry
            .clone()
            .sized(&month, WidgetHost::Desktop, WidgetSize::Medium)
            .expect("the month draws Medium");
        assert_eq!(
            size(&chosen, "quire.month", WidgetHost::Desktop),
            Some(WidgetSize::Medium)
        );
        assert_eq!(
            size(&chosen, "quire.month", WidgetHost::Tile),
            Some(WidgetSize::Large),
            "the other host keeps its own"
        );
        let battery = WidgetKind::fixed("quire.battery");
        assert_eq!(
            registry
                .clone()
                .sized(&battery, WidgetHost::Desktop, WidgetSize::Large),
            Err(UnsizedKind {
                kind: battery,
                size: WidgetSize::Large
            }),
            "the batteries draw no Large"
        );
        let unknown = WidgetKind::fixed("org.example.none");
        assert!(
            registry
                .sized(&unknown, WidgetHost::Tile, WidgetSize::Small)
                .is_err()
        );
    }
}
