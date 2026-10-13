# Architecture

sill is a desktop shell for COSMIC: the bar, dock, launcher, wallpaper, notifications, control
center, lock screen and the rest, drawn with quire's design system on shell-host (Blitz on
Wayland layer surfaces). cosmic-comp is the compositor. KWin is a test compositor only.
CONVENTIONS.md holds the rules every repo shares; this file is sill's map, its "one home" table,
its traits, its recipes and its repo rules. Where they disagree, this file wins for sill.

## Sibling repositories

| Repo | What sill takes from it |
| --- | --- |
| quire (`../quire`, path deps) | `ds-core` (vocabulary, units, `Word`, colour, time), `ds-style` (appearance, tokens, materials), `ds-motion` (timelines, `Presence`, gestures), `ds` (components, `DocumentHost`, menus, `ds::prelude`), `ds-shell` (the app-facing parts: account sheets, consent alert, missing-helper sheet; sill takes the account sheets from it), `ds-lint`, `ds-behaviour` (the switcher and hot-corner machines), `ds-settings` (`SettingsDoc`, `Store`, schema derive, the switcher and hot-corner sections; dioxus behind a feature), `ds-blitz` (`provide_host()`, measure, focus, caret, snap), `ds-harness` (`Driver`, `DocQuery`; dev only) |
| shell-host (`../shell-host`) | Host, layer surfaces and popups, input, frames, regions, session lock, scroll engine, `Visibility`, `Inject` (pointer and key, `debug-inject`) |
| anyview (`../anyview`, path deps) | the viewer's format library and its desktop contract: `anyview-core` (kinds, sniffing, the shared file actions and their keys, `Resume`, `Sequence`), `anyview-peek` (every kind's light peek and the `Card` that draws it; the launcher's preview), `anyview-store` (the history the viewer keeps, read for file rows), `anyview-platform` (the viewer's D-Bus requests, `Handoff` and `Play`, which sill calls through `forward_over`; and its freedesktop thumbnail cache, read for a video's frame), `anyview-image` (`Rgba8`, the picture a cached thumbnail becomes). Taken by path until anyview is merged and its revision is pinned in quire's `docs/workspace-deps.toml` (FINDINGS) |
| keycap (`../keycap`, path dep) | `keycap-protocol`: the socket messages, `ProfileName`, `Status`; `sill-model` and `sill-services` take it (serde, serde_json, thiserror only) |
| docket (`../docket`, path deps) | `companion-wire` (the bodies of `org.quire.Companion1`: asks, answers, cards, plans, forms; serde only), `voice-wire` (the bodies of `org.quire.Voice1`: begin, the stream's events, ends, refusals, and the framing; serde only), `voice-dbus` (the caller's proxies and bus names of `org.quire.Voice1`; zbus only, no audio stack) and `docket-core` (the wire vocabulary: `ConfirmRequest`, `ConfirmAnswer`, `Invocation`, `Manifest`; serde data over porter, almanac and stoker types, no runtime), `docket-dbus` (`ControlProxy`, `ConfirmProxy`, the bus names) and `docket-client` (`serve_on`, the `IntentProvider` trait); `sill-model` takes the first (and `prov`), `sill-services` all three. Its `scripts/check-intents.sh` is copied into `scripts/`. Because docket's own path dependencies resolve beside it, a checkout of sill also needs `../porter`, `../stoker`, `../almanac` and `../cua` |
| cua (`../cua`, path deps) | `cua-bus` (a run's state, outcome, budget and window as serde data, for `sill-model`'s run rows) and `cua-dbus` (the `org.quire.Cua1` proxies, `sill-services` only) |
| almanac (`../almanac`, path deps) | `almanac-core` (memoryd's vocabulary: `MemoryItem`, the timeline) and `almanac-dbus` (the `org.quire.Memory1` proxies, `sill-services` only) |
| casement-protocols (`../casement-protocols`, path deps) | `casement-protocols` (the desktop's private protocols; `sill-model` and `sill-services` take only its pure `vocab` enums: `WindowStateKind`, `ProofKind`, `PlaceTarget`, …, no Wayland) and `desktop-client` (one typed client for wlroots-style compositors: toplevels from the ext list and wlr management, workspaces from ext-workspace, outputs from wlr-output-management; `sill-services` takes it with the `client` feature, its tests with `fake`). Taken by path like the other siblings |
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
| 0 | `sill-launcher` | Launcher core: `Provider`, `Query` (generation, cancel), `Item`, `Activation`, matching (nucleo), ranking, sections, frecency, the shared actions a row's keys are (`keys.rs`), preview facts. Pure; a future repo | `almanac-core`, `docket-core`, `ds-core` (`MemoryVerb`), `prov`; nucleo-matcher, serde |
| 0 | `sill-testkit` | Dev only: `PrivateBus`, `TempDir`, signal helpers, stand-in programs in `examples/` | none; std only (its dev dependencies, the crates it tests, are outside the table) |
| 0 | `sill-shell-kit` | The shell-only parts over `ds`, moved here from quire's `ds-shell` (2026-10-07): bar, dock, control center, lock, switcher, OSD, idle dim, notifications, thumbnails, battery, clock, now playing, user picture, emoji, month grid, date picker, widget contract, registry, catalog and the widgets, with the shell's tokens (`tokens/`), sheets (`sheets.rs`), `KIT`, `kits()` and `stylesheet()` (the whole shell stylesheet: quire's plus these) and their SSR goldens and behaviour tests | none of ours; quire's `ds`, `ds-core`, `ds-shell` (its `KIT` and sheets). Dev: `ds-blitz`, `ds-harness`, `ds-lint`, `ds-settings` |
| 1 | `sill-model` | Pure vocabulary: ids, units, `Attempt`, every service's `State`/`Command`/`Event`/`Effect` and its `step`, vocabulary enums shared by a settings key and a service | `sill-launcher`, `keycap-protocol`, `ds-core` (`Stamp`, `Machine`), `docket-core`, `almanac-core`, `companion-wire`, `cua-bus`, `prov`, `porter-core` (the accounts sheet's `SheetView`/`SheetInput`, serde data), `casement-protocols` (`vocab` only: the desktop views' enums); serde, jiff, thiserror |
| 2 | `sill-settings` | `settings.toml` as data: one struct per domain, `Section`, `ShellFile`, schema, typed values parsed at load. No I/O, no runtime, no dioxus | `sill-model`, `sill-launcher`, `keycap-protocol`, `ds-core`, `ds-settings` (no features) |
| 2 | `sill-ipc` | The wire: `Request`/`Response`, one JSON line, 64 KiB cap. Serialises `sill-model` types | `sill-model`, `sill-launcher`; serde, serde_json, thiserror |
| 3 | `sill-services` | Effects: `Env` seams, `Service` trait, runner, `Handle`, `Control`, one dir per service `{mod.rs, backend/}`. UI-free | `sill-model`, `sill-launcher`, `keycap-protocol`, `ds-core` (`Stamp`), `docket-core`, `docket-dbus`, `docket-client`, `companion-wire`, `cua-bus`, `cua-dbus`, `almanac-core`, `almanac-dbus`, `prov`, `porter-core`, `porter-dbus` (who owns a bus name: caller identity; accountd's `Manager` and `Account` proxies), `porter-client` (`dbus` feature only: the typed guest calls and change stream for the guest-asks service), `voice-dbus`, `anyview-core`, `anyview-platform` (the viewer's D-Bus request, for the launch service), `casement-protocols` (`vocab`), `desktop-client` (`client` feature: the wire behind `desktop_wl` and the native displays route); tokio, zbus, rustix, cctk, libpulse, notify, nonstick, ddc, image, toml |
| 4 | `sill-search` | The launcher's runner (`ResultsService`), `providers/` (the files provider reads the viewer's history and takes the shared file actions), clip images | `sill-services`, `sill-model`, `sill-launcher`, `anyview-core`, `anyview-store`, `ds-core`, `docket-core`, `almanac-core`, `companion-wire`, `prov`, `ds-settings` (no features: the schema files the settings provider indexes); fend, emojis |
| 5 | `sill-ui-kit` | What every surface shares: `Services` registry, `SettingsFeed`, `SurfaceRoot`, `Surface` and its `mount`, `links/`, hooks, `look`, `panel`, `shell_style`, `MenuPopup<M>`, `app_icons`, `status`, `notification_card`, `widgets/`, `harness` (feature: fake services, `SurfaceHarness`) | `sill-shell-kit`, `sill-search`, `sill-services`, `sill-settings`, `sill-model`, `sill-launcher`, `companion-wire`, `docket-core`, `porter-core`, `prov`, `ds-core`, `ds`, `ds-settings` (dioxus), `ds-blitz`, `ds-harness` (optional: feature `harness`), `shell-host`; dioxus, resvg, image; dev: `ds-lint` |
| 6 | `sill-bar` | Bar, its menus, tray, control center and its modules | `sill-ui-kit`, `sill-model`, `sill-settings`, `sill-launcher`, `sill-services`*; `sill-shell-kit`, `ds`, `ds-settings`, `ds-blitz`, `shell-host`; dev: `ds-lint` |
| 6 | `sill-dock` | Dock, magnify, context menu | `sill-ui-kit`, `sill-model`, `sill-settings`, `sill-launcher`, `sill-services`*; `sill-shell-kit`, `ds`, `ds-blitz`, `shell-host`; dev: `ds-lint` |
| 6 | `sill-launcher-ui` | Launcher panel and catcher, rows, preview pane, actions | `sill-ui-kit`, `sill-model`, `sill-settings`, `sill-launcher`, `sill-search`*, `sill-services`*, `anyview-core`, `anyview-image`, `anyview-peek` (what a file's pane looks like), `anyview-platform` (the thumbnail cache behind a video's frame, `preview::frames`), `almanac-core`, `companion-wire`, `docket-core`, `prov` (a companion row's pane and keys); `ds`, `ds-blitz`, `shell-host`; dev: `ds-lint`, `ds-shell` |
| 6 | `sill-notify-ui` | Banners, notification center, screenshot thumbnail | `sill-ui-kit`, `sill-model`, `sill-settings`, `sill-services`*; `sill-shell-kit`, `ds`, `ds-settings`, `ds-blitz`, `shell-host` |
| 6 | `sill-widgets-ui` | Desktop widget layer and its context menu, Edit Widgets | `sill-ui-kit`, `sill-model`, `sill-launcher`, `sill-settings`; `sill-shell-kit`, `ds`, `shell-host`; dev: `ds-lint` |
| 6 | `sill-overlays` | OSD, power menu, polkit prompt, switcher, idle dim, hot corners, wallpaper, lock | `sill-ui-kit`, `sill-model`, `sill-settings`, `sill-launcher`, `sill-services`*; `ds`, `ds-behaviour` (the switcher and hot-corner machines), `ds-shell` (the accounts sheet), `sill-shell-kit`, `ds-settings`, `shell-host`, `porter-core` (the accounts sheet's props) |
| 7 | `sill` | Binary and library: `cli/`, `daemon/`, `ipc/`, `policy/` | every crate above except `sill-testkit`; `ds-blitz`, `ds-settings`, `shell-host`; clap, latchkey, `ds-lint` and `sill-shell-kit` (`sill style check` lints against its `kits()`); dev: `sill-testkit`, `sill-ui-kit` (harness), `ds` |

`*` A surface crate names `sill-services` (and `sill-search` for the launcher) only for service
marker types (`BatteryService`), `Control` and `Handle`, the type arguments of `use_service`.
It never names `Env`, `Service::run`, `spawn` or a `backend`. Surface crates never depend on one
another, and none depends on `sill`.

Boundary script (`scripts/check-boundary.sh`; `RULES` check the output of `cargo tree -i`, never
its status, and `EDGES` compare each crate's direct path dependencies with the table above):

| Crates | Reach none of (through any path) |
| --- | --- |
| `sill-launcher`, `sill-model`, `sill-ipc` | dioxus, dioxus-core, zbus, wayland-client, tokio, libpulse-binding, shell-host, ds, ds-shell, sill-shell-kit, ds-blitz, blitz-dom |
| `sill-settings`, `sill-search` | dioxus, dioxus-core, blitz-dom, shell-host, ds, ds-shell, sill-shell-kit, ds-blitz (`ds-settings` brings zbus and tokio to both) |
| `sill-services` | the same, and `ds-settings` |
| `sill-testkit` | every other sill crate |
| `sill-shell-kit` | every other sill crate, zbus, tokio, wayland-client, libpulse-binding, shell-host, ds-blitz, blitz-dom |
| `sill-ui-kit` | every surface crate, `sill` |
| each surface crate | every other surface crate, `sill` |

## 2. Inside each crate: top-level modules, lowest layer first

A module names only modules above it in its own table. `mod.rs` and `lib.rs` declare and
re-export; the one type a module is named after may sit in its `mod.rs`.

| Crate | Modules (layer order) |
| --- | --- |
| `sill-model` | `units`, `ids`, `attempt`, `vocab` (enums shared with settings), then one dir per service (`battery`, `clock`, ... 31 services, `companion` and `intents` among them: its `sheet/`, `terminal/` and `shell/` sub-dirs are the confirmation queue, the terminal's grants and the shell's own actions; `results` holds search state; `focus` holds the focus mode, which is not a service; `keyboard` holds the pure keyboard-profile choice, which is not a service either, and `keycap` the keycap service's state and its pure decisions: the focus to send, the notice to post, how to read the unit, how long to wait to reconnect). Each service dir: `mod.rs`, `model.rs` (State), `command.rs` (Command, Event, Effect), `step.rs`, `tests.rs`; a sub-concept gets a sub-dir with the same roles. Service dirs never import each other |
| `sill-settings` | `section` (trait), one file per domain (`bar`, `dock`, ..., 22 domains), `file` (`ShellFile`, `SettingsDoc` impl), `schema`, `keys` (typed enums and unit newtypes that no service shares) |
| `sill-ipc` | `line` (encode, decode, cap), `error`, `request`, `response`, `show`, `status`, `act`, `debug` (one word file per request group) |
| `sill-services` | `error`, `control`, `stream`, `env/`, `service`, then `file_touch` (a debounced watch on one file), then one dir per service (29, all but `results`) in dependency order of their `Deps`; a service dir never imports another service dir: cross-service data is `sill-model` types in `Deps` |
| `sill-search` | `preview` (a clipboard image), `providers/<kind>/` (`files/`: `actions` (the shared file actions as a row's), `viewed` (the viewer's history as rows and uses)), `results/` (`registry`, `run`, `sink`, `history`, `frecency_file`, `effect`), `ResultsService` in `results/mod.rs` |
| `sill-shell-kit` | `tokens`, then leaf parts `battery`, `clock`, `emoji`, `month_grid`, `now_playing`, `notifications`, `osd`, `idle_dim`, `dock`, `bar`, `control_center`, `switcher`, `date_picker` < `user_picture`, `thumbs` < `lock` < `widget` (`contract`, `registry`, `wire`, `card`, `frame`, `gallery`, one dir per kind) and `catalog` < `sheets`, `kit` < `prelude` |
| `sill-ui-kit` | `look`, `links/`, `settings`, `shell_style`, `services`, `hooks/`, `panel`, `root`, `surface/` (`mount`, `lock`, `start`, `place`, `naming`), `menu/`, `status/`, `app_icons/`, `notification_card/`, `widgets/`, `harness/` (feature `harness`) |
| each surface crate | one dir per surface, named after it (`osd/`, `dock/`, `banners/`); `lib.rs` re-exports each surface marker type and nothing else |
| `sill` | `policy/`, `ipc/`, `cli/`, `daemon/` (`daemon` is the top: it names every crate) |

### The one shape of a surface (every dir in a surface crate)

| File | Holds |
| --- | --- |
| `mod.rs` | declarations only |
| `surface.rs` | the marker type named after the surface and its thin `impl Surface` |
| `layer.rs` | `fn layer(&OutputInfo, &ShellFile, slot) -> LayerConfig` and its constants |
| `model.rs` | state types (data only) |
| `step.rs` | the pure transitions; `impl Machine` (quire's, `ds::base::machine`) for a timed one |
| `keys.rs` | the keymap table, when the surface takes keys |
| `io.rs` | `use_<surface>`: reads services and settings, follows the link, feeds `use_machine`, carries effects |
| `view.rs` | components; no decisions beyond reading the model |
| `style.css` | the concept's stylesheet, `include_str!`ed in `view.rs` |
| `tests.rs` | unit tests; integration tests are `crates/<crate>/tests/it/<surface>.rs` on `SurfaceHarness<S>` |

A concept over about 300 lines becomes a sub-directory with the same roles (`dock/machine/`,
`launcher/preview/`, `control_center/pages/`). No files named `drive`, `link`, `presence`, `session`, `machine` or `panel`: those names for "the model" and "the io" are
`model.rs`, `step.rs`, `io.rs`.

---

## 3. Where the old code went

<!-- paths: skip -->
The old locations of what moved, and where each landed.

### `sill-services` (ends with 30 services and the seams)

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
| `desktop/{backend,command,model,note}.rs` | `sill-model/src/desktop/` (the arm names and the globals offer, the desktop's views and commands, `supports`, and `ShellNote`, the compositor's shell protocol in the shell's words) |
| `desktop_wl/{choose,ids,toplevels,workspaces,view,command,outputs,offer,run}.rs` | `sill-services/src/desktop_wl/` (the standard-protocol arm over `desktop-client`; its thread is started by `cosmic_wl/backend/thread.rs`, which picks the arm with `desktop_wl::backend_of`: `desktop_client::backend_for` on the offered globals, Quire, else Wlr, else Cosmic; the ext list beside COSMIC's own toplevel info stays Cosmic). On a Quire offer the same thread serves the toplevels and workspaces on the environment connection |
| `desktop_wl/quire/{handout,surfaces,view,command,notes,arm,shell}.rs` | the Quire arm (P2-SQ): `handout` is the seam for `org.quire.Compositor1.Connect("shell")` (`Handout`, `ZbusHandout`, `FakeHandout`, `shell_connection`); `surfaces` records which registered surface has which `ShellRole` (and `role_of_surface`, the `Surface::NAME` table); `view` and `command` are the pure mappings (`quire_view`, `quire_wire_of`) between `DesktopState::quire` and the shell's `DesktopView` and `DesktopCommand`; `notes` is `note_of` (the shell protocol's events as `ShellNote`s); `arm` is `QuireArm<D: Desktop, S: NoteSink>` (sync: drain events, hand notes to the sink, return the view; execute: a command as requests; assign: `SetShellRole`), tested over `FakeDesktop::with_config`; `shell` runs the arm on the shell host's connection (`spawn_shell_arm(&wl_surface)` takes the connection from the surface, binds a second queue with `DesktopConfig::holding_shell()`, publishes a `watch` of `DesktopView` and an mpsc of notes). The daemon side is `sill/src/daemon/compositor.rs`: the host's route (`Handed(fd)` when the compositor handed one), the `ShellSeat` that starts the arm at the first mapped surface with a role and registers each mapped surface, and `apply`, which carries each `ShellNote` out (hot corner, the companion's tap, the kill chord) or names the P3 lane that will (`P3-SW`, `P3-MC`, `P3-MN`, `P3-SILL`) |
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
| `pins/{fs_watch,run}.rs`, `pins/test_support.rs` | `pins/backend/run.rs`, and `fs_watch.rs` becomes the shared `file_touch.rs` (`keycap` watches `keycap.toml` with it); `test_support.rs` goes to `sill-testkit` |
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
| `sill-settings/src/{bar,calendar,control_center,display,dock,hot_corners,idle,keyboard,launcher,menus,notifications,osd,power_menu,screenshot,scroll,scrollbar,session,spaces,switcher,widgets,window}.rs` | stay, each `impl Section`; keys with no reader deleted; `flatten extra` and `version` deleted |
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
| `src/hooks/*`, `host_modality.rs`, `src/lib.rs` host config helpers | `sill-ui-kit/src/{hooks,root}`; `clock.rs` (`FrameClock`) is `ds::base::time::stamp::FrameClock` |
| `src/app_icons/`, `surfaces/notification_card/`, `surfaces/user_picture.rs`, `surfaces/naming.rs` | `sill-ui-kit/src/{app_icons,notification_card,surface}`; `user_picture` is `sill-shell-kit`'s |
| `src/widgets/` (providers, layout, wake) | `sill-ui-kit/src/widgets/<kind>/` |
| `bar/menu_track/` | deleted; `ds`'s `menu_track` |
| `bar/{menu_popup,menus,dbusmenu}.rs`, `dock/{menu_popup,context_menu,menu_document,menu_geometry}.rs`, `launcher` actions menu | `impl MenuModel` in the owning surface; the popup is `sill-ui-kit::menu::MenuPopup` |
| `bar/{glyph_state,status_item,status}.rs`, `low_battery.rs` | `sill-ui-kit/src/status/` (service state to `sill-shell-kit` glyph props) |
| `bar/*` (rest), `control_center/*` | `sill-bar/src/bar/`, `sill-bar/src/control_center/` |
| `control_center/{fit,sizes}.rs`, estimate half of `geometry.rs`, `bar/geometry.rs`, `menu_size_tests.rs`, `tests/cc_estimates.rs` | deleted (FitContent popups measure) |
| `dock/*` | `sill-dock/src/dock/` |
| `launcher/*` | `sill-launcher-ui/src/launcher/`; `OpenSerial`, `PanelFocus`, `LauncherShown` to `sill-ui-kit/src/links/launcher.rs`, `Visible` to `shell_host::Visibility` |
| `banners/`, `notification_center/`, `screenshot_thumb/` | `sill-notify-ui/src/` |
| `desktop_widgets/`, `edit_widgets/` | `sill-widgets-ui/src/{desktop,edit}/` |
| `osd/`, `power_menu/`, `polkit_prompt/`, `confirm_sheet/`, `voice_consent/`, `accounts_sheet/`, `capture_sheet/`, `switcher/`, `idle_dim/`, `hot_corners/`, `wallpaper/`, `lock/` | `sill-overlays/src/<same>/` |
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
| `policy.rs`, `calendar.rs`, `displays.rs`, `idle.rs`, `launcher_policy.rs`, `notifications/policy.rs`, `screenshots.rs` (policy half), `clipboard.rs` (policy half) | `policy/{mod,follow}.rs` and one file per bridge: `calendar`, `displays`, `idle`, `keyboard`, `notifications`, `screenshots`, `search`, `clipboard` |
| `lock/`, `idle_dim.rs`, `corners.rs`, `osd.rs`, `scroll.rs`, `launch.rs` | `daemon/followers/{lock,idle_dim,corners,osd,scroll,launch}.rs`: act on the host from a service or a setting |
| `power_menu.rs`, `switcher.rs`, `widgets.rs`, `edit_widgets.rs`, `polkit.rs`, `session.rs`, `notifications/mod.rs`, `screenshots.rs`, `clipboard.rs` (verb halves) | the verb halves are `ipc/dispatch/{show,status,act}.rs` arms; the spawn halves are gone (generic `daemon/surfaces.rs`) |
| `launcher_debug.rs`, `notifications/fake.rs` | `ipc/dispatch/debug.rs` (debug feature) |
| `tests/{ipc,latency}.rs` | stay |

<!-- paths: end -->

---

## 4. The one home table

Look a concept up here before writing it. If it has a home, extend it.

| Concept | Home |
| --- | --- |
| Everything ambient (buses, Wayland route, directories, wall clock, DDC, session actions, authenticator, idle times, polkit route, processes, children, keycap's socket path) | `sill_services::env::Env` (`keycap_socket`: `/run/keycap/keycap.sock` from the process, none when isolated, so no test or script reaches the real daemon) |
| Starting a child nobody waits on: the file opener, the trash, a default handler | `sill_services::env::Children` (`Env::children`): `RealChildren` in the daemon, `RecordedChildren` in `Env::isolated`, which keeps each `Start` and runs nothing, so a test asserts the argv and never opens a file in the person's session. `tests/spawn_sites.rs` lists the only files that may name `Command::new` |
| The focused window's keyboard profile | `sill_model::keyboard::wanted` over `ProfileSource`s (the user's overrides, what the app declares, keycap's own per-app profiles such as `ghostty`, then the category table); sent by `sill_services::keycap` (`focus_to_send` decides when) |
| The person's per-app keyboard profiles (`keyboard.overrides`) | `sill_settings::keyboard` (`KeyboardSettings`; `overrides` parses the rows once, per row, with a line for each one dropped), told to keycap by `sill::policy::keyboard` as `KeycapCommand::Overrides`; `KeycapAsk` is the part of the command on the IPC wire |
| The confirmation sheet's queue, its answers and receipts | `sill_model::intents::sheet` (`Sheets`, `SheetChoice`, `step`; the receipt says `SheetFallback`, never `HardwareSeat`), served as `org.quire.Confirm1` by `sill_services::intents` |
| What the sheet says (title, byline, facts, reasons) | `sill_model::intents::sheet::sheet_view`: fixed words and the router's own formatting, never a model's text |
| Hold-to-talk: the hold's phases, the words heard, the refusal told, the consent sheet's state | `sill_model::voice` (`Voice`, `step`); the bus half is `sill_services::voice` (`org.quire.Voice1` as the shell on intents' seat; the stream is `voice_wire::unframe`) |
| What the person is told while holding or after a refused hold | `sill_model::voice::{listening_line, notice_line}`; the launcher's prompt shows the line as its placeholder |
| `voice.hold_to_talk` and `voice.consent` (the forms `voiced` reads) | `sill_settings::voice` (`VoiceSettings`; `VoiceConsent`'s serde is the contract); written by `sill::voice::write_consent` from the consent sheet's answer |
| The consent sheet | `sill_overlays::voice_consent` (quire's `Sheet`, mapped on the banners' output while the service says `Asking`) |
| Arming, Escape and the hold of the confirmation sheet | `sill_overlays::confirm_sheet::gate` (a `Machine` on quire's clock) |
| The terminal's standing grants, the breaker, Resume | `sill_model::intents::terminal`; read and changed through intentd's `Control` by `sill_services::intents`; drawn by the control center's `Terminal` module |
| The shell's own actions as `org.quire.Shell` | declared in `dist/intents/org.quire.Shell.toml` (installed by `dist/install.sh`), read by `sill_model::intents::shell`, served by `sill_services::intents` (the provider), done by `sill::intents` (the same requests `sill <verb>` makes) |
| The shell's window and workspace actions (`shell.window.*`, `shell.app.*`, `shell.workspace.list`, `shell.workspace.switch` to a workspace) | read by `sill_model::intents::shell::window_ask_of` into a `WindowAsk`, decided against the compositor's state by `plan` (the Hide, Minimise and raise commands are `app_menu::own_commands`, the one implementation the bar's app-name menu uses too), the listing text by `crates/sill-model/src/intents/shell/listing.rs`, sent to the compositor service by `sill_services::intents` (`crates/sill-services/src/intents/backend/windows.rs`); `shell.screenshot` answers with the path the folder watch saw (`crates/sill-services/src/intents/backend/shots.rs`) |
| Notices about keycap (failed, conflict, paused) | `sill_model::keycap::{attention, notice_for, notice_text}`; posted by `sill_services::keycap::backend::notice` (no action: `sill keycap retry` and `resume` are the way out) |
| Which accounts need signing in again, the notice that says so and its "Sign in again" button | `sill_model::accounts::{step, notice_text, sign_in_key, account_of, shown_for}` (the button's action key carries the account); `sill_services::accounts` follows accountd, posts and withdraws the notice, and answers the button's `ActionInvoked` (only from the notifications server's own connection) with the account's `Reauthenticate` |
| Each output's work area (the output less the bar's and dock's exclusive zones) and scale, for apps | `sill_model::work_areas::{work_area, work_rect, Zone}` (pure reduction); `sill::work_areas` derives the zones from the bar's and dock's own layer configurations and publishes; `sill_services::work_areas` serves `org.quire.Outputs1` (property `WorkAreas`, `a(us(iiuu)(iiuu)u)`: id, name, frame, work, scale in 120ths; read-only; `PropertiesChanged` on change); quire's `ds-desktop::Outputs` reads it |
| Each workspace's id, name, output, `SpaceLook` and activity, for apps that keep their Spaces from the desktop (`SpacesSource::Desktop`) | `sill_model::workspace_looks::published` (pure: the compositor's workspaces with a look per position); `sill::workspace_looks` gives each the look every surface draws (`sill_ui_kit::look`) as quire's JSON and publishes; `sill_services::workspace_looks` serves `org.quire.Workspaces1` (property `Workspaces`, `a(ssssb)`: id, name, output, look JSON, active; read-only; `PropertiesChanged` on change) |
| Where a minimised window flies to (the dock icon of its app, resting size, not magnified) | `sill_model::desktop::target::{OutputRects, MinimiseTargets}` (pure: `of` maps icons and windows to targets, `since` is the diff into `SetMinimiseTarget`/`ClearMinimiseTarget`); `sill_dock::dock::rest_rects::app_rects` computes the icons' rectangles; `sill_ui_kit::dock_rects::DockRects` carries them; `sill::daemon::compositor::ShellSeat::retarget` sends the diff, on the Quire arm only |
| The accounts sheet (accountd's consent question and add-account steps): what is open, what is typed, what is sent back | `sill_model::accounts::sheet::{AccountsSheets, step, SheetAction}` (the draft holds a typed secret until the reply that carries it); `sill_services::accounts::sheet` serves `org.quire.AccountsSheet1` (only the owner of `org.quire.Accounts1` may open, update or close; `Input` goes to the opener alone); `sill_overlays::accounts_sheet` draws it (`props.rs` maps `SheetView` to quire's `ds-shell::accounts` props) |
| Where the double tap and a summon go: the two machines, the Ask, the answers, the runs, the journal, the lookups | `sill_model::companion::{Tap, Summon, step}` (quire's `Machine`, stepped by `sill_services::companion`, which publishes `Companion`); the daemon's half (what is focused, the launcher's open, switch and close, the four `sill companion` verbs) is `sill::companion` |
| The companion's presence (the orb and the glow) | `sill_model::companion::presence_of`, the only derivation (`ds_core::vocab::CompanionPresence`) |
| What a card press is doing (which action a button calls, when its press ended, which kinds of thing open in their apps, which handles still wait for their text) | `sill_model::companion::{card_call, cards_of, promote_card, openers_of, unresolved_handles}` and `Busy`; the service carries out `Answer.Act`, `Session.Display` and the registry read (`sill_services::companion::backend`) |
| What the settings say about the double tap and the summon | `sill_settings::companion_params` (`companion.*`, design/22 section 3.30) as `CompanionParams`, sent by `sill::policy::companion::CompanionBridge`; every duration is `sill_model::units::Ms` |
| A run's state and the controls it offers; which runs a stop-all pauses | `sill_model::companion::{run_state_of, run_controls, going_runs}` (cuad's phase and outcome in, `RunState` out) |
| What a companion row says (an answer, a journal row, a memory row) | `sill_model::companion::{answer_line, activity_line, memory_line}`: fixed words and the person's own, never a model's formatting |
| The companion's wire as quire's view types (an answer card, its footer, a plan, a form, a run's state) | `sill_ui_kit::companion`, one total function per pair, each with a `*_total` test |
| The screen-sharing picker (transom asks what an app may share; the person picks) | `sill_model::capture_pick::{Ask, Candidates, plan, Picks, toggle, reply, step, windows_of}` (pure: kinds, several-or-one and pointer modes decide the choices; a pick becomes the `(sources, cursor)` reply); `sill_services::capture_pick` exports `org.quire.Shell1` on the seat's connection (no name of its own; only the owner of `org.freedesktop.impl.portal.desktop.quire` may ask; a second call while one is open is refused); the daemon feeds windows from the compositor arm's `DesktopView` (`ShellMsg::Desktop`) and screens come from the displays service; `sill_overlays::capture_sheet` draws it on quire's `Sheet`, `SegmentedControl`, `List` and `Button` |
| The connection everything said to intentd and the companion goes out on | `sill_services::seat` (the intents service publishes the connection that owns `org.quire.Shell`; intentd names a caller's role from the names its connection owns, so a turn or a search from any other connection is not the launcher's) |
| The launcher in companion mode: its keys, the pane that follows the selected row, what an ask keeps | `sill_launcher_ui::launcher::companion` (`companion_step`, `pane_of`, `ask_input`, `command_of`); the mode itself is `Opening::mode` (`sill_ui_kit::links`), switched in place by `LauncherLink::switch_mode` |
| The focused app's menus: the service, the titles the bar makes, the shell's own rows, what an agent's activation costs | `sill_model::app_menu` (`AppMenus`, `titles`, `own_groups`, `route_of`, `effect_of`: the one table), served by `sill_services::app_menu` (`AppMenuService` follows each followed app's `org.quire.AppMenu.<app_id>`; `MenuAgent` is `shell.menu.list` and `shell.menu.activate`); the bus name is quire's `AppMenuAddress`, handed in as `sill_ui_kit::app_menu::menu_name_of`; the bar draws them as `BarMenu::AppMenu` through the one `MenuPopup`, rows through `bar/dbusmenu.rs` (also the tray's) |
| Who owns a bus name, as an app (proof for a command id an app claims) | `porter_dbus::ProcCallers::caller_of_pid` over `Env::proc_root`, in `sill_services::app_menu::agent::identity` |
| Watching one file for edits | `sill_services::file_touch` |
| A bus daemon that starts late or restarts (UPower, power-profiles, NetworkManager, BlueZ) | `sill_services::owner` (`appeared`, `changed` on `NameOwnerChanged`); a service runs its session only while the name is owned and resets its state when it is not. A `notify` watch signals only on writes, never on the open/read/close a scan itself makes (`apps::backend::run::changes`) |
| Idle cost (CPU, threads, RSS of the daemon doing nothing) | `dev/accept/idle-cpu.sh` (nested, 60 s sample; fails above 1 % CPU or 120 threads); the runtime's blocking pool is capped in `daemon::run` |
| Service trait, runner, `Handle<S>`, `Control<C>` | `sill_services::{service, control}` |
| A service's State, Command, Event, Effect | `sill_model::<service>::{model, command}` |
| A service's pure transition | `sill_model::<service>::step::step` |
| A service's I/O | `sill_services::<service>::backend` |
| Every running service's handle, `Member`, `use_service` | `sill_ui_kit::services`, `sill_ui_kit::hooks` |
| Starting every service in `Deps` order | `sill::daemon::services::start` |
| Units and ids (`Cpx`, `Minutes`, `AppId`, `WorkspaceKey`) | `sill_model::{units, ids}` (`Percent`, `Fraction`, `Stamp`: `ds-core`) |
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
| What a file looks like in the pane (sniffing, the peek of each kind, the pane that draws it) | `anyview_peek::probe`, `peek_with`, `Card`; sill's part is `sill_launcher_ui::launcher::preview::load` (the worker and the budget), `frames` (`ThumbnailFrames`: `anyview_peek::VideoFrames` over the freedesktop thumbnail cache in the injected `Dirs::cache`, so a video without a cover shows the file manager's picture of it, or the facts card when there is none or it is stale) and `content` (the pane in quire's `PaneContent::Slot`). sill names no format |
| Which actions a file's row has, and their keys | `anyview_core::actions_for`, `reach`, `shortcut`, through `sill_search::providers::files::actions::{carried, row_actions}` (`carried` is the one match on `FileAction`: what the launcher does for each) |
| The files the viewer opened, as launcher rows and as frecency | `anyview_store::read_history` through `sill_search::providers::files::viewed` |
| Opening a file in the viewer, playing audio | `OpenTarget::{Viewer, Play}` (`sill-launcher`), carried out by `sill_services::launch::backend::viewer` as one call on `anyview_platform::linux::forward_over`; the results the viewer walks are added by `sill_launcher_ui::launcher::activate::with_results` |
| Surface root: services, feed, host seam, appearance, scale, frame clock | `sill_ui_kit::root::SurfaceRoot` (the host seam is `ds_blitz::provide_host()`, called by `mount`, `mount_lock` and each popup root; the seat's `HostSignals` (scale, modality) come from `use_host_signals()`, called by `mount`, `mount_lock` and `PopupFrame`, and a surface reads its scale with `use_host_scale()`) |
| Active workspace look, space look | `sill_ui_kit::look` (`use_active_look`, `use_space_look`) |
| Open/close a panel from outside it (serial, output) | `sill_ui_kit::links::Reveal<O>`; every link type is one file in `sill_ui_kit::links` |
| Visible or hidden layer surface | `shell_host::Visibility` |
| Layer namespace `sill-<surface>-<output>` | `sill_ui_kit::surface::namespace` |
| Spawning and closing surfaces per output | `sill::daemon::surfaces` (generic over `Surface`) |
| Which surfaces exist on which output | `sill::daemon::outputs` |
| A timed pure state machine and its timer; a machine's `Stamp` and the `FrameClock` that makes it | quire: `ds::base::machine` (`Machine`, `Elapsed`), `ds::machine` (`use_machine`, `MachineRef`), `ds::base::time::stamp` (`Stamp`, `FrameClock`); sill has none of its own |
| Popup menus (bar, tray, dock, launcher actions) | `sill_ui_kit::menu` (`MenuModel`, `MenuPopup<M>`) over `ds`'s `menu_track`; a context menu a layer opens at its pointer is `context_menu_popup::<M>` (the popup root, reporting over a channel) |
| Popup sizing | shell-host `FitContent`; no estimation code |
| App icons, look, cache | `sill_ui_kit::app_icons` |
| Status glyph props from service state | `sill_ui_kit::status` over `sill-shell-kit` glyphs |
| Notification card | `sill_ui_kit::notification_card` |
| Widget provider, `Card<P>`, registry, layout mapping | `sill_ui_kit::widgets` |
| Desktop widget grid; the desktop's context menu; Edit Widgets sheet | `sill_widgets_ui::{desktop, desktop::menu, edit}` |
| Where each output's desktop widgets and their cells are kept | `sill_ui_kit::links::PlacementFile` (`desktop-widgets.json`: per output a widget list, once Edit Widgets has edited it, and the cells) |
| Control-center module | `sill_bar::control_center::modules::<name>`, picked by `module(kind)` |
| Dock magnification | `sill_dock::magnify` |
| Switcher app derivation | `sill_overlays::switcher::apps` |
| Easing and springs | `ds-motion` |
| Colour, tint, material | `ds-core`, `ds-style`; sill never computes colour |
| Keymaps | `<surface>/keys.rs` table; key claims are `ds`'s; a shortcut is a chordkit action resolved by quire's keymap (`sill_ui_kit::root::provide_keymap` gives every root keycap's source), never a modifier test or a chord type of our own |
| Stylesheets | `<concept>/style.css` |
| Session lock: service, surface, host half | `sill_services::locker`, `sill_overlays::lock`, `sill::daemon::followers::lock` |
| Errors | `sill_services::ServiceError`, `sill_ipc::error::IpcError`, one `thiserror` enum per crate |
| Private bus, scratch dirs, stand-in programs | `sill_testkit` |
| Surface test driver | `sill_ui_kit::harness::SurfaceHarness<S>` |
| Lint of surface markup | `crates/sill/tests/lint.rs`, one table |
| Dev script helpers | `dev/lib/*.sh` |
| A demo desktop (nested, scratch, left running) with a scripted tour, shots and a video | `dev/demo.sh start\|tour\|video\|shot\|stop` |

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

Implementors: the 30 `<Module>Service` markers (`enum XService {}`), 29 in `sill-services`,
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

Implementors: the 21 domain structs. One per settings table; a new key is a field, not a
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

Implementors: `calendar`, `companion`, `displays`, `idle`, `keyboard`, `notifications` (`changed` waits on the focus
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
clipboard, emoji, web, things, companion, runs, memory (the last four read the companion service's
state; things and memory ask it for a lookup and wait for the answer under the query's generation). One per source of rows; `activate` is pure and returns an `Activation`.

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

### `Machine` (quire: `ds::base::machine`, hook `ds::machine`)

The trait, `Elapsed`, `use_machine`, `use_machine_in` and `MachineRef` are quire's, with the same
signatures (`step(self, In, at: Stamp, &Params, &Ctx) -> (Self, Vec<Out>)`, `wake(&self) ->
Option<Stamp>`, no `Default`: the surface seeds the first state; `Ctx` is what a step reads from
outside the machine, read by a closure at every step; see quire ARCHITECTURE.md section 5), and so are `Stamp` (whole milliseconds from an origin) and the
`FrameClock` that makes one from the thread's clock, which a harness's virtual clock drives. sill
keeps neither a trait nor a stamp of its own.

Intended implementors: dock hover/press/reveal/label/bounce, switcher, osd, banners deck, hot
corners, screenshot thumb, power menu, polkit stage, notification-center keys, launcher keys. One
per state that changes over time or by input. A state that only maps values is a free `step`.
Implemented so far: the confirm sheet's `AnswerGate` (seeded arming), the hot corner (quire's
`ds_behaviour::hot_corner::Corner`, run by `use_machine`), and the switcher (`switcher/opening.rs`
wraps quire's `Switcher` to add the modifiers' state to its context). The other surfaces keep a
pure `step` taking a `Stamp` and run their own timers in `io.rs`; each moves to `use_machine`
when it is next touched.

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
    type W: sill_shell_kit::prelude::Widget;        // a value: `Widget: Default`
    const KIND: WidgetKind;
    const DESKTOP_SIZE: WidgetSize;           // its one size on the desktop (tiles are all Medium)
    fn use_feed() -> Feed<Self::W>;           // reads the services; one hook, one shape
    fn registry_id() -> sill_shell_kit::prelude::WidgetKind;  // provided: `W::kind()`, the name in quire's registry
}
pub struct Feed<W: sill_shell_kit::prelude::Widget> {
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
    fn entries(&self) -> Vec<ds::prelude::MenuItem<Self::Pick>>;
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
   installed the host seam (`ds_blitz::provide_host()`) and the seat's `HostSignals`; a `Ds` root passes no `scale`.
3. A link the daemon or another surface drives is one file in `sill-ui-kit/src/links/<name>.rs`
   (a newtype over `Reveal<O>` or a small watch channel) and a field of `Links` in
   `links/mod.rs`. A surface never imports another surface.
4. Settings keys: recipe below. Services it reads: `use_service` in `io.rs` only.
5. Timed state: `impl Machine` (`ds::base::machine`) in `step.rs`, run by `ds::machine::use_machine` in `io.rs`.
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
4. Tests: a row in `sill-bar/tests/control_center_render.rs`, whose loop runs every `ControlCenterModule::ALL`
   through tile, dropdown and detail on `SurfaceHarness::<ControlCenter>`.

### Add a widget

1. The `sill_shell_kit::prelude::Widget` (its recipe is `sill-shell-kit`'s: `widget/<kind>/{mod, model, view, style.css}` and a row in `widget/registry.rs::builtin()`), then a `WidgetKind` variant in
   `sill-settings/src/widgets.rs`.
2. `sill-ui-kit/src/widgets/<kind>/{mod,provider,io,tests}.rs` (+ `style.css` if it has one):
   `impl WidgetProvider` (`use_feed` over services) on a unit struct in `io.rs`.
3. `const DESKTOP_SIZE` on the provider, and the one arm in `widgets/card.rs::with_provider`;
   `WidgetKind::ALL`, the slug, `kind_id`, `size_of` and the element id follow. Add the
   variant's row to the table test in `widgets/tests.rs` (slug, element id, size).
4. Tests: entry table in `tests.rs`; a row in `sill-widgets-ui/tests/desktop_widgets.rs` and in
   `sill-notify-ui/tests/center_widgets.rs` render loops (both hosts).

### Add a launcher provider

1. `ProviderKind` variant, and an `Activation` variant only if none fits, in `sill-launcher`.
2. `sill-search/src/providers/<kind>/mod.rs`: `impl Provider`; effects take `Env` values
   (paths, buses) through the constructor.
3. Register in `sill-search/src/results/registry.rs`, in rank order.
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
   A list of tables is a `Vec<Row>` with `#[derive(SettingsRow)]` on the row (quire design/22
   §9.1); the file keeps the person's text, a function in the domain file turns the rows into
   typed values once per load and names each row it dropped, and a malformed row costs only
   itself (`keyboard.rs` is the pattern).
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
   `XDG_*`, its own `XDG_RUNTIME_DIR`, on the compositor `start_compositor` brings up:
   `SILL_COMPOSITOR=casement|cosmic|kwin` (the nested cosmic-comp, shell-host's headless KWin, or
   a headless casement started in the jail through the casement checkout's `dev/up.sh`); a
   script without that variable keeps its own `--default`. Compositor-specific work goes
   through the lib's helpers (`set_scale`, `output_size`, `shot`, `maximised_height`,
   `surface_click`, `input`), never a compositor's own tool; a step the compositor cannot prove
   yet returns `expected_skip WHY` (exit 2).
4. Logs and captures go under `$SILL_OUT_ROOT` and `$SILL_SHOTS` (`target/` and `dev/out/`,
   untracked, or the jail's scratch dir under `dev/accept.sh --compositor casement`, where
   nothing calls cargo: `build` only checks what `dev/accept.sh --build` built outside). What
   needs a person goes to `docs/manual-checks.md`.
5. `dev/accept.sh [--compositor casement|cosmic|kwin] [--jobs N] [NAME...]` runs scripts and
   prints one PASS, SKIP (and why) or FAIL line each; on casement each in its own jail.

---

## 7. Surfaces

| Surface | Crate | Layer | Notes |
| --- | --- | --- | --- |
| `bar` | `sill-bar` | Top | Menus (app, status items, tray) through one `MenuPopup`; menu tracking is `ds`'s |
| `control_center` | `sill-bar` | bar popup | `CcModule` grid; hung from the output's right edge |
| `dock` | `sill-dock` | Bottom | Magnify, machines, context menu, badges, running dots |
| `launcher`, `launcher_catcher` | `sill-launcher-ui` | Overlay | Panel plus click catcher, one `Reveal` link |
| `banners`, `notification_center`, `screenshot_thumb` | `sill-notify-ui` | Overlay | Cards from `sill-ui-kit::notification_card` |
| `desktop_widgets`, `edit_widgets` | `sill-widgets-ui` | Bottom, Overlay | Providers from `sill-ui-kit::widgets`; the desktop layer takes the pointer over the whole output and opens its context menu (a `MenuPopup` of `desktop::menu`) on a secondary press; each output's layer and sheet follow that output's own widget list |
| `osd`, `power_menu`, `polkit_prompt`, `confirm`, `accounts_sheet`, `switcher`, `idle_dim`, `hot_corners`, `wallpaper` | `sill-overlays` | Overlay (wallpaper Background) | Hot corners: one slot per corner. `confirm` is `confirm_sheet/`; `accounts_sheet` (`accounts-sheet`) is accountd's consent question and add-account steps, each a centred titled window on the banners' output (`sill_services::accounts::sheet` serves `org.quire.AccountsSheet1`); `confirm` is `confirm_sheet/`: intentd's question on quire's `Sheet`, mapped on the banners' output; `voice-consent` is `voice_consent/`, the first-use microphone question the same way |
| `lock` | `sill-overlays` | session lock | `LockSurface`; one per output through shell-host |

The notification center draws widgets and the desktop layer draws them too, so providers live
in `sill-ui-kit`, and the center's "Edit Widgets" and the desktop menu's ask through `links::edit_widgets`, which also carries the desktop's writes (`WidgetsChange`).

## 8. The daemon

`sill run` (`daemon/run.rs`) takes the single-instance lock, builds `Env` (`run_args.rs`),
starts services (`daemon/services.rs`) and settings (`daemon/settings/`), starts the IPC server
(`ipc/server.rs`), connects the host and reconciles one row of surfaces per output
(`outputs.rs`, `daemon/surfaces.rs`). `Links` is built once in `run.rs`. The `debug`
feature adds `sill debug ...` and the fake DDC flags; a build without it refuses them.

Settings have one path: the file settles in `daemon/settings/watch.rs`, is published on the
feed (surfaces read it) and followed by each `Policy` (services are told). Focus mode writes
the file and the file changes the mode; the file stays the one truth.

The person's stylesheet (`~/.config/sill/style.css`, quire's `UserStyle`, ARCHITECTURE.md
section 10 in quire) takes the same path: `daemon/settings/user_style.rs` watches it through
`ds_settings::Store`, publishes it on the feed, and `SurfaceRoot` passes it to `Ds`, so every
surface restyles live. Each surface root carries `data-surface = Surface::NAME`, which makes
the surface names part of the public selector surface: renaming one breaks the person's file
and needs a note. `sill style check` prints `ds_lint::user_stylesheet`'s notes for the file.
The person's rules are unlayered and win over both cascade layers: quire's `@layer ds`, and
`@layer app`, where every sill surface's own CSS goes (`ds::prelude::AppStyle { css }`, never a
bare `style { {CSS} }`). A user rule written `[data-surface=bar]` (plain, no `*|`) matches.

`dist/` holds what a real login needs: `sill-session`, `sill-session.target`, `sill.desktop`,
`sill.service`, `intents/org.quire.Shell.toml` (what `quire-do shell` lists and intentd gates), the COSMIC seed files (shortcuts, key repeat), the PAM service and the DDC/CI
rules. `dist/install.sh` (`--dry-run` prints) and `dist/uninstall.sh` put them in place and take
them out, sharing `dist/install-lib.sh`; they never touch Plasma or Toshy, and skip the DDC rules
and `sill.service`. `sill-session` seeds the COSMIC files into `~/.config/cosmic` when missing
(never overwriting), sets `COSMIC_RENDER_DEVICE` from `sill session detect-gpu` (the GPU the monitor
is on, when there are several; a person's own value wins), sets the fcitx5 environment when fcitx5
is installed and starts the wallet's Secret Service; `~/.config/sill/session.env` overrides its settings. `dev/session-smoke.sh`
runs the script itself nested. `docs/first-login.md` is the user's checklist.

The desktop's own compositor (`daemon/compositor.rs`, `daemon/targets.rs`): `ShellSeat` holds the
Quire arm when the compositor handed the shell a connection, and `ShellSeat::command` is its one
way to send a `DesktopCommand`. The docks publish their icons' resting rectangles into
`Services::dock_rects`; `targets::forward` turns that watch and the arm's `DesktopView` into host
messages (`ShellMsg::DockRects`, `ShellMsg::Desktop`); the seat keeps the compositor's minimise
targets equal to the icons by sending only the diff (`MinimiseTargets::since`), and sends them
all again when a shell surface registers. On the Wlr and Cosmic arms the seat is absent and
nothing is sent.

## 9. Test harness

> Test paths below written `tests/<name>.rs` mean `tests/it/<name>.rs`: a crate's integration tests are
> modules of one executable (`tests/it/main.rs`), and a separate target needs a stated reason
> (CONVENTIONS.md section 8).

| Kind of thing | How it is tested |
| --- | --- |
| Pure (`sill-model` steps, `sill-launcher`, `sill-settings` parse, `sill-ipc` words) | `CASES` tables beside the code; serde round trips against exact bytes |
| Service backend | `sill-services/tests/<x>_bus.rs`: `Env::isolated(tmp)` with `Buses` set to a `PrivateBus` address; peers are `sill-testkit` stand-ins. Runs in the gate. Only a test that needs a compositor is `#[ignore]`d and run from a dev script |
| Providers, runner | `VecSink` and a fake `Sources::Providers`; the runner's latency test |
| Surface | `SurfaceHarness<S: Surface>` (`sill-ui-kit`, feature `harness`) over quire's `ds_harness::Harness`: `SurfaceHarness::<S>::setup(link, slot)` then `.settings(ShellFile)`, `.viewport(..)`, `.appearance(..)`, `.start()`. It mounts `surface::mount::<S>` on fake `Services` (`fake::services()`; state set through `run.feeds`, commands read), a `PinnedAppearance`, `Clock::Virtual`, the surface's link (`run.link`) and the settings; the viewport is the surface's layer size unless overridden. `run.harness` is the `Harness` (`advance`, `html`, `pointer_move`, `count`). One live harness per thread. Real-input checks are dev scripts on shell-host's headless KWin |
| Machines | table tests of `step`; `use_machine` is tested once, in quire's `ds-conformance/tests/machine.rs` |
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
  ./scripts/check-intents.sh   # every .desktop file has a valid intents manifest (docket's check)
  cargo deny check licenses
  ```
