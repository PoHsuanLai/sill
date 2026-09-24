# Design map

Which design paragraph each module implements, and who fills what. The program plan
(`~/.claude/plans/vast-toasting-peach.md`, "Design: `<shell>` repo" and "Findings: COSMIC") and
quire's `design/` docs are canonical: 20-SURFACES (bar, dock, launcher, wallpaper), 21-SPACES,
22-SETTINGS (§3 shell domains, §9 schema), 10-BEHAVIOUR-dock, 11-BEHAVIOUR-scroll (sill owns only
the config), 13-BEHAVIOUR-menus-windows and 06-INTERACTIONS §20. This file only says where a
paragraph lives; `FINDINGS.md` records every place the code departs from a doc and why.

State after the W0 freeze: every module exists with its public types and signatures. **Pure**
modules are implemented and table-tested; **stub** modules have `todo!()` bodies under
`#![allow(unused_variables, dead_code)] // Freeze stubs` and, where the behaviour is easy to
state, an ignored test naming the wave that fills it (`tests/skeleton.rs`).

## Crates

| Crate | Role | Boundary (`scripts/check-boundary.sh`) |
| --- | --- | --- |
| `sill-ipc` | the IPC words: `Request`/`Response`, adjacently tagged `kind`/`v`, one JSON line each | pure: no dioxus, zbus, wayland-client, tokio, libpulse |
| `sill-launcher` | `Provider`, `Query{text, generation, cancel}`, `ResultSink`/`Batch`, `Item`, `Action`, `Activation`, `rank()`, `Frecency` | pure, same list |
| `sill-services` | every effect behind a `watch` channel: `spawn(rt) -> (Receiver<T>, Control<C>)` plus a pure `reduce_*`; `Services`; `cosmic_wl`; `match_app`, `expand_exec`, `magnify`, `launch`; the five providers | UI-free: no dioxus, blitz, shell-host, ds, ds-settings |
| `sill-settings` | `sill/settings.toml`: 11 domain structs with `#[derive(SettingsSchema)]`, lenient load, atomic save, directory watch, diff | brings dioxus through ds-settings, which is why it is not a module of sill-services |
| `sill-surfaces` | `hooks/` (one `use_*` per source) and `surfaces/{bar,wallpaper,dock,launcher}` components, plus the pure machines of design/10 and design/13 | none |
| `sill` | the binary: `main`, `cli`, `client`, `daemon`, `ipc_server`, `msg`, `outputs`, `launch` | none |

## `sill-ipc` (pure, done)

| Module | Implements |
| --- | --- |
| `request.rs` | plan CLI/IPC: `Ping`, `Launcher{Toggle,Open{query},Close}`, `Dock{Pin,Unpin}`, `Reload`, `Quit`, `Debug{Stats,Pointer,Leave,Capture,Click{surface, at, button: PointerButton}}`; `PointerButton::{Primary,Secondary,Middle}` (`Click` and `PointerButton`: M2 freeze amendment, FINDINGS F76, F80) |
| `response.rs` | `Pong`, `Done`, `Stats(Vec<SurfaceStats>)` (mirror of shell-host `FrameStats`), `Failed(Failure{kind, message})` |
| `line.rs` | newline-delimited framing, 64 KiB line cap, `read_message`/`write_message` |
| `tests/serde.rs` | every variant's pinned wire line, round trips, malformed and overlong lines refused |

## `sill-launcher` (pure, done)

| Module | Implements |
| --- | --- |
| `provider.rs`, `sink.rs`, `query.rs` | plan "Launcher v1": providers off-thread, batches stamped with a generation, stale generations cancelled (`Cancel::begin`) |
| `item.rs`, `ids.rs`, `activation.rs` | rows, actions (Ctrl+K / Alt+K, 06 §20.2), `ProviderKind` in Tab order, the closed `Activation` vocabulary |
| `matching.rs` | nucleo `Pattern` smart case, normalised to thousandths of the needle's self-score |
| `rank.rs` | `0.7 match + 0.3 frecency + prefix boost` (boost 0.1, FINDINGS F9) |
| `frecency.rs` | age buckets 100/70/50/30/10 x count, `now` an argument, `frecency.json` lenient |

## `sill-services`

| Module | Implements | State |
| --- | --- | --- |
| `units.rs`, `control.rs`, `error.rs`, `curve.rs` | shared integers (`Percent`, `Permille`, `Cpx`, `Stamp`), `Control<C>`, `ServiceError`, a CSS `cubic-bezier` evaluator (quire has none, F16) | pure, done |
| `services.rs` | plan: "`Services` = one Clone struct of receivers+controls passed via `Root::context`" | `Services::spawn` stub (M1 daemon) |
| `clock/` | hooks table `use_clock`: `format_clock` (`Thu 24 Sep 14:05`), `until_next_minute` | pure done; `spawn` stub |
| `battery/`, `network/`, `volume/` | hooks table `use_battery` (UPower), `use_network` (NetworkManager), `use_volume` (libpulse) | `reduce_*` done; `spawn` stubs |
| `tray/` | hooks table `use_tray` (system-tray SNI + dbusmenu); `TrayCommand::Scroll{item, delta: ScrollDelta{axis: ScrollAxis, steps}}` sent as `org.kde.StatusNotifierItem.Scroll` (M2 amendment, F79) | done (M1) |
| `cosmic_wl/` | plan: own Wayland connection + cctk states; `Toplevels` (MRU), `Workspaces`, `WlCommand`. M2 amendment (F76-F78): `Workspace` gains `coordinates: Vec<u32>`, `capabilities: WorkspaceCaps` (a set of `WorkspaceCap::{Activate,Deactivate,Remove,Assign,Rename,SetTiling,Pin,Move}`) and `pinning: Pinning::{Pinned,Unpinned}`; `WlCommand` gains `PinWorkspace(WorkspaceKey)`, `UnpinWorkspace(WorkspaceKey)`, `MoveToplevelToWorkspace{toplevel: WindowKey, workspace: WorkspaceKey}`; no `SetMinimizeRect` (shell-host G2 designed, not built) | done (M1) |
| `apps/` | hooks table `use_apps`; `match_app` (exact, lowercase, StartupWMClass, reverse-DNS label suffix, Flatpak id); `expand_exec` (spec quoting, `%f %F %u %U %i %c %k %%`) | `reduce_apps`, `match_app`, `expand_exec` done; scan, icons, `spawn` stubs |
| `badges/` | hooks table `use_launcher_entry`: `LauncherEntry.Update` props | `reduce_badges`, `parse_app_uri` done; `spawn` stub |
| `pins/` | design/10 §10.6 `dock.json` | `reduce_pins`, JSON done; file I/O, `spawn` stubs |
| `launch/` | plan "Launch": activation token, D-Bus activation, terminal wrapper, `systemd-run --user --scope` | `reduce_launches` done; `launch_argv`, `scope_unit`, `spawn` stubs |
| `magnify/` | design/10 §10.5.1 zoom and §10.5.2 pointer-anchored layout, `rest_centers` | pure, done (F13) |
| `results/` | the launcher's search runner: merge batches, drop stale generations, frecency file | `reduce_results` done; runner and file stubs |
| `providers/` | apps, windows, system, calculator (fend, 50 ms), settings (schemas + cosmic-settings pages) | stubs |
| `wallpaper/` | design/20 §1.4 cache `<hash>-<w>x<h>.png`, Lanczos3, light/dark pair | `cache_file_name` done; hash, `spawn` stubs |
| `sound/` | design/13 §13.3.10 UI sounds | stub (M6) |

## `sill-settings` (done)

design/22 §3.4-3.8 and §3.11-3.14, plus the three §3.1 rows quire's FINDINGS F15 gives sill.
One file per domain: `bar`, `dock` (+ `dock_words`), `launcher`, `scroll`, `scrollbar`, `menus`,
`switcher`, `sound`, `notifications`, `control_center`, `spaces`. `file.rs` is `ShellFile` with
`load` (via `ds_settings::lenient`) and atomic `save`; `watch.rs` the 30 ms debounced directory
watch; `diff.rs` §4.4's `apply`; `schema.rs` the schema `sill --write-schema <dir>` installs;
`feed.rs` the `SettingsFeed` root context. Tests: every domain round-trips, a bad value costs
its own key, unknown keys survive, a renamed write fires one change, the diff yields exactly the
changed domain, every §3 key has a `KeySpec` and every `KeySpec` a row (ds-settings' exclusion
list mirrored), §5 pages and exposures, deep links.

## `sill-surfaces`

| Module | Implements | State |
| --- | --- | --- |
| `hooks/*` | plan hooks table, one file per source; `use_settings` (`SettingsFeed`), `use_appearance` (`ds_settings::use_environment`) | done; `use_space_look` stub (quire has no `spaces.json` store, F17) |
| `measure.rs` | `pub const MEASURE: ds::HostMeasure`, the Blitz rect read quire's `use_rect` needs (Q10); the daemon puts it on every surface root (`daemon::surface_root`), a surface on every popup root it opens. Moved from `surfaces/bar/measure.rs` at the M2 amendment (F81) | done |
| `clock.rs` | `FrameClock`: `Instant` to the machines' `Stamp` | done |
| `surfaces/bar/` | design/20 §1.1; design/13 §13.3.1-13.3.4; 06 §20.3 | `Bar` and items stubs; `layer.rs`, `popup.rs` done |
| `surfaces/bar/menu_track/` | design/13 §13.4 `MenuTrack`, §13.5 safe triangle (placed in quire by the doc, F15) | pure, done |
| `surfaces/wallpaper/` | design/20 §1.4 | component stub; `layer.rs` done |
| `surfaces/dock/machine/` | design/10 §10.4 (hover, label, press/clicks/drag/spring, bounce, reveal), §10.5.3-10.5.5 | pure, done (F14) |
| `surfaces/dock/` | design/10 §10.3; 06 §20.1 | `Dock`, `Tile`, `dock_items`, `context_menu` stubs; `layer.rs`, `geometry.rs` done |
| `surfaces/launcher/` | design/20 §1.3; design/13 §13.3.9; 06 §20.2 | components and `keys::step` stubs; `layer.rs` (`panel_top`) done |
| `surfaces/switcher/machine.rs` | design/13 §13.3.5, §13.4 | pure, done (surface later) |
| `surfaces/notifications/banner.rs` | design/13 §13.3.6, §13.4, §13.5 | pure, done (surface M6) |

## `sill` (binary)

`cli.rs` (clap; `request()` done and tested), `client.rs`, `daemon.rs`, `ipc_server.rs`
(2 s reply timeout), `msg.rs` (`ShellMsg`), `outputs.rs` (`reconcile`), `launch.rs`: stubs for M1
daemon and M2 launch. Feature `debug` enables shell-host's `debug-inject` for
`sill debug pointer|leave|capture|click` (`click <surface> <x> <y> [--button
primary|secondary|middle]`, M2 amendment F80; shell-host injects primary clicks only, G10).
`daemon::surface_root` gives every spawned surface root `Services`, `SettingsFeed` and
`sill_surfaces::MEASURE` (F81). `--write-schema <dir>` writes `sill.settings.toml`.

## `dist/` and `dev/`

`dist/sill.service` (graphical-session.target, Restart=on-failure);
`dist/cosmic/com.system76.CosmicSettings.Shortcuts/v1/{system_actions,custom}` (Launcher,
AppLibrary; Ctrl+Alt+Space for nested use); `dist/sill-session` documented only (a real login
without cosmic-session; not an M4 gate). `dev/{nested,run,shot}.sh` wrap
`shell-host/scripts/dev/`; `dev/shot.sh --surface` uses `sill debug capture`;
`dev/accept-bar.sh` is the M1 skeleton (every unfilled step exits 2).

## Wave ownership (proposed)

Rules for every wave: `lib.rs`/`mod.rs` re-exports and every implemented pure module are
frozen (a signature change is stop-and-report); a filler may add private helpers and tests to
the files it owns; implemented reducers and machines may get bug fixes only from the wave that
owns their file below, with a test that fails first.

### M1 (bar)

| Agent | Owns exclusively |
| --- | --- |
| svc-dbus | `sill-services/src/{battery,network,volume}/mod.rs` (the `spawn`s) |
| svc-tray | `sill-services/src/tray/mod.rs`; `sill-surfaces/src/surfaces/bar/tray.rs` |
| svc-cosmic | `sill-services/src/cosmic_wl/{mod,state,handlers}.rs` |
| bar-ui | `sill-surfaces/src/surfaces/bar/**` minus `tray.rs`; `sill-services/src/clock/mod.rs`; `sill-surfaces/src/hooks/use_space_look.rs` |
| daemon | `crates/sill/src/**` minus `launch.rs`; `sill-services/src/services.rs`; `sill-ipc/**` (tests only: the protocol is done); `dist/**`; `dev/**` |

### M2 + M3 (wallpaper, dock)

| Agent | Owns exclusively |
| --- | --- |
| wallpaper | `sill-services/src/wallpaper/**`; `sill-surfaces/src/surfaces/wallpaper/**` |
| apps-index | `sill-services/src/apps/{mod,scan,icons}.rs` (`match_app`, `expand_exec`, `index` are done) |
| launch | `sill-services/src/launch/{mod,plan}.rs`; `sill-services/src/pins/mod.rs` (dock_config); `crates/sill/src/launch.rs` |
| badges | `sill-services/src/badges/mod.rs` |
| dock-ui | `sill-surfaces/src/surfaces/dock/**` (the machine included, for fixes) ; `dev/accept-dock.sh` (new) |

### M4 (launcher)

| Agent | Owns exclusively |
| --- | --- |
| launcher-core | `sill-launcher/**`; `sill-services/src/results/**` |
| providers-a | `sill-services/src/providers/{apps,windows,system}.rs` |
| providers-b | `sill-services/src/providers/{calculator,settings}.rs` |
| launcher-ui | `sill-surfaces/src/surfaces/launcher/**` |
| perf-ime | `dev/accept-launcher.sh` (new); latency logging in `crates/sill/src/daemon.rs` by agreement with no other M4 owner of that file |

Not in M1-M4: `sill-services/src/sound/` (M6), `surfaces/switcher/` and
`surfaces/notifications/` components (their machines are done), `sill-settings/**` (done; a
new key is a design/22 change first).
