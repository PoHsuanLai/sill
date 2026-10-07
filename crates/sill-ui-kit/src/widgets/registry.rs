//! The widgets sill can place: every kind in the settings' order, each named as quire's registry
//! names it, the registry with sill's two widgets added and every kind held to its one size in
//! each host, and the stylesheets sill's own widgets need.

use ds::prelude::*;
use sill_settings::WidgetKind;
use sill_shell_kit::prelude::*;
use sill_shell_kit::widget::registry::provide_widget_registry;

use super::{OnProvider, WidgetProvider, now_playing, size_of, up_next, with_provider};

/// `kind`'s name in quire's registry.
pub fn kind_id(kind: WidgetKind) -> sill_shell_kit::prelude::WidgetKind {
    struct RegistryId;
    impl OnProvider for RegistryId {
        type Out = sill_shell_kit::prelude::WidgetKind;
        fn on<P: WidgetProvider>(self) -> sill_shell_kit::prelude::WidgetKind {
            P::registry_id()
        }
    }
    with_provider(kind, RegistryId)
}

/// The settings' kind named `id`; none for a widget sill does not place.
pub fn kind_of(id: &sill_shell_kit::prelude::WidgetKind) -> Option<WidgetKind> {
    WidgetKind::ALL
        .iter()
        .copied()
        .find(|kind| kind_id(*kind) == *id)
}

/// Every widget sill can place, quire's three and sill's two, each taking [`size_of`] in each
/// host, so quire's gallery previews and adds that one size and offers no other.
pub fn registry() -> WidgetRegistry {
    let quire = WidgetRegistry::quire();
    match all_sized(quire.clone()) {
        Ok(all) => all,
        Err(why) => {
            eprintln!("sill: widget registry: {why}");
            quire
        }
    }
}

/// Why [`registry`] could not be built as sill wants it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum RegistryError {
    #[error("{0}")]
    Taken(#[from] sill_shell_kit::widget::TakenKind),
    #[error("{0}")]
    Unsized(#[from] sill_shell_kit::widget::UnsizedKind),
}

/// `quire` with sill's two widgets added and every kind held to its one size in each host.
pub(super) fn all_sized(quire: WidgetRegistry) -> Result<WidgetRegistry, RegistryError> {
    let all = quire
        .with::<up_next::UpNext>()?
        .with::<now_playing::NowPlaying>()?;
    let hosts = [WidgetHost::Tile, WidgetHost::Desktop];
    let sized = WidgetKind::ALL
        .iter()
        .flat_map(|kind| hosts.map(|host| (*kind, host)))
        .try_fold(all, |registry, (kind, host)| {
            registry.sized(&kind_id(kind), host, size_of(kind, host))
        })?;
    Ok(sized)
}

/// Provide [`registry`] to a host root's subtree (the desktop layer, the notification center,
/// Edit Widgets).
pub fn provide_registry() {
    provide_widget_registry(registry());
}

/// The element id of a widget's card on the desktop layer: its input and blur region.
pub fn widget_element_id(kind: WidgetKind) -> String {
    format!("widget-{}", kind.slug())
}

/// sill's own widgets' stylesheets (quire's widgets style themselves), for a host root to inject
/// beside its own: one widget's rules per file.
pub fn widgets_css() -> String {
    [up_next::CSS, now_playing::CSS].join("\n")
}
