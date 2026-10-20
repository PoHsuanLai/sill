//! `use_hours`: the clock the desktop chose (`[region] hours`), with Automatic settled by the
//! session's locale. Every surface that writes a time of day asks here, so the bar, the lock
//! screen and the launcher agree.

use dioxus::prelude::{try_consume_context, use_hook};
use ds_settings::{ResolvedHours, time_locale};
use sill_model::clock::ClockHours;

use super::use_appearance;

/// The locale a surface writes times in, in place of the process's: a test or a headless render
/// provides it as a root context so its text never depends on the machine it runs on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedLocale(pub String);

/// The hours to write a time of day in: `region.hours` of the desktop's file, read live.
pub fn use_hours() -> ClockHours {
    let pinned = use_hook(try_consume_context::<PinnedLocale>);
    let locale = use_hook(move || pinned.map_or_else(process_locale, |pinned| pinned.0));
    clock_hours(use_appearance()().hours(&locale))
}

/// The time locale the process was started with: `LC_ALL`, `LC_TIME`, then `LANG`, read here at
/// the edge, once per surface.
fn process_locale() -> String {
    let var = |name: &str| std::env::var(name).ok();
    time_locale(
        var("LC_ALL").as_deref(),
        var("LC_TIME").as_deref(),
        var("LANG").as_deref(),
    )
    .to_owned()
}

/// `hours` in the shell's own words.
fn clock_hours(hours: ResolvedHours) -> ClockHours {
    match hours {
        ResolvedHours::H12 => ClockHours::H12,
        ResolvedHours::H24 => ClockHours::H24,
    }
}

#[cfg(test)]
mod tests {
    use ds_settings::ResolvedHours;
    use sill_model::clock::ClockHours;

    use super::clock_hours;

    #[test]
    fn the_desktops_hours_become_the_shells() {
        assert_eq!(clock_hours(ResolvedHours::H12), ClockHours::H12);
        assert_eq!(clock_hours(ResolvedHours::H24), ClockHours::H24);
    }
}
