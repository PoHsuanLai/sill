# Architecture

sill is a desktop shell for COSMIC: the bar, dock, launcher, wallpaper, notifications, control
center, lock screen and the rest, drawn with quire's design system on shell-host (Blitz on
Wayland layer surfaces). cosmic-comp is the compositor. KWin is used only as a test compositor.

## Sibling repositories

- **quire** (`../quire`, path dependencies): `ds`, the components, tokens and motion every
  surface draws with; `ds-settings`, the settings derive, lenient load and the appearance
  environment; `ds-native`, the Blitz measurer, focus and caret providers and the headless
  `Harness` every UI test runs on.
- **shell-host** (`../shell-host`): the host. It owns the Wayland connection, layer surfaces and
  popups, input, frame scheduling, regions (input and blur), the session lock, the scroll engine
  and, with the `debug-inject` feature, injected pointer events. sill hands it `Root`s (a Dioxus
  component plus root contexts) and gets `HostEvent`s back.
- **latchkey** (from mailo, git): the single-instance agent socket.

## Crates

| Crate | What it is | Boundary |
| --- | --- | --- |
| `sill-ipc` | The IPC words: `Request`/`Response`, adjacently tagged, one JSON line each, 64 KiB cap | pure |
| `sill-launcher` | Launcher core: `Provider`, `Query` with generation and cancel, `Item`, `Activation`, matching (nucleo), ranking, sections, frecency, previews | pure |
| `sill-services` | Every effect: one module per service, the launcher's providers and results runner, `cosmic_wl` | UI-free |
| `sill-settings` | `$XDG_CONFIG_HOME/sill/settings.toml`: one struct per domain with `#[derive(SettingsSchema)]`, lenient load, atomic save, the watch, the schema, `SettingsFeed` | brings dioxus (through ds-settings) |
| `sill-surfaces` | The surfaces (Dioxus components), their pure machines, the hooks that read services and settings, the widgets, app icons | none |
| `sill` | The binary and its library: CLI, IPC client and server, the daemon, per-output surface bookkeeping, policy followers | none |

`scripts/check-boundary.sh` enforces the first three columns: `sill-ipc` and `sill-launcher`
reach no dioxus, zbus, wayland-client, tokio or libpulse; `sill-services` reaches no dioxus,
blitz, shell-host, ds or ds-settings.

## The service pattern

Every service in `sill-services` is a marker type implementing `service::Service`, started by one
runner:

```rust
pub trait Service: Sized + 'static {
    const NAME: &'static str;
    type State: Clone + Default + PartialEq + Send + Sync + 'static;
    type Command: Send + 'static;
    type Deps: Send + 'static; // other services' receivers and controls, () if none
    fn initial(env: &Env, deps: &Self::Deps) -> Self::State { Self::State::default() }
    fn run(env: &Env, deps: Self::Deps, io: ServiceIo<Self>)
        -> impl Future<Output = Result<(), ServiceError>> + Send + 'static;
}
pub struct Handle<S: Service> { pub state: watch::Receiver<S::State>, pub control: Control<S::Command> }
pub fn spawn<S: Service>(rt: &tokio::runtime::Handle, env: &Env, deps: S::Deps) -> Handle<S>;
```

- The marker is named after its module (`BatteryService` in `battery`, `CosmicWlService` in
  `cosmic_wl`). `State` is plain data with a default that means "absent" or "nothing yet"; a
  service whose system half is missing (no UPower, no NetworkManager, no protocol from the
  compositor) keeps publishing it. `initial` is the value published before the task runs (the
  clock's time, the dock pins read from disk; the default elsewhere).
- `run` takes from `Env` what it reaches outside the process before it returns its task, and the
  runner spawns that task. A service on its own thread starts the thread from `run` (`cosmic_wl`;
  volume's libpulse mainloop runs on the blocking pool). A task that ends in an error is logged
  once (`sill: <name>: <error>`), except a bus absent by choice (`ServiceError::NoBus`); its
  receivers keep the last value either way.
- `Deps` names the other services a service reads or drives, so the order in
  `Services::spawn` is the dependency order: displays follow the volume, launch reads the app
  index, session sends Lock to the locker, idle reads the lock and the battery and drives the
  locker and auth (`IdleLinks`), results reads apps, `cosmic_wl` and the clipboard and sends to
  `cosmic_wl` and launch (`results::Sources::Services`; `Sources::Providers` runs given providers
  alone, for a benchmark).
- One publisher, one state: a service with two outputs publishes them together. `cosmic_wl`
  publishes `CosmicWl { workspaces, toplevels }` (one Wayland connection and one command set;
  splitting it would need two connections), notifications publish `NotesState { live, history }`
  (one server's state, stepped together). A reader that wants one part projects it, so it wakes
  only when that part changes (`use_service_part`).
- The logic is pure: a function named `step` beside the state it steps, `step(state, event) ->
  state`, or `-> (state, effects)` where the service must act (`idle::step`, `locker::step`,
  `notifications::step`, `cosmic_wl::toplevels::step`, ...), table-tested. The `live.rs` or
  `run.rs` beside it does the I/O: it seeds, follows D-Bus signals or Wayland events, feeds
  `step`, publishes with `send_if_modified` and carries out the effects.
- A service reaches nothing outside the process except through `sill_services::env::Env`: the
  buses (`Buses`, each a `BusRoute`: `Usual`, `Address`, `Absent`), the compositor
  (`WaylandRoute`), the person's directories (`Dirs`), the wall clock (`WallClock`), the DDC buses
  (`DdcBus`), where session actions go (`SessionActions`), the lock screen's password check
  (`Authenticator`), idle's times and the polkit agent's route (`PolkitRoute`). The daemon builds
  `Env::from_process()` (a `debug` build swaps single seams for stand-ins with `sill run` flags,
  `crates/sill/src/run_args.rs`); a live test sets the fields it needs on
  `Env::isolated(scratch)`, which reaches nothing at all.
- Services that could touch the person's session (notifications, idle, polkit agent, clipboard,
  calendar events, screenshots, displays) are passive until the daemon sends their first policy.
- `Services` (`services.rs`) is one `Clone` struct with a `Handle` per service, each field named
  after its module, built by `Services::spawn(rt, &env)` and given to every surface root as a
  context. `Member` maps a service type to its field, so `sill-surfaces` reads any service with
  `use_service::<S>()` (or `use_service_part::<S, _>(part)`) and commands it with
  `use_control::<S>()`.

The services: clock, battery, network (and Wi-Fi), volume, displays (scale policy, EDID,
DDC/CI, backlights, warmth, fontconfig), tray, `cosmic_wl` (toplevels and workspaces on its own
Wayland connection), apps (desktop entries and icons), badges (LauncherEntry), pins, launch
(scopes, activation tokens, opening files), results (the launcher's search runner over the
providers: apps, windows, system, calculator, settings, files, clipboard, emoji, web),
wallpaper, Bluetooth, power profiles, now playing (MPRIS), keyboard backlight, session
(logind), locker, auth (PAM through `sill auth-check`), polkit agent, notifications
(`org.freedesktop.Notifications` server and history), events (calendar), screenshots,
clipboard history, idle. `switcher.rs` and `magnify/` are pure helpers without a task.

## Settings

`sill-settings` holds one module per domain (bar, dock, launcher, scroll, scrollbar, menus,
switcher, notifications, control_center, spaces, display, osd, power_menu, session, idle,
widgets, calendar, screenshot, hot_corners, window, focus), each a struct whose paths are
`<domain>.<field>` and whose rows live in quire's design/22. `ShellFile` is the whole file.
`watch` re-reads it on change (debounced); the daemon publishes each settled file on the
`SettingsFeed`, which every surface root carries and every policy follower reads. `sill
--write-schema <dir>` writes the schema quire's settings tools read.

## Surfaces

Each surface is a component in `sill-surfaces/src/surfaces/`, with its layer configuration
(`layer.rs`), its pure machines (table-tested) and its view. Most are one warm surface per
output, spawned hidden at start and shown with `set_visibility`.

| Surface | Layer | Notes |
| --- | --- | --- |
| `bar` | Top | Menus (app, status items, tray's dbusmenu) in one reusable popup; menu tracking machine |
| `control_center` | (bar popup) | Module grid: Wi-Fi, Bluetooth, Focus, Display, Keyboard Brightness, Sound, Now Playing, Appearance, Battery; hung from the output's right edge |
| `dock` | Bottom | Magnification, the dock machine, context menu popup, badges, running dots |
| `wallpaper` | Background | Cover-fit cached image, light/dark cross-fade |
| `launcher` | Overlay | Panel plus a click catcher; sections, preview pane, emoji grid, actions menu |
| `osd` | Overlay | Volume and brightness level for sill's own changes |
| `power_menu` | Overlay | Centred sheet over a scrim; the confirmation for Log Out, Restart, Shut Down |
| `banners` | Overlay | Notification banners, one column on the main output |
| `notification_center` | Overlay | Right-edge column: history grouped by app, then widgets |
| `notification_card` | (component) | One card for banners and center rows |
| `screenshot_thumb` | Overlay | Floating thumbnail after a screenshot; drag out |
| `hot_corners` | Overlay | 2 px squares per active corner |
| `desktop_widgets` | Bottom | Widget grid with drag, per-output placements |
| `edit_widgets` | Overlay | Widget gallery sheet |
| `switcher` | Overlay | Cmd+Tab app switcher |
| `polkit_prompt` | Overlay | sill's polkit agent's prompt |
| `lock` | session lock | One lock surface per output through shell-host's session lock |
| `idle_dim` | Overlay | The dim before the displays go off |

`widgets/` holds the widget providers (calendar month, Up Next, batteries, Now Playing, world
clock) drawn in the notification center and on the desktop. `hooks/` has `use_service`,
`use_service_part` and `use_control` for every service, and one hook per other feed (settings,
appearance, focus mode). `app_icons/` resolves and re-tints app icons into a file cache.

## The daemon

`sill run` (`crates/sill/src/daemon.rs`) takes the single-instance lock, spawns `Services` and
the IPC server, connects the host, and keeps one row of surfaces per output
(`outputs.rs`: reconcile on output events, spawn what is missing, close what was removed).
`surface_root` gives every root `Services`, `SettingsFeed`, the measurer and focus contexts,
and the links (`links.rs`: small watch channels by which the CLI, the bar and other surfaces
open the launcher, notification center, power menu, control center, switcher, edit widgets).
Modules beside it follow a service or the settings and act on the host: `displays`, `lock`,
`idle`, `idle_dim`, `notifications`, `polkit`, `screenshots`, `calendar`, `corners`,
`clipboard`, `launcher_policy`, `scroll`, `widgets`, `edit_widgets`, `power_menu`,
`switcher`. What a service is told for the settings is a `policy::Policy` (`type Command; type
Out; fn of(&self, &ShellFile) -> Out; fn command(Out) -> Command`, and `changed` for a second
source, such as notifications' focus mode); `policy::follow` sends it at start and again whenever
`of`'s answer changes. The bridges: `calendar::Bridge`, `displays::Bridge`, `idle::Bridge`,
`launcher_policy::{SearchBridge, ClipboardBridge}`, `notifications::Bridge`,
`screenshots::Bridge`.

The CLI (`cli*.rs`) sends one request per connection and prints the answer; every request but
`quit` and `ping` starts a daemon when none runs. The `debug` feature adds `sill debug ...`
(stats, captures, injected pointer, fake notifications, screenshots, corners, auth verdicts,
polkit requests, switcher, launcher keys, idle wake) and the fake DDC; a build without it
refuses them.

## Session

`dist/` holds what a real login needs: `sill-session` (a two-stage script run by cosmic-comp as
its kiosk command, starting cosmic-settings-daemon, xdg-desktop-portal-cosmic and `sill run`,
each restarted on exit), `sill-session.target`, `sill.desktop`, `sill.service`, the COSMIC
shortcut overrides (`dist/cosmic`), the PAM service (`dist/pam.d/sill`) and the DDC/CI access
rules (`dist/udev`, `dist/modules-load.d`). Nothing installs them.

## Tests and dev scripts

- Unit and table tests beside the code; integration tests per crate under `tests/`.
- UI tests render real components in `ds_native::Harness` on `Clock::Virtual` with fake
  services (`sill-surfaces/tests/support/fake.rs`) and a pinned appearance.
- `*_live.rs` tests are `#[ignore]`d and run against a private D-Bus
  (`tests/support/private_bus.rs`) or the nested compositor.
- `dev/accept-*.sh` run the real daemon on the nested cosmic-comp (`dev/nested.sh`) or, for real
  pointer and keyboard input, on shell-host's headless KWin (`*-real-input.sh`), always with a
  private bus, a fake DDC and scratch XDG dirs (`dev/common.sh`). `docs/manual-checks.md` lists
  what needs a person; `docs/cosmic-gaps.md` what COSMIC does not do yet.
