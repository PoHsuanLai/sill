//! The picker's render: one transparent viewport root holding, while an offer shows on this
//! output, a key target (focused per offer, so Escape and the arrows reach it), the dimmed
//! backdrop that turns the choice down when pressed, and, in the half the first window left
//! free, quire's `AppSwitcher` with one tile per window on offer. The tile's key is the window's
//! identifier, its name the window's title, its icon the app's.
//!
//! [`super::io`] follows the link and writes the answer; this file only draws.

use dioxus::prelude::*;
use ds::focus::soon::focus_soon;
use ds::prelude::*;
use shell_host::OutputInfo;
use sill_launcher::AppId;
use sill_model::desktop::SplitCard;
use sill_shell_kit::prelude::*;
use sill_shell_kit::switcher::app_switcher::AppKey;
use sill_shell_kit::switcher::app_switcher::SwitcherApp as Tile;

use super::io::use_picker;
use super::model::{Step, index_of, moved, other_half};
use sill_ui_kit::app_icons::{LogicalPx, TileIcons, icon_side, metrics_of, use_tile_icons};
use sill_ui_kit::hooks::use_settings;
use sill_ui_kit::links::SplitPickerLink;
use sill_ui_kit::root::SurfaceRoot;

/// The picker's stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const SPLIT_PICKER_CSS: &str = include_str!("style.css");

pub(super) fn split_picker(output: OutputInfo) -> Element {
    let file = use_settings()();
    let icons = use_tile_icons(&output, LogicalPx(icon_side(&file.switcher).0));
    let link = use_context::<SplitPickerLink>();
    let picker = use_picker(&link, &output);
    let mut selected = use_signal(|| None::<String>);
    let metrics = metrics_of(&file.switcher);
    let offer = picker.shown();
    rsx! {
        SurfaceRoot {
            output: output.clone(),
            material: Material::Sheet,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            AppStyle { css: SPLIT_PICKER_CSS }
            if let Some(offer) = offer {
                {
                    let windows: Vec<&str> = offer.cards.iter().map(|c| c.window.as_str()).collect();
                    let current = selected
                        .read()
                        .clone()
                        .filter(|key| index_of(&windows, key).is_some())
                        .or_else(|| windows.first().map(|first| (*first).to_owned()));
                    let tiles: Vec<Tile> = offer.cards.iter().map(|card| tile(card, &icons)).collect();
                    let by_key = picker.clone();
                    let by_press = picker.clone();
                    let by_pick = picker.clone();
                    let listed: Vec<String> = windows.iter().map(|w| (*w).to_owned()).collect();
                    rsx! {
                        div {
                            key: "{offer.serial.0}",
                            class: "sill-split",
                            tabindex: "0",
                            onmounted: move |event: MountedEvent| focus_soon(event.data()),
                            onkeydown: move |event: KeyboardEvent| {
                                let all: Vec<&str> = listed.iter().map(String::as_str).collect();
                                let at = selected
                                    .peek()
                                    .as_deref()
                                    .and_then(|key| index_of(&all, key))
                                    .unwrap_or(0);
                                match event.key() {
                                    Key::Escape => by_key.cancel(),
                                    Key::Enter => {
                                        if let Some(window) = all.get(at) {
                                            by_key.pick(window);
                                        }
                                    }
                                    Key::ArrowRight => select(&mut selected, &all, moved(all.len(), at, Step::Forward)),
                                    Key::ArrowLeft => select(&mut selected, &all, moved(all.len(), at, Step::Back)),
                                    _ => {}
                                }
                            },
                            div {
                                class: "sill-split-scrim",
                                onpointerdown: move |_| by_press.cancel(),
                            }
                            if let Some(current) = current {
                                div { class: "sill-split-stage", "data-half": other_half(offer.side),
                                    div { class: "sill-split-panel",
                                        AppSwitcher {
                                            apps: tiles,
                                            selected: AppKey(current),
                                            metrics,
                                            onhover: move |key: AppKey| selected.set(Some(key.0)),
                                            onactivate: move |key: AppKey| by_pick.pick(&key.0),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The tile for one window on offer.
fn tile(card: &SplitCard, icons: &TileIcons) -> Tile {
    Tile::new(
        card.window.clone(),
        card.title.clone(),
        icons.source(&AppId(card.app.clone())),
    )
}

/// Select the tile at `index` among `windows`.
fn select(selected: &mut Signal<Option<String>>, windows: &[&str], index: usize) {
    selected.set(windows.get(index).map(|window| (*window).to_owned()));
}
