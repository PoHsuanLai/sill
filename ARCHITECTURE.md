# Architecture

sill is a desktop shell: the bar, dock, launcher, wallpaper, notifications, control
center, lock screen and the rest, drawn with quire's design system on shell-host (Blitz on
Wayland layer surfaces). casement is the compositor. KWin is a test compositor only.
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
| latchkey (git) | the single-instance agent socket: the daemon holds `Agent::new("sill").listen()`, the command line `connect_or_start`. The agent's directory is private and checked (`Error::Unsafe` for a link or another user's), a socket whose peer is another user's process is `Error::WrongOwner`, and `is_running` knocks, so the IPC server sees an empty connection now and then and answers it with nothing |

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
| 0 | `sill-shell-kit` | The shell-only parts over `ds`, moved here from quire's `ds-shell` (2026-10-07): bar, dock, control center, lock, switcher, OSD, idle dim, notifications, thumbnails, battery, clock, now playing, user picture, emoji, month grid, date picker, the widget contract and its two widgets (calendar, up next), with the shell's tokens (`tokens/`), sheets (`sheets.rs`), `KIT`, `kits()` and `stylesheet()` (the whole shell stylesheet: quire's plus these) and their SSR goldens and behaviour tests | none of ours; quire's `ds`, `ds-core`, `ds-shell` (its `KIT` and sheets). Dev: `ds-blitz`, `ds-harness`, `ds-lint`, `ds-settings` |
| 1 | `sill-model` | Pure vocabulary: ids, units, `Attempt`, every service's `State`/`Command`/`Event`/`Effect` and its `step`, vocabulary enums shared by a settings key and a service, and the shell's actions (`actions/`: `ActionDef`, `Effect`, `Ask`) | `sill-launcher`, `chordkit` (`StandardAction`), `keycap-protocol`, `ds-core` (`Stamp`, `Machine`), `docket-core`, `almanac-core`, `companion-wire`, `cua-bus`, `prov`, `porter-core` (the accounts sheet's `SheetView`/`SheetInput`, serde data), `porter-infer` (`ModelRef`, `ServedBy`: serde data a helper's model is named by), `casement-protocols` (`vocab` only: the desktop views' enums), `ds-helpers` (no features: `match_app` and `AppKeys`, the one matcher of a window to its desktop entry); serde, jiff, thiserror |
| 2 | `sill-settings` | `settings.toml` as data: one struct per domain, `ShellFile`, schema, typed values parsed at load. No I/O, no runtime, no dioxus | `sill-model`, `sill-launcher`, `keycap-protocol`, `ds-core`, `ds-settings` (no features) |
| 2 | `sill-ipc` | The wire: `Request`/`Response`, one JSON line, 64 KiB cap. Serialises `sill-model` types | `sill-model`, `sill-launcher`; serde, serde_json, thiserror |
| 3 | `sill-services` | Effects: `Env` seams, `Service` trait, runner, `Handle`, `Control`, one dir per service `{mod.rs, backend/}`. UI-free | `sill-model`, `sill-launcher`, `keycap-protocol`, `ds-core` (`Stamp`), `docket-core`, `docket-dbus`, `docket-client`, `companion-wire`, `cua-bus`, `cua-dbus`, `almanac-core`, `almanac-dbus`, `prov`, `porter-core`, `porter-infer` (`Slot` and `ModelRef`, to keep a helper's pool), `porter-dbus` (who owns a bus name: caller identity; accountd's `Manager` and `Account` proxies), `porter-client` (`dbus` and `infer` features only: the typed guest calls and change stream for the guest-asks service; the places and engines a helper's model is picked from), `voice-dbus`, `anyview-core`, `anyview-platform` (the viewer's D-Bus request, for the launch service), `casement-protocols` (`vocab`), `desktop-client` (`client` feature: the wire behind `desktop_wl` and the native displays route); tokio, zbus, rustix, libpulse, notify, nonstick, ddc, image, toml |
| 4 | `sill-search` | The launcher's runner (`ResultsService`), `providers/` (the files provider reads the viewer's history and takes the shared file actions), clip images | `sill-services`, `sill-model`, `sill-launcher`, `anyview-core`, `anyview-store`, `ds-core`, `docket-core`, `almanac-core`, `companion-wire`, `prov`, `ds-settings` (no features: the schema files the settings provider indexes); fend, emojis |
| 5 | `sill-ui-kit` | What every surface shares: `Services` registry, `SettingsFeed`, `SurfaceRoot`, `Surface` and its `mount`, `links/`, hooks, `look`, `panel`, `shell_style`, `sheet_stage` (the enter / shown / leaving-with-exit stage of a sheet), `MenuPopup<M>`, `app_icons`, `status`, `notification_card`, `widgets/`, `harness` (feature: fake services, `SurfaceHarness`) | `sill-shell-kit`, `sill-search`, `sill-services`, `sill-settings`, `sill-model`, `sill-launcher`, `companion-wire`, `docket-core`, `porter-core`, `prov`, `ds-core`, `ds`, `ds-settings` (dioxus), `ds-blitz`, `ds-harness` (optional: feature `harness`), `shell-host`; dioxus, resvg, image; dev: `ds-lint` |
| 6 | `sill-bar` | Bar, its menus, tray, control center and its modules | `sill-ui-kit`, `sill-model`, `sill-settings`, `sill-launcher`, `sill-services`*; `sill-shell-kit`, `ds`, `ds-settings`, `ds-blitz`, `shell-host`; dev: `ds-lint` |
| 6 | `sill-dock` | Dock, magnify, context menu | `sill-ui-kit`, `sill-model`, `sill-settings`, `sill-launcher`, `sill-services`*; `sill-shell-kit`, `ds`, `ds-blitz`, `shell-host`; dev: `ds-lint` |
| 6 | `sill-launcher-ui` | Launcher panel and catcher, rows, preview pane, actions | `sill-ui-kit`, `sill-model`, `sill-settings`, `sill-launcher`, `sill-search`*, `sill-services`*, `anyview-core`, `anyview-image`, `anyview-peek` (what a file's pane looks like), `anyview-platform` (the thumbnail cache behind a video's frame, `preview::frames`), `almanac-core`, `companion-wire`, `docket-core`, `prov` (a companion row's pane and keys); `ds`, `ds-blitz`, `shell-host`; dev: `ds-lint`, `ds-shell` |
| 6 | `sill-notify-ui` | Banners, notification center, screenshot thumbnail | `sill-ui-kit`, `sill-model`, `sill-settings`, `sill-services`*; `sill-shell-kit`, `ds`, `ds-settings`, `ds-blitz`, `shell-host` |
| 6 | `sill-overlays` | OSD, power menu, polkit prompt, switcher, idle dim, hot corners, wallpaper, lock | `sill-ui-kit`, `sill-model`, `sill-settings`, `sill-launcher`, `sill-services`*; `ds`, `ds-behaviour` (the switcher and hot-corner machines), `ds-shell` (the accounts sheet), `sill-shell-kit`, `ds-settings`, `shell-host`, `porter-core` (the accounts sheet's props) |
| 7 | `sill` | Binary and library: `actions/`, `cli/`, `daemon/`, `ipc/`, `policy/` | every crate above except `sill-testkit`; `ds-blitz`, `ds-settings`, `shell-host`, `shell-verbs` (the verbs casement's keymap sends and the command line each spells); clap, latchkey, `ds-lint` and `sill-shell-kit` (`sill style check` lints against its `kits()`); dev: `sill-testkit`, `sill-ui-kit` (harness), `ds` |

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
| `sill-services` | `error`, `control`, `stream`, `env/`, `service`, then `debounced` (a watch that reports a burst of events as one settled touch), then one dir per service (29, all but `results`) in dependency order of their `Deps`; a service dir never imports another service dir: cross-service data is `sill-model` types in `Deps` |
| `sill-search` | `preview` (a clipboard image), `providers/<kind>/` (`files/`: `actions` (the shared file actions as a row's), `viewed` (the viewer's history as rows and uses)), `results/` (`registry`, `run`, `sink`, `history`, `frecency_file`, `effect`), `ResultsService` in `results/mod.rs` |
| `sill-shell-kit` | `tokens`, then leaf parts `battery`, `clock`, `emoji`, `month_grid`, `now_playing`, `notifications`, `osd`, `idle_dim`, `dock`, `bar`, `control_center`, `switcher`, `date_picker` < `user_picture`, `thumbs` < `lock` < `widget` (`contract`, `card`, `frame`, `timeline`, `calendar`) < `sheets`, `kit` < `prelude` |
| `sill-ui-kit` | `look`, `links/`, `settings`, `shell_style`, `sheet_stage/`, `services`, `hooks/`, `panel`, `root`, `surface/` (`mount`, `lock`, `start`, `place`, `naming`, `outputs`, `lifecycle`, `preset`), `menu/`, `status/`, `app_icons/`, `notification_card/`, `widgets/`, `harness/` (feature `harness`) |
| each surface crate | one dir per surface, named after it (`osd/`, `dock/`, `banners/`); `lib.rs` re-exports each surface marker type and nothing else |
| `sill` | `policy/`, `ipc/`, `cli/`, `daemon/` (`daemon` is the top: it names every crate) |

### The one shape of a surface (every dir in a surface crate)

| File | Holds |
| --- | --- |
| `mod.rs` | declarations only |
| `surface.rs` | the marker type named after the surface and its `impl Surface`: the declarations (outputs, keyboard, role, drawing order, presence, link) and the thin `layer` and `root` |
| `layer.rs` | only when the surface's layer is its own: `fn layer(&OutputInfo, &ShellFile, slot) -> LayerConfig` and its constants. A sheet, a strip, a corner column or a whole-output surface names a `LayerPreset` in `surface.rs` instead |
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
| `apps/{exec,index}.rs` | `sill-model/src/apps/` |
| `apps/{scan,icons}.rs` | `apps/backend/` (`scan.rs` the scan over `ds_helpers::load_entries`, `entry.rs` one loaded entry as an `App`, `locales.rs` the session's locales) |
| `auth/{types,gate,protocol}.rs` | `sill-model/src/auth/` |
| `auth/{account,helper,pam,live}.rs` | `auth/backend/` (`pam.rs` is public: `sill auth-check` calls it) |
| `auth/authenticator.rs` | `env/authenticator.rs` (the `Authenticator` seam and `FakeAuthenticator`) |
| `auth` account lookup | `env/passwd.rs` (the `Passwd` seam: `SystemPasswd` over `nix`, `FakePasswd`) |
| The polkit agent's user names (which polkit identity, named) | `polkit_agent/backend/names.rs` (the `Names` seam: `SystemNames` over `nix`'s `User::from_uid`, so directory-service accounts resolve; a fixed fake in its test; `user_for` is pure over the seam) |
| `bluetooth/{model,kind}.rs` | `sill-model/src/bluetooth/` |
| `bluetooth/{bluez,live}.rs` | `bluetooth/backend/{bluez,run,devices,pairing}.rs` (`live.rs` is 569 lines) |
| `network/{model}.rs`, `network/wifi/model.rs` | `sill-model/src/network/`, `sill-model/src/wifi/` |
| `network/live.rs`, `network/wifi/live.rs` | `network/backend/`, `wifi/backend/`; Wi-Fi is its own dir |
| `clipboard/{model,reduce,content,offer}.rs` | `sill-model/src/clipboard/` |
| `clipboard/{pipes,wire,wire_state,wire_dispatch,wl,live}.rs` | `clipboard/backend/` |
| `windows/{ids,caps,command,toplevels,workspaces}.rs` | `sill-model/src/windows/` |
| `desktop/{backend,command,model,note}.rs` | `sill-model/src/desktop/` (the arm names and the globals offer, the desktop's views and commands, `supports`, and `ShellNote`, the compositor's shell protocol in the shell's words) |
| `desktop_wl/{choose,ids,toplevels,workspaces,view,command,outputs,offer,run}.rs` | `sill-services/src/desktop_wl/` (the standard-protocol arm over `desktop-client`; its thread is `thread.rs`, which picks the arm with `desktop_client::backend_for` on the offered globals: Quire, else Wlr). On a Quire offer the same thread serves the toplevels and workspaces on the environment connection |
| `desktop_wl/quire/{handout,surfaces,view,command,notes,arm,queue_wake,shell}.rs` | the Quire arm (P2-SQ): `handout` is the seam for `org.quire.Compositor1.Connect("shell")` (`Handout`, `ZbusHandout`, `shell_connection`); `surfaces` records which registered surface has which `ShellRole` (a role is the surface's own `Surface::ROLE`); `view` and `command` are the pure mappings (`quire_view`, `quire_wire_of`) between `DesktopState::quire` and the shell's `DesktopView` and `DesktopCommand`; `notes` is `note_of` (the shell protocol's events as `ShellNote`s); `arm` is `QuireArm<D: Desktop, S: NoteSink>` (sync: drain events, hand notes to the sink, return the view; execute: a command as requests; assign: `SetShellRole`), tested over `FakeDesktop::with_config`; `shell` runs the arm on the shell host's connection (`spawn_shell_arm(&wl_surface)` takes the connection from the surface, binds a second queue with `DesktopConfig::holding_shell()`, publishes a `watch` of `DesktopView` and an mpsc of notes); `queue_wake` is the `Waker` over a calloop ping that `DesktopClient::poll_dispatch_pending` leaves in the client's queue, so an event another thread's read queued wakes the arm's loop (no timer polls it). The daemon side is `sill/src/daemon/compositor.rs`: the host's route (`Handed(fd)` when the compositor handed one), the `ShellSeat` that starts the arm at the first mapped surface with a role and registers each mapped surface, and `apply`, which carries each `ShellNote` out (hot corner, the companion's tap, the kill chord) or names the P3 lane that will (`P3-SW`, `P3-MC`, `P3-MN`, `P3-SILL`) |
| `displays/{model,policy,identity,scale,ppi,plan,brightness,hw_volume,gamut,warmth}*`, `displays/edid/`, `displays/ddc/{vcp,matching,retry,debounce}.rs` | `sill-model/src/displays/` (`edid/`, `ddc/`) |
| `displays/{backlight,drm,randr,output,outputs_route,font,fontconfig_file,run}.rs`, `displays/ddc/{bus,i2c,worker}.rs` | `displays/backend/` |
| `displays/ddc/file_bus.rs`, `displays/ddc/fake.rs` | `env/ddc.rs` (`DdcBus`, `I2cBuses`, `FileBus`, `FakeBus`); `displays/kscreen*` is already gone |
| `events/{model,agenda,occur,rule,rule_expand,ics,ics_line,ics_value}.rs` | `sill-model/src/events/` |
| `events/{read,live,live_watch,sources,provider}.rs` | `events/backend/` |
| `idle/{model,policy,grace,reduce,reduce_rules}.rs` | `sill-model/src/idle/` (`reduce*` becomes `step.rs`) |
| `idle/{inhibit,resolve,screensaver,logind,wl,wl_state,wl_dispatch,live}.rs` | `idle/backend/` |
| `launch/{model,plan}.rs` | `sill-model/src/launch/` |
| `launch/{dbus,open,run,terminal}.rs`, `launch/test_support.rs` | `launch/backend/`; `test_support.rs` goes to `sill-testkit::TempDir` |
| `locker/{cause,machine,mode}.rs` | `sill-model/src/locker/` (`machine.rs` becomes `step.rs`) |
| `locker/{marker,logind,live}.rs` | `locker/backend/` |
| `notifications/{state,reduce,rules,content,markup,markup_parse,ids,capabilities,policy,limits,origin,rate}.rs` | `sill-model/src/notifications/` |
| `notifications/{dbus,guard,wire,picture,senders,live,history_file,name_watch}.rs` | `notifications/backend/` |
| `pins/model.rs` | `sill-model/src/pins/` |
| `pins/{fs_watch,run}.rs`, `pins/test_support.rs` | `pins/backend/run.rs`, and `fs_watch.rs` becomes the shared `debounced.rs` (`keycap` watches `keycap.toml` with it); `test_support.rs` goes to `sill-testkit` |
| `polkit_agent/{types,machine,identity,helper}.rs` | `sill-model/src/polkit_agent/` |
| `polkit_agent/{bus,dbus,run,live}.rs` | `polkit_agent/backend/` |
| `screenshots/{reduce,names}.rs`; `tool.rs` (parse only) | `sill-model/src/screenshots/`; the tool string is parsed at load by `sill-settings` into a typed `ShotTool` |
| `screenshots/{child,folder_watch,live,preview}.rs` | `screenshots/backend/` |
| `wallpaper/{cache,geometry}.rs` | `sill-model/src/wallpaper/` |
| `wallpaper/{config,resize,watch,backdrop}.rs` (`config` also reads the old `wallpaper-source` file while the setting is empty), logic in `mod.rs` | `wallpaper/backend/` |
| `tray/{model,icon}.rs`; `tray/host.rs` (498) | `sill-model/src/tray/`; `tray/backend/{host,items,menu}.rs` |
| `now_playing/{model,art}.rs` (names); `mpris.rs`, `live.rs` (498) | `sill-model/src/now_playing/`; `now_playing/backend/{mpris,run}.rs`, `now_playing/backend/art/{mod,fetcher,curl,cache}.rs` |
| `battery`, `kbd_backlight`, `power_profile`, `volume`, `session`, `badges`, `clock`: `model.rs`, `format.rs` | `sill-model/src/<x>/`; `live.rs`, `upower.rs`, `ppd.rs` -> `<x>/backend/` |
| `providers/`, `results/{registry,run,sink,history,frecency_file,effect}.rs`, `preview.rs` | `sill-search/src/providers/`, `results/`, `preview.rs` |
| `results/{model,arrange,policy}.rs` | `sill-model/src/results/` |
| `services.rs` (`Services`, `Member`) | `sill-ui-kit/src/services.rs`; `Services::spawn` becomes `sill/src/daemon/services.rs::start` |
| `magnify/` | `sill-dock/src/magnify/` |
| `switcher.rs` | `sill-overlays/src/switcher/apps.rs` |
| `curve.rs` (unused) | deleted; easing is `ds-motion`'s |
| `programs.rs` | deleted; nothing read the process scan |
| `env/scratch.rs`, `examples/*` (13 stand-ins), `tests/support/*` | `sill-testkit` (`TempDir`, `PrivateBus`, `signals`; stand-ins in its `examples/`) |
| `tests/*_live.rs` | `crates/sill-services/tests/<service>_bus.rs` on `PrivateBus`; `#[ignore]` only when a compositor is needed |
| `error.rs`, `control.rs`, `stream.rs`, `service.rs`, `env/` | stay |

### `sill-settings`, `sill-ipc`, `sill-launcher`

| From | To |
| --- | --- |
| `sill-settings/src/{bar,calendar,control_center,display,dock,hot_corners,idle,keyboard,launcher,menus,notifications,osd,power_menu,screenshot,scroll,scrollbar,session,spaces,switcher,widgets,window}.rs` | stay; keys with no reader deleted; `flatten extra` and `version` deleted |
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
| `osd/`, `power_menu/`, `polkit_prompt/`, `confirm_sheet/`, `voice_consent/`, `accounts_sheet/`, `capture_sheet/`, `permission_sheet/`, `switcher/`, `idle_dim/`, `hot_corners/`, `wallpaper/`, `window_menu/`, `spaces_bar/`, `split_picker/`, `lock/` | `sill-overlays/src/<same>/` |
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
| Everything ambient (buses, Wayland route, directories, wall clock, DDC, session actions, authenticator, the accounts database (`passwd`), idle times, polkit route, children, keycap's socket path, the means to tell a bus caller apart) | `sill_services::env::Env` (`keycap_socket`: `/run/keycap/keycap.sock` from the process, none when isolated, so no test or script reaches the real daemon) |
| Starting a child nobody waits on: the file opener, the trash, a default handler | `sill_services::env::Children` (`Env::children`): `RealChildren` in the daemon, `RecordedChildren` in `Env::isolated`, which keeps each `Start` and runs nothing, so a test asserts the argv and never opens a file in the person's session. `tests/spawn_sites.rs` lists the only files that may name `Command::new` |
| Waiting on a peer (an app, a system daemon, the compositor, the companion) | `sill_services::deadline`: named limits (`BUS_CALL`, `LAUNCH_COMMAND`, `NETWORK_REFRESH`, `TRAY_CALL`, `COMPANION_CALL`) and `within` / `call`; a launch's activation token has `sill_ui_kit::surface::activation_token` (300 ms, then the launch goes ahead without it) |
| Keeping a service alive: a bus that is not up yet, the growing wait between attempts, a panic at the service boundary | `sill_services::supervise` (`Backoff`, `panic_text`; a bus is reached through `Bus::connect_waiting`, or `Bus::try_connect` for once and no wait) and `service::spawn`, which logs how a service's task ended. Volume reconnects to the sound server itself; Wi-Fi follows NetworkManager's bus name and its device through `owner` |
| Following a daemon that starts after sill, leaves or restarts (a well-known bus name) | `sill_services::owner::Owner::of(connection, NAME)`: `while owner.session(session(..)).await == Next::Again { gone }` runs one session per owner and says what leaving means between them, until a session says `Ended::Commands` (a plain future, not a closure over the state, so the service future stays `Send`); `owner::while_owned` for a follower with no state of its own. A service never types the `appeared` / `changed` loop |
| Publishing a state a service steps | `sill_services::machine`: `Published<T, E>` (the step returns only the next value: `feed(event)` steps the value readers hold) and `Machine<S, E, F>` (the step returns effects: `feed(event)` returns them to carry out). Both publish only a change, so readers wake on a difference and never on an equal value |
| Claiming a well-known name on a bus and serving objects on it | `sill_services::serve::claim(bus, NAME, objects)` (connect, export the objects, then ask for the name without queueing) with `sill_model::serve::{ClaimError, ServeLink}`; `Env::claims` (`NameClaims::{Claim, Passive}`) says whether a service claims at start, and `Env::isolated` is `Passive`, so a test's `start` takes no name. The outputs, workspaces, accounts sheet and intents seat take no `Serve` command |
| Telling the person through a notification, and answering its buttons | `sill_model::notifications::{Notices<K>, shown_with_action}` (which id carries which key) and `sill_services::notices::Board<K>` (post, replace, withdraw, learn, answer; the server's `ActionInvoked` and `NotificationClosed` are heard once in `notices::said`). A service keeps its words (`notice_text`) and how an action key names its notice |
| A service's own Wayland connection on a thread | `sill_services::wayland_thread`: implement `Follow` for the thread's state (bind, order, gone) and `start::<State>`; the epoch, the events, the orders and the calloop loop are there |
| Writing a state file | `sill_services::durable` (`write`, `write_file`, `set_aside`): unique temp name, fsync file and folder, rename, private mode or `AsFound`, written through a link; a file that cannot be read is moved to `.damaged` before the next save. Switch it to quire's public durable writer once pa-qfiles lands (FINDINGS "pa-silllive") |
| Quitting the shell while the session is locked | `ipc::dispatch::quit` refuses `Quit` (`FailureKind::Refused`); `daemon::run` exits with failure when the host ends while locked, and `dist/sill.service` restarts on failure with no start limit |
| The shell's watchdog: `READY=1` on the host's start, `WATCHDOG=1` every 15 s only while `Host::pulse`'s beats moved (`dist/sill.service` is `Type=notify`, `WatchdogSec=30`) | `sill::watchdog` (`NOTIFY_SOCKET` read once; unset means no-ops) |
| Telling the update service the release is good | `sill::release`: `slipway confirm` through `Env::children`, 30 s after the host answers a ping, only on the usual session bus and never when the session fell back to the safe compositor (`Env::session_mode`, from `QUIRE_SESSION_MODE=safe`; unset or any other value is normal); the program is `slipway` beside this one in the release's `bin` (`Env::release_bin`), else `/usr/local/bin/slipway`, and a missing one is logged as an error |
| The focused window's keyboard profile | `sill_model::keyboard::wanted` over `ProfileSource`s (the user's overrides, what the app declares, keycap's own per-app profiles such as `ghostty`, then the category table); sent by `sill_services::keycap` (`focus_to_send` decides when) |
| The person's per-app keyboard profiles (`keyboard.overrides`) | `sill_settings::keyboard` (`KeyboardSettings`; `overrides` parses the rows once, per row, with a line for each one dropped), told to keycap by `sill::policy::keyboard` as `KeycapCommand::Overrides`; `KeycapAsk` is the part of the command on the IPC wire |
| The confirmation sheet's queue, its answers and receipts | `sill_model::intents::sheet` (`Sheets`, `SheetChoice` with "Never" only where the request's limits offer it, `step`; the receipt says `SheetFallback`, never `HardwareSeat`), served as `org.quire.Confirm1` by `sill_services::intents` |
| What the sheet says (title, byline, facts, reasons, the limit that made it ask) | `sill_model::intents::sheet::sheet_view`: fixed words and the router's own formatting, never a model's text; a limit's line and its Undo are `SheetHeld`, and Undo is `IntentsCommand::UndoLimit` (intentd's `Control`, removing the entry by its id; the sheet loses the line once intentd has done it) |
| Hold-to-talk: the hold's phases, the words heard, the refusal told, the consent sheet's state | `sill_model::voice` (`Voice`, `step`); the bus half is `sill_services::voice` (`org.quire.Voice1` as the shell on intents' seat; the stream is `voice_wire::unframe`) |
| What the person is told while holding or after a refused hold | `sill_model::voice::{listening_line, notice_line}`; the launcher's prompt shows the line as its placeholder |
| `voice.hold_to_talk` and `voice.consent` (the forms `voiced` reads) | `sill_settings::voice` (`VoiceSettings`; `VoiceConsent`'s serde is the contract); written by `sill::voice::write_consent` from the consent sheet's answer |
| A sheet that plays its exit before its surface unmaps (the capture, confirm, Force Quit, polkit and voice consent sheets) | `sill_ui_kit::sheet_stage` (`SheetStage<T>`: pure `advance` in `step.rs`, `use_sheet_stage` follows a wanted value and maps the surface, `settle` is the sheet's `on_hidden`); one that leaves with an answer handed over once the exit settles (the power menu): `SheetExit<A>` |
| The consent sheet | `sill_overlays::voice_consent` (quire's `Sheet`, mapped on the banners' output while the service says `Asking`) |
| Arming (a still sheet for `ARM_MS`: a key or a pointer move starts the wait again), Escape and the hold of the confirmation sheet | `sill_overlays::confirm_sheet::gate` (a `Machine` on quire's clock) |
| The terminal's standing grants, the breaker, Resume | `sill_model::intents::terminal`; read and changed through intentd's `Control` by `sill_services::intents`; drawn by the control center's `Terminal` module |
| The shell's own actions as `org.quire.Shell` | declared in `dist/intents/org.quire.Shell.toml` (installed by `dist/install.sh`), read by `sill_model::intents::shell`, served by `sill_services::intents` (the provider), done by `sill::intents` (the same requests `sill <verb>` makes) |
| The shell's window and workspace actions (`shell.window.*`, `shell.app.*`, `shell.workspace.list`, `shell.workspace.switch` to a workspace) | read by `sill_model::intents::shell::window_ask_of` into a `WindowAsk`, decided against the compositor's state by `plan` (the Hide, Minimise and raise commands are `app_menu::own_commands`, the one implementation the bar's app-name menu uses too), the listing text by `crates/sill-model/src/intents/shell/listing.rs`, sent to the compositor service by `sill_services::intents` (`crates/sill-services/src/intents/backend/windows.rs`); `shell.screenshot` answers with the path the folder watch saw (`crates/sill-services/src/intents/backend/shots.rs`) |
| Desktop entries: the folders searched, the walk, the desktop id (`kde/foo.desktop` is `kde-foo`), the shadowing of an id, and which entry a window belongs to | `ds_helpers` (quire `desktop_entries/`): `XdgEnv::applications_dirs`, `load_entries`, `match_app` (five rules; `sill_model::apps::AppIndex::matching` only builds the `AppKeys`). sill keeps no copy of any of them; `sill_services::apps::scan` turns each loaded entry into an `App` |
| The XDG data folders (`XDG_DATA_HOME`, `XDG_DATA_DIRS`, the `applications` folders) | `ds_helpers::XdgEnv`, read once by `Dirs::from_process` in `Env` (`Dirs::{data, data_dirs, applications}`); nothing else reads those variables, the settings provider takes `Dirs::data_dirs` |
| The names one app is known by (desktop id, window id, bus names, intents app) | `ds_helpers::AppIdentity`, read from the entry's `X-Quire-` keys; sill carries `App::{bus_name, intents_app}` and asks `sill_model::windows::intents_app_for` (the activation bus name and the companion's app name) instead of guessing one from another |
| What an app's notification may hold, whose it is, who may replace it | `sill_model::notifications::{limits, origin, rate}` (the ceilings, `Origin::{System, App}` from the `sill-` desktop-entry namespace, the per-sender `Allowance`), the banner limits and the critical sweep in `rules.rs`; `sill_services::notifications::backend::{guard, wire, picture, senders}` refuse a call longer than `NOTIFY_CALL_MAX_BYTES` unread (`guard`), cut the rest at the boundary and keep each posting's connection |
| Which pictures a notification's sender may name (`app_icon`, `image-path`) | `sill_model::notifications::ImageDirs` and `Posting::within`: a file is kept only inside the icon and pixmap folders or sill's notification cache (`Dirs::notification_image_dirs`), written without `.` or `..`; a name carries no folder; anything else is no picture. `sill_services::notifications` applies it to every bus call (not to the shell's own `Post`); `image-data` is pixels and is not path-checked. The host's file roots are the second check, on the real file after symlinks |
| App text made safe to hold, show or log (cut, control and bidirectional marks dropped, journal escape) | `sill_model::untrusted::{clip_bytes, cut_bytes, clip_chars, for_display, for_log}` |
| Limits on a tray item's menu and icon, on a layout, on a cover fetch, on an icon file | `sill_model::tray::{TRAY_ITEMS_MAX, MENU_ROWS_MAX, MENU_DEPTH_MAX, TRAY_ICON_NAME_MAX_BYTES, TRAY_ICON_FILE_MAX_BYTES, ..}` (read by `sill_services::tray::backend::menu` and `sill_model::app_menu::rows_of`); a tray icon name is a theme name (`sill_model::tray::theme_icon_name`: no slash, no `..`, bounded), checked where it enters and again before the lookup, and its file is read with `ds_helpers::read_capped`; `sill_services::now_playing::backend::art::ArtFetcher` (its pure rules are `sill_model::now_playing::{art_source, evictions, next_start}`); `sill_model::apps::{icon_file_fits, ICON_FILE_MAX_BYTES}` and `sill_services::apps::IconRoots` (an absolute `Icon=` must lie in `Dirs::icon_file_roots`, links resolved) (read by the apps scan and by `sill_ui_kit::app_icons::render::{ICON_MAX_BYTES, ICON_MAX_SIDE}`) and the cache's `prune` |
| Notices about keycap (failed, conflict, paused) | `sill_model::keycap::{attention, notice_for, notice_text}`; posted by `sill_services::keycap::backend::notice` (no action: `sill keycap retry` and `resume` are the way out) |
| Which accounts need signing in again, the notice that says so and its "Sign in again" button | `sill_model::accounts::{step, notice_text, sign_in_key, account_of, shown_for}` (the button's action key carries the account); `sill_services::accounts` follows accountd, posts and withdraws the notice, and answers the button's `ActionInvoked` (only from the notifications server's own connection) with the account's `Reauthenticate` |
| Each output's work area (the output less the bar's and dock's exclusive zones) and scale, for apps | `sill_model::work_areas::{work_area, work_rect, Zone}` (pure reduction); `sill::work_areas` derives the zones from the bar's and dock's own layer configurations and publishes; `sill_services::work_areas` serves `org.quire.Outputs1` (property `WorkAreas`, `a(us(iiuu)(iiuu)u)`: id, name, frame, work, scale in 120ths; read-only; `PropertiesChanged` on change); quire's `ds-desktop::Outputs` reads it |
| Each workspace's id, name, output, `SpaceLook` and activity, for apps that keep their Spaces from the desktop (`SpacesSource::Desktop`) | `sill_model::workspace_looks::published` (pure: the compositor's workspaces with a look per position); `sill::workspace_looks` gives each the look every surface draws (`sill_ui_kit::look`) as quire's JSON and publishes; `sill_services::workspace_looks` serves `org.quire.Workspaces1` (property `Workspaces`, `a(ssssb)`: id, name, output, look JSON, active; read-only; `PropertiesChanged` on change) |
| Where a minimised window flies to (the dock icon of its app, resting size, not magnified) | `sill_model::desktop::target::{OutputRects, MinimiseTargets}` (pure: `of` maps icons and windows to targets, `since` is the diff into `SetMinimiseTarget`/`ClearMinimiseTarget`); `sill_dock::dock::rest_rects::app_rects` computes the icons' rectangles; `sill_ui_kit::dock_rects::DockRects` carries them; `sill::daemon::compositor::ShellSeat::retarget` sends the diff, on the Quire arm only |
| The accounts sheet (accountd's consent question and add-account steps): what is open, what is typed, what is sent back | `sill_model::accounts::sheet::{AccountsSheets, step, SheetAction}` (the draft holds a typed secret until the reply that carries it); `sill_services::accounts::sheet` serves `org.quire.AccountsSheet1` (only accountd, inferd or syncd, each as the owner of its own name, may open, update or close, by `sill_services::callers`; `Input` goes to the opener's unique name alone); `sill_overlays::accounts_sheet` draws it (`props.rs` maps `SheetView` to quire's `ds-shell::accounts` props) |
| Where the double tap and a summon go: the two machines, the Ask, the answers, the runs, the journal, the lookups | `sill_model::companion::{Tap, Summon, step}` (quire's `Machine`, stepped by `sill_services::companion`, which publishes `Companion`); the daemon's half (what is focused, the launcher's open, switch and close, the four `sill companion` verbs) is `sill::companion` |
| The companion's presence (the orb and the glow) | `sill_model::companion::presence_of`, the only derivation (`ds_core::vocab::CompanionPresence`) |
| What a card press is doing (which action a button calls, when its press ended, which kinds of thing open in their apps, which handles still wait for their text) | `sill_model::companion::{card_call, cards_of, promote_card, openers_of, unresolved_handles}` and `Busy`; the service carries out `Answer.Act`, `Session.Display` and the registry read (`sill_services::companion::backend`) |
| What the settings say about the double tap and the summon | `sill_settings::companion_params` (`companion.*`, design/22 section 3.30) as `CompanionParams`, sent by `sill::policy::companion::CompanionBridge`; every duration is `sill_model::units::Ms` |
| A run's state and the controls it offers; which runs a stop-all pauses | `sill_model::companion::{run_state_of, run_controls, going_runs}` (cuad's phase and outcome in, `RunState` out) |
| What a companion row says (an answer, a journal row, a memory row) | `sill_model::companion::{answer_line, activity_line, memory_line}`: fixed words and the person's own, never a model's formatting |
| Which assistant answered, when the person chose a coding agent as the assistant's model (inferd's `agent/<program>`, docket's `agents.toml`) | nothing of sill's: the card's footer is `companion_wire::footer_line` over the answer's `Routing` notes (`sill_ui_kit::companion`), and companiond puts the agent's note there ("Answered by Claude Code (you chose it)"); a turned-off or unlisted agent ends the turn as `RefusalWire::NoModel`, which opens the model picker (`sill_model::companion::model_picker`), where Settings (detent) shows why the row can't be chosen; a sign-in or a failure is a `RefusalWire::Failed` sentence the card shows as it is. sill reads no agent file and names no agent |
| The companion's wire as quire's view types (an answer card, its footer, a plan, a form, a run's state) | `sill_ui_kit::companion`, one total function per pair, each with a `*_total` test |
| The screen-sharing picker (transom asks what an app may share; the person picks) | `sill_model::capture_pick::{Ask, Candidates, plan, Picks, toggle, reply, step, windows_of}` (pure: kinds, several-or-one and pointer modes decide the choices; a pick becomes the `(sources, cursor)` reply); `sill_services::capture_pick` exports `org.quire.Shell1` on the seat's connection (no name of its own; only the portal backend, as the owner of `org.freedesktop.impl.portal.desktop.quire`, may ask, by `sill_services::callers`; a second sharing or colour call while a sheet is open is refused; `PickColor` is served beside it, answered with an empty array while no eyedropper surface exists; `AskPermission(app_label, title, subtitle, body, grant_label, deny_label) -> u32` is the same service's third question, checked like the others and queued with the compositor's questions (below), its words cleaned and cut by `PermissionAsk::from_wire` and its answer 0 yes, 1 no, 2 closed); the daemon feeds windows from the compositor arm's `DesktopView` (`ShellMsg::Desktop`) and screens come from the displays service; `sill_overlays::capture_sheet` draws it on quire's `Sheet`, `SegmentedControl`, `List` and `Button`, and `sill_overlays::permission_sheet` draws the permission question on quire's `Alert` (the app's icon found by name in the app index, the yes button first so it is the default, the no button the one Escape presses) |
| The colour picker's eyedropper (an app asks for one colour from the screen; the person points at it) | `sill_model::capture_pick::{eyedrop_step, Frozen, pixel_under, colour_at, pick_at, loupe_at}` (pure: which screens to take a picture of, the pictures held per question, the pixel under the pointer at any scale, the magnifier's place); `sill_model::desktop::{DesktopCommand::CaptureOutput, ShellNote::OutputPicture, OutputShot}` carry a whole-output picture (`QuireArm::pending` holds it beside a Space picture; `desktop_wl::quire::pictures::shot_of` makes the pixels and the PNG); `sill_ui_kit::links::EyedropperLink` holds the pictures for the surfaces and counts them; `sill::eyedropper` is the daemon's half (asks for the pictures, takes them in, tells the capture service an eyedropper exists); `sill_overlays::eyedropper::ColorSampler` draws it |
| Who may call the shell's bus objects (`org.quire.Shell1`, `org.quire.AccountsSheet1`, `org.quire.Confirm1`) and whose menu the bar believes | `sill_services::callers`: `Claim` and `admit` (the caller holds the role of the service whose well-known name it owns, over peephole's `BusPeers`; roles `portal`, `accounts`/`inference`/`sync`, `router`, proved `Full`), `CallerSource` (`Env::callers`: `DesktopCallers` over `/etc/quire/desktop-callers.toml` in the daemon, an empty table in `Env::isolated`, a stand-in in a test that keeps the peers and introduces connections by unique name), `menu_shown`; the pure comparison of a menu's owner with the focused window's program is `sill_model::app_menu::{menu_trust, MenuOwner, MenuClient}` (QUIRE-12; `MenuClient::reported` is the compositor's subject for the window's program, from `WindowModel::subjects`, projected by `desktop_wl::project_subjects` from `quire_desktop_app_v1.subject`) |
| The display trial (a program other than the shell changed the displays and the compositor asks whether to keep them; `quire_shell_v1` version 6): the wire's words as the shell's types, the question's words and the countdown arithmetic, the link, the sheet, the answer | `sill_model::desktop::{ShellNote::DisplaysTrial, ShellNote::DisplaysTrialEnd, DesktopCommand::AnswerTrial, DisplayTrial, TrialId, TrialAnswer}`; every word and the countdown are `sill_model::desktop::{TRIAL_TITLE, REVERT_LABEL, KEEP_LABEL, countdown_line, seconds_left, until_next_second}`; `sill_ui_kit::links::DisplayTrialLink` (the open trial with a serial that counts every send, so a restart with the same number is told from the first; the person's answer; the banners' link, which names the output that shows it) is written by `sill::display_trial::apply` (from `daemon::compositor::apply`) and followed by `sill::outgoing`, which sends `AnswerTrial`; the sheet is `sill_overlays::display_trial` (quire's `Alert`; Revert first, so it is the default and Escape; Keep Changes second; the line under the headline is "Reverting in N seconds", changed on each second by quire's timer; the card is keyed by the serial, so a restart starts the sheet and its arming over; an answer or the compositor's end takes it down; nothing in sill reverts, the compositor does when the time runs out). The wait before a question takes an answer is `sill_overlays::arming::QUESTION_ARM_AFTER` (600 ms), the permission sheet's too |
| The compositor's permission question (a program wants the screen, the keys, the clipboard, ...): what it says, the line it waits in, the answer | `sill_model::desktop::{Consent, ShellNote::PermissionAsk, ShellNote::PermissionWithdraw, DesktopCommand::AnswerPermission}` (the wire's words as the shell's types); every word of the sheet is `sill_model::capture_pick::consent_words` (headline per power, the Allow Once line, the Settings line, the buttons; a power's own sentence is `Power::blurb`); `consent_ask` makes the sheet's `PermissionAsk` (a program is named by its desktop entry only when the compositor vouches for it); the one line with `AskPermission` is `CapturePicker::{permission, waiting}` and `step` (one sheet at a time, `WAITING_MAX` waiting, a repeat is the same question, a withdrawal takes it down unanswered, the portal leaving takes down only the portal's); `sill_services::capture_pick` is told with `CaptureCommand::{Consent, ConsentWithdrawn}` by `sill::daemon::compositor::apply` and answers through `CaptureLinks::answers`, which `sill::permission_answers` carries to the arm; the sheet is `sill_overlays::permission_sheet` (Escape answers `escape_answer`: no button for the portal, a dismissal for the compositor). Every question of the sheet, the portal's and the compositor's, takes no answer for `permission_sheet::PERMISSION_ARM_AFTER` (600 ms) after it is drawn: quire's `Alert` `arm_after` disables the buttons and ignores Return, Escape and clicks, so a key or click already in flight cannot allow a power. casement may refuse an answer that comes sooner than 500 ms after its question; it does not yet. The confirmation sheet keeps its own gate (`ARM_MS`, above) |
| The shell's names on the session bus taken without queueing, and what happens when one is taken | `org.quire.Shell` and `org.quire.Confirm1` by `sill_services::intents` (`ClaimError::NameTaken`, from `serve::claim`), `org.quire.AccountsSheet1` by `sill_services::accounts::sheet` (all three claim at start unless `Env::claims` is `Passive`); `sill::name_guard` ends the host and makes the daemon exit with a failure and one line when the seat's names are taken (the unit restarts it a second later) |
| The single-instance lock file watched, and what a daemon does when it is displaced | `sill::door_guard` asks `Door::standing()` (latchkey's `Listening::standing`) every `STANDING_EVERY` (a minute, latchkey's advice) on the daemon's async runtime, no thread of its own; a lock file that is gone or another file ends the host with a failure and one line, as `name_guard` does (the unit restarts the shell, which takes the lock file that is there now). `sill::ipc::door::Door` is the shared handle: a door dropped while displaced is never dropped for real, so its socket unlink cannot remove a rival daemon's socket. The accept loop ends on `ipc::server::Serving::stop` (a flag read after each accept, and a knock while the lock file is still ours) |
| The connection everything said to intentd and the companion goes out on | `sill_services::seat` (the intents service publishes the connection that owns `org.quire.Shell`; intentd names a caller's role from the names its connection owns, so a turn or a search from any other connection is not the launcher's) |
| The launcher in companion mode: its keys, the pane that follows the selected row, what an ask keeps | `sill_launcher_ui::launcher::companion` (`companion_step`, `pane_of`, `ask_input`, `command_of`); the mode itself is `Opening::mode` (`sill_ui_kit::links`), switched in place by `LauncherLink::switch_mode` |
| The focused app's menus: the service, the titles the bar makes, the shell's own rows, what an agent's activation costs | `sill_model::app_menu` (`AppMenus`, `titles`, `own_groups`, `route_of`, `effect_of`: the one table), served by `sill_services::app_menu` (`AppMenuService` follows each followed app's `org.quire.AppMenu.<app_id>`; `MenuAgent` is `shell.menu.list` and `shell.menu.activate`); the bus name is quire's `AppMenuAddress`, handed in as `sill_ui_kit::app_menu::menu_name_of`; the bar draws them as `BarMenu::AppMenu` through the one `MenuPopup`, rows through `bar/dbusmenu.rs` (also the tray's) |
| Who owns a bus name, as an app (proof for a command id an app claims) | peephole's standing of the owner's unique name (`Peers::who`), in `sill_services::app_menu::agent::identity` (a Flatpak scope with unreadable metadata, read over `Env::proc_root`, is `Identity::Unreadable`); whether the owner is the focused window's program at all is `sill_services::callers::menu_shown`, asked before a menu is read, shown or clicked |
| What a fresh account's dock and launcher start with (the desktop's own apps that are installed, in the Mac's order, once) | `sill_model::seed::{first_apps, first_dock, first_frecency, Store}` (pure: the table of places, the decision from whether the store was ever written); `sill_services::first_run::store_of` reads that fact; `sill_services::pins` seeds `dock.json` and `sill_search::results` the launcher's `frecency.json` once the apps service has scanned. A store that exists, even an emptied one, is never seeded |
| Where in Settings a link points (page, key or account), and every way sill opens Settings | `ds_settings::target::SettingsTarget` (quire's: `page:wallpaper`, `key:dock.magnified_size_px`, `account:<id>`), carried as its text by `sill_launcher::SettingsLink` (the pure crates cannot name `ds-settings`), built where the link is made with `SettingsTarget`'s `Display` (the search provider from the page its schema declares through `PageTable::resolve`, the bar's Wi-Fi and Bluetooth feet, the sync-hold notice) and opened with `LaunchCommand::OpenSettings(SettingsOpen { link, token })`, which the launch service sends as `ActivateAction("open-page", [id])` (a page), `("open-account", [id])` (an account) or `("open-page", [text])` (anything else) to `org.quire.Settings` (`launch/backend/dbus.rs`); the model picker is `sill_model::launch::{assistant_model, assistant_model_setup}`; the token comes from `sill_ui_kit::surface::open_settings_with_token` |
| A turn that meets no model: what it shows and what it opens | `sill_model::companion::model_picker` (pure: `Picker`, `picker_of`, `opens`) asks for `CompanionEffect::PickModel` once, when an answer asked here becomes `RefusalWire::NoModel`; the companion service sends the launch service `assistant_model()`. The turn wears no words (`answer_line` has an empty second line; `sill_ui_kit::companion::answer_view` gives a card with an empty body) |
| The first-run offer of the model picker, once per user | `sill_model::seed::{first_model_offer, ModelOffered}` (pure: from whether the record was ever written) and `sill_services::model_offer::offer_first_run` (writes `$XDG_STATE_HOME/sill/model-offered.json`, then opens `assistant_model_setup()`), called by `sill::daemon::run` once the host is connected |
| Opening a terminal that runs a command (the counterpart of opening a link in the default browser); the default terminal; the wrapper of a `Terminal=true` app | `sill_services::launch::backend::terminal/` (`choose.rs`: the Default Terminal Execution lists, then temor's `org.quire.Temor.desktop`, then terminals found by name, then the TerminalEmulator category; `open.rs`: `open_in_terminal`, the only place a terminal process starts; the places searched come from `Env`: `Dirs::{config_dirs, program_dirs}` and `current_desktop`) |
| The helpers the person keeps: the list, a helper's card, what it may use and how far, its model, its jobs, what it told the person | `sill_model::companion::{HelpersModel, HelpersMsg, HelpersCommand, Draft, update_helpers}` (pure, held in `Companion::helpers`; a card saves with a name, what it is for and an app; a missing model is allowed and the card says to choose one, which opens Settings' model picker; removing asks first; a helper that runs an app names it in plain words (`sign_in.rs`: `kind_line`, `login_line`, `sign_in_buttons`) and signs in and out from its card or from a job waiting for it; `routines/` is the routines part, with the sheet that confirms a new one and the words that say a missed run is skipped, and the routines helpers suggest (`proposals.rs`: accepted or turned down by the person, the suggester's reason shown quoted); `automatic.rs` holds the models ticked for one helper's Automatic pool; `models.rs` groups this computer's models under the engine that runs each), `sill_services::companion` (`backend/helpers.rs`: the `org.quire.Companion1.Helpers2` calls and its `Changed` signal; `backend/routines.rs`: the routines' calls; `backend/proposals.rs`: the routines helpers suggest (list, accept, decline; the only file naming those calls); `backend/terminal.rs`: the window a sign-in or a handover asks for, running the command the companion resolved (inside the helper's sandbox with only its own variables when it has one) through the desktop's terminal opener, which tells the screen when it closes (a handover's close asks the companion for the work back; `handover.rs` in the model decides what the card offers: Open in Terminal, or In Terminal with Bring Back); `backend/terminals.rs`: the sessions started in a terminal, on `org.quire.Companion1.Terminals` (the only file naming those calls); `backend/pool.rs`: keeps a helper's Automatic pool through porter's client (`Accounts::set_helper_pool`); the apps a helper may be given are `offered_apps` of intentd's manifests), `sill_model::companion::terminals` (the sessions started in a terminal: the list, Bring Back, the words of their "In Terminal" rows; listed by `sill_search::providers::CompanionProvider` in the companion filter), `sill_search::providers::HelpersProvider` (the Helpers filter, Tab's second stop in companion mode, with the New Helper row) and `sill_launcher_ui::launcher::helpers` (the card in the preview pane; Pause, Resume and Remove are the row's actions, `CompanionAct::Helper`) |
| The Force Quit Applications panel: which apps it lists, which are hung, its opening, the app confirmed | `sill_model::desktop::{force_quit_rows, hung_first, ForceQuitRow, Standing}` (pure), `sill_ui_kit::links::ForceQuitLink` (opening, rows, the confirmation), `sill_overlays::force_quit` (the surface and its `step`), `sill::force_quit` (the confirmation as `DesktopCommand::ForceQuitApp`) |
| Night Shift: casement's software night light asked for by the person, the manual turn-on's end, who warms the screen | `sill_model::night_shift::{NightShift, step, shown, until_tomorrow}` (pure; the switch, the warmth slider's scale `warmth_at`/`warmth_fraction`), `sill_services::night_shift` (reads and merges `[night_light]` in `~/.config/casement/settings.toml`, keeps the end in `$XDG_STATE_HOME/sill/night-shift.json`, ends it with the clock), `sill_model::night_shift::{skip_outputs, unmatched}` (pure: the connectors of the DDC monitors sill warms, and the ones it cannot name) written to `night_light.skip_outputs` by `sill_services::night_shift` as the displays service's monitors change, `sill::policy::night_shift` (`display.night_warmth = hardware` is the backend told to it), the Night Shift row of the Display module (`sill_bar::control_center::modules::night_shift`) |
| Watching files or folders for edits | `sill_services::debounced` (`Debounced`, `Place`, `EventFilter`) |
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
| A settings domain, its typed values | `sill_settings::<domain>` |
| The settings file type and its `SettingsDoc` | `sill_settings::file` |
| Loading, saving, watching, publishing settings | `sill::daemon::settings` |
| Changing one key of `settings.toml` | `sill::daemon::settings::write_key` over `ds_settings::SettingsWriter` (in place: every other byte, comment and key stays; no other writer in sill) |
| Whether a time of day is written 12 or 24 hour | `[region] hours` of quire's desktop-wide file, settled by the session locale (`LC_ALL`, `LC_TIME`, `LANG`) in `sill_ui_kit::hooks::use_hours`; written with `sill_model::clock::{format_clock, time_of_day, time_digits, time_of_minute}`; the clock service publishes the bare minute (`ClockMinute`), never text |
| The settings a surface reads | `sill_ui_kit::settings` (`SettingsFeed`), `use_section(|file| &file.<domain>)` |
| Settings turned into a service command | `sill::policy::<service>` (`Policy`) |
| Focus mode value, link, writer | `sill_model::focus`, `sill_ui_kit::links::focus`, `sill::daemon::settings::focus` |
| The name of an action of the shell | `sill_launcher::actions::ActionId` (a `Word`; not `sill_launcher::ActionId`, a launcher row's own action) |
| What an action is called, asks, is bound to and does | `sill_model::actions::def` (`ActionDef`: label in the Mac's words, `confirm`, `chord: Option<StandardAction>`, closed `Effect`) |
| Carrying an action out | `sill::actions::perform` (the one interpreter: output, activation token, ask first; the pieces are `actions/{panel,launch,app,pref,show_desktop,workspace,next_window,switcher}.rs`) |
| A surface asking for an action | `sill_ui_kit::links::ActionLink` (`do_it`, `do_to`, `Origin`); the daemon forwards it with `actions::spawn_asks` |
| IPC words | `sill_ipc::{request, response, show, status, act, debug}` |
| Answering an IPC request | `sill::ipc::dispatch::<group>` (one fn per domain); `Request::Do` goes to `sill::actions::perform` |
| A command-line verb | `sill::cli::<group>`, mapped in `cli/request.rs`; the verbs casement sends are `shell_verbs::ShellVerb`, mapped to actions in `cli/verbs.rs::action_of` |
| Launcher matching, ranking, frecency, sections | `sill_launcher` |
| A launcher result source | `sill_search::providers::<kind>` (`Provider`) |
| The search runner and its history | `sill_search::results` |
| Launcher row to menu row | `sill_launcher_ui::launcher::rows::row` (one exhaustive fn) |
| What a file looks like in the pane (sniffing, the peek of each kind, the pane that draws it) | `anyview_peek::probe`, `peek_with`, `Card`; sill's part is `sill_launcher_ui::launcher::preview::load` (the worker and the budget), `frames` (`ThumbnailFrames`: `anyview_peek::VideoFrames` over the freedesktop thumbnail cache in the injected `Dirs::cache`, so a video without a cover shows the file manager's picture of it, or the facts card when there is none or it is stale) and `content` (the pane in quire's `PaneContent::Slot`). sill names no format |
| Which actions a file's row has, and their keys | `anyview_core::actions_for`, `reach`, `shortcut`, through `sill_search::providers::files::actions::{carried, row_actions}` (`carried` is the one match on `FileAction`: what the launcher does for each) |
| The files the viewer opened, as launcher rows and as frecency | `anyview_store::read_history` through `sill_search::providers::files::viewed` |
| An app's icon in a tile or a row, for the surface on an output | `sill_ui_kit::app_icons::use_tile_icons` (the split picker and Force Quit use it) |
| Opening a file in the viewer, playing audio | `OpenTarget::{Viewer, Play}` (`sill-launcher`), carried out by `sill_services::launch::backend::viewer` as one call on `anyview_platform::linux::forward_over`; the results the viewer walks are added by `sill_launcher_ui::launcher::activate::with_results` |
| Surface root: services, feed, host seam, appearance, scale, frame clock | `sill_ui_kit::root::SurfaceRoot` (the host seam is `ds_blitz::provide_host()`, called by `mount`, `mount_lock` and each popup root; the seat's `HostSignals` (scale, modality) come from `use_host_signals()`, called by `mount`, `mount_lock` and `PopupFrame`, and a surface reads its scale with `use_host_scale()`) |
| Active workspace look, space look | `sill_ui_kit::look` (`use_active_look`, `use_space_look`) |
| Open/close a panel from outside it (serial, output) | `sill_ui_kit::links::Reveal<O>`; every link type is one file in `sill_ui_kit::links` |
| Visible or hidden layer surface | `shell_host::Visibility` |
| Layer namespace `sill-<surface>-<output>` | `sill_ui_kit::surface::namespace` |
| Spawning and closing surfaces per output | `sill::daemon::surfaces` (generic over `Surface`) |
| Which surfaces exist on which output | `sill::daemon::outputs` |
| A timed pure state machine and its timer; a machine's `Stamp` and the `FrameClock` that makes it | quire: `ds::base::machine` (`Machine`, `Elapsed`), `ds::machine` (`use_machine`, `MachineRef`), `ds::base::time::stamp` (`Stamp`, `FrameClock`); sill has none of its own |
| Popup menus (bar, tray, dock, launcher actions) | `sill_ui_kit::menu` (`MenuModel`, `MenuPopup<M>`) over `ds`'s `menu_track`; a context menu a layer opens (at its pointer, or over a dock tile) is `use_context_menu::<M>` (one popup at a time, the report channel and the task that hands each pick to the owner; `use_context_menu_closing` also tells the owner when the popup is gone; the desktop menu, the window menu and the dock menu use it) over `context_menu_popup::<M>` (the popup root, reporting over a channel); where the popup goes is a `PopupGeom` (`AtPoint { clear }`, `OverTile { gap }`), whose size is one number. The bar's menus stay on their own tracking |
| Where a panel opens: the focused window's output, else the pointer's (the first output until the daemon knows the pointer); which outputs a surface lives on | `sill_ui_kit::surface::OutputPolicy` (`Surface::OUTPUTS`); `sill::outputs::Outputs::{pick, open_on}` |
| A surface's `LayerConfig` | `sill_ui_kit::surface::LayerPreset`; the blur radius is `sill_ui_kit::panel::PANEL_RADIUS` |
| A panel's open, toggle, close and focus on its link | `sill_ui_kit::links::{PanelLink, Opening}` |
| What a surface sends back for the desktop to carry out | `sill_ui_kit::links::{Outbox, Sent}` on the link; `sill::outgoing::{Outgoing, forward}` |
| Popup sizing | shell-host `FitContent`; no estimation code |
| App icons, look, cache | `sill_ui_kit::app_icons` |
| Status glyph props from service state | `sill_ui_kit::status` over `sill-shell-kit` glyphs |
| Notification card | `sill_ui_kit::notification_card` |
| Widget provider, `Card<P>`, the center's two cards | `sill_ui_kit::widgets` |
| The desktop's context menu (Change Wallpaper…) | `sill_overlays::wallpaper::menu`, on the wallpaper surface |
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
| Which folders a document may read pictures from (the host serves a `file:` URL only inside them) | `sill_ui_kit::root::file_roots(&Dirs)`, passed to shell-host as `HostConfig::file_roots` by `host_config(&Dirs)`: `Dirs::icon_file_roots` (`Dirs::icon_dirs`: icons and pixmaps of the data directories, `~/.icons`, `/usr/share/pixmaps`; then `Dirs::app_stores`: `/opt`, `~/.local/opt` and each snap's `current` folder, where a desktop entry may name its icon by absolute path; the apps scan reads the same list, so what it keeps is what the host serves), sill's cache folder (tinted app icons, wallpaper and its blurred copy, now-playing covers, screenshot previews), the freedesktop thumbnail cache `Dirs::thumbnails` (a picture file's thumbnail in a launcher row, `sill_launcher_ui::launcher::row_thumb`), the Pictures folder, `~/.face` and AccountsService's icons folder. A new place that draws a file adds its folder to this list, never a second list; `Dirs::data_dirs` is `$XDG_DATA_DIRS` read once at the edge |
| A demo desktop (nested, scratch, left running) with a scripted tour, shots and a video | `dev/demo.sh start\|tour\|video\|shot\|stop` |

---

## 5. Canonical traits

### `Service` (`sill_services::service`)

```rust
pub trait Service: Sized + 'static {
    const NAME: &'static str;
    type State: Clone + Default + PartialEq + Send + Sync + 'static; // sill_model type; default = absent
    type Command: Send + 'static;                                    // sill_model type
    type Deps: Clone + Send + 'static;                               // sill_model receivers, Controls; () if none; cloned for each restart
    fn initial(env: &Env, deps: &Self::Deps) -> Self::State { Self::State::default() }
    fn run(env: &Env, deps: Self::Deps, io: ServiceIo<Self>)
        -> impl Future<Output = Result<(), ServiceError>> + Send + 'static;
}
pub struct Handle<S: Service> { pub state: watch::Receiver<S::State>, pub control: Control<S::Command> }
pub fn spawn<S: Service>(rt: &tokio::runtime::Handle, env: &Env, deps: S::Deps) -> Handle<S>;
```

`spawn` keeps the service running: a task that panics, returns an error, or ends cleanly after
running ten seconds or more, while its `Control`s are held, is started again after a growing wait (`restart.rs`: 0.5 s doubling, `RESTARTS_MAX` = 5 in a
row, a run of a minute resets the count) and then left stopped with one log line. Each run gets
fresh channel halves; the keeper carries commands in and state out, so a restart begins from the
last published state. A bus absent by choice and a compositor without the protocol are not
restarted.

Implementors: the 30 `<Module>Service` markers (`enum XService {}`), 29 in `sill-services`,
`ResultsService` in `sill-search`. Add an implementor for a new long-lived backend that
publishes one state. Add nothing for a helper without a task. Not a `Service`: a one-shot call
(`launch::open`), a pure derivation (`switcher::apps`), or a UI link.

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
    const OUTPUTS: OutputPolicy = Every;            // Every | FocusedWindow | Main
    const KEYBOARD: FocusLoss<Self::Link> = Ignore; // Ignore | CloseLink(Link::keyboard)
    const ROLE: Option<ShellRole> = None;           // what the compositor knows it by
    const LAYER_ORDER: u8;                          // spawn order = drawing order within a layer
    const PRESENCE: Presence;                       // Always | FollowsPanel | SheetStage | Daemon | SelfMapped
    fn link(links: &Links) -> Self::Link;           // its field of `Links`
    fn needs(root: HostRoot, links: &Links) -> HostRoot;  // other links its document reads by type; default none
    fn slots(place: &Place) -> Vec<Self::Slot>;     // which surfaces on this output; empty = none
    fn layer(output: &OutputInfo, settings: &ShellFile, slot: Self::Slot) -> LayerConfig;
    fn start() -> Start;                            // provided: `Always` is shown, the rest hidden
    fn root(output: OutputInfo, slot: Self::Slot) -> Element;
}
pub struct Place<'a> { pub output: &'a OutputInfo, pub rank: OutputRank, pub settings: &'a ShellFile }
// The session lock is one surface, so `LockSurface` is a value (sill-overlays' `LOCK`), not a trait.
pub struct LockSurface {
    pub name: &'static str,
    pub config: fn(&ShellFile) -> LockConfig,
    pub root: fn(OutputInfo) -> Element,
}
```

Implementors of `Surface`: the 25 layer surfaces (25 `impl Surface for` sites, one per `surface.rs` in `sill-bar`, `sill-dock`, `sill-launcher-ui`, `sill-notify-ui` and `sill-overlays`); the lock is the `LOCK` value and is not on this path. A new visible thing on its own layer surface
is a new `Surface`. A popup of an existing surface is a `MenuPopup` or a component, not a
`Surface`. `root` runs inside `surface::mount::<S>`, the one root component every spawned surface
has, so a `root` may call hooks. `SurfaceRoot` stays a plain component that each view wraps its
content in: the material, chrome and extent it takes are the surface's own, which a wrapper
generic over `S` could not know. The daemon gives every root its `S::link` as a root context, and the other links `S::needs` names (a bar reads the focus mode, the power menu and the notification center by type); a
slot is erased to its `Debug` text in the output's row (`OutputSurfaces::spawned`), which also
keeps the layer namespace `debug stats` reports. A slot the settings drop is closed, and a layer
size that changed is re-applied, by `daemon/spawn.rs`.

The declarations are read by one place each. `OUTPUTS`: `daemon/spawn.rs` spawns a `Main` surface on the main display only (the sheets that ask the person; the main display is the first output the compositor announced), and every opener of a panel (a verb, a chord, the companion, a hot corner) asks `Outputs::pick` / `Outputs::open_on::<S>` for the output: the focused window's, and the pointer's where that is known (the daemon does not know it yet, so the first output stands in, in that one function). `KEYBOARD`: a `CloseLink` surface's link hears each keyboard focus change of the surface through `ClosesOnFocusLoss`, and the daemon routes every `HostEvent::KeyboardFocus` to it (`Spawned::focus`), so a new surface needs no line in `daemon/events.rs`. Mac behaviour, per surface: the launcher, power menu, notification center, switcher and split picker close when they lose the keyboard after holding it; force quit, the display trial, the other sheets and the eyedropper stay. `ROLE`: recorded at spawn (`Spawned::role`) for the compositor's registration. `LAYER_ORDER` and `PRESENCE`: `daemon/surfaces.rs` sorts its list of kinds by `LAYER_ORDER`, and `start` follows `PRESENCE`. A sheet, a strip, a corner column or a whole-output surface builds its `LayerConfig` from a `LayerPreset` (`FullOutputSheet`, `PanelOnly`, `Strip`, `Corner`, `Whole`), whose blur radius is `sill_ui_kit::panel::PANEL_RADIUS`, the mirror of quire's `--r-panel` token.

A panel's link is a `PanelLink<P>` over `Reveal<Opening<P>>` (`links/reveal.rs`): `open`, `reopen`, `toggle`, `close`, `focus`, `subscribe`, with the panel's own payload `P` (`()` for force quit and the notification center, `LauncherAsk`, `PowerMenuPayload`); what a panel adds is an `update` on it. What a surface says back for the desktop to carry out (a window menu pick, a Split View answer, the display trial's answer, a confirmed force quit, a Spaces bar click) is an `Outbox<T>` on its link, and `sill::outgoing` forwards every outbox to the host with one task per link type (`Outgoing`, listed in `outgoing::spawn_all`).

### `Machine` (quire: `ds::base::machine`, hook `ds::machine`)

The trait, `Elapsed`, `use_machine`, `use_machine_in` and `MachineRef` are quire's, with the same
signatures (`step(self, In, at: Stamp, &Params, &Ctx) -> (Self, Vec<Out>)`, `wake(&self) ->
Option<Stamp>`, no `Default`: the surface seeds the first state; `Ctx` is what a step reads from
outside the machine, read by a closure at every step; see quire ARCHITECTURE.md section 5), and so are `Stamp` (whole milliseconds from an origin) and the
`FrameClock` that makes one from the thread's clock, which a harness's virtual clock drives. sill
keeps neither a trait nor a stamp of its own.

Intended implementors: dock hover/press/reveal/label, switcher, osd, banners deck, hot
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
pub trait WidgetProvider: Clone + PartialEq + Default + 'static {
    type W: sill_shell_kit::prelude::Widget;        // a value: `Widget: Default`
    const KIND: WidgetKind;
    fn use_feed() -> Feed<Self::W>;           // reads the services; one hook, one shape
}
pub struct Feed<W: sill_shell_kit::prelude::Widget> {
    pub entry: W::Entry,                                   // the entry now
    pub onintent: Option<EventHandler<W::Intent>>,         // the card's controls; none without any
}
#[component] pub fn Card<P: WidgetProvider>(provider: P) -> Element;  // a WidgetCard tile, `SIZE` (Medium)
pub fn with_provider<A: OnProvider>(kind: WidgetKind, action: A) -> A::Out;  // the only match on the kind
pub fn card(kind: WidgetKind) -> Element;             // `with_provider(kind, ..)`
```

Widgets live only in the notification center: `CenterWidgets` draws the fixed list
`[Calendar, UpNext]`, each a Medium tile; nothing is placed, picked or edited. `WidgetKind`
derives quire's `Word`: `ALL` and the slug (the element id `widget-<slug>`) come from the
derive. A provider is a `Default` unit struct (`CalendarProvider`), passed to `Card` only to
name the type. Calendar's `use_feed` keeps the shown month in a signal and returns the
`MonthIntent` handler. Now Playing and the batteries are control-center modules, not widgets
(`sill_ui_kit::now_playing::use_media`, `sill_ui_kit::status::device::device_of`).

Implementors: calendar, up next.

### `MenuModel` (`sill_ui_kit::menu`)

```rust
pub trait MenuModel: 'static {
    type Pick;
    fn entries(&self) -> Vec<ds::prelude::MenuItem<Self::Pick>>;
    fn pick(&self, path: &ItemPath) -> Option<Self::Pick>;
}
#[component] pub fn MenuPopup<M: MenuModel>(model: M, on_pick: EventHandler<M::Pick>, on_close: EventHandler<()>, surface: &'static str) -> Element;
```

Implementors: `DockMenu` (the dock's tile menu), `DesktopMenu` (the wallpaper's), `WindowMenuModel` (the window menu). The bar's app menu, the tray and the control center are a different machine (menu tracking with hover and release semantics, `sill-bar/src/bar/menu_popup.rs`), and the launcher's actions menu is not a `MenuModel`. A context menu a layer surface opens is `use_context_menu::<M>` (below).

### Enums that stay enums

| Enum | Home | Why |
| --- | --- | --- |
| IPC `Request` `{Ping, Quit, Reload, Do, Show, Status, Act, Debug}`, `Response` | `sill-ipc` | closed wire set; one dispatch fn per domain, and `Do` is every action of the shell |
| `ActionId`, `Effect`, `Subject` | `sill-launcher`, `sill-model` | closed; a new action is a compile error in `def` and in `perform` until it is described and done |
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
   `std::env`, `Command::new` or `Zoned::now` outside `Env`. Build the backend from the kit, and
   write only what is specific to the service:
   - reach a bus with `bus.connect_waiting().await?` (`try_connect` only where a service has its
     own way to ask again);
   - follow a peer's name with `owner::Owner`, looping on `owner.session(session(..))` (seed,
     then follow) and publishing what an owner leaving means in the loop body;
   - publish through `machine::Published` (the step returns the next value) or
     `machine::Machine` (it also returns effects), never a hand-copied `send`;
   - serve a name with `serve::claim` and publish `ServeLink` (`ClaimError` says why not); claim
     only when `Env::claims` is `Claim`;
   - tell the person through `notices::Board<K>` and `Notices<K>`: the service supplies the
     `Posting`, the action key and `shown_for`;
   - watch files and folders with `debounced::Debounced`, and a Wayland connection with
     `wayland_thread::Follow`.
5. `pub mod <x>;` in `sill-services/src/lib.rs`; re-export the marker only.
6. `sill-ui-kit/src/services.rs`: a `Handle<XService>` field and a `members!` line; the harness
   `FakeServices` gets its default. `sill/src/daemon/services.rs::start`: one `spawn` line, after
   the services in its `Deps`.
7. A service that claims a bus name needs nothing to be told: `Env::claims` decides. If it must
   stay passive until the person's settings say what to do, add a `Policy` (recipe below).
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
   the roles in "The one shape of a surface": `mod.rs`, `surface.rs`, `model.rs`, `step.rs`,
   `io.rs`, `view.rs`, `style.css`, `tests.rs` (and `layer.rs` only when no `LayerPreset` fits).
   `lib.rs` declares `pub mod <name>` and re-exports the marker type; `mod.rs` re-exports the
   stylesheet const (`NAME_CSS`) the lint row needs.
2. `surface.rs`: `pub enum Name {}` and `impl Surface`. Declare, and the daemon does the rest:
   `NAME`, `Link` and `fn link(links)` (its field of `Links`), `Slot`, `OUTPUTS` (the default,
   `Every`, is one per output; `FocusedWindow` for a panel that opens where the focused window
   is; `Main` for a question for the person), `KEYBOARD` (`Ignore`, or `CloseLink(Link::keyboard)`
   for a panel that closes when it loses the keyboard; the link implements
   `ClosesOnFocusLoss`), `ROLE` (only if the compositor knows the surface), `LAYER_ORDER` (a
   number between its neighbours' in the table of `kinds()`; later draws over earlier on one
   layer), `PRESENCE` (how it is mapped: `Always`, `FollowsPanel`, `SheetStage`, `Daemon`,
   `SelfMapped`), `needs` (only if its document reads another link by type), `slots`, `layer`
   (`LayerPreset::..config(Self::NAME, output)`) and `root`. `root` is `view::name(output)`; no
   `Ds` boilerplate: `SurfaceRoot` supplies services, feed, appearance, scale and the frame
   clock, and `mount` has already installed the host seam (`ds_blitz::provide_host()`) and the
   seat's `HostSignals`; a `Ds` root passes no `scale`.
3. A link the daemon or another surface drives is one file in `sill-ui-kit/src/links/<name>.rs`
   and a field of `Links` in `links/mod.rs`, built in `build_links` in `daemon/run.rs`. A panel
   is a `PanelLink<P>` (the payload is what an opening carries); an answer for the desktop is an
   `Outbox<T>`. A surface never imports another surface.
4. Settings keys: recipe below. Services it reads: `use_service` in `io.rs` only.
5. Timed state: `impl Machine` (`ds::base::machine`) in `step.rs`, run by `ds::machine::use_machine` in `io.rs`.
6. `sill/src/daemon/surfaces.rs`: add `kind::<Name>()` to `kinds()` and import the marker from the
   surface's crate; the order is the surface's `LAYER_ORDER`, and the generic `daemon/spawn.rs`
   spawns, resizes and closes. There is no per-surface spawn code, `outputs.rs` field or
   namespace helper (`debug stats` reads `OutputSurfaces::spawned`), and no line in
   `daemon/events.rs` for the keyboard. If the surface sends answers, implement
   `sill::outgoing::Outgoing` for its link and list it in `outgoing::spawn_all`.
7. If it is a panel the person opens: an action (recipe below), and its arm in
   `sill/src/actions/panel.rs`, which picks the output (`origin.output`, else
   `reach.outputs.open_on::<Name>(reach.services)`).
8. Tests, in the surface's crate: `crates/<crate>/tests/<name>.rs` on
   `SurfaceHarness::<Name>::setup(link, slot).settings(..).viewport(..).appearance(..).start()`
   (render, keys, motion on the virtual clock), and unit tests in `src/<name>/tests.rs`. A
   full-output sheet is also a row of `crates/sill-overlays/tests/it/layers.rs`. The lint
   is the one place outside the crate: a `SurfaceCase` row in `SURFACES` in
   `crates/sill/tests/lint.rs` (the stylesheet const and a function drawing each state to lint).
   Then `dev/accept/<name>.sh`.
9. Add the row to the surfaces table in this file, and the surface name to `docs/selectors.md`.

### Add a control-center module

1. `ControlCenterModule` variant in `sill-settings/src/control_center.rs` (a `Word`; `ALL`
   follows from the derive).
2. `sill-bar/src/control_center/modules/<name>.rs`: the component over `use_service`, and `impl
   CcModule for NameModule`; a detail page in `pages/<name>.rs` if it has one.
3. The one arm in `modules/mod.rs::module`. No other `match` on the kind.
4. Tests: a row in `sill-bar/tests/control_center_render.rs`, whose loop runs every `ControlCenterModule::ALL`
   through tile, dropdown and detail on `SurfaceHarness::<ControlCenter>`.

### Add a widget

1. The `sill_shell_kit::prelude::Widget` (`sill-shell-kit`'s `widget/<kind>/{mod, model, view, style.css}`),
   then a `WidgetKind` variant in `sill-settings/src/widgets.rs`.
2. `sill-ui-kit/src/widgets/<kind>/{mod,io,tests}.rs` (+ `style.css` if it has one):
   `impl WidgetProvider` (`use_feed` over services) on a unit struct in `io.rs`.
3. The one arm in `widgets/card.rs::with_provider`, and the kind in `CENTER`
   (`sill-notify-ui/src/notification_center/widgets.rs`); the slug and element id follow.
4. Tests: the entry table in the kind's `tests.rs`; `sill-notify-ui/tests/center_widgets.rs`
   counts the center's tiles.

### Add a launcher provider

1. `ProviderKind` variant, and an `Activation` variant only if none fits, in `sill-launcher`.
2. `sill-search/src/providers/<kind>/mod.rs`: `impl Provider`; effects take `Env` values
   (paths, buses) through the constructor.
3. Register in `sill-search/src/results/registry.rs`, in rank order.
4. The row's arm in `sill-launcher-ui/src/launcher/rows.rs::row` (exhaustive).
5. Tests: `query` and `activate` tables beside the provider with `VecSink`; a rank test row in
   `sill-launcher`.

### Add a settings key

1. The design/22 row in quire first (with `RETIRING` when removing one).
2. `sill-settings/src/<domain>.rs`: a field of a typed value: a `Word` enum, a `ds-settings`
   unit newtype, or a two-variant enum (no `bool`); the default is the design's. A string that
   names a closed set is an enum parsed at load; a shared enum lives in `sill-model`.
   A list of tables is a `Vec<Row>` with `#[derive(SettingsRow)]` on the row (quire design/22
   §9.1); the file keeps the person's text, a function in the domain file turns the rows into
   typed values once per load and names each row it dropped, and a malformed row costs only
   itself (`keyboard.rs` is the pattern).
3. A new domain is a file, a `ShellFile` field and a `schema.rs` entry.
4. The reader: a surface reads `use_section(|file| &file.domain)`; a service is told through its
   `Policy`. `sill --write-schema <dir>` regenerates the schema.
5. Tests: `sill-settings/tests/keys.rs` cross-checks the key against quire's rows both ways;
   a parse table for a new enum.

### Add an action

An action is a thing a person can name, reachable from more than one entrance (a menu, a corner,
a launcher row, a chord, the command line, an agent). Service protocol (`*Command`), tray and
app-served menus, window-menu entries, sliders and a search row's own data are not actions.

1. The name: a variant of `sill_launcher::actions::ActionId` (a `Word`; kebab-case on the wire).
2. What it is: its row in `sill_model::actions::def`: the label in the Mac's words (the one place
   a label is written), the `StandardAction` whose chord does it when there is one, and the
   `Effect`. A new kind of effect is a variant of `Effect` (and of `Effect::needs` and
   `Effect::confirm`, which have no wildcard); a power action that ends the apps asks first
   through `SessionAction::route`, nothing else does.
3. Doing it: the arm in `sill/src/actions/mod.rs::perform` for its `Effect`, in the file of its
   kind (`panel.rs`, `app.rs`, `pref.rs`, `workspace.rs`, ...). The arm owns the output
   (`origin.output`, else `Outputs::open_on`), the activation token of a launch
   (`launch::with_token`, always asked of `origin.surface`) and the refusal text; the entrance
   decides none of them.
4. Asking for it, by entrance, never by a command of its own:
   - a surface: `ActionLink::do_it(action, Origin::of(surface, output))` or `do_to(action,
     subject, origin)` (the link is a root context its `Surface::needs` names; the daemon forwards
     it to `perform`); a menu item is `Ask::of(action)` or `Ask::on(action, Subject::App(..))`;
   - a launcher System row: `Activation::Do(action)` from its provider;
   - a hot corner: its row in `corners::corner_effect`;
   - the command line: the request is `Request::Do(Ask)`, built in `cli/request.rs` (or the verb
     group's file) and covered by its table test. A verb casement sends is a variant of
     `shell_verbs::ShellVerb` (that crate, in casement-protocols, first), with its row in
     `cli/verbs.rs::action_of`; clap's grammar for the same words must agree, which
     `every_verb_the_compositor_sends_parses_and_reaches_its_action` checks. Never rename a
     spelling that casement already binds;
   - intents: its row in `sill::intents::request_of`.
   - a panel's open and close (as against its toggle) and the launcher's seeded open stay `Request::Show`
     (`ShowRequest`, a `Reveal`), answered by the same `actions::panel` fns the action uses.
5. Tests: the table tests of `sill_model::actions` cover the row (label, effect, chord, wire form
   of an `Ask`); add the entrance's row to its table (`cli/tests.rs`, `corners.rs`, the surface's
   `pick.rs`). A pure helper of the arm (a plan) is tested beside it.

### Add a popup menu

1. In the owning surface, `menu.rs`: `struct XMenu`, `impl MenuModel` (`entries`); the `Pick`
   enum is the surface's own.
2. In `io.rs`: `use_context_menu::<XMenu>("x-menu", |pick| ..)` (the name is the popup
   document's `data-surface`; add it to `docs/selectors.md`) and call
   `menu.open(model, look, geom, anchor)` where `geom` is a `PopupGeom` (`AtPoint { clear }` for a
   menu at the pointer, `OverTile { gap }` for one over a tile) and `anchor` the pointer's
   one-pixel `point_anchor(x, y)` or the tile's rectangle. The hook owns the one-popup-at-a-time
   rule, the report channel, the task that hands each pick over and the popup's size; use
   `use_context_menu_closing` when the owner must also hear that the popup is gone. No menu
   markup, no tracker, no `geometry.rs`. (The bar's menus are the one exception: their menu
   tracking is its own machine.)
3. What a pick does: a row that is an action of the shell names it (`ActionId`, with `Subject`
   when it acts on an app) and asks for it on the `ActionLink` (recipe above); only a row whose
   items come from another process or the surface itself (a tray row, a window's own entries)
   has a `Pick` of its own.
4. Tests: the `entries` table in `tests.rs`; keyboard and hover switching are `ds`'s, and where
   the popup goes is `PopupGeom`'s own test.

### Find the front window, the main group or a window's app

Never write `find(|w| w.activity == Activity::Active)` or match an app id to an entry for a window; ask
`sill_model::windows`:

- `Toplevels::active()` is the window with focus, `None` when nothing has it. Anything that
  acts on focus (keys, profiles, the companion, next-window, the dock's Hide) uses it.
- `Toplevels::front(output)` is the window a person calls frontmost on `output` (any for
  `None`): the active window if it is there, else the most recently activated one there. It
  is for showing "the front app" (the bar).
- `Workspaces::main_group()` is the group an index or a step counts in (the first with an active
  workspace, else the first); `WorkspaceGroup::active()` and `active_index()` are its active
  workspace.
- `app_of(&Toplevel, &AppIndex)` (or `app_for(raw, &AppIndex)` for a bare app id such as a
  notification's) is the one fallback rule: an `AppMatch::Entry(AppId)` when `AppIndex::matching` (quire's
  `match_app`) found a
  desktop entry, else `AppMatch::Unmatched(raw)`. `.id()`, `.entry(&index)` and `.name(&index)`
  read it. Tests: add a row to the table beside the function, not a new copy of the rule.

### Get the appearance

A surface calls `use_appearance()` and nothing else. It reads the root's pinned or switchable
appearance (tests) or the `SettingsFeed::appearance` watch, which the daemon fills from its one
watch of `quire/appearance.toml` and the settings portal (`daemon/settings/appearance.rs`). A
root with neither gets `Environment::default()`; no hook ever starts a watcher. A view that
changes the appearance calls `sill_ui_kit::appearance_write::write_appearance(next)`, the one
writer of the file; the daemon's watch publishes the change back. A new daemon-side reader of
the appearance clones `settings_feed.appearance`.

### Add a dev script

1. Per surface or feature exactly one file, `dev/accept/<name>.sh`; variants are flags
   (`--real-input`, `--bus`), never a second file.
2. It sources `dev/lib/{bus,daemon,compositor,input,pixels,assert}.sh` and defines none of
   their functions; a missing helper is added there.
3. It starts the daemon with a private bus on both addresses, a fake DDC, scratch `HOME` and
   `XDG_*`, its own `XDG_RUNTIME_DIR`, on the compositor `start_compositor` brings up:
   `SILL_COMPOSITOR=casement|kwin` (a headless casement started in the jail through the casement
   checkout's `dev/up.sh`, or shell-host's headless KWin); a
   script without that variable keeps its own `--default`. Compositor-specific work goes
   through the lib's helpers (`set_scale`, `output_size`, `shot`, `maximised_height`,
   `surface_click`, `input`), never a compositor's own tool; a step the compositor cannot prove
   yet returns `expected_skip WHY` (exit 2).
4. Logs and captures go under `$SILL_OUT_ROOT` and `$SILL_SHOTS` (`target/` and `dev/out/`,
   untracked, or the jail's scratch dir under `dev/accept.sh --compositor casement`, where
   nothing calls cargo: `build` only checks what `dev/accept.sh --build` built outside). What
   needs a person goes to `docs/manual-checks.md`.
   `start_casement` starts casement with `--system-dir` naming a floor it writes in the script's
   output folder (`casement-floor/privacy.toml`, removed by `stop_casement`), allowing exactly the
   programs the scripts run, each for the one power it takes (`CASEMENT_FLOOR_ROWS`: `grim` the
   screen, `wl-copy` and `wl-paste` the clipboard). A floor row names an image the system owns; a
   program built under the checkout (the daemon, `secret_source`) cannot be named and is asked like
   any stranger. A script that runs another program with a power adds its row there.
5. `dev/accept.sh [--compositor casement|kwin] [--jobs N] [NAME...]` runs scripts and
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
| `osd`, `power_menu`, `polkit_prompt`, `confirm`, `accounts_sheet`, `switcher`, `idle_dim`, `hot_corners`, `wallpaper`, `window_menu`, `spaces_bar`, `split_picker` | `sill-overlays` | Overlay (wallpaper Background; `spaces_bar` Top) | `spaces_bar` is Mission Control's Spaces bar: while the compositor's overview is Mission Control and entering, open or tracking (`ShellNote::Overview` -> `sill::spaces_bar::apply_note` -> `SpacesBarLink`, which `daemon/events.rs` also feeds the desktop's view) and `ShellNote::OverviewOutput` says it is up on that output, the surface on that output hangs quire's `SpacesBar` under the menu bar (one tile per Space of that output, `sill_model::desktop::bar_spaces`, over its picture from `ShellNote::SpacePicture` (every Space of the output, hidden ones too) and over that output's wallpaper (`wallpaper::url_for`, passed to each tile as `SpaceTile::backdrop`), lit by `ShellNote::DropTarget`, and the plus, lit by `ShellNote::DropTargetNew`). The surface is the menu bar's height plus `BAR_EXPANDED_PX` tall, taller than the 112 px strip (`STRIP_PX`) the compositor keeps free, and takes input on the bar's own element (`RegionSpec::Element`, id `spaces-bar`). It publishes where its cards and its plus lie (quire's `on_layout`, moved by the output's origin into `BarLayout` on the link's feed; no geometry is computed in sill) and `spaces_bar::spawn_outbox` turns the layouts into `DesktopCommand::SetSpaceTargets`, then `SetNewSpaceTarget` (the plus; `None` once the bar goes), and, per `thumbs_wanted`, one `CaptureSpace` request per Space (kept across switches, asked again only after an answer and a change in the Space's windows). The surface unmaps when quire's lift settles. A tile or the plus writes `BarPick` to the link and `sill::outgoing` sends `SwitchSpace` then `Overview(MissionControl, Off)`, or `CreateSpace`. `split_picker` is the Split View choice: when a window takes one half of the screen the compositor offers the windows that can fill the other (`ShellNote::SplitChoose`, `SplitCandidate`, `SplitChoiceEnd` -> `SplitPickerLink`, which `sill::split_picker::apply` writes with the desktop's titles and output), and the surface on that output dims the screen and shows quire's `AppSwitcher` (one tile per window: its app icon through `sill_ui_kit::app_icons::TileIcons`, its title) in the free half; a tile, Return, Escape or a press elsewhere writes the answer to the link and `sill::outgoing` sends `DesktopCommand::PickSplit` or `CancelSplit`. `window_menu` draws nothing and takes no input: it is the parent of the popup quire's `Menu` shows when the compositor asks for a window's menu (the pointer rested on its green light; `ShellNote::WindowMenu` -> `WindowMenuLink`), and a pick goes back through the link to `DesktopCommand::PickWindowMenu`. Hot corners: one slot per corner. `confirm` is `confirm_sheet/`; `accounts_sheet` (`accounts-sheet`) is accountd's consent question and add-account steps, each a centred titled window on the banners' output (`sill_services::accounts::sheet` serves `org.quire.AccountsSheet1`); `confirm` is `confirm_sheet/`: intentd's question on quire's `Sheet`, mapped on the banners' output; `voice-consent` is `voice_consent/`, the first-use microphone question the same way |
| `force_quit` | `sill-overlays` | Overlay | Force Quit Applications (`sill force-quit {open,close,toggle}`; Option+Command+Escape once casement runs that command for the chord): a quire `Sheet` centred on the output of the active window, a `List` of the apps the desktop lists (`sill_model::desktop::force_quit_rows` from the compositor's `DesktopView`, fed to `ForceQuitLink` by `daemon/events.rs`) with "(not responding)" after a hung one's name and a hung app preselected, a Force Quit button that asks quire's `Alert` ("Do you want to force quit ...?", Cancel the default) before `sill::outgoing` sends `DesktopCommand::ForceQuitApp`. The selection and the question are `force_quit/step.rs`; Escape answers the question, then closes the panel; a press away closes it |
| `display-trial` | `sill-overlays` | Overlay | `display_trial/`: "Keep these display settings?" after a program other than the shell changed the displays (`ShellNote::DisplaysTrial` -> `DisplayTrialLink`), on quire's `Alert`, centred on the banners' output; the line under the headline counts the seconds down, Revert is the default and Escape, Keep Changes keeps; answers nothing for `QUESTION_ARM_AFTER` after it is drawn; answers through the link and `DesktopCommand::AnswerTrial` |
| `permission` | `sill-overlays` | Overlay | `permission_sheet/`: an app's yes-or-no question (`org.quire.Shell1.AskPermission` from the portal backend) on quire's `Alert`, centred on the banners' output; the app's icon, the headline, the app's name and explanation, the yes button (default) and the no button (Escape); answers nothing for `PERMISSION_ARM_AFTER` after it is drawn; answers through `CaptureCommand::PermissionAnswer` |
| `capture` | `sill-overlays` | Overlay | `capture_sheet/`: the screen-sharing picker, on quire's `Sheet`, on the main display (`OutputPolicy::Main`, like every sheet that asks the person); answers through `CaptureCommand` |
| `eyedropper` | `sill-overlays` | Overlay | `eyedropper/`: a still picture of each output while the colour picker asks for a colour (`EyedropperLink`); a click picks, Escape cancels |
| `lock` | `sill-overlays` | session lock | `LOCK`, a `LockSurface`; one per output through shell-host |

The wallpaper surface takes the pointer over the whole output (`RegionSpec::Whole`) and, on a
secondary press, opens the desktop's context menu (`wallpaper/menu/`: a `MenuPopup` with
Change Wallpaper…, which opens Settings at its Wallpaper page; the picture is the `wallpaper.picture` setting, which `sill::policy::wallpaper` sends the wallpaper service).
The notification center is the only place widgets are drawn; their providers live in
`sill-ui-kit::widgets`.

## 8. The daemon

`sill run` (`daemon/run.rs`) takes the single-instance lock, builds `Env` (`run_args.rs`),
starts services (`daemon/services.rs`) and settings (`daemon/settings/`), starts the IPC server
(`ipc/server.rs`), connects the host and reconciles one row of surfaces per output
(`outputs.rs`, `daemon/surfaces.rs`). `Links` is built once in `run.rs`. The `debug`
feature adds `sill debug ...` and the fake DDC flags; a build without it refuses them.

Settings have one path: the file settles in `daemon/settings/watch.rs`, is published on the
feed (surfaces read it) and followed by each `Policy` (services are told). Focus mode writes
the file and the file changes the mode; the file stays the one truth. The appearance has the same path with its own task
(`daemon/settings/appearance.rs`: `appearance.toml` and the settings portal, one watch for the
whole shell, published as `SettingsFeed::appearance`).

The person's stylesheet (`~/.config/sill/style.css`, quire's `UserStyle`, ARCHITECTURE.md
section 10 in quire) takes the same path: `daemon/settings/user_style.rs` watches it through
`ds_settings::Store`, publishes it on the feed, and `SurfaceRoot` passes it to `Ds`, so every
surface restyles live. Each surface root carries `data-surface = Surface::NAME`, which makes
the surface names part of the public selector surface: renaming one breaks the person's file
and needs a note. `sill style check` prints `ds_lint::user_stylesheet`'s notes for the file.
The person's rules are unlayered and win over both cascade layers: quire's `@layer ds`, and
`@layer app`, where every sill surface's own CSS goes (`ds::prelude::AppStyle { css }`, never a
bare `style { {CSS} }`). A user rule written `[data-surface=bar]` (plain, no `*|`) matches.

`dist/` holds what sill adds to a login: `sill.service`, `intents/org.quire.Shell.toml` (what `quire-do shell` lists and intentd gates), the PAM service, the keycap layer (the person's file: keycap's user layer refuses a rule whose trigger has no modifier or only Shift, so both Caps Lock rules are the desktop's vendor file, `/usr/lib/keycap/keycap.toml`) and the DDC/CI
rules. `dist/install.sh` (`--dry-run` prints) and `dist/uninstall.sh` put them in place and take
them out, sharing `dist/install-lib.sh`; they never touch Plasma or Toshy, and skip the DDC rules
and `sill.service`, which the desktop repo's installer puts, root-held, under `/etc/systemd/user` (the account service trusts the unit only from root-held places, and its `UnsetEnvironment` clears the loader variables). The login itself, "sill (casement)", is the desktop repo's `session/`.
`docs/first-login.md` is the user's checklist.

The daemon exits with a failure and one line when another program owns `org.quire.Shell` or
`org.quire.Confirm1` (`name_guard.rs`) or when its single-instance lock file is removed
(`door_guard.rs`); callers of the shell's bus objects are told apart from
`/etc/quire/desktop-callers.toml` (`Env::callers`, loaded once at start, an empty table when it
cannot be read: every call is then refused). A build with the `debug` feature also takes
`sill run --dev-callers FILE`: the person's own rows file (peephole's developer rows, `Proof::Dev`,
`callers/dev.rs`, feature `sill-services/dev-callers`) names the stand-ins a checkout built, for that
daemon only; a release build has no flag and no `dev-rows` feature.

The desktop's own compositor (`daemon/compositor.rs`, `daemon/targets.rs`): `ShellSeat` holds the
Quire arm when the compositor handed the shell a connection, and `ShellSeat::command` is its one
way to send a `DesktopCommand`. The docks publish their icons' resting rectangles into
`Services::dock_rects`; `targets::forward` turns that watch and the arm's `DesktopView` into host
messages (`ShellMsg::DockRects`, `ShellMsg::Desktop`); the seat keeps the compositor's minimise
targets equal to the icons by sending only the diff (`MinimiseTargets::since`), and sends them
all again when a shell surface registers. On the Wlr arm the seat is absent and
nothing is sent.

### Window models

There are two models of "the windows", from two wire clients on two Wayland connections:

| | `WindowModel` | `DesktopView` |
|---|---|---|
| Built by | the windows thread (`desktop_wl::run`) on its own connection, from the standard protocols (ext toplevel list, wlr management, ext workspace) | the Quire arm (`desktop_wl::quire::shell`) on shell-host's connection, from `quire_desktop_v1` |
| Window id | `WindowKey(u64)`, the wire client's per-connection `ToplevelKey` | `identifier: String`, the compositor's stable name |
| Space id | `WorkspaceKey(u64)` per connection, optional `stable_id` | `porter_core::SpaceId` |
| Only here | workspace capabilities, pinning, urgency, coordinates, groups, `Sizing`, `WlCommand`s | apps with proof, life, attention; role, modality, geometry, lease, sensitivity, responsiveness; Space kind and position |
| Readers | dock, bar, companion, keyboard, search, corners, workspace verbs, switcher | Spaces bar, split picker, force quit, capture window list, minimise targets |

They cannot become one store in this lane. The `WindowKey` of one connection means nothing on
the other (keys are counters per client), and `Toplevel` carries no `identifier`, the only
value both sides share; `Workspace` has no `SpaceId`, and `DesktopView` has nothing for
capabilities or pinning. `desktop_wl::view::desktop_view` (a `DesktopView` projected from the
standard client's state plus `Toplevels` recency) exists but only a test calls it. The
`DesktopView` is cloned into `ShellSeat.targets.view`, `SpacesBarFeed.view`,
`ForceQuitLink.rows` and the capture service's window list.

Proposal, in order: (1) add `identifier` to `Toplevel` (the standard client already reads
it: `desktop_wl::view::identifier`), so a window can be named in either model; (2) have the
windows thread own the Quire state too when the offer is Quire (one connection, one
`DesktopState`), and publish `DesktopView` as a projection of that state beside `WindowModel`,
so the two can no longer disagree on which window is active; (3) have readers of the clones
hold the one `watch::Receiver<DesktopView>` instead of a copy. Step 2 needs the windows thread
to use the shell-host connection or the arm to use a private one, which is the open design
question; until then a reader that needs a field only `DesktopView` has reads it, and every
other reader uses `WindowModel`.

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
- **Strong copyleft sill reads for facts only:** systemd.
- **Gate** (`~/sill-wt/sill-gate.sh`, or by hand; check every exit code):

  ```bash
  cargo fmt --all --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace --features sill/debug
  ./scripts/check-boundary.sh
  ./scripts/check-intents.sh   # every .desktop file has a valid intents manifest (docket's check)
  cargo deny check licenses bans sources
  cargo deny check advisories   # needs the advisory database: wherever the network does
  ```
