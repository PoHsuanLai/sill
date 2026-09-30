# Architecture

sill is a desktop shell for COSMIC: the bar, dock, launcher, wallpaper, notifications, control
center, lock screen and the rest, drawn with quire's design system on shell-host (Blitz on
Wayland layer surfaces). cosmic-comp is the compositor. KWin is a test compositor only.
CONVENTIONS.md holds the rules every repo shares; this file is sill's map, its "one home" table,
its traits, its recipes and its repo rules. Where they disagree, this file wins for sill.

## Sibling repositories

| Repo | What sill takes from it |
| --- | --- |
| quire (`../quire`, path deps) | `ds-core` (vocabulary, units, `Word`, colour, time), `ds-style` (appearance, tokens, materials), `ds-motion` (timelines, `Presence`, gestures), `ds` (components, `DocumentHost`, menus, `ds::prelude`), `ds-shell` (shell components, widget contract, status glyphs, battery), `ds-lint`, `ds-settings` (`SettingsDoc`, `Store`, schema derive; dioxus behind a feature), `ds-native` (`provide_host()`, measure, focus, caret, snap, `Harness`, `Driver`, `DocQuery`) |
| shell-host (`../shell-host`) | Host, layer surfaces and popups, input, frames, regions, session lock, scroll engine, `Visibility`, `Inject` (pointer and key, `debug-inject`) |
| latchkey (git) | the single-instance agent socket |

Consumers of quire import `ds::prelude::*` plus the named crate for anything outside it.

---

## 1. Crate map and layers

Layers, lowest first. A crate depends only on the crates listed for it; every other edge is
forbidden and Cargo enforces it (a surface crate that does not list another cannot name it).
`scripts/check-boundary.sh` keeps the table exact: its `EDGES` rows are each crate's direct normal
and build path dependencies, sill and quire and shell-host alike; dev dependencies are not
listed except where noted.

| # | Crate | What it is | May depend on (sill crates, then key externals) |
| --- | --- | --- | --- |
| 0 | `sill-launcher` | Launcher core: `Provider`, `Query` (generation, cancel), `Item`, `Activation`, matching (nucleo), ranking, sections, frecency, chords, preview facts. Pure; a future repo | none; nucleo-matcher, serde |
| 0 | `sill-testkit` | Dev only: `PrivateBus`, `TempDir`, signal helpers, stand-in programs in `examples/` | none; std only (its dev dependencies, the crates it tests, are outside the table) |
| 1 | `sill-model` | Pure vocabulary: ids, units, `Attempt`, every service's `State`/`Command`/`Event`/`Effect` and its `step`, vocabulary enums shared by a settings key and a service | `sill-launcher`; serde, jiff, thiserror |
| 2 | `sill-settings` | `settings.toml` as data: one struct per domain, `Section`, `ShellFile`, schema, typed values parsed at load. No I/O, no runtime, no dioxus | `sill-model`, `ds-core`, `ds-settings` (no features) |
| 2 | `sill-ipc` | The wire: `Request`/`Response`, one JSON line, 64 KiB cap. Serialises `sill-model` types | `sill-model`, `sill-launcher`; serde, serde_json, thiserror |
| 3 | `sill-services` | Effects: `Env` seams, `Service` trait, runner, `Handle`, `Control`, one dir per service `{mod.rs, backend/}`. UI-free | `sill-model`, `sill-launcher`; tokio, zbus, cctk, libpulse, notify, nonstick, ddc, image |
| 4 | `sill-search` | The launcher's runner (`ResultsService`), `providers/`, excerpts and clip images | `sill-services`, `sill-model`, `sill-launcher`, `ds-settings` (no features: the schema files the settings provider indexes); fend, emojis |
| 5 | `sill-ui-kit` | What every surface shares: `Services` registry, `SettingsFeed`, `SurfaceRoot`, `Surface` and its `mount`, `links/`, `Machine`, hooks, `look`, `panel`, `shell_style`, `MenuPopup<M>`, `app_icons`, `status`, `notification_card`, `widgets/`, `harness` (feature: fake services, `SurfaceHarness`) | `sill-search`, `sill-services`, `sill-settings`, `sill-model`, `sill-launcher`, `ds`, `ds-shell`, `ds-settings` (dioxus), `ds-native`, `shell-host`; dioxus, resvg, image; dev: `ds-lint` |
| 6 | `sill-bar` | Bar, its menus, tray, control center and its modules | `sill-ui-kit`, `sill-model`, `sill-settings`, `sill-launcher`, `sill-services`*; `ds`, `ds-shell`, `ds-settings`, `ds-native`, `shell-host`; dev: `ds-lint` |
| 6 | `sill-dock` | Dock, magnify, context menu | `sill-ui-kit`, `sill-model`, `sill-settings`, `sill-launcher`, `sill-services`*; `ds`, `ds-shell`, `ds-native`, `shell-host`; dev: `ds-lint` |
| 6 | `sill-launcher-ui` | Launcher panel and catcher, rows, preview pane, actions | `sill-ui-kit`, `sill-model`, `sill-settings`, `sill-launcher`, `sill-search`*, `sill-services`*; `ds`, `ds-native`, `shell-host`; dev: `ds-lint`, `ds-shell` |
| 6 | `sill-notify-ui` | Banners, notification center, screenshot thumbnail | `sill-ui-kit`, `sill-model`, `sill-settings`, `sill-services`*; `ds`, `ds-shell`, `ds-settings`, `ds-native`, `shell-host` |
| 6 | `sill-widgets-ui` | Desktop widget layer, Edit Widgets | `sill-ui-kit`, `sill-settings`; `ds`, `ds-shell`, `shell-host`; dev: `sill-model`, `ds-lint` |
| 6 | `sill-overlays` | OSD, power menu, polkit prompt, switcher, idle dim, hot corners, wallpaper, lock | `sill-ui-kit`, `sill-model`, `sill-settings`, `sill-launcher`, `sill-services`*; `ds`, `ds-shell`, `ds-settings`, `shell-host` |
| 7 | `sill` | Binary and library: `cli/`, `daemon/`, `ipc/`, `policy/` | every crate above except `sill-testkit`; `ds-native`, `ds-settings`, `shell-host`; clap, latchkey, `ds-lint` and `ds-shell` (`sill style check`); dev: `sill-testkit`, `sill-ui-kit` (harness), `ds` |

`*` A surface crate names `sill-services` (and `sill-search` for the launcher) only for service
marker types (`BatteryService`), `Control` and `Handle`, the type arguments of `use_service`.
It never names `Env`, `Service::run`, `spawn` or a `backend`. Surface crates never depend on one
another, and none depends on `sill`.

Boundary script (`scripts/check-boundary.sh`; `RULES` check the output of `cargo tree -i`, never
its status, and `EDGES` compare each crate's direct path dependencies with the table above):

| Crates | Reach none of (through any path) |
| --- | --- |
| `sill-launcher`, `sill-model`, `sill-ipc` | dioxus, dioxus-core, zbus, wayland-client, tokio, libpulse-binding, shell-host, ds, ds-shell, ds-native, blitz-dom |
| `sill-settings`, `sill-search` | dioxus, dioxus-core, blitz-dom, shell-host, ds, ds-shell, ds-native (`ds-settings` brings zbus and tokio to both) |
| `sill-services` | the same, and `ds-settings` |
| `sill-testkit` | every other sill crate |
| `sill-ui-kit` | every surface crate, `sill` |
| each surface crate | every other surface crate, `sill` |

## 2. Inside each crate: top-level modules, lowest layer first

A module names only modules above it in its own table. `mod.rs` and `lib.rs` declare and
re-export; the one type a module is named after may sit in its `mod.rs`.

| Crate | Modules (layer order) |
| --- | --- |
| `sill-model` | `units`, `ids`, `attempt`, `vocab` (enums shared with settings), then one dir per service (`battery`, `clock`, ... 27 services; `results` holds search state; `focus` holds the focus mode, which is not a service). Each service dir: `mod.rs`, `model.rs` (State), `command.rs` (Command, Event, Effect), `step.rs`, `tests.rs`; a sub-concept gets a sub-dir with the same roles. Service dirs never import each other |
| `sill-settings` | `section` (trait), one file per domain (`bar`, `dock`, ..., 20 domains), `file` (`ShellFile`, `SettingsDoc` impl), `schema`, `keys` (typed enums and unit newtypes that no service shares) |
| `sill-ipc` | `line` (encode, decode, cap), `error`, `request`, `response`, `show`, `status`, `act`, `debug` (one word file per request group) |
| `sill-services` | `error`, `control`, `stream`, `env/`, `service`, then one dir per service (26, all but `results`) in dependency order of their `Deps`; a service dir never imports another service dir: cross-service data is `sill-model` types in `Deps` |
| `sill-search` | `preview`, `providers/<kind>/`, `results/` (`registry`, `run`, `sink`, `history`, `frecency_file`, `effect`), `ResultsService` in `results/mod.rs` |
| `sill-ui-kit` | `look`, `clock`, `links/`, `settings`, `shell_style`, `services`, `hooks/`, `machine`, `panel`, `root`, `surface/` (`mount`, `lock`, `start`, `place`, `naming`), `menu/`, `status/`, `app_icons/`, `notification_card/`, `widgets/`, `harness/` (feature `harness`) |
| each surface crate | one dir per surface, named after it (`osd/`, `dock/`, `banners/`); `lib.rs` re-exports each surface marker type and nothing else |
| `sill` | `policy/`, `ipc/`, `cli/`, `daemon/` (`daemon` is the top: it names every crate) |

### The one shape of a surface (every dir in a surface crate)

| File | Holds |
| --- | --- |
| `mod.rs` | declarations only |
| `surface.rs` | the marker type named after the surface and its thin `impl Surface` |
| `layer.rs` | `fn layer(&OutputInfo, &ShellFile, slot) -> LayerConfig` and its constants |
| `model.rs` | state types (data only) |
| `step.rs` | the pure transitions; `impl Machine` for a timed one |
| `keys.rs` | the keymap table, when the surface takes keys |
| `io.rs` | `use_<surface>`: reads services and settings, follows the link, feeds `use_machine`, carries effects |
| `view.rs` | components; no decisions beyond reading the model |
| `style.css` | the concept's stylesheet, `include_str!`ed in `view.rs` |
| `tests.rs` | unit tests; integration tests are `crates/<crate>/tests/<surface>.rs` on `SurfaceHarness<S>` |

A concept over about 300 lines becomes a sub-directory with the same roles (`dock/machine/`,
`launcher/preview/`, `control_center/pages/`). No `drive.rs`, `link.rs`, `presence.rs`,
`session.rs`, `machine.rs`, `panel.rs`: those seven names for "the model" and "the io" are
`model.rs`, `step.rs`, `io.rs`.

---

## 3. Where today's code goes

### `sill-services` (ends with 26 services and the seams)

Rule for every service dir `X`: a file that names zbus, wayland, notify, `std::fs`, `Command`,
libpulse, PAM or tokio is effect code; every other file is pure. Pure files go to
`sill-model/src/X/` (state to `model.rs`, commands and events to `command.rs`, reducers to
`step.rs`, the rest keeps its name); effect files go to `sill-services/src/X/backend/`; the
`impl Service` stays in `X/mod.rs` and its `run` calls `backend::run`. `*_tests.rs` follow their
code into `tests.rs`. Exceptions and renames:

| From (`crates/sill-services/src/`) | To |
| --- | --- |
| `units.rs` | `sill-model/src/units.rs`; `Percent` becomes `ds_core`'s; `Presence` becomes `Found`, `Visibility` becomes `Revealed` (no clash with `ds-motion::Presence` or `shell_host::Visibility`) |
| `attempt.rs` | `sill-model/src/attempt.rs` |
| `apps/{exec,index,match_app}.rs` | `sill-model/src/apps/` |
| `apps/{scan,icons}.rs` | `apps/backend/` |
| `auth/{types,gate,protocol}.rs` | `sill-model/src/auth/` |
| `auth/{account,helper,pam,live}.rs` | `auth/backend/` (`pam.rs` is public: `sill auth-check` calls it) |
| `auth/authenticator.rs` | `env/authenticator.rs` (the `Authenticator` seam and `FakeAuthenticator`) |
| `bluetooth/{model,kind}.rs` | `sill-model/src/bluetooth/` |
| `bluetooth/{bluez,live}.rs` | `bluetooth/backend/{bluez,run,devices,pairing}.rs` (`live.rs` is 569 lines) |
| `network/{model}.rs`, `network/wifi/model.rs` | `sill-model/src/network/`, `sill-model/src/wifi/` |
| `network/live.rs`, `network/wifi/live.rs` | `network/backend/`, `wifi/backend/`; Wi-Fi is its own dir |
| `clipboard/{model,reduce,content,offer}.rs` | `sill-model/src/clipboard/` |
| `clipboard/{pipes,wire,wire_state,wire_dispatch,wl,live}.rs` | `clipboard/backend/` |
| `cosmic_wl/{ids,caps,command,describe,toplevels,workspaces}.rs` | `sill-model/src/cosmic_wl/` |
| `cosmic_wl/{execute,handlers,keys,publish,state,thread}.rs` | `cosmic_wl/backend/` |
| `cosmic_wl/nested.rs` | `crates/sill-services/tests/cosmic_wl_nested.rs` (`#[ignore]`, nested compositor) |
| `displays/{model,policy,identity,scale,scale_pass,plan,brightness,hw_volume,seen,gamut,warmth}*`, `displays/edid/`, `displays/ddc/{vcp,matching,retry,debounce}.rs` | `sill-model/src/displays/` (`scale/`, `edid/`, `ddc/`) |
| `displays/{backlight,drm,randr,output,sink,font,fontconfig_file,run}.rs`, `displays/ddc/{bus,i2c,worker}.rs` | `displays/backend/` |
| `displays/ddc/file_bus.rs`, `displays/ddc/fake.rs` | `env/ddc.rs` (`DdcBus`, `I2cBuses`, `FileBus`, `FakeBus`); `displays/kscreen*` is already gone |
| `events/{model,agenda,occur,rule,rule_expand,ics,ics_line,ics_value}.rs` | `sill-model/src/events/` |
| `events/{read,live,live_watch,sources,provider}.rs` | `events/backend/` |
| `idle/{model,policy,grace,reduce,reduce_rules}.rs` | `sill-model/src/idle/` (`reduce*` becomes `step.rs`) |
| `idle/{cosmic,inhibit,resolve,screensaver,logind,wl,wl_state,wl_dispatch,live}.rs` | `idle/backend/` |
| `launch/{model,plan}.rs` | `sill-model/src/launch/` |
| `launch/{dbus,open,run,terminal}.rs`, `launch/test_support.rs` | `launch/backend/`; `test_support.rs` goes to `sill-testkit::TempDir` |
| `locker/{cause,machine,mode}.rs` | `sill-model/src/locker/` (`machine.rs` becomes `step.rs`) |
| `locker/{marker,logind,live}.rs` | `locker/backend/` |
| `notifications/{state,reduce,rules,content,markup,markup_parse,ids,capabilities,policy}.rs` | `sill-model/src/notifications/` |
| `notifications/{dbus,wire,live,history_file,name_watch}.rs` | `notifications/backend/` |
| `pins/model.rs` | `sill-model/src/pins/` |
| `pins/{fs_watch,run}.rs`, `pins/test_support.rs` | `pins/backend/`; `test_support.rs` goes to `sill-testkit` |
| `polkit_agent/{types,machine,identity,helper}.rs` | `sill-model/src/polkit_agent/` |
| `polkit_agent/{bus,dbus,run,live}.rs` | `polkit_agent/backend/` |
| `screenshots/{reduce,names}.rs`; `tool.rs` (parse only) | `sill-model/src/screenshots/`; the tool string is parsed at load by `sill-settings` into a typed `ShotTool` |
| `screenshots/{child,folder_watch,live,preview}.rs` | `screenshots/backend/` |
| `wallpaper/{cache,geometry}.rs` | `sill-model/src/wallpaper/` |
| `wallpaper/{config,resize,watch,backdrop}.rs`, logic in `mod.rs` | `wallpaper/backend/` |
| `tray/{model,icon}.rs`; `tray/host.rs` (498) | `sill-model/src/tray/`; `tray/backend/{host,items,menu}.rs` |
| `now_playing/{model,art}.rs` (names); `mpris.rs`, `live.rs` (498) | `sill-model/src/now_playing/`; `now_playing/backend/{mpris,run,art}.rs` |
| `battery`, `kbd_backlight`, `power_profile`, `volume`, `session`, `badges`, `clock`: `model.rs`, `format.rs` | `sill-model/src/<x>/`; `live.rs`, `upower.rs`, `ppd.rs` -> `<x>/backend/` |
| `providers/`, `results/{registry,run,sink,history,frecency_file,effect}.rs`, `preview.rs` | `sill-search/src/providers/`, `results/`, `preview.rs` |
| `results/{model,arrange,policy}.rs` | `sill-model/src/results/` |
| `services.rs` (`Services`, `Member`) | `sill-ui-kit/src/services.rs`; `Services::spawn` becomes `sill/src/daemon/services.rs::start` |
| `magnify/` | `sill-dock/src/magnify/` |
| `switcher.rs` | `sill-overlays/src/switcher/apps.rs` |
| `curve.rs` (unused) | deleted; easing is `ds-motion`'s |
| `programs.rs` | `env/processes.rs`: the `Processes` seam (`/proc` scan; `Env::isolated` sees none) |
| `env/scratch.rs`, `examples/*` (13 stand-ins), `tests/support/*` | `sill-testkit` (`TempDir`, `PrivateBus`, `signals`; stand-ins in its `examples/`) |
| `tests/*_live.rs` | `crates/sill-services/tests/<service>_bus.rs` on `PrivateBus`; `#[ignore]` only when a compositor is needed |
| `error.rs`, `control.rs`, `stream.rs`, `service.rs`, `env/` | stay |

### `sill-settings`, `sill-ipc`, `sill-launcher`

| From | To |
| --- | --- |
| `sill-settings/src/{bar,calendar,control_center,display,dock,hot_corners,idle,launcher,menus,notifications,osd,power_menu,screenshot,scroll,scrollbar,session,spaces,switcher,widgets,window}.rs` | stay, each `impl Section`; keys with no reader deleted; `flatten extra` and `version` deleted |
| `dock_words.rs`, `launcher_kinds.rs`, enums a service reads | `sill-model/src/vocab/` or the service dir; the settings field uses that type |
| `focus.rs` (`FocusMode`) | `sill-model/src/focus/` |
| `focus_link.rs` | `sill-ui-kit/src/links/focus.rs` (the link) and `sill/src/daemon/settings/focus.rs` (the task that writes the file) |
| `watch.rs`, `file.rs` `load`/`save`/`try_load`/`unknown_keys` | `sill/src/daemon/settings/{watch,store}.rs` over `ds_settings::Store`; `file.rs` keeps `ShellFile` and `impl SettingsDoc` |
| `feed.rs` (`SettingsFeed`) | `sill-ui-kit/src/settings.rs` |
| `sill-ipc/src/{desktop,idle,launcher,session}.rs` | `show.rs`, `status.rs`, `act.rs`, `debug.rs`; the 28 `Report*`/`Fake*` mirrors go (a report is the `sill-model` type, serialised) |
| `sill-launcher/src/{preview,preview_tests}.rs` | stay (pure facts); `sill-services/src/preview.rs` moves to `sill-search` |

### `sill-surfaces` (ends deleted)

| From (`crates/sill-surfaces/`) | To |
| --- | --- |
| `src/hooks/*`, `clock.rs` (`FrameClock`), `host_modality.rs`, `src/lib.rs` host config helpers | `sill-ui-kit/src/{hooks,clock,root}` |
| `src/app_icons/`, `surfaces/notification_card/`, `surfaces/user_picture.rs`, `surfaces/naming.rs` | `sill-ui-kit/src/{app_icons,notification_card,surface}`; `user_picture` is `ds-shell`'s |
| `src/widgets/` (providers, layout, wake) | `sill-ui-kit/src/widgets/<kind>/` |
| `bar/menu_track/` | deleted; `ds`'s `menu_track` |
| `bar/{menu_popup,menus,tray_menu}.rs`, `dock/{menu_popup,context_menu,menu_document,menu_geometry}.rs`, `launcher` actions menu | `impl MenuModel` in the owning surface; the popup is `sill-ui-kit::menu::MenuPopup` |
| `bar/{glyph_state,status_item,status}.rs`, `low_battery.rs` | `sill-ui-kit/src/status/` (service state to `ds-shell` glyph props) |
| `bar/*` (rest), `control_center/*` | `sill-bar/src/bar/`, `sill-bar/src/control_center/` |
| `control_center/{fit,sizes}.rs`, estimate half of `geometry.rs`, `bar/geometry.rs`, `menu_size_tests.rs`, `tests/cc_estimates.rs` | deleted (FitContent popups measure) |
| `dock/*` | `sill-dock/src/dock/` |
| `launcher/*` | `sill-launcher-ui/src/launcher/`; `OpenSerial`, `PanelFocus`, `LauncherShown` to `sill-ui-kit/src/links/launcher.rs`, `Visible` to `shell_host::Visibility` |
| `banners/`, `notification_center/`, `screenshot_thumb/` | `sill-notify-ui/src/` |
| `desktop_widgets/`, `edit_widgets/` | `sill-widgets-ui/src/{desktop,edit}/` |
| `osd/`, `power_menu/`, `polkit_prompt/`, `switcher/`, `idle_dim/`, `hot_corners/`, `wallpaper/`, `lock/` | `sill-overlays/src/<same>/` |
| `power_menu/script.rs`, `launcher/script.rs`, `launcher/debug_link.rs` | deleted once shell-host has `Inject::Key` |
| `src/style/*.css` | `<concept>/style.css` beside its `view.rs` (`bar_popup.css`, `dock_popup.css` become `menu/style.css`) |
| `examples/*_demo.rs` | deleted; the acceptance scripts run the real daemon |
| `tests/<surface>_{render,keys,motion,...}.rs` | `<surface crate>/tests/<surface>.rs` on `SurfaceHarness<S>` |
| `tests/*_lint.rs` (15), `tests/*_support/`, `tests/support/fake.rs`, `tests/launcher_harness`, `tests/cc_support` | one table `crates/sill/tests/lint.rs`; `sill-ui-kit/src/harness/` |
| `tests/{status_glyphs,host_faces,battery_snapshot,cancelled_animation,calendar_render,widgets_*}.rs` | `sill-ui-kit/tests/` |

### `sill` (the binary)

| From (`crates/sill/src/`) | To |
| --- | --- |
| `cli.rs`, `cli_{display,desktop,launcher,notifications,session}.rs`, `client.rs`, `auth_check.rs` | `cli/{mod,request,display,desktop,launcher,notifications,session,debug,auth_check}.rs`, `cli/client.rs` |
| `daemon.rs` (1,406) | `daemon/{mod,run,events,surfaces,services,msg}.rs`, `daemon/settings/`; the `dispatch_*` fns to `ipc/dispatch/` |
| `run_args.rs`, `msg.rs` | `daemon/args.rs`, `daemon/msg.rs` |
| `outputs.rs` | `daemon/outputs.rs` (a map keyed by surface name and slot, replacing the 19-field struct) |
| `ipc_server.rs` | `ipc/server.rs` |
| `links.rs` (global `OnceLock`) | deleted; `Links` is built in `daemon/run.rs` and passed to `daemon/surfaces.rs` |
| `policy.rs`, `calendar.rs`, `displays.rs`, `idle.rs`, `launcher_policy.rs`, `notifications/policy.rs`, `screenshots.rs` (policy half), `clipboard.rs` (policy half) | `policy/{mod,follow}.rs` and one file per bridge: `calendar`, `displays`, `idle`, `notifications`, `screenshots`, `search`, `clipboard` |
| `lock/`, `idle_dim.rs`, `corners.rs`, `osd.rs`, `scroll.rs`, `launch.rs` | `daemon/followers/{lock,idle_dim,corners,osd,scroll,launch}.rs`: act on the host from a service or a setting |
| `power_menu.rs`, `switcher.rs`, `widgets.rs`, `edit_widgets.rs`, `polkit.rs`, `session.rs`, `notifications/mod.rs`, `screenshots.rs`, `clipboard.rs` (verb halves) | the verb halves are `ipc/dispatch/{show,status,act}.rs` arms; the spawn halves are gone (generic `daemon/surfaces.rs`) |
| `launcher_debug.rs`, `notifications/fake.rs` | `ipc/dispatch/debug.rs` (debug feature) |
| `tests/{ipc,latency}.rs` | stay |

---

## 4. The one home table

Look a concept up here before writing it. If it has a home, extend it.

| Concept | Home |
| --- | --- |
| Everything ambient (buses, Wayland route, directories, wall clock, DDC, session actions, authenticator, idle times, polkit route, processes) | `sill_services::env::Env` |
| Service trait, runner, `Handle<S>`, `Control<C>` | `sill_services::{service, control}` |
| A service's State, Command, Event, Effect | `sill_model::<service>::{model, command}` |
| A service's pure transition | `sill_model::<service>::step::step` |
| A service's I/O | `sill_services::<service>::backend` |
| Every running service's handle, `Member`, `use_service` | `sill_ui_kit::services`, `sill_ui_kit::hooks` |
| Starting every service in `Deps` order | `sill::daemon::services::start` |
| Units and ids (`Stamp`, `Cpx`, `Minutes`, `AppId`, `WorkspaceKey`) | `sill_model::{units, ids}` (`Percent`, `Fraction`: `ds-core`) |
| Retry and stale-answer stamps | `sill_model::attempt` |
| A closed vocabulary shared by a settings key and a service | `sill_model::vocab` or the service dir, implementing `ds_core::Word` |
| A settings domain, its typed values | `sill_settings::<domain>`, `impl Section` |
| The settings file type and its `SettingsDoc` | `sill_settings::file` |
| Loading, saving, watching, publishing settings | `sill::daemon::settings` |
| The settings a surface reads | `sill_ui_kit::settings` (`SettingsFeed`), `use_section::<S>()` |
| Settings turned into a service command | `sill::policy::<service>` (`Policy`) |
| Focus mode value, link, writer | `sill_model::focus`, `sill_ui_kit::links::focus`, `sill::daemon::settings::focus` |
| IPC words | `sill_ipc::{request, response, show, status, act, debug}` |
| Answering an IPC request | `sill::ipc::dispatch::<group>` (one fn per domain) |
| A command-line verb | `sill::cli::<group>`, mapped in `cli/request.rs` |
| Launcher matching, ranking, frecency, sections | `sill_launcher` |
| A launcher result source | `sill_search::providers::<kind>` (`Provider`) |
| The search runner and its history | `sill_search::results` |
| Launcher row to menu row | `sill_launcher_ui::launcher::rows::row` (one exhaustive fn) |
| Surface root: services, feed, host seam, appearance, scale, frame clock | `sill_ui_kit::root::SurfaceRoot` (the host seam is `ds_native::provide_host()`, called by `mount`, `mount_lock` and each popup root; the seat's `HostSignals` (scale, modality) come from `use_host_signals()`, called by `mount`, `mount_lock` and `PopupFrame`, and a surface reads its scale with `use_host_scale()`) |
| Active workspace look, space look | `sill_ui_kit::look` (`use_active_look`, `use_space_look`) |
| Open/close a panel from outside it (serial, output) | `sill_ui_kit::links::Reveal<O>`; every link type is one file in `sill_ui_kit::links` |
| Visible or hidden layer surface | `shell_host::Visibility` |
| Layer namespace `sill-<surface>-<output>` | `sill_ui_kit::surface::namespace` |
| Spawning and closing surfaces per output | `sill::daemon::surfaces` (generic over `Surface`) |
| Which surfaces exist on which output | `sill::daemon::outputs` |
| A timed pure state machine and its timer | `sill_ui_kit::machine` (`Machine`, `use_machine`) |
| Popup menus (bar, tray, dock, launcher actions) | `sill_ui_kit::menu` (`MenuModel`, `MenuPopup<M>`) over `ds`'s `menu_track` |
| Popup sizing | shell-host `FitContent`; no estimation code |
| App icons, look, cache | `sill_ui_kit::app_icons` |
| Status glyph props from service state | `sill_ui_kit::status` over `ds-shell` glyphs |
| Notification card | `sill_ui_kit::notification_card` |
| Widget provider, `Card<P>`, registry, layout mapping | `sill_ui_kit::widgets` |
| Desktop widget grid; Edit Widgets sheet | `sill_widgets_ui::{desktop, edit}` |
| Control-center module | `sill_bar::control_center::modules::<name>`, picked by `module(kind)` |
| Dock magnification | `sill_dock::magnify` |
| Switcher app derivation | `sill_overlays::switcher::apps` |
| Easing and springs | `ds-motion` |
| Colour, tint, material | `ds-core`, `ds-style`; sill never computes colour |
| Keymaps | `<surface>/keys.rs` table; key claims are `ds`'s |
| Stylesheets | `<concept>/style.css` |
| Session lock: service, surface, host half | `sill_services::locker`, `sill_overlays::lock`, `sill::daemon::followers::lock` |
| Errors | `sill_services::ServiceError`, `sill_ipc::error::IpcError`, one `thiserror` enum per crate |
| Private bus, scratch dirs, stand-in programs | `sill_testkit` |
| Surface test driver | `sill_ui_kit::harness::SurfaceHarness<S>` |
| Lint of surface markup | `crates/sill/tests/lint.rs`, one table |
| Dev script helpers | `dev/lib/*.sh` |

---

## 5. Canonical traits

### `Service` (`sill_services::service`)

```rust
pub trait Service: Sized + 'static {
    const NAME: &'static str;
    type State: Clone + Default + PartialEq + Send + Sync + 'static; // sill_model type; default = absent
    type Command: Send + 'static;                                    // sill_model type
    type Deps: Send + 'static;                                       // sill_model receivers, Controls; () if none
    fn initial(env: &Env, deps: &Self::Deps) -> Self::State { Self::State::default() }
    fn run(env: &Env, deps: Self::Deps, io: ServiceIo<Self>)
        -> impl Future<Output = Result<(), ServiceError>> + Send + 'static;
}
pub struct Handle<S: Service> { pub state: watch::Receiver<S::State>, pub control: Control<S::Command> }
pub fn spawn<S: Service>(rt: &tokio::runtime::Handle, env: &Env, deps: S::Deps) -> Handle<S>;
```

Implementors: the 27 `<Module>Service` markers (`enum XService {}`), 26 in `sill-services`,
`ResultsService` in `sill-search`. Add an implementor for a new long-lived backend that
publishes one state. Add nothing for a helper without a task. Not a `Service`: a one-shot call
(`launch::open`), a pure derivation (`switcher::apps`), or a UI link.

### `Section` (`sill_settings::section`)

```rust
pub trait Section: Clone + PartialEq + Send + Sync + 'static {
    const KEY: &'static str;                 // the table name, `bar`
    fn of(file: &ShellFile) -> &Self;
}
```

Implementors: the 20 domain structs. One per settings table; a new key is a field, not a
`Section`. `use_section::<S>() -> ReadSignal<S>` and `Policy::of` read through it.

### `Policy` (`sill::policy`)

```rust
pub trait Policy: Send + 'static {
    type Command: Send + 'static;
    type Out: PartialEq + Clone + Send + Sync + 'static;
    fn of(&self, file: &ShellFile) -> Self::Out;
    fn command(out: Self::Out) -> Self::Command;
    fn changed(&mut self) -> impl Future<Output = Result<(), RecvError>> + Send { pending() }
}
pub fn follow<P: Policy>(rt: &Handle, settings: watch::Receiver<ShellFile>, policy: P, control: Control<P::Command>);
```

Implementors: `calendar`, `displays`, `idle`, `notifications` (`changed` waits on the focus
mode), `screenshots`, `search`, `clipboard`. One per service that is passive until told. A
policy builds typed `sill-model` values; a service never sees a settings string.

### `Provider` (`sill_launcher::provider`)

```rust
pub trait Provider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn query(&self, query: &Query, sink: &mut dyn ResultSink);
    fn activate(&self, item: &Item, choice: &Choice) -> Activation;
}
```

Implementors in `sill-search/providers/`: apps, windows, system, calculator, settings, files,
clipboard, emoji, web. One per source of rows; `activate` is pure and returns an `Activation`.

### `Surface` and `LockSurface` (`sill_ui_kit::surface`)

```rust
pub trait Surface: 'static {
    const NAME: &'static str;                       // `osd`; namespace `sill-osd-<output>`
    type Link: Clone + Send + Sync + 'static;       // a type in sill_ui_kit::links; `()` if none
    type Slot: Copy + Eq + Hash + Debug + 'static;  // `()`; hot corners: `Corner`
    fn slots(place: &Place) -> Vec<Self::Slot>;     // which surfaces on this output; empty = none
    fn layer(output: &OutputInfo, settings: &ShellFile, slot: Self::Slot) -> LayerConfig;
    fn start() -> Start;                            // Start::{Shown, Hidden}
    fn root(output: OutputInfo, slot: Self::Slot) -> Element;
}
pub struct Place<'a> { pub output: &'a OutputInfo, pub rank: OutputRank, pub settings: &'a ShellFile }
pub trait LockSurface: 'static {
    const NAME: &'static str;
    type Link: Clone + Send + Sync + 'static;
    fn config(settings: &ShellFile) -> LockConfig;
    fn root(output: OutputInfo) -> Element;
}
```

Implementors: the 16 layer surfaces and `lock`. A new visible thing on its own layer surface
is a new `Surface`. A popup of an existing surface is a `MenuPopup` or a component, not a
`Surface`. `root` runs inside `surface::mount::<S>`, the one root component every spawned surface
has, so a `root` may call hooks. `SurfaceRoot` stays a plain component that each view wraps its
content in: the material, chrome and extent it takes are the surface's own, which a wrapper
generic over `S` could not know. The daemon gives every root its `S::Link` as a root context; a
slot is erased to its `Debug` text in the output's row (`OutputSurfaces::spawned`), which also
keeps the layer namespace `debug stats` reports. A slot the settings drop is closed, and a layer
size that changed is re-applied, by `daemon/spawn.rs`.

### `Machine` (`sill_ui_kit::machine`)

```rust
pub trait Machine: Clone + PartialEq + Default + 'static {
    type In; type Out; type Params: Clone + PartialEq + 'static;
    fn step(self, input: Self::In, at: Stamp, params: &Self::Params) -> (Self, Vec<Self::Out>);
    fn wake(&self) -> Option<Stamp>;                // when to step again with no input
}
pub fn use_machine<M: Machine>(params: M::Params, on_out: impl FnMut(M::Out) + 'static) -> MachineRef<M>;
impl<M: Machine> MachineRef<M> { pub fn send(&self, input: M::In); pub fn state(&self) -> ReadSignal<M>; }
```

Implementors: dock hover/press/reveal/label/bounce, switcher, osd, banners deck, hot corners,
screenshot thumb, power menu, polkit stage, notification-center keys, launcher keys. One per
state that changes over time or by input. A state that only maps values is a free `step`.

### `CcModule` (`sill_bar::control_center::modules`)

```rust
pub trait CcModule: Sync {
    fn span(&self) -> Span;                                          // one or two columns
    fn tile(&self, cx: &CcCx) -> Element;                            // the grid tile
    fn dropdown(&self, cx: &CcCx) -> Option<Element>;                // bar-item dropdown body
    fn menu_bar(&self, settings: &ControlCenterSettings) -> InMenuBar;
    fn bar_item(&self) -> Option<BarItemFace>;
    fn detail(&self, page: Detail, cx: &CcCx) -> Option<Element>;
}
pub fn module(kind: ControlCenterModule) -> &'static dyn CcModule;  // the only match on the kind
```

`CcCx { output: Option<OutputName>, on_detail: EventHandler<Detail> }`. Implementors: Wi-Fi,
Bluetooth, Focus, Display, Keyboard, Sound, Now Playing, Appearance, Battery.

### `WidgetProvider` (`sill_ui_kit::widgets`)

```rust
pub trait WidgetProvider: Clone + PartialEq + 'static {
    type W: ds::Widget;                       // a value: `ds::Widget: Default`
    const KIND: WidgetKind;
    const DESKTOP_SIZE: WidgetSize;           // its one size on the desktop (tiles are all Medium)
    fn use_feed() -> Feed<Self::W>;           // reads the services; one hook, one shape
    fn registry_id() -> ds::WidgetKind;       // provided: `W::kind()`, the name in quire's registry
}
pub struct Feed<W: ds::Widget> {
    pub entry: W::Entry,                                   // the entry now
    pub onintent: Option<EventHandler<W::Intent>>,         // the card's controls; none without any
}
#[component] pub fn Card<P: WidgetProvider>(provider: P, place: Place) -> Element;  // quire's WidgetCard
pub fn with_provider<A: OnProvider>(kind: WidgetKind, action: A) -> A::Out;  // the only match on the kind
pub fn card(kind: WidgetKind, place: Place) -> Element;             // `with_provider(kind, CardAt(place))`
```

`size_of(kind, host)` reads `DESKTOP_SIZE` (desktop) or Medium (tile); `kind_id`, `kind_of`
and `size_of` all go through `with_provider`. `WidgetKind` derives quire's `Word`: `ALL` and the
slug (the element id `widget-<slug>`) come from the derive, the settings key stays serde
snake_case. A provider is a `Default` unit struct
(`CalendarProvider`), passed to `Card` only to name the type. Calendar's `use_feed` keeps the
shown month in a signal and returns the `MonthIntent` handler; Now Playing's returns the
`Transport` handler (`use_media`, shared with the control center).

Implementors: calendar, up next, batteries, now playing, world clock.

### `MenuModel` (`sill_ui_kit::menu`)

```rust
pub trait MenuModel: 'static {
    type Pick;
    fn entries(&self) -> Vec<ds::MenuEntry>;
    fn pick(&self, path: &ItemPath) -> Option<Self::Pick>;
}
#[component] pub fn MenuPopup<M: MenuModel>(model: M, anchor: Anchor, on_pick: EventHandler<M::Pick>) -> Element;
```

Implementors: bar app menu, tray (dbusmenu), dock context menu, launcher actions.

### Enums that stay enums

| Enum | Home | Why |
| --- | --- | --- |
| IPC `Request` `{Ping, Quit, Reload, Show, Status, Act, Debug}`, `Response` | `sill-ipc` | closed wire set; one dispatch fn per domain |
| `Activation`, `ProviderKind`, `Item` rows | `sill-launcher` | closed; a variant must break the exhaustive `row` match |
| `BusRoute` `{Usual, Address, Absent}`, `WaylandRoute` `{Environment, Socket}` | `sill_services::env` | route choices, not behaviours |
| `Reveal<O>` `{Hidden{last}, Revealed(O)}` | `sill-ui-kit::links` | a link's value |
| `Start`, `OutputRank`, `Insertion` | `sill-ui-kit::surface` | closed |
| `ControlCenterModule`, `WidgetKind`, `ProviderKind` | settings and launcher | matched once, in `module` and `card` and `row` |
| service `Command`, `Event`, `Effect` | `sill-model` | closed sets stepped by `match` |

---

## 6. Recipes

Each ends with its tests. Name the concept once and use it in every file name below.

### Add a service

1. `sill-model/src/<x>/{mod,model,command,step,tests}.rs`: `State` (default = absent), `Command`,
   `Event`, `Effect` if `step` returns effects, `pub fn step(state, event) -> State` (or `->
   (State, Vec<Effect>)`), a `CASES` table test. Derive serde only for what crosses IPC or a file.
2. `pub mod <x>;` in `sill-model/src/lib.rs`.
3. A new ambient thing (a bus, a directory, a program) is a field on `Env` in
   `sill-services/src/env/`, set in `from_process` and `isolated`, with a stand-in.
4. `sill-services/src/<x>/mod.rs`: `pub enum XService {}` and `impl Service` (`run` calls
   `backend::run`); `backend/run.rs` and its effect files. Never `Connection::system()`,
   `std::env`, `Command::new` or `Zoned::now` outside `Env`.
5. `pub mod <x>;` in `sill-services/src/lib.rs`; re-export the marker only.
6. `sill-ui-kit/src/services.rs`: a `Handle<XService>` field and a `members!` line; the harness
   `FakeServices` gets its default. `sill/src/daemon/services.rs::start`: one `spawn` line, after
   the services in its `Deps`.
7. If the service must stay passive until told, add a `Policy` (recipe below).
8. Tests: the `step` table in `sill-model`; `sill-services/tests/<x>_bus.rs` on `PrivateBus`
   (stand-in in `sill-testkit/examples/` if a peer is needed); the `services_start` test in
   `sill-ui-kit` gains an absent-default assertion.
9. A surface reads it with `use_service::<XService>()`.

### Add a policy (a service told by settings)

1. `sill-settings/src/<domain>.rs` holds typed fields (recipe: settings key).
2. `sill/src/policy/<x>.rs`: `struct XBridge`, `impl Policy` (`of` builds typed values,
   `command` wraps them); add `mod`.
3. `daemon/services.rs::start`: `follow(rt, settings.clone(), XBridge, handle.control.clone())`.
4. Tests: `of` table in the same file over `ShellFile` values; `follow` is tested once, in
   `policy/follow.rs`.

### Add a surface

1. Choose the crate from the surfaces table below (a new crate is a new row of section 1 and of
   `EDGES` in `scripts/check-boundary.sh`). The surface lives in that crate, in `src/<name>/` with
   the roles in "The one shape of a surface": `mod.rs`, `surface.rs`, `layer.rs`, `model.rs`,
   `step.rs`, `io.rs`, `view.rs`, `style.css`, `tests.rs`. `lib.rs` declares `pub mod <name>` and
   re-exports the marker type; `mod.rs` re-exports the stylesheet const (`NAME_CSS`) the lint
   row needs.
2. `surface.rs`: `pub enum Name {}` and `impl Surface` (`NAME`, `Link`, `Slot`, `slots`,
   `layer`, `start`, `root`). `root` is `view::name(output)`; no `Ds` boilerplate: `SurfaceRoot`
   supplies services, feed, appearance, scale and the frame clock, and `mount` has already
   installed the host seam (`ds_native::provide_host()`) and the seat's `HostSignals`; a `Ds` root passes no `scale`.
3. A link the daemon or another surface drives is one file in `sill-ui-kit/src/links/<name>.rs`
   (a newtype over `Reveal<O>` or a small watch channel) and a field of `Links` in
   `links/mod.rs`. A surface never imports another surface.
4. Settings keys: recipe below. Services it reads: `use_service` in `io.rs` only.
5. Timed state: `impl Machine` in `step.rs`, run by `use_machine` in `io.rs`.
6. `sill/src/daemon/surfaces.rs`: in `spawn_missing`, at the surface's place in the drawing order
   (later spawns draw over earlier ones on one layer), add
   `spawn_surface::<Name>(ctx, outputs, spawn, &links.name)` (`&()` when `Link = ()`) and import
   the marker from the surface's crate; the generic `daemon/spawn.rs` spawns, resizes and closes.
   There is no per-surface spawn code, `outputs.rs` field or namespace helper (`debug stats`
   reads `OutputSurfaces::spawned`). The link is a field of `Links`, handed to the root by
   `surface_root`.
7. IPC verb if it opens from the command line (recipe below).
8. Tests, in the surface's crate: `crates/<crate>/tests/<name>.rs` on
   `SurfaceHarness::<Name>::setup(link, slot).settings(..).viewport(..).appearance(..).start()`
   (render, keys, motion on the virtual clock), and unit tests in `src/<name>/tests.rs`. The lint
   is the one place outside the crate: a `SurfaceCase` row in `SURFACES` in
   `crates/sill/tests/lint.rs` (the stylesheet const and a function drawing each state to lint).
   Then `dev/accept/<name>.sh`.
9. Add the row to the surfaces table in this file.

### Add a control-center module

1. `ControlCenterModule` variant in `sill-settings/src/control_center.rs` (a `Word`; `ALL`
   follows from the derive).
2. `sill-bar/src/control_center/modules/<name>.rs`: the component over `use_service`, and `impl
   CcModule for NameModule`; a detail page in `pages/<name>.rs` if it has one.
3. The one arm in `modules/mod.rs::module`. No other `match` on the kind.
4. Tests: a row in `tests/control_center.rs`, whose loop runs every `ControlCenterModule::ALL`
   through tile, dropdown and detail on `SurfaceHarness::<ControlCenter>`.

### Add a widget

1. The `ds_shell::Widget` (quire's recipe), then a `WidgetKind` variant in
   `sill-settings/src/widgets.rs`.
2. `sill-ui-kit/src/widgets/<kind>/{mod,provider,io,tests}.rs` (+ `style.css` if it has one):
   `impl WidgetProvider` (`use_feed` over services) on a unit struct in `io.rs`.
3. `const DESKTOP_SIZE` on the provider, and the one arm in `widgets/card.rs::with_provider`;
   `WidgetKind::ALL`, the slug, `kind_id`, `size_of` and the element id follow. Add the
   variant's row to the table test in `widgets/tests.rs` (slug, element id, size).
4. Tests: entry table in `tests.rs`; a row in `sill-widgets-ui/tests/desktop.rs` and in
   `sill-notify-ui/tests/center.rs` render loops (both hosts).

### Add a launcher provider

1. `ProviderKind` variant, and an `Activation` variant only if none fits, in `sill-launcher`.
2. `sill-search/src/providers/<kind>/mod.rs`: `impl Provider`; effects take `Env` values
   (paths, buses) through the constructor.
3. Register in `providers/registry.rs`, in rank order.
4. The row's arm in `sill-launcher-ui/src/launcher/rows.rs::row` (exhaustive).
5. Tests: `query` and `activate` tables beside the provider with `VecSink`; a rank test row in
   `sill-launcher`.

### Add a menu

1. In the owning surface, `menu.rs`: `struct XMenu`, `impl MenuModel` (`entries`, `pick`); the
   `Pick` enum is the surface's own.
2. `view.rs` renders `MenuPopup { model, anchor, on_pick }`. No menu markup, no tracker.
3. Tests: `entries` and `pick` tables in `tests.rs`; keyboard and hover switching are `ds`'s.

### Add a settings key

1. The design/22 row in quire first (with `RETIRING` when removing one).
2. `sill-settings/src/<domain>.rs`: a field of a typed value: a `Word` enum, a `ds-settings`
   unit newtype, or a two-variant enum (no `bool`); the default is the design's. A string that
   names a closed set is an enum parsed at load; a shared enum lives in `sill-model`.
3. A new domain is a file, a `Section` impl, a `ShellFile` field and a `schema.rs` entry.
4. The reader: a surface reads `use_section::<Domain>()`; a service is told through its
   `Policy`. `sill --write-schema <dir>` regenerates the schema.
5. Tests: `sill-settings/tests/keys.rs` cross-checks the key against quire's rows both ways;
   a parse table for a new enum.

### Add an IPC verb

1. Pick the group: `Show` (open, close, toggle a panel: `Reveal`), `Status` (read a state),
   `Act` (change something), `Debug` (probes, behind `debug`). Add the variant to that group's
   file in `sill-ipc/src/` (`show.rs`, `status.rs`, `act.rs`, `debug.rs`); an answer is a
   `sill-model` type in `Response::Status`, never a mirror.
2. A round-trip row in `sill-ipc/tests/serde.rs` with the exact bytes.
3. `sill/src/cli/<group>.rs`: the clap argument; the arm in `cli/request.rs::request`, covered
   by its table test.
4. `sill/src/ipc/dispatch/<group>.rs`: one arm that calls a link, a service control or a
   follower and returns a `Response`; no logic in the arm.
5. Tests: a row in `crates/sill/tests/ipc.rs` on a fake dispatcher; the surface's acceptance
   script uses the verb.

### Add a dev script

1. Per surface or feature exactly one file, `dev/accept/<name>.sh`; variants are flags
   (`--real-input`, `--bus`), never a second file.
2. It sources `dev/lib/{bus,daemon,compositor,input,pixels,assert}.sh` and defines none of
   their functions; a missing helper is added there.
3. It starts the daemon with a private bus on both addresses, a fake DDC, scratch `HOME` and
   `XDG_*`, its own `XDG_RUNTIME_DIR`, on the nested compositor or shell-host's headless KWin.
4. Output PNGs go to `dev/out/` (untracked). What needs a person goes to
   `docs/manual-checks.md`.

---

## 7. Surfaces

| Surface | Crate | Layer | Notes |
| --- | --- | --- | --- |
| `bar` | `sill-bar` | Top | Menus (app, status items, tray) through one `MenuPopup`; menu tracking is `ds`'s |
| `control_center` | `sill-bar` | bar popup | `CcModule` grid; hung from the output's right edge |
| `dock` | `sill-dock` | Bottom | Magnify, machines, context menu, badges, running dots |
| `launcher`, `launcher_catcher` | `sill-launcher-ui` | Overlay | Panel plus click catcher, one `Reveal` link |
| `banners`, `notification_center`, `screenshot_thumb` | `sill-notify-ui` | Overlay | Cards from `sill-ui-kit::notification_card` |
| `desktop_widgets`, `edit_widgets` | `sill-widgets-ui` | Bottom, Overlay | Providers from `sill-ui-kit::widgets` |
| `osd`, `power_menu`, `polkit_prompt`, `switcher`, `idle_dim`, `hot_corners`, `wallpaper` | `sill-overlays` | Overlay (wallpaper Background) | Hot corners: one slot per corner |
| `lock` | `sill-overlays` | session lock | `LockSurface`; one per output through shell-host |

The notification center draws widgets and the desktop layer draws them too, so providers live
in `sill-ui-kit`, and the center's "Edit Widgets" asks through `links::edit_widgets`.

## 8. The daemon

`sill run` (`daemon/run.rs`) takes the single-instance lock, builds `Env` (`daemon/args.rs`),
starts services (`daemon/services.rs`) and settings (`daemon/settings/`), starts the IPC server
(`ipc/server.rs`), connects the host and reconciles one row of surfaces per output
(`daemon/outputs.rs`, `daemon/surfaces.rs`). `Links` is built once in `run.rs`. The `debug`
feature adds `sill debug ...` and the fake DDC flags; a build without it refuses them.

Settings have one path: the file settles in `daemon/settings/watch.rs`, is published on the
feed (surfaces read it) and followed by each `Policy` (services are told). Focus mode writes
the file and the file changes the mode; the file stays the one truth.

The person's stylesheet (`~/.config/sill/style.css`, quire's `UserStyle`, ARCHITECTURE.md
section 11 in quire) takes the same path: `daemon/settings/user_style.rs` watches it through
`ds_settings::Store`, publishes it on the feed, and `SurfaceRoot` passes it to `Ds`, so every
surface restyles live. Each surface root carries `data-surface = Surface::NAME`, which makes
the surface names part of the public selector surface: renaming one breaks the person's file
and needs a note. `sill style check` prints `ds_lint::user_stylesheet`'s notes for the file.

`dist/` holds what a real login needs: `sill-session`, `sill-session.target`, `sill.desktop`,
`sill.service`, the COSMIC shortcut overrides, the PAM service and the DDC/CI rules. Nothing
installs them.

## 9. Test harness

| Kind of thing | How it is tested |
| --- | --- |
| Pure (`sill-model` steps, `sill-launcher`, `sill-settings` parse, `sill-ipc` words) | `CASES` tables beside the code; serde round trips against exact bytes |
| Service backend | `sill-services/tests/<x>_bus.rs`: `Env::isolated(tmp)` with `Buses` set to a `PrivateBus` address; peers are `sill-testkit` stand-ins. Runs in the gate. Only a test that needs a compositor is `#[ignore]`d and run from a dev script |
| Providers, runner | `VecSink` and a fake `Sources::Providers`; the runner's latency test |
| Surface | `SurfaceHarness<S: Surface>` (`sill-ui-kit`, feature `harness`) over quire's `ds_native::Harness`: `SurfaceHarness::<S>::setup(link, slot)` then `.settings(ShellFile)`, `.viewport(..)`, `.appearance(..)`, `.start()`. It mounts `surface::mount::<S>` on fake `Services` (`fake::services()`; state set through `run.feeds`, commands read), a `PinnedAppearance`, `Clock::Virtual`, the surface's link (`run.link`) and the settings; the viewport is the surface's layer size unless overridden. `run.harness` is the `Harness` (`advance`, `html`, `pointer_move`, `count`). One live harness per thread. Real-input checks are dev scripts on shell-host's headless KWin |
| Machines | table tests of `step`; `use_machine` is tested once in `sill-ui-kit` |
| Markup lint | `crates/sill/tests/lint.rs`: `const SURFACES: &[SurfaceCase]`, one loop over all 17, each row names its surface |
| IPC and CLI | `crates/sill/tests/ipc.rs` (fake dispatcher, cap, timeout), `cli/request.rs` table |
| Daemon latency | `crates/sill/tests/latency.rs` |
| Acceptance | `dev/accept/<surface>.sh`; a person's checks in `docs/manual-checks.md` |

## 10. Repo rules

- **Crate boundaries** are the tables in section 1; UI reaches services only through
  `sill-ui-kit`'s `use_service`, `use_service_part` and `use_control`.
- **A service** has the shape of `Service`; its seams come from `Env` and nothing else; other
  services it reads are its `Deps` (model types). A service whose system half is missing still
  yields its receiver, at its default (`Unavailable` in the state, never a silent `Default`). A
  service that could touch the person's session is passive until its first `Policy` command.
- **Never pass a Dioxus signal through `Root::context`**; roots get `Services`, `SettingsFeed`
  and links (watch channels).
- **Design-system rules** are quire's: every class is `ds-`; sill never styles `.ds-*` or
  `[data-theme|accent|motion|material]`, writes no raw `button`/`input`/menu markup, and takes
  every colour, duration, easing, keyframe and font from quire's tokens. Motion is quire's;
  sill writes no keyframes.
- **Every time-based UI test runs on `Clock::Virtual`** and pins `PinnedAppearance`.
- **A new settings key** is a design/22 row in quire first; an unknown key in a file is a load
  diagnostic, not preserved.
- **Every IPC wire form is pinned** by a row in `sill-ipc/tests/serde.rs`.
- **No `unsafe`** anywhere in the workspace.
- **Dependencies:** sill-only dependencies sit after the pinned block in the root `Cargo.toml`.
- **Tests** sit in the concept's `tests.rs` or `tests/`; no sibling `*_tests.rs`.
- **The real system, named:** no test or script reaches the real session or system bus,
  NetworkManager, UPower, logind, BlueZ, `/dev/i2c-*`, the real `~/.config` or `~/.cache`, or
  the person's Wayland session. Scripts give the daemon a private bus on both addresses, a fake
  DDC, scratch XDG dirs and its own `XDG_RUNTIME_DIR`. No test locks a session it did not
  start, calls logind's `Suspend`, or registers a polkit agent outside a private bus.
- **Strong copyleft sill reads for facts only:** cosmic-comp, cosmic-greeter, cosmic-idle,
  cosmic-osd, systemd.
- **Gate** (`~/sill-wt/sill-gate.sh`, or by hand; check every exit code):

  ```bash
  cargo fmt --all --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace --features sill/debug
  ./scripts/check-boundary.sh
  cargo deny check licenses
  ```

## 11. Migration order

Each step lands through the gate worktree and keeps master green; quire's own steps land first
where noted, and sill compiles against each.

1. `sill-testkit` (done): move `env/scratch.rs`, `tests/support/*`, the 13 examples and both
   `test_support.rs`; services tests use it. `_live` tests move to private buses.
2. `Env` completion (done): `Processes` seam (from `programs.rs`), `DdcBus` files into `env/`,
   `Authenticator` into `env/`; delete `curve.rs`.
3. `sill-model` (done): `units`, `ids`, `attempt` first, then one service per commit (state, command,
   step out; effect files into `backend/`), keeping `sill-services` re-exports off: callers
   change in the same commit. Wi-Fi splits from network here.
4. `sill-search` (done): `providers/`, `results/` runtime, `preview.rs` out; `results` state to
   `sill-model`. `services.rs` moves to `sill-surfaces` as a module (it becomes `sill-ui-kit`'s
   in step 9); `Services::spawn` becomes the daemon's `start`.
5. quire steps land (`ds-settings` dioxus optional, `SettingsDoc`, `Word`, the crate split and
   `ds::prelude`); mechanical sill import rewrite in one commit per crate.
6. `sill-settings` pure (done): `Section`, typed values at load, mirrors and `schema_file.rs` gone,
   `focus_link`, `watch`, `feed`, `load`/`save` to their new homes, dead keys retired.
7. `sill-ipc` (done): regrouped, mirrors removed, `sill` `cli/` and `ipc/dispatch/` split, `policy/`
   dir, `Policy` bridges moved.
8. `daemon.rs` (done): split into `daemon/{run,events,surfaces,services,settings,msg}`; per-surface
   spawn blocks wait for step 10.
9. `sill-ui-kit` inside `sill-surfaces` first (done): `hooks`, `links/` (all links as types, the global
   `links.rs` gone), `Reveal<O>`, `SurfaceRoot`, `look`, `Machine` and `use_machine`,
   `MenuPopup<M>`, `status`, `widgets` providers, `harness`. Then rename the crate `sill-ui-kit`.
10. `Surface` trait and generic `daemon/surfaces.rs` (done); one surface per commit converted to the
    one shape (order: osd, hot_corners, idle_dim, wallpaper, power_menu, switcher, polkit_prompt,
    lock, banners, notification_center, screenshot_thumb, dock, launcher, bar, desktop_widgets,
    edit_widgets); each commit moved its tests onto `SurfaceHarness` and its `*_lint.rs` into the
    table. As built: `lock` is a `LockSurface` (`mount_lock`, the `LockHarness`; shell-host's
    session lock spawns one per output, `lock/host.rs`), the launcher's click catcher is
    a surface of its own (`LauncherCatcher`) beside the `Launcher` panel, and `control_center`
    is not a surface: it is the bar's popup and the `Bar` surface mounts it. `OutputSurfaces` is
    only the row's `spawned` list, found through `surface_of::<S>()`; the wallpaper's `Prepare`
    stays a daemon step (`prepare_wallpapers`), and `bar.height_px`'s reserved height is
    re-applied by `daemon/events.rs`'s `resize_bars` because the generic resize re-applies only
    the layer size.
11. Removals inside surfaces (done): G7 estimate stack (FitContent), `bar/menu_track/` (ds's),
    `CcModule`, `WidgetProvider`, dead components, `Visible` to `shell_host::Visibility`.
12. (done) Split the surface crates out of `sill-ui-kit`'s host: `sill-overlays`, `sill-notify-ui`,
    `sill-widgets-ui`, `sill-dock`, `sill-launcher-ui`, `sill-bar`; magnify to `sill-dock`,
    switcher derivation to `sill-overlays`; delete `sill-surfaces`.
13. (done) shell-host `Inject::Key`: delete the script stand-ins, `launcher_debug` and the
    `DryRunSession` key paths. `dev/key-sink.py` stays: a focus-holding test client for
    `bar.sh --real-input`, not a key stand-in.
14. (done) `dev/lib/*.sh` and one script per surface with `--real-input`; untrack `dev/out/*.png`.
15. (done) Update `check-boundary.sh` to section 1's table; prune FINDINGS to open items and
    standing facts.
16. User styles (done): `daemon/settings/user_style.rs`, the feed field, `SurfaceRoot`'s `user_style`
    and `data-surface`, the `sill style check` verb, a dev script that edits the file under a
    running daemon on headless KWin and proves every surface restyles without a restart
    (quire's step 16 lands first).

