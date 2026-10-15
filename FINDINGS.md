# Findings

Reference, not history: the facts sill's code rests on (what the compositors, the system services and the
libraries were found to do), the decisions that depart from quire's design docs and why, and the
items still open. "Q" marks something wanted from quire, "G" from shell-host or from sill itself.
A settled item is deleted, not marked closed.

## Standing facts: compositors

- **casement is the product target; KWin is a test compositor only.** Headless KWin (shell-host's
  `kwin-virtual.sh` plus `fake-input.sh`) is where real pointer and keyboard input is scripted
  where casement cannot yet. Nothing in sill is specific to
  KWin: on any compositor that lacks a protocol sill asks for, the matching service stays at its
  default ("protocol absent") and one line goes to stderr.
- **The window service has its own Wayland connection** (a protocol object cannot cross
  connections, and the host's connection is the host's), a calloop loop over it and a forwarding
  thread for its `Control`. Toplevels publish on each handle's `done`, workspaces on the manager's
  `done` and when an output is added or renamed. Without `ext_foreign_toplevel_list_v1` or
  `ext_workspace_manager_v1` (or with no socket at all) both receivers keep their empty
  defaults and commands are dropped; the watch senders stay held, so receivers read empty
  values, not a closed channel.
- **One binary, the arm from the globals.** The thread lists the registry and
  `sill_services::desktop_wl::backend_of` picks, by asking `desktop_client::backend_for` of the
  offered globals: Quire (`quire_desktop_manager_v1` and the ext list) or Wlr (the wlr manager,
  or the ext list alone), both over `desktop-client`. Never a compositor's name or
  the environment. The displays use the native output management.
- **The standard protocols do not say which workspace a window is on, or which window was last
  active.** The Wlr arm keeps recency itself and puts a window on the active workspaces of the
  groups that show its outputs; `WindowView::space` is `Unknown`. It cannot place a window, move
  one to a Space, reorder or pin a workspace, or send a minimise target (the sill command carries
  no `wl_surface`): `supports(Wlr, ..)` is `No` for those and the arm drops the old commands that
  mean them. A Space's id is the compositor's workspace id as a slug, else `ws-<key>`.
- **Commands are gated on what the compositor advertised.** A command for an unknown key or a
  missing capability is dropped.
- **A locked session outlives its locker.** A compositor keeps the session locked when the lock
  client dies, until another client locks and unlocks. sill never kills a locker; on a test
  compositor `examples/session_locker.rs 0` recovers a leftover lock.
- **A headless compositor counts no synthetic input as activity**: virtual-keyboard keys never
  produce `ext_idle_notification_v1.resumed`, so `sill debug idle-wake` stands in. An `Exclusive`
  layer gets no `wl_keyboard.enter` while its window is unfocused on a host, and a popup opened
  without a real input serial gets no keyboard.
- **Harness facts.** A daemon's `agent.sock` path must fit 108 bytes, so scratch
  `XDG_RUNTIME_DIR`s live under `/tmp`.
- **Headless KWin facts.** It offers `ext_data_control_manager_v1` v1 (no wlr data control), no
  `ext-session-lock-v1` (a lock there is `Refused`), and no `zwlr_output_power_manager_v1`. Each
  `fake-input.sh` client is a new input device and the seat's capabilities are re-announced for
  it; scripts keep one client for the whole run. `kwin-virtual.sh stop` kills every process
  whose environment names its private bus, so a script reads `.kwin-env` without sourcing it.
- **Layer ordering.** An all-anchored Overlay with exclusive zone `Auto` sits below the bar's
  reserve and above the dock's (the notification center relies on it). Layer-shell does not order
  two surfaces on one layer, so the desktop widgets are on `Bottom`, not a second `Background`.
- **Shortcuts.** The window-switch chord is `sill switcher next|prev`. The keyboard remapper maps
  physical Cmd+Tab to Alt+Tab with `bind`, so Alt stays held for as long as Cmd is and the
  release reads as Alt going up. Super+Escape is the lock chord, `loginctl lock-session`.
- **Screenshots are not announced.** A screenshot portal's response reaches only its caller; sill
  sees every screenshot, whoever takes it, by an inotify watch of the screenshot folders (a file
  closed after writing or renamed into place, named as a screenshot).
- **The wallpaper source** is a path read by hand from one file; a source the pinned `image`
  cannot decode (JPEG XL, say) falls back to `--paper`.

## Standing facts: system services and libraries

- **UPower**: the display device's five properties, followed by property streams. Keyboard
  backlight through `org.freedesktop.UPower.KbdBacklight` (UPower finds the LED, needs no udev
  rule, and signals every change).
- **NetworkManager**: `PrimaryConnection` and `Connectivity`, plus `ActivatingConnection` and its
  `State`/`Type`, since a join in progress is usually not the primary connection. A join is
  stamped (`attempt.rs`): a start while one runs keeps its stamp, every failure mints a new one.
  An access point's own `Strength` is not followed.
- **libpulse on the standard mainloop**, on one dedicated thread polling `iterate(false)`: the
  threaded mainloop's callbacks run on a C thread and need raw pointers into a `RefCell`, which
  `unsafe_code = "deny"` rules out. Every sink or server event re-reads the default sink.
- **power-profiles-daemon**: `org.freedesktop.UPower.PowerProfiles` (ppd 0.20 and later) only.
- **BlueZ, MPRIS, power profiles**: hand-written `zbus` proxies. `bluer`, `bluez-async` and
  `mpris` all run on libdbus, a second D-Bus stack. MPRIS sends no position while a track plays,
  so the Now Playing module re-reads `Position` when it mounts.
- **StatusNotifierItem through `system-tray` 0.8.9**: items are addressed by bus destination
  alone and `Client::activate` always targets `/StatusNotifierItem`, so every `TrayItemId` is
  `<destination>/StatusNotifierItem`. The crate has no scroll or context-menu call and keeps its
  connection private, so the host opens a second session connection to send `Scroll` and
  `ContextMenu`. It reads the process's own session bus, so `crates/sill-services/tests/tray_bus.rs` runs the service as
  a child (`sill-services`' `tray_host` stand-in) whose environment alone names the private bus, beside the
  fake item `tray_item`. A dbusmenu `LayoutUpdated` for a parent other than the root replaces the
  item's whole cached menu with that subtree (the crate's own handling); apps send the root.
- **A tray item's menu is a bar menu** (`BarMenu::Tray(slot)`, the slot being the item's place in
  the tray): one popup, hover switching with the other titles, `AboutToShow` of the root sent when
  it opens, a pick sent as dbusmenu `Event "clicked"`. Its submenus are the popup `Menu`'s own, timed
  by `menus.submenu_*`; the bar's machine sees every row as a leaf. A submenu needs room beside its
  parent in the popup's document, so `PopupFrame { submenu_room }` pads the card's left by a column
  per level of nesting (the status popups hang from their right edge, so the card does not move) and
  `menu/submenu.css` lays quire's later overlays over the first, the box a submenu is placed against
  (it names quire's `.ds-overlay`, which Strict forbids: the lint test excuses those two selectors).
  `AboutToShow` is sent for the root only: a lazily filled submenu is not asked first.
- **The focused app's menus are bar menus** (`sill_services::AppMenuService`, state `AppMenus`).
  A quire app serves `com.canonical.dbusmenu` at `org.quire.AppMenu.<app_id>` (name from quire's
  `AppMenuAddress::for_app_id`, which `sill_ui_kit::app_menu::menu_name_of` hands the services; the
  services crate links no design system). Each bar tells the service its focused toplevel's `app_id`
  (`AppMenuCommand::Follow`, one follower per output); the service follows the union of those apps
  while the name has an owner (`owner::appeared`/`changed`): `GetLayout(0, -1, [])`, again after
  `LayoutUpdated` or `ItemsPropertiesUpdated`, nothing when it loses its owner (design/27 §5.2 rule c:
  the bar keeps today's App menu). The first menu of the app is the app-name menu's own rows (About,
  Settings, Quit) above the shell's rows (windows, Minimise, Hide, Hide Others, Show All; Quit only when
  the app has no menu of its own); the others are titles after the name (`BarMenu::AppMenu(i)`) of the
  same machine, so hover switching crosses them, the tray and the status items. A pick is `Event
  "clicked"`, an open is `AboutToShow` for that menu's id. `bar/dbusmenu.rs` is the one rows-to-entries
  conversion for the tray and for apps (it also draws the row's `shortcut`). One menu per app, not per
  window (the Mac's model).
- **`shell.menu.list` and `shell.menu.activate`** (`dist/intents/org.quire.Shell.toml`, served by
  `sill_services::app_menu::MenuAgent`) read the menu fresh from the focused (or named) window's app.
  The listing is one line per row, its id first, labelled `Untrusted(App(<app>))`. Everything an app
  sends (labels, titles, `x-quire-command-id`) is a claim, so the effect table is
  `sill_model::app_menu::{route_of, effect_of}`: the shell's own rows (raise or minimise a window, Hide,
  Hide Others, Show All) are `Read`; Quit is `Destructive`; a command id counts only when the app is
  `Identity::Verified` (porter's caller identity of the name's owner, from its cgroup scope through
  `porter_dbus::ProcCallers::caller_of_pid` over `Env::proc_root`, is an app whose own menu name is the
  one served) and its manifest declares that action: then the item is that action, performed through
  Intents1 as the shell with the call's own origin, and takes the action's declared effect. Every other
  item, whatever its title ("View", "Window") or id, is a `Destructive` dbusmenu click. A person's own
  click in the bar is unaffected. Tests: `crates/sill-model/src/app_menu/tests.rs` (the tables),
  `crates/sill-services/tests/app_menu_bus.rs` (private bus, `sill-testkit`'s `app_menu_app`, a
  recording intentd).
- **`com.canonical.Unity.LauncherEntry`** is a broadcast with no well-known name: a raw
  `MatchRule` through `MessageStream::for_match_rule`. Values are decoded per key; a wrong-typed
  key is dropped, not the message. A clear is a `NameOwnerChanged` with an empty new owner for a
  tracked sender.
- **logind**: availability from `CanSuspend`/`CanReboot`/`CanPowerOff`; zbus is built without
  `blocking-api`, so a sync provider bridges with `Handle::block_on` inside `spawn_blocking`.
  logind waits only for delay inhibitors held when a sleep begins, so the locker holds its sleep
  delay inhibitor at all times in a followed session, not from `PrepareForSleep(true)`.
  `BlockInhibited` is not signalled; the idle service polls it.
- **Notifications**: the name is asked for with `AllowReplacement | DoNotQueue`, never
  `ReplaceExisting`, so another daemon keeps it and is reported. Until the daemon's first policy
  the service is passive. A stock `session.conf` bus activates another desktop's notification
  daemon on a `Notify` nobody answers; `dev/private-bus.conf` is `session.conf` without service
  directories, and every live test and script uses it.
- **polkit**: an agent never checks a password. It hands the cookie to
  `polkit-agent-helper-1` (setuid, `/usr/lib/polkit-1/` on Fedora; or the socket-activated
  helper on polkit 124+ when enabled), which runs PAM and answers polkitd itself. The line
  protocol: `PAM_PROMPT_ECHO_OFF|ON <prompt>`, `PAM_ERROR_MSG`, `PAM_TEXT_INFO`, then `SUCCESS`
  or `FAILURE`. Registration is per logind session, so sill's agent registers only where no
  other agent holds the session.
- **PAM without root**: `pam_unix` in a non-root process calls the setuid `unix_chkpwd`. sill runs
  PAM in a `sill auth-check` child per attempt (the `nonstick` crate: pre-generated bindings, no
  libclang), the secret NUL-terminated on its stdin, every copy zeroed, killed after 15 s. The
  service is `sill` when `/etc/pam.d/sill` exists (`dist/pam.d/sill`), else `login`. `pam` and
  `pam-client` need bindgen and libclang at build time.
- **Clipboard**: `ext_data_control_v1` (else wlr), selection only, never primary. An offer
  carrying a secret hint (`x-kde-passwordManagerHint`, `org.nspasteboard.*Type`) is skipped
  before any of it is read. Reads go through a pipe on the loop (2 s limit), text 1 MiB, images
  up to `launcher.clipboard_image_max_mb`. Binding the device sends the current selection, which
  is kept (Q320). Nothing is written to disk or logged but counts and fault kinds.
- **Files**: `auto` asks a running Tracker (`org.freedesktop.Tracker3.Miner.Files`, SPARQL over
  `Endpoint.Query` with a pipe, `GRAPH tracker:FileSystem` and `DISTINCT`), else `fd` (or
  `fdfind`), else `plocate`; it never activates a dormant indexer. Programs run with no shell,
  the text one argument after `--`, killed at the budget. `plocate --limit` counts matches
  outside the home folder; `fd` honours `.gitignore`. Names match without case.
- **Opening and launching**: `systemd-run --user --scope` when
  `$XDG_RUNTIME_DIR/systemd/private` exists, else the bare command in its own process group
  (`systemd-run` without a reachable manager exits 1, like the program would). The activation
  token goes in `XDG_ACTIVATION_TOKEN` and `DESKTOP_STARTUP_ID`. A reveal calls
  `FileManager1.ShowItems` when the name is owned or activatable. URIs are percent-encoded as
  GLib writes them. Unit names escape as `systemd-escape` does (a literal `-` included).
  `Terminal=true` runs `$TERMINAL`, else ghostty, wezterm, foot, wrapped `-e`.
- **Desktop entries**: `OnlyShowIn`/`NotShowIn` are matched against `quire`, not the live
  `XDG_CURRENT_DESKTOP`. `TryExec` that resolves nowhere drops the entry. Flatpak's export
  directories are added even when `XDG_DATA_DIRS` lacks them. An absolute `Icon=` is taken on
  trust. The SVG-over-largest-PNG preference is a heuristic over `freedesktop_icons`' first
  matching directory.
- **Calendar**: `.ics` files and vdir directories, parsed by hand (unfolding, VEVENT, DATE/UTC/
  TZID/floating, DURATION, EXDATE, RECURRENCE-ID; RRULE FREQ daily-yearly, INTERVAL, COUNT,
  UNTIL, BYDAY with ordinals, BYMONTHDAY, BYMONTH, WKST; anything else shows once and logs
  once). `icalendar` and `rrule` bring chrono and a second zone database.
- **Currency**: the ECB's daily file, fetched with `curl` at most once a day (an hour after a
  failure) into the cache, only while `launcher.currency = "ecb"` and only when the text could
  name a currency; never on a keystroke's path.
- **Emoji**: the `emojis` crate joined with CLDR 48.2's English annotations
  (`data/emoji-keywords.tsv`, from `scripts/gen-emoji-keywords.sh`). An emoji matches when every
  typed word begins a word of its name, keywords or shortcodes. A 300-cell grid scrolls inside
  a 60 Hz frame at p95 on vello_cpu and vello_hybrid at 100-200 %.

- **keycap**: the daemon's socket is `SOCK_SEQPACKET` (`keycap-protocol`), held by one thread
  (`poll` on the socket and an eventfd), reconnecting after 1 s doubling to 30 s. Each connection
  starts with the current `Focus` and the text of `~/.config/keycap/keycap.toml`, whatever was sent
  before. Unit state comes from systemd's `GetUnit` and `ActiveState` on the system bus after the
  socket has been down 3 s, then every 5 s: `GetUnit` answers `NoSuchUnit` for an installed unit
  that is not loaded (stopped and collected), which reads as `NotInstalled`, and that is as
  silent as `Unknown`. A stopped unit is silent on purpose; only a unit that gave up (five
  failures in 30 s) is `Failed`. keycap's notices carry no action, so they point at `sill keycap retry` and
  `sill keycap resume`.
  `keyboard.overrides` (design/22 §3.25) is a list of `{ app, profile }` tables, which needs
  quire's `ds-settings` list-of-tables keys (`SettingsRow`, quire PR #2): sill builds against
  quire's `claude/settings-rows` branch until that PR merges. A row whose `profile` is not a
  profile name or whose `app` is empty is dropped, a row that is not a table of two texts is
  dropped on load, each costs only itself and is logged as `sill: settings.toml: ...`, and when
  two rows name one app the last wins. The settings `Policy` sends `KeycapCommand::Overrides`
  and the service re-chooses the focused window's profile.

- **accounts**: `sill_services::accounts` follows accountd (`org.quire.Accounts1`, session bus) through
  `NameOwnerChanged`. On meeting it, it calls the shell's `Manager.NeedingReauth` (the accounts that need
  signing in again, with labels; the call also puts sill on accountd's roster, which is what receives
  unicast signals), then hears `NeedsReauth` and watches each such account's `State` until it is not
  `needs_reauth` (a `PropertiesChanged` on the `Account` object, or the object going away). The
  notice is a notification with desktop entry `sill-accounts`, never expiring, with one button "Sign in
  again" whose action key is `sign-in-again:<account object path>`: the key carries the account. The
  answer is the notifications server's own `ActionInvoked(id, key)` signal, taken only from the
  connection that owns `org.freedesktop.Notifications`, only for an account the service tracks, and only
  when `id` is a notice sill itself posted for that account: `sill_model::accounts::Posted` keeps the id
  per account (learnt from the notes when the notice first appears there, replaced by a fresh post,
  dropped on the server's `NotificationClosed` and when the account is well again). It outlives the
  notes' own copy because a pressed button leaves the notes before its `ActionInvoked` is heard; the
  flip side is that a press within microseconds of the post, before the service has read the notes, is
  not answered (a person is slower; the test waits a beat). A foreign id, a closed id, or the right id
  with another account's key does nothing, and so does another program's signal.
  The press calls `Account.Reauthenticate("", {})`; accountd asks sill's own `AccountsSheet1` to draw the sign-in. The notice is dismissed when the account is no longer
  `needs_reauth`, however that came about (the button, Settings' `accounts.<id>.reauth`, another
  device's sign-in). A notice already held (banner or history) is replaced, not doubled. An accountd with
  no `NeedingReauth` lists nothing and is followed by its signal alone. Test: `accounts_bus.rs`.

- **accounts sheet**: `sill_services::accounts::sheet` serves `org.quire.AccountsSheet1` (`/org/quire/AccountsSheet1`) on the
  session bus once the daemon says to serve (passive before, like intents); `Open(handle, parent_window, view)`,
  `Update(handle, view)` and `Close(handle)` take a `SheetView` as JSON (at most 64 KiB; a handle is 1-64 of
  `[A-Za-z0-9_.-]`; at most 8 sheets, one per handle). Only the owner of `org.quire.Accounts1` may call them, and `Update` and
  `Close` only from the connection that opened the handle; any other caller gets `AccessDenied` and nothing is
  drawn. `Input(handle, input)` is emitted to the opener's unique name, never broadcast, because it may carry a typed
  password; the service never logs an input. accountd leaving the bus takes every sheet down. The sheet is a
  `sill-overlays` surface (`accounts-sheet`, the banners' output, Overlay, keyboard exclusive, no blur): a centred
  window of its own (the window gradient, `--r-window`, `--shadow-window`) with quire's `WindowTitlebar` carrying the step's
  title, never an attached or dropping sheet. The typed values are the shell's (`sill_model::accounts::sheet::Draft`);
  a secret is emptied from the draft by the same step that sends it, and a form that comes back refused keeps the address and
  asks for the password again. "Other..." on the provider list picks the first `ProviderRow` whose `kind` is `Generic`;
  "Add Account..." on the consent alert answers `ConsentAnswer::AddAccount`. The shell opens the page of every browser step it
  is shown (accountd opens none), and "Open Again" sends `SheetInput::OpenAgain`: accountd shows the same `BrowserWait`
  again, which opens the page again, through the launch service. Every step, the consent question included, is a centred
  titled window (`ConsentBody` and the bare steps with `StepTitle::Host`); never `AccountSheet`. Test: `accounts_sheet_bus.rs`, `tests/accounts_sheet.rs`, `dev/accept/accounts.sh`.

## Standing facts: displays

- **Scale policy**: density along the diagonal from the EDID's preferred-timing millimetres (else
  the base block's centimetres, else the compositor's size) over `display.density_target_ppi`
  (109). `PreferWhole` (default): from 1.5 up, round half-up to a whole scale if that leaves at
  least 1280 logical px across, else the nearest of 1, 1.25, 1.5, 1.75, 2, 2.5, 3. No usable size
  (0 mm, or outside 40-600 ppi): 2 at 3840 px wide and over, else 1. A 27" 4K is 2, a 32" 4K
  1.25, a 13.5" 2256 x 1504 1.75.
- **Applied once per identity**, on first sight (`$XDG_STATE_HOME/sill/displays.json`, the key an
  FNV-1a of make, product and serial; the serial is never stored in clear), through
  the native output management. A recorded
  display is never rescaled; `display.scale_overrides` applies a row when it is added or edited.
  A failed set records nothing.
- **EDID** is parsed by hand (a fixed layout). Gamut: Display P3 when the panel covers 90 % of P3
  and is at most 1.15 x its area; Wide at 1.2 x sRGB or more; else sRGB.
  `display.color_management` has nothing to hand the gamut to until the compositor has
  `wp_color_manager_v1`.
- **DDC/CI** (`ddc` 0.2, `ddc-i2c` with `with-linux`, no libudev): monitors are joined to
  connectors by EDID over every `/dev/i2c-N` whose adapter hangs off a PCI display controller
  (NVIDIA has no `ddc` link in sysfs). The bus is lossy: about one transaction in 25 fails, and a
  write after a failed read can be dropped with no error, so reads retry three times and every
  write is verified by reading it back (up to three times). A probe takes 160-180 ms, a read
  about 100 ms, a verified write 155-350 ms. One worker thread owns every bus; writes debounce
  per display and feature 120 ms after the last value, and at most 250 ms after the first one
  they replaced, so a drag writes during the drag. While the control center's Display module is
  open the service re-probes (or re-reads) every 2 s, so a monitor that missed the first probe
  comes alive.
- **Brightness routing**: eDP, LVDS and DSI go to a backlight through logind's
  `Session.SetBrightness`; other connectors to the DDC display on them. Keys move along 16
  steps. The capsule sets exactly the display it shows.
- **Monitor speakers**: when the default sink's ELD name matches a DDC display with VCP 0x62, a
  person's level or mute change is sent on as the same share of the monitor's range (and 0x8D).
  Software and hardware levels multiply.
- **Night warmth**: 6500-2700 K linearly in mireds, per-channel gains from a public-domain
  blackbody fit; each display's own gains are restored when it is turned off. The originals are
  in memory only: a crash while warm leaves the monitor warm.
- **Font rendering**: greyscale, `rgba none`, `hintslight` under 150 ppi and `hintnone` from 150,
  written to `$XDG_CONFIG_HOME/fontconfig/conf.d/50-sill-rendering.conf` only under `auto` and
  only when no other file sets those properties; a same-named file without sill's header is never
  touched.
- **Permissions**: Fedora's `ddcutil` package tags VGA-class i2c buses `uaccess`. Without it,
  `dist/udev/60-sill-ddc.rules` (any `0x03????` class, which also covers the `0x0302xx` discrete
  GPU of hybrid laptops) and `dist/modules-load.d/sill-ddc.conf` do the same. `uaccess` reaches
  only the active local session.
- **The fake DDC** (`env::ddc::FileBus`): a `debug` build's `sill run
  --fake-ddc FILE` puts it in the daemon's `Env` in place of the kernel's buses, so the worker
  talks to monitors in a JSON file, never `/dev/i2c-*`. A build without `debug` takes no such
  flag.

## Standing facts: shell-host, Blitz and the harness

- **A `.ds` root is as tall as its content**, and neither `#main` nor `.ds` has a height: a root
  holding only floating content is 0 px tall, so nothing in it is hit-tested. A full-surface root
  is `RootExtent::Viewport`; a popup's card lies in flow in its frame (the next fact). Taffy places
  a `position:fixed` box against its parent, not the viewport. An empty absolutely placed box
  with `inset:0` gets no hit area; size it by the viewport.
- **A popup is `PopupSize::FitContent`, measured through its frame.** shell-host measures the
  first content child of `#main` and the far edges of every absolutely positioned box beneath
  it (shell-host F38). quire lays a floating card absolutely in `.ds-overlay`, which fills the
  root: measured as it is, the popup would be the whole surface, and shrunk to nothing every
  press misses (Blitz hit-tests an overlay by its own box, so the overlay must cover the card).
  `menu/style.css` therefore puts one absolutely placed frame at the surface's origin, padded by
  the popup's shadow room, and quire's `RootExtent::Popup` lays the overlay, its outside catcher
  and the card (not a submenu) in flow inside it: the root is the card's size, the popup is the
  card plus its room, and a press reaches it. A press in the transparent room no longer closes the
  menu (the catcher is 0 px tall); one outside the popup is the compositor's grab.
- **A popup is its own `VirtualDom`** and inherits no context from the surface that opened it; it
  calls `ds_blitz::provide_host()` itself, as `mount` and `mount_lock` do for a surface; its `PopupFrame` provides the seat's `HostSignals`. The screenshot drag icon is the one `Ds` root outside them: it is a picture that takes no input and has no focusable child, so modality and scale do not apply.
- **shell-host unmaps by destroying and rebuilding the layer surface**, so the presenter is
  recreated on every show. Warm surfaces are spawned hidden at start and shown with
  `set_visibility`.
- **Element regions** (`RegionSpec::Elements`) are resolved after every layout and follow CSS
  transforms; one `RegionSpec::Element` keeps its last rect once its element is gone.
- **Tokio**: services spawn on the `Handle` they are given, never with a bare `tokio::spawn`;
  the settings watch runs on a `ds_blitz::TokioSpawner` over the daemon's runtime.
- **Every Harness test runs on `Clock::Virtual`** and pins `PinnedAppearance`: quire's live
  `use_environment` reads the settings portal and `appearance.toml` off-thread on wall time and
  wakes a settled document with no change (Q455). The bar clock is a `watch` fed by
  sill-services' ticker, not a ds timer, and must stay one (a re-armed `ds::sleep` would chase
  the virtual drain forever).
- **The shell renders on the compositor's GPU** (shell-host F80). With `HostConfig::adapter` at
  `Auto`, shell-host reads `main_device` from the compositor's default dmabuf feedback, maps it
  to PCI ids through sysfs and ranks the wgpu adapter with those ids first; sill sets nothing.
  `WGPU_ADAPTER_NAME` still wins. The chosen adapter is on stderr (`shell-host: adapter ...`),
  so the daemon log (and the journal) says which GPU the shell took.
- **G295: Blitz keeps a cancelled CSS animation's last value.** When `animation-name` goes away
  mid-animation, the cancelling restyle cascades the value at that moment and nothing restyles
  the node again (stylo sets `ElementAnimationSet::dirty`, Blitz never reads it). The stale
  transform is painted and hit-tested; `get_client_bounding_rect` reads the resting box.
  `tests/cancelled_animation.rs` is ignored until quire's Blitz pin restyles dirty sets before
  dropping them.

## Standing facts: tests never touch the real system

- Bus tests (`*_bus.rs`) run in the gate; only tests that need a compositor or an external
  tool are `#[ignore]`d. Every test that speaks D-Bus goes through
  `sill-testkit/src/private_bus.rs` (`dev/private-bus.conf`). Every other test that starts
  services does so on `Env::isolated` (no bus, no compositor, scratch directories).
- Every `dev/accept/*.sh` that starts a daemon gives it a private bus on both
  `DBUS_SESSION_BUS_ADDRESS` and `DBUS_SYSTEM_BUS_ADDRESS` (`dev/lib/bus.sh`:
  `PRIVATE_BOTH_BUSES`, `start_private_bus`, guarded by `require_private_system_bus`), a fake
  DDC (`use_fake_ddc`, `require_fake_ddc`), scratch `HOME` and XDG dirs (`scratch_new`) and its
  own `XDG_RUNTIME_DIR`.
  `dev/run.sh` is the interactive launcher and reaches the real session on purpose.
- Residual: `dev/accept/lock.sh` step 8 runs a build without `debug`, which has no fake DDC, so that
  one daemon start probes the real `/dev/i2c-*` (read only) for about a second. The lock
  screen shows the real account's name from `/etc/passwd` (by uid), which no environment
  variable sandboxes.

## Standing facts: the seat at intentd

- **One connection owns three roles.** intentd names a caller's role from the well-known names its
  connection owns (`dist/intentd.toml`: `org.quire.Shell` is `launcher`, `confirm` and `control`;
  `Confirm1`'s owner is trusted only while it also owns that name). So `sill_services::intents`
  claims `org.quire.Shell` first (the provider, `docket_client::serve_on`), then
  `org.quire.Confirm1`, and makes its `Control` calls from the same connection. It is passive until
  the daemon's `IntentsCommand::Serve`, like the polkit agent.
- **Phase A, not unforgeable.** The sheet is a layer surface on a stock compositor; its receipt says
  `InputProof::SheetFallback`, never `HardwareSeat`. Anything that can type into the session
  (uinput, an injected key) can answer it. Phase B is the same surface on the compositor's
  trusted surface ("What casement still has to provide" below).
- **The answer is a signal to the asker alone.** `Confirm` returns `/org/quire/Confirm1/request/<n>`
  and the person's answer goes back as `org.quire.Intents1.Request.Response(0, <ConfirmAnswer
  JSON>)` to the caller's unique name (a withdrawn question: code 1). Only the owner of
  `org.quire.Intents1` may call `Confirm`, `Cancel` or the provider, and only a question's own
  asker may close its request object. A question expires on its own clock (`expires`), is
  dropped when intentd leaves the bus, and is never an allow.
- **The sheet arms for 500 ms and a destructive action is a 1 s hold**, both constants in
  `crates/sill-overlays/src/confirm_sheet/gate.rs` until design/22 has `confirm.arm_ms` and `confirm.hold_ms`. Return and
  every button are ignored while it arms; Escape denies at any time; the keyboard lands on Allow
  for a press and on Don't Allow when Allow needs a hold.
- **The shell's actions use `read` for what only changes the view** (the launcher, a workspace),
  `undoable_write` with a token for a dock pin and unpin, and `destructive` for the lock and a
  screenshot: the effect classes were made for apps with data and have no word for either. The
  parsed manifest is held to the model by `crates/sill-services/tests/shell_manifest.rs`.
- **"Resume the terminal" calls `Control.Resume(Only(desktop))`**, the terminal's Space, so it
  also lifts a halt of that Space; a global halt (`all`) stays. The Terminal module draws only
  while there is a grant or a pause to show.
- **Checkouts.** docket's own path dependencies resolve beside it, so every checkout of sill also
  needs `../porter`, `../stoker` and `../almanac` (and `../docket`); `~/sill-wt/gate` needs the
  same links as its other siblings.

## Standing facts: the companion

- **Everything said to intentd and the companion goes out on the shell's own connection.** intentd
  names a caller's role from the names its connection owns, so a turn (`Session.Turn`), a search,
  an undo or a journal read from any other connection would not be the launcher's. The intents
  service claims `org.quire.Shell`, then publishes that connection on `sill_services::seat`; the
  companion service follows it (and every follower it starts goes when the seat is given up).
  Without the seat the machines still run (a summon opens the launcher) and a lookup answers
  `Failed`.
- **The two machines run inside the companion service**, on quire's `Machine` (`Tap`, `Summon` in
  `sill_model::companion`), stamped in milliseconds from the service's start and woken by one timer
  for the earlier of their deadlines; at rest there is none. A tap or a summon first finds out
  whether the focused app owns its bus name, in order, so two quick taps stay two taps. The
  launcher effects a summon asks for leave on a channel to the daemon (`ShellMsg::Summon`), and the
  panel closing is told back by a watcher of the launcher link.
- **An answer is a card in the pane, and the pane follows the selected row.** In companion mode the
  panel starts on the Companion filter (the Ask row, the answers, recent activity; Tab goes round
  Companion, Runs, Memory and never to "every provider"); an answer or run row on the selection is
  the pane without Space. Enter on an answer is its card's default button and on a row with none
  leaves the panel as it is (`Mode(Companion)` is the no-op activation), so only Promote closes.
  The card is drawn without its buttons: the pane lists the row's actions, with their keys.
- **Lookups wait on state.** Things and Memory are asked by a command to the service, which
  publishes the answer under the query's generation; the provider's `query` blocks on that
  generation (or a newer one replacing it) and never sleeps. Late hits (`Intents1.Search.Hits`)
  extend the same generation's list.
- **A card press is `Answer.Act`, and what it comes to is the answer's plan.** `Act` answers at
  once with the request's path; the answer then follows with `Updated`: the body becomes a plan with
  one step per pressed card, the phase `NeedsYou(Confirm(id))` while the shell's own sheet is up
  (the pane is `Pane::Confirming`, the row says "Needs your confirmation", the orb waits), then
  `Done`, also when the step failed (the row says "A step failed"; the plan shows the refusal).
  The card is busy (`Busy::Acting { card, step }`: no button on the row) from the press until its
  step is neither pending nor running, or the task fails or stops; a press `Act` refuses
  (`Busy::Refused`) is said and offers the button again. The button's `ActionRef` names its step.
- **A handle in an answer is shown through `Intents1.Session.Display`.** The service asks once per
  handle of an answer it asked for (it knows the session), keeps the text in `AnswerFacts::shown`
  and the card and the row draw it; it is state for the screen only and nothing sends it to a
  model. An answer the shell did not ask for has no session here, so its handles read as an
  ellipsis; a task that has ended has closed its router session, and a handle asked for after
  that stays an ellipsis too.
- **A thing's Enter needs its app's `<kind>.open`.** The service reads `Intents1.Registry` (and
  again on `ManifestChanged`) into `Companion::openers` (`openers_of`: each declared entity kind
  whose app declares `docket_core::open_name`); the things provider performs the open call as the
  launcher with no session and no window, and a thing whose app declares none has no Enter action.
- **A run is named by cuad's `Id` property**, read when its object is added; each path is kept with
  its id so `RunRemoved`, which comes after the object is gone, finds it.
- **`companion.*` is a settings domain** (`sill_settings::CompanionSettings`, design/22 section
  3.30): `double_tap_ms` and `app_wait_ms` (`Ms`, held to 150..=600 and 40..=500) and `in_field`
  reach the service as `CompanionParams` (`CompanionBridge`), whose times are `sill_model::units::Ms`.

## Decisions in force (departures from the design docs)

### Settings

- **`sill-settings` is pure data**: the domain structs, `Section`, `ShellFile` as a
  `ds_settings::SettingsDoc`, and the schema. The daemon reads, saves, watches and publishes the
  file (`sill::daemon::settings`, over `ds_settings::Store`); surfaces receive it through a second
  root context, `SettingsFeed` (`sill-ui-kit`), and read one domain with `use_section`.
- **One-variant enums cannot be settings** (the derive refuses them): `launcher.material` and
  `power_menu.material` are `PanelMaterial::{Sheet, Popover}`.
- **Retired keys**: `control_center.material`, `dock.position`, `session.user_picture`,
  `spaces.lookup_order`, `notifications.banner_entry_direction` and, with step 4a.8, `bar.title_hit_height_px` (a
  bar item's hit box is the item's own), `dock.progress_style`,
  `dock.badge_size_px`, `dock.badge_radius_px`, `dock.progress_ring_diameter_px` and
  `dock.progress_ring_stroke_px` had no reader (quire's `DockTile` draws its own `Badge` and progress
  bar), so the fields are gone and design/22's rows say `RETIRING`; a file that still sets one is
  told the key is unknown. `dock.hold_to_menu_ms` and `notifications.banner_hold_ms` are read (the
  dock machine's press, the banner hold) and stay.
- **`screenshot.tool` is typed at load** (`ShotTool`, stored as its text). `calendar.sources`,
  `screenshot.folders`, `display.scale_overrides` and `widgets.world_clocks` stay text: their
  parse needs `$HOME` (`~/`) or belongs to the reader, and a typed form would rewrite the
  person's text on save.
- **Numbers the docs leave open**: the rank's prefix boost 0.1; frecency buckets at 4/14/31/90
  days (Firefox's); the clock `Thu 24 Sep 14:05`, 24-hour; the banner hold 5200 ms (the key over
  design/13's 5000).
- **Deep links need unique wording**: `deep_link` resolves only when exactly one key's label or
  help mentions the query.
- **Unknown keys in `settings.toml`** are reported as a load diagnostic and not kept: each is one
  line on the daemon's stderr (`sill: settings.toml: unknown key ...`), on every read, and the next
  save drops it. A value that does not parse costs only its own key; a file that is not TOML at
  all keeps the last good file in the watch and is reported invalid.
- **Places are state, not settings**: dock pins (`$XDG_CONFIG_HOME/sill/dock.json`) and, per
  output connector, the desktop's widget list and cells (`$XDG_STATE_HOME/sill/desktop-widgets.json`:
  `{"outputs":[{"output":"DP-1","widgets":[...],"places":[...]}]}`). `widgets.desktop_widgets` is
  the default list: an output has a `widgets` list of its own once Edit Widgets has edited it
  there, and follows the setting until then: each monitor edits its own layout, which design/28 4.1
  (membership global) does not say. The center's `widgets.center` stays one list.

### Surfaces

- **Magnify** (design/10 §10.5.2): slots are all `T + g`, the pointer is clamped to the
  outermost slot edges, and a wider gap (the Trash separator) keeps its rest width; geometry is
  `Cpx`, hundredths of a pixel. A tile's id is an FNV-1a of what it stands for, so a bounce stays
  on its app.
- **The control center is a bar menu** (design/13 §13.3.7 has it join the bar's hover switch),
  drawn in the bar's one popup and hung from the output's right edge. `sill control-center
  toggle` writes a link the bar follows, since an IPC request has no input serial for a grab.
- **The notification center is a layer, not a popup**: a full-height column at the right edge,
  as macOS's; its transparent rest is the click-outside catcher. The clock opens it. Banners
  show on one output (the main display's stand-in).
- **Notifications**: a timed-out banner stays in the history and emits nothing; a swipe keeps it
  in the center (`notifications.swipe = "keep_in_center"`, as macOS); the close button, Clear
  and `CloseNotification` are dismissals. A click with no `default` action dismisses. A
  notification never wakes sleeping displays (and there is no key for it).
- **The launcher's session rows**: Lock and Suspend run at once; Log Out, Restart and Shut Down
  open the power menu on that button, which is the confirmation: quire's `Alert` about that action, that button first and the default (Return), the other actions logind offers after it, then Cancel; an action logind refuses is left out, not drawn disabled, and there are no arrow keys (Tab and Shift+Tab move, Space presses, Escape cancels).
- **The calendar is a widget** (the clock opens the notification center, not a date popup).
- **Widgets have one size per host**: Medium for every kind in the center; on the desktop the
  month and the batteries Small, the rest Medium. A drop lands in the cell nearest where it is
  let go, as the slot guide showed; the hand's velocity shapes the spring, not the cell.
- **Hot corners** are 2 px Overlay squares with exclusive zone `Ignore`, dwell 150 ms, re-arm
  500 ms. There is no modifier option: a keyboard-less layer has no portable source for held
  modifiers.
- **Bluetooth asks before turning off**, and "Turn Off" is the default button (Return
  confirms), not a destructive one (Q570).
- **The dim before screen off is an overlay**, never a brightness change: DDC writes are slow and
  go to NVRAM, and a crash while dimmed would leave a display dark.

### Lock and idle

- **sill's own lock screen is the only locker, and the default.** No lockout after wrong
  passwords (only PAM's own delay). A crash marker (`$XDG_RUNTIME_DIR/sill/locked`) makes a
  restarted sill lock again at once. Nested in another desktop sill follows no logind signal
  (so it never locks the host's session); in a sill session it follows `Lock` and
  `PrepareForSleep`.
- **sill owns idle in a sill session**: the dim, displays
  off through `zwlr_output_power_v1`, the lock as the displays go off (not 0.5 s after, so a
  quick wake cannot show the desktop), the lock screen dark again after
  `idle.locked_screen_off_s`, and idle suspend through logind. Times come from sill's own keys
  (a screen-off time per power source). Idle is `Off` when nested in another desktop.
- **Inhibitors**: Wayland idle inhibitors are the compositor's (sill asks
  `get_idle_notification`, which honours them); sill serves `org.freedesktop.ScreenSaver` and
  honours logind's idle inhibitors. While any is held no idle watch is held.
- **The lock grace** (`session.lock_grace_s`, 5 s) opens the lock without a password only when
  sill's idle path turned the displays off and locked, and real input came strictly less than
  the grace later, with no sleep in between, and only for sill's own lock. Every other cause
  (a manual lock, Super+Escape, a lid, a suspend, a restored or adopted lock) asks. A grace is an
  unforgeable `GraceTicket` the idle service hands the auth service; nothing on IPC carries one.

- **The screenshot thumbnail takes the keyboard only when asked** (`sill focus-latest`, an `Act`
  verb answered on `BannerLink`'s key channel). The layer is `KeyboardMode::None`; on the ask it
  becomes `Exclusive` (an `OnDemand` layer is focused only by a click, so it cannot be asked
  for), the seat modality is set to `Keyboard` so quire's ring shows before any key arrives, and
  the picture is focused by selector. Every way out of the shown state, and the compositor taking
  the keyboard away, sets `None` again, and the hold is paused while the card holds it.
  Escape dismisses like the hold running out (the file is kept); Delete and Backspace are the
  Delete button; Enter and Space are quire's picture.
- **The banners take the keyboard the same way**, one card at a time. The transients tell
  `BannerLink` when a card appears and when none is up; `sill focus-latest` asks it, and it names
  the one that appeared last (`KeyWhat::Asked(Transient)`), so only that surface answers (the
  compositor's focus loss names its surface the same way). The keyboard goes to the newest
  listed card's plate (quire's `NotificationCard` plate is a focusable button: Enter and Space
  press it, Tab walks its action buttons, which activate on Enter) and pauses that card's hold
  only (`Entry::keys`, folded with the pointer so the hold resumes when neither is on it). Up and
  Down follow the stack as drawn (newest at the top or the bottom, by `banner_position`), the ends
  do not wrap, and the thumbnail is a separate surface they never reach. Escape, Delete and
  Backspace dismiss the card and hand the keyboard to the next older card (else the newer); a
  press, a swipe or the service dropping the holder gives it back instead, as does the last one
  leaving. The notification card has no reply field; when it gets one, Tab reaches it by
  document order.

## Open items

### What casement still has to provide

Wants for the compositor, kept so casement's lanes see them. `verified` means observed or read from
a source; `assumed` means not yet checked.

| Want | Why sill needs it | Status |
| --- | --- | --- |
| Colour management: `wp_color_manager_v1`, surfaces tagged sRGB or Display P3, mapped to the panel | P3 monitors show sRGB content oversaturated; `display.color_management` has nothing to hand the gamut to | verified absent on the stock compositors tried |
| Gamma control (`zwlr_gamma_control_manager_v1`) or a colour-transform matrix | Night light in software for panels without DDC/CI (sill does it in hardware over DDC today) | verified absent on the stock compositors tried |
| A placement request for a client's own window (Left half, Right half, Centre, Fill) | The green-button menu ships Fill only; no protocol lets a client ask for a tile | verified: protocol listings |
| Tiled states sent only when a window is really tiled | A floating window marked tiled on all edges never shows its client-side resize margin | verified on a stock compositor |
| Option-drag resize, symmetric about the centre; quarter tiling by dragging to a corner | macOS behaviour sill's windows should match | assumed |
| A minimise rectangle for the dock tile, and a genie or scale animation toward it | `set_rectangle` needs the dock's `wl_surface` on shell-host's connection (G2); the animation is compositor-side | verified (G2); animation assumed |
| Toplevel urgency | A window asking for attention cannot bounce its dock tile; `LauncherEntry` `urgent` is the workaround | verified: protocol read |
| Stable workspace ids (`ext-workspace` `id`) | Spaces keep their identity across restarts | verified: stock compositors send none |
| Modifier state for an unfocused layer surface | Hot corners that need a held key, as macOS offers; asking for keyboard focus would steal it from the window | KWin sends it; others assumed not to |
| A confirmation only the person can answer: hardware input only, nothing drawn over it, no synthetic or agent input | The sheet is a layer surface any injected input can answer (`SheetFallback`); the design is `quire_trusted_surface_v1` (spec 4.2) | assumed absent elsewhere |
| A layer surface under a still pointer gets `enter` | Hover state is wrong until the pointer moves | verified on a stock compositor |
| A press on a layer surface while a popup of that layer holds the grab ends the grab (`popup_done`) and is delivered | A second popup opened with the grab still up is a protocol error (`not_the_topmost_popup`) that ends sill's connection (G368) | assumed for a real press |
| A lone modifier tap as a shortcut | Double-tap Cmd summons the companion; today a unique chord (`Ctrl+Alt+Shift+Super F19`) is sent by the keyboard remapper, and the `Tap` machine in the companion service decides which tap is the second | assumed: the remap's effect on Cmd-click and Cmd-drag is the open spike (docs/manual-checks.md "The companion") |
| Timely frame callbacks for fullscreen layer surfaces | Delayed callbacks stall the lock screen; shell-host has a watchdog | upstream reports on stock compositors |
| Background blur that also boosts saturation (about x1.8) with a tunable sigma (22 logical px for widget cards) | macOS vibrancy behind widgets and panels (quire design/23) | blur only elsewhere |
| A locker that dies while locked: policy | A compositor that keeps the lock until another client locks and unlocks is secure but strands the person; sill relocks from its crash marker | design choice for casement |
| An Inhibit portal | Flatpak apps' "keep the screen on" | casement's portal at P2 |
| Synthetic input that counts as idle activity | So an idle test can wake the seat (`sill debug idle-wake` stands in) | test harness only |
| A virtual-keyboard path that reaches clients in a headless session | Key tests on the headless compositor | test harness only |

- **Surfaces read the windows model, not `DesktopView`.** The Wlr arm publishes through the compositor
  service's existing state (`sill_model::windows`), so the dock, switcher, bar, corners and keycap's focus
  feed run on casement unchanged; `desktop_wl::desktop_view` builds the frozen `DesktopView` from
  the same state and nothing reads it yet. Ends when the surfaces take `DesktopView` and
  `DesktopCommand`.
- **The Quire arm.** `backend_for` returns `Quire` when `quire_desktop_manager_v1` is offered and
  the thread declines it (the windows and workspaces stay empty). Ends when the arm over ext and
  `quire_desktop_v1` exists.
- **The dock's minimise target on Wlr.** `DesktopCommand::SetMinimiseTarget` carries no surface and
  the wlr request needs the dock's own `wl_surface` on shell-host's connection, so minimise
  animates to the compositor's default place. Ends when shell-host exports the surface to a client
  that can register it with `DesktopClient::register_surface`.

- **Accounts: what waits on other repos.** (1) A provider's `mark_face` cannot reach the add list: `ds_shell::ProviderEntry` has no face (`gap(quire)` at `mark_of`); quire v0.2.31's `ProviderEntry::faced` is the way in. (2) Settings is opened by activating `org.quire.Settings` (`ActivateAction("open-account", [account path])` at `/org/quire/Settings`), so it runs as `org.quire.Settings.service`; detent must serve `org.freedesktop.Application` with that action, and its desktop entry should say `DBusActivatable=true` (the launch service already activates any such entry over the bus, and Settings by name whatever its entry says). (3) The app-password hint under an iCloud, Fastmail or Yahoo password is the empty entry's hint, shortened: quire has no help line under a field for porter's full sentence.

### sill

- **anyview is taken by path.** The root `Cargo.toml` names `anyview-core`, `anyview-image`, `anyview-peek`,
  `anyview-platform` and `anyview-store` at `../anyview/crates/...`, a sibling checkout. Ends when
  anyview's integration branch is merged: the five become the git revision pinned in quire's
  `docs/workspace-deps.toml` (as pdfrum is), copied into the pinned block here.
- **The pane's peek budget is a constant.** `preview::load::PANE_BUDGET` (64 MiB read, one megapixel
  of picture, 500 ms) stands in for settings keys; anyview's `PeekBudget::time` is not enforced. Ends
  when quire's `22-SETTINGS` has `viewer.peek.*` rows and `launcher.preview_*` reads them.
- **A row offers only the shared file actions the launcher carries out.** Open (⌘O, and Enter), Reveal
  (⌘R), Copy Path (⌥⌘C) and Play are carried; Open With, Copy File (⌘C), Share, Rename, Duplicate, Move to
  Trash, Convert To and Play in Mini Window are in `anyview_core::actions_for` and not on the row, each
  waiting for what carries it (an app chooser, file offers on the clipboard service, a trash). Copy Path
  moved from ⌘C to the shared table's ⌥⌘C. Ends when `files::actions::carried` has an arm for each.
- **Opening from the pane hands over no place.** The pane shows a file's start and holds no scroll, so
  `OpenTarget::Viewer` carries no `Resume` and the viewer continues where the file was last left. The
  wire has the place for a pane that keeps one. Ends when the pane scrolls or plays.
- **The pane does not grow into the viewer's window.** The handoff opens the viewer's window beside the
  closing launcher with no shared-element animation. Ends when a window can start from a rectangle
  (the launcher passes the pane's) and quire has the motion for it.
- **The viewer is called without an activation token.** `Handoff` and `Play` take none, so the viewer's window
  depends on the compositor to raise it. Ends with anyview's matching item.
- **An audio file shows facts, not its cover or length, and the play control is an action.** The pane has
  no audio peek until anyview-media lands; "Play" in the actions sends `Play` over D-Bus and the viewer
  opens the file until its background session exists. Ends with anyview-media.
- **A PDF's first page is the one picture that is not a texture.** It comes through `PdfPage::Ready`'s
  PNG `data:` URL (quire's FINDINGS), so it is encoded and decoded once per page. Ends with quire's
  `pdf_thumb_pixels`.
- **The pane's picture is a `TextureLayer`, unseen on a real surface.** The harness's CPU painter draws
  none (a picture's pane is blank there), and no one has looked at it on a shell-host surface, whose
  renderer must hand its device to quire's `Gpu` for the texture to draw. Ends when
  `docs/manual-checks.md` "A file from the launcher in the viewer" is ticked on casement; if the picture is
  blank there, the pane needs the PNG fallback its FINDINGS names.
- **Each file peek writes the pane's stylesheet.** `anyview_peek::Card` draws its own `AppStyle`, one
  `<style>` per pane. Ends when the launcher draws `anyview_peek::STYLE` once at its root.
- **A file row's glyph and actions come from its media type, the pane from its bytes.** A row built from
  the viewer's history has its kind's commonest type (`image/png` for any raster); a file whose name lies
  gets the wrong actions and the right pane. Ends if the index's rows are sniffed.
- **G352** The control center has no sink list: the volume service follows only the default sink.
- **G361** One widget of a kind per surface: the drag, regions and state file are keyed by kind.
- **G364** World Clock builds its entry from the clock at each minute tick; it does not send
  dated timeline entries, so a city's date flips on the first tick after its midnight.
- **G365** A submenu in a `FitContent` popup opens below its parent, not beside it: quire draws a
  submenu in an overlay of its own, which a popup root lays in flow as an empty strip under the card,
  and places the submenu against that strip. The bar's popup is fixed (`PopupFrame { submenu_room }`
  and `menu/submenu.css`, see the tray fact); the dock's `Options` and the desktop menu's Show View
  Options still open below. Ends when their popups pass a room, or when quire places a popup's
  submenu against the card's overlay.
- **G366** Change Wallpaper... opens Settings (its Wallpaper page when the machine has
  that desktop entry): sill has no wallpaper picker of its own, and the choice of app is not a
  setting (`PICKERS` in `sill-widgets-ui/src/desktop/menu/model.rs`; design/22 has no row for
  one).
- **G367** A secondary press on a widget opens the desktop's menu, not the widget's own
  (design/28 4.1: Edit "<widget>", Remove, sizes).
- **G368** The menu opens only when none is up: a second popup made while one holds the grab is
  a protocol error that ends the connection. A secondary press that the compositor does not
  treat as ending the grab (KWin sends no `popup_done` for a press on the popup's own client)
  does nothing until the menu is dismissed.
- **G369** Tray menus: a radio row is drawn with the check mark a toggle has (the menu has no radio
  mark); a lazily filled submenu is not sent `AboutToShow` before it opens; an item's menu is named
  by its place in the tray, so one registered before it going away while its menu is open swaps the
  open menu to the next item's; `ItemIsMenu` is not read (an item with a dbusmenu always opens it
  on the primary press); a dbusmenu row's disposition (informative, warning, alert) is not drawn.
- **G370** App menus: the foreign Qt and GTK menus are not shown, because the compositor has no
  appmenu protocol (`org_kde_kwin_appmenu`) to hand a toplevel's address over; casement P3-MN
  (`menu_source`) adds the server side, then sill reads the address it gives per toplevel instead of
  deriving it from the `app_id`, which also makes the menu per window. Until then only apps that serve
  `org.quire.AppMenu.<app_id>` (quire apps) have menus in the bar.
- **G371** App menus: `shell.menu.activate`'s Zoom and full-screen items are not provided: the shell
  has no `WlCommand` for maximise or fullscreen, so the Read set is Minimise, Hide, Hide Others, Show
  All and raising a window. Ends when the windows model gains those commands (then two `OwnItem`s).
- **G372** (resolved) App menus: `shell.menu.activate` is declared `destructive`, now the ceiling,
  with `per_call = "classified"`, so the router asks `Classify` first (`MenuAgent::classify`, over the
  pure `route_of` and `effect_of`). The answers: `{"kind":"effect","v":"read"}` for the shell's own
  rows (Hide, Hide Others, Show All, Minimise, raising a window); `{"kind":"effect","v":"destructive"}`
  for Quit and every foreign, unverified or spoofed item; `{"kind":"delegates","v":{"app":"dev.notes",
  "name":"<command id>"}}` for a command id of an app proven by porter's caller identity and declared in
  that app's own manifest (`app` is the intents manifest app id; the dbusmenu bus name only proves the
  owner). `Perform` re-derives the class and refuses `ClassificationChanged` on a mismatch, accepting
  `destructive` always (the router's re-ask); a delegation is not done by the shell: it answers
  `Follow::Next` for exactly that action with `Origin::AppInternal`, which the router gates on its own
  (no click, no `Run.Perform`). Tests: `tests/app_menu_bus.rs` (classify table, changed-layout,
  destructive-accepted, and the real router with a cross-app delegate).
- **G373** App menus: the verified path needs the app launched in a systemd scope
  (`app-<id>-<n>.scope`, which the launcher and `sill`'s own start); an app started from a
  terminal has no scope, so its command ids are claims and its items are clicks. The app's
  `AboutToShow` is sent for each menu's own id, not for lazily filled submenus.

- **G331** The desktop grid is measured when the layer mounts; a live output resize re-places
  only on the next spawn.
- **G351** `sill idle status` counts `ScreenSaver` inhibits but does not name the apps.
- **G350** No failure reason on a failed Wi-Fi or Bluetooth row (NetworkManager's `StateReason`
  and BlueZ's error are not read).
- **G353** Two devices connecting at once share one connect stamp.
- **G301** A Bluetooth failure beside a connected device does not shake the glyph.
- MPRIS `Playback::Buffering` is never produced.
- **G400** Real Escape after a pointer press in the control center does not close it (right after
  opening it does).
- **G470** After a Clear in the notification center, the widgets slot under the list jumps up
  instead of healing.
- **G421** `calendar.week_numbers` is inert: neither compact month draws week numbers.
- The center's tiles are 336 wide (160 pt cells): `notifications.center_width_px` under 360 overflows them.
- The popup reserves Battery's height when the machine has no battery (harmless, transparent).
- `ClockRow` carries the day and offset as one string; the reference draws two lines.
- **Q320** Should the selection present when clipboard history turns on be kept? It is today.
- **Q321** `clip_items` reads the clock for its age words; it should take `now`.
- **Q322** `ClipboardReport` has no count of skipped (secret) selections.
- The switcher: keyboard focus arriving after the show delay would read a held chord as a quick
  tap.
- **G292** Timing budgets flake on a loaded debug build (launcher-ui's 100 ms, the calendar
  edit's 2 s, the banner's 300 ms); `dev/accept/notifications.sh --real-input`'s overflow step fails
  when a `sill debug notify` misses its 2 s reply under load.
- One real-clock test, `results::tests::a_newer_keystroke_cancels_the_older_query...`, can fail
  under load; a paused tokio clock would pin it.
- `dev/accept/bar.sh --real-input`'s `step_center_then_menu` failed once on a
  center/menu focus race; not yet looked at.
- **Three `companion.*` keys have no reader.** `companion.summon` (off must stop the tap and the
  summon), `companion.notify` (a notification when a task needs the person) and
  `companion.confirm_arm_ms` (the confirmation sheet's arm time) are in the domain and the schema;
  nothing reads them. Ends when the tap machine, the notifications service and the confirmation
  sheet each take theirs. `agent.undo.keep_h` is docket's file, not sill's.
- **quire's companion components are placeholders.** `AnswerCard`, `ContextChips`, `RunRow`,
  `ActivityStrip`, `MemoryTimeline` draw an empty root, and `CommandPalette` has no `mode` or
  `chips` prop, so the prompt keeps the search text as state (`Session::kept`) and shows no chip.
  Ends with quire's W1 to W3 fills; then the pane's slot draws the real card and the chip shows.
- **A thing whose app declares no `<kind>.open` has no Enter action and no dimmed look.** A launcher
  `Item` has no way to say "not openable", so the row is shown as any other and Enter does nothing
  (mailo declares `mail.thread.open` only once it adds it, docket FINDINGS f4-docket-3 section 2).
  Ends when `Item` carries a dimmed state, or the row offers another action.
- **Memory rows have no verbs.** `CompanionCommand::Memory` is accepted and does nothing: Forget
  needs the preview alert (design/32 section 4.9), Keep a `ConfirmReceipt` only the shell's own UI
  can make, Export a file handle. The timeline lists events; facts and the pending list are not on
  it yet.
- **Runs sit under their own Tab.** Design/32 section 4.4 lists active runs in the companion
  mode's own rows; here they are the Runs filter (Tab). "Watch" is the compositor's and the service
  does nothing for it.
- **Every ask is in the `desktop` Space.** `AskInput::space` carries the Space an ask is made in and
  the launcher passes `SpaceId::desktop()`, because sill has no Space store: nothing mints a
  `SpaceId` (SPEC P6), names or persists one, or maps a workspace to one. Ends when a store gives
  `focused_space(workspace)` and the panel passes its answer. The memory timeline read and the
  memory rows also say `desktop`.
- **Nothing in the launcher says something to a subagent.** `CompanionCommand::Tell` (open a session,
  record the person's turn, `Message.Send` as a request, `Companion1.Told`, close) is done and
  tested; no surface issues it, because the launcher has no roster of subagents to pick one from.
  A plan card has no Stop or Undo All button: the plan's own step machine is quire's (`plan_step`,
  unfilled).
- **The summon's origin is always the double tap**; a voice summon waits for voice.
- **No Toshy snippet ships for route B.** The session's shortcuts bind `Ctrl+Alt+Shift+Super F19` to `sill companion tap`
  (and `F20` to `summon`, route A); the remap of a lone Cmd tap to that chord is written in the keyboard
  remapper's own config until spike T says tap-versus-hold is safe on Cmd.

- **accountd does not yet serve the shell what the accounts service reads (porter ask).** sill is accountd's
  `SheetHost` and holds no grants, so today accountd sends it no `NeedsReauth`, refuses it every `Account`
  property and `Reauthenticate` (`UnknownObject`), and has no call that lists the accounts needing sign-in or
  announces an account's recovery: the accounts service meets accountd and then sees nothing. The ask is
  four small changes in `accountd` and `porter-dbus`: (1) `Manager.NeedingReauth() -> a(os)`, `SheetHost`
  only; (2) a `SheetHost` caller sees every account's properties and may call `Reauthenticate` without a
  grant; (3) `NeedsReauth` also goes to the roster's `SheetHost` connections; (4) `State` changes send
  `PropertiesChanged` (invalidating `State`) to the same. Ends when accountd serves them and the accounts
  service's local `ManagerSignIn` proxy is replaced by `porter_dbus::ManagerProxy::needing_reauth`.

- **The accounts sheet's props live in `sill-overlays`, not `sill-model`.** PLAN §2.4 and the lane brief put the
  `SheetView` to props table in `sill-model`; the props are quire's (`ds-shell::accounts::model` names `MarkProvider`
  and its own `Word` enums), and `sill-model` may reach no design system (`scripts/check-boundary.sh`). The pure half
  (what is open, what is typed, what is sent back) is `sill_model::accounts::sheet`; the table is
  `sill_overlays::accounts_sheet::props` (with `words.rs`, `events.rs`). `sill-model` takes `porter-core` for it.

### quire

- **Q3** No per-workspace look store; `use_space_look` computes the preset for the position.
- **Q62** Ctrl+K while the launcher's actions menu is open does not close it.
- **Q63** Mounting the palette at every opening costs 15-20 ms against the 100 ms warm-open
  budget; keeping it mounted is a design decision.
- **Q64** The panel redraws about every 4 ms during its entrance.
- **Q361** The compact month's today disc barely holds two Inter digits.
- **Q400** A caller that runs "Show More" itself cannot book the palette's expand motion.
- **Q410-Q413** Accent B contrast: the match mark on a selected row 4.2:1; the polkit prompt's
  "Details" 4.3:1; the compact month's label on a glass plate 3.7-3.9:1; a disabled primary
  button no longer stands out.
- **Q451** `TrackPosition` takes no report age.
- **Q455** `use_environment` sets its signal even when the value is equal.
- **Q456** A harness-level environment pin, so consumers need no `PinnedAppearance` of their own.
- **Q461** The gallery cannot say what a host refuses; **Q462** it cannot reorder.
- **Q480** `IdleDim.level` is documented as the kept brightness but drawn as the scrim's
  opacity; sill passes `100 - idle.dim_level_pct`.
- **Q570** The inline alert's scrim ignores the container's radius and its panel lets the rows
  show through.
- **Q6001** The Up Next and Now Playing widgets style their inner words in sill's own CSS (a
  weekday in the accent, a 24 px light day number, a faint "No upcoming events"): `Label` has no
  accent role and no display-light style, so those rules stay until it has.
- **Q6002** `Alert` has no disabled button and no arrow keys, so the power menu leaves out an
  action logind refuses instead of drawing it disabled, and Tab replaces the arrows.
- **Q184** Hot corners have no behaviour section in design/13.
- design/22 still shows the old defaults for `bar.title_padding_px`,
  `bar.open_title_pill_height_px`, `bar.item_radius_px`, `bar.status_gap_px`,
  `menus.highlight_radius_px` and `control_center.grid_gap_px`/`grid_padding_px`.
- The Harness's `count` misses `[aria-selected="true"] .ds-chord` (an attribute selector with a
  descendant).

- **`ConsentBody` has no `StepTitle::Host`.** The account steps take `title: StepTitle::Host` and draw no title under the
  accounts sheet's titlebar; `ConsentBody` keeps its own headline ("Photos wants to ...", `.ds-alert-title`), so the window's
  titlebar says "Account Access" instead of repeating it. Wanted: the same prop on `ConsentBody`, so the titlebar can carry the
  headline. (`ConsentAsk.app.isolation`, "unsandboxed" per design/31 R12, also has no prop on it.)

### shell-host

- **G2** Minimize rectangles: `set_rectangle` needs the dock's `wl_surface`, which lives on
  shell-host's connection, while the toplevel manager lives on the windows thread's.
- **G4** No drag and drop into shell surfaces (file drops on dock tiles, spring-loading).
- **G11** `sill debug` cannot inject a wheel event, so the tray scroll is proved by tables only.
- **G20** `sill debug key` sends a key event to a surface but no modifier state to the seat, so
  a held chord (the switcher's Alt) still needs a real modifier: `wtype -M alt` on the nested
  compositor in `dev/accept/switcher.sh`.
- **G180** A document hears the keyboard's modifiers only on its next key or pointer event, so
  a quick tap that beats the switcher's map, or Caps Lock before the first key on the lock
  screen, is not seen.
- **G290** The presented frame puts two widgets' contents in the center's first tile when all
  five are shown; `sill debug capture` of the same surface is right.
- **G6012** `dev/accept/widgets.sh --real-input` fails `step_real_add` and `step_real_remove` on sill
  master too (checked in a scratch worktree of master d079c35a): the gallery's default selection
  is Up Next where the script expects World Clock, and Remove is clicked at a fixed offset. The
  script's coordinates need re-deriving from the gallery.
- **G6010** Click-outside is done by covering the output. The notification center, Edit Widgets
  and the power menu draw sill's `ClickAway` (a transparent full-output element) under their
  panel, and the launcher runs a whole second layer surface (`sill-launcher-catcher`) for it,
  because a sheet dims nothing and catches no click (design/30 Part 4). shell-host should deliver
  a press outside a surface's panel region as an event (or dismiss a surface on it), so
  `ClickAway`, the launcher's catcher surface and the `-scope` wrappers that let presses through
  to it can go.
- **G6011** A popup document is framed by sill's `PopupFrame` (sill-ui-kit `menu/frame.rs`): an
  absolutely placed box whose padding is the popup's transparent shadow room, so
  `PopupSize::FitContent` measures the card plus that room. shell-host should take the room in
  `PopupSize::FitContent` and lay the root itself, which deletes `PopupFrame` and its rule.

## voice: what hold-to-talk waits on

Open items of the voice client (`sill_services::voice`, `sill_model::voice`, `sill_overlays::voice_consent`):

- **design/22 has no `voice` section.** `voice.hold_to_talk` ships and `voice.consent` is state
  (`#[settings(skip)]`); `crates/sill-settings/tests/keys.rs` lists `voice.hold_to_talk` as
  `UNCATALOGUED`. Ends when quire's design/22 gains the rows and `SECTIONS` names them.
- **The hold's edges come from a source that detects the hold itself** (`sill voice
  hold-start|hold-end|hold-cancel`). The raw-edge machine (`HoldKey`: command down, up, other key,
  pointer) is not built; it ends when keycap or the compositor sends raw edges.
- **Cancel carries a cause** (docket a0c168a): Escape when the launcher is dismissed, OtherInput
  for `sill voice hold-cancel`, Shell for the service's own cancel after a broken stream.
  `FocusLost` is modelled but nothing sends it: sill does not watch the focus leaving the prompt
  while a hold is on. A cancel voiced refuses as `Malformed` leaves its microphone open, so the
  service says "Voice stopped working."; so does a frame over `MAX_FRAME` (stream closed).
  `Cancelled(TooLong)` shows "That took too long."; the other causes show nothing.
- **Speech out is not wired.** The service subscribes to `Utterance.Ended` only; `Speech.Finished`
  and `Voice1.Speak`/`Hush`/`Prepare` come with the answer being spoken.
- **The companion's presence does not say Listening yet.** The prompt's placeholder shows the
  words; the orb and glow follow when `presence_of` gains the voice phase.

## answer-session: handles shown through the answer's own session

- **The "…" answers.** sill took an answer's router session from the one its own ask opened, so an
  answer it did not ask (started from the CLI, another surface, an agent, or before sill restarted) had
  no session and its handles were never shown: the card kept "…" for good. A failed `Session.Display`
  also left the handle unresolved and marked as asked. Test: `companion_session_bus.rs`.
- **Now.** The service reads `Answer.Session` once per answer when its object appears, keeps it in
  `AnswerFacts.session`, and shows handles through it. An object without the property (older docket)
  is shown through the session the ask opened, if sill asked it, and the service logs that once.
- **After Close** `Session.Display` is refused: the handle reads as its plain fallback, `#n`, and is
  not asked again.

## routing-footer: how an answer was reached

- **Now.** The service reads `Answer.Routing` (JSON `Vec<RouteNote>`) when an answer's object appears and on every
  `PropertiesChanged`, as `CompanionEvent::Routing`, kept in `AnswerFacts.routing`. The launcher's answer card shows
  `companion_wire::footer_line` of it as one caption line (`.sill-route-line`, quire tokens only); no notes, no line.
- **Older companiond.** An object without the property keeps `[]`: no footer, nothing logged.
- **Declined.** A named model that cannot serve arrives as `RefusalWire::Failed(reason)`; the card shows that reason.
- Test: `companion_routing_bus.rs`, `launcher_companion.rs`.

## Space removal: not a shell path

- sill has no Space-removal path. Settings (detent) owns the removal sheet and calls
  `org.quire.Memory1 Control.RemoveSpace` before `Spaces1.Remove`. The shell neither removes Spaces nor shows the sheet.

## p2-sq: the Quire arm

- **Wired.** `desktop_wl::backend_of` (replaces `sill_model::desktop::backend_for`; `GlobalOffer::names`).
  The model grew the commands the Quire arm carries (`AssignSpaces`, `ClearMinimiseTarget`, the app
  verbs, Space create/remove/move, `Overview`, `ShowDesktop`, `PickSwitcher`, `CancelSwitcher`),
  `ShellNote` (the shell protocol in the shell's words) and the fields `AppView::visibility` and
  `WindowView::{role, modality, geometry, responsiveness, lease, sensitivity}` (the wlr arm fills defaults).
  `desktop_wl::quire` holds the pure mappings (`quire_view`, `quire_wire_of`, `note_of`), the handout seam
  (`ZbusHandout` calls `org.quire.Compositor1.Connect("shell")`, `FakeHandout`), `QuireArm` over any
  `Desktop`, and the thread that runs it on the host's connection. The daemon asks the compositor for the
  shell connection before `Host::connect`, gives it to the host as `WaylandRoute::Handed`, and starts the
  arm at the first mapped surface that has a role. Notes: hot corner -> the corner link (first output, the
  protocol names none), summon gesture -> the companion's tap, kill chord -> the companion's stop-all.
  Tests: `tests/it/desktop_quire.rs` (fake desktop with `holding_shell`, scripted `QuireEvent`s, a stand-in
  `Compositor1` on a private bus).
- **Waits for P3-SILL.** No-ops with a doc naming the lane, in `compositor.rs`: modifiers over shell
  surfaces (Option variants), the switcher panel and `PickSwitcher`/`CancelSwitcher` answers, Mission
  Control chrome, edge reveals, menu flash, bell, reopen and the hold overlay. Nothing in the UI reads
  the arm's `DesktopView` or sends it commands yet (`ShellSeat` has no `command` method until a surface
  needs one); the toplevels and workspaces the dock and switcher draw still come from the windows thread's
  environment connection (which on a Quire offer is the standard arm over the projected flags).
- **Waits for casement's server lanes.** The whole path is unrun against a compositor: the handed
  connection, `quire_shell_v1` on it, `SetShellRole`, the notes. Activation sends serial 0 (desktop-client's
  decision). A window with no ext handle is invisible.
- **Deviations to confirm.** (1) `supports(Wlr, SetMinimiseTarget)` stays `No`: the Wlr arm runs on its
  own connection, which holds no dock surface; on the Quire arm the command names the dock's registered
  surface for the window's output. (2) The ext list alone is the Wlr arm. (3) A second queue on the host's connection gets no wake-up
  when the host's thread reads the socket first, so the arm thread also dispatches on a 20 ms tick; a
  waker from shell-host (or the host exposing its `Connection`) would remove it. (4) The arm's
  connection comes from `wl_surface.backend()` of the first mapped surface; shell-host exposes no
  `Connection` accessor.
- **Not touched.** The old `desktop_wl` protocol code (kept: the environment route serves KWin and tests).

## nm-proxies

- **What changed.** An MPL NetworkManager crate is gone from the workspace and from
  `sill-services`. The network and Wi-Fi backends use sill's own `zbus` proxies in
  `crates/sill-services/src/nm/`, one file per interface, written from the NetworkManager D-Bus
  documentation and carrying only the members sill uses: `NetworkManager` (`GetAllDevices`,
  `ActivateConnection`, `PrimaryConnection`, `ActivatingConnection`, `Connectivity`,
  `WirelessEnabled` read and write, `WirelessHardwareEnabled`), `Connection.Active` (`State`,
  `Type`, `Devices`), `Device` (`Disconnect`, `DeviceType`, `State`), `Device.Wireless`
  (`RequestScan`, `AccessPoints`, `ActiveAccessPoint`, `LastScan`, `AccessPointAdded` and
  `AccessPointRemoved`), `AccessPoint` (`Ssid`, `Strength`, `Flags`, `WpaFlags`, `RsnFlags`),
  `Settings` (`Connections`, `NewConnection`, `ConnectionRemoved`) and `Settings.Connection`
  (`GetSettings`). `DeviceType` and `DeviceState` are two small enums over the numbers sill
  tells apart; every other value reads as `Other`.
- **Behaviour.** Unchanged: the same reads, the same fallbacks. The one edit in a body is that
  `saved_ssid_of` takes the SSID out of the settings map instead of cloning it.
- **Test.** `tests/it/nm_bus.rs` serves a stand-in NetworkManager on a private bus and drives
  both services through it, including the four Wi-Fi commands.
- **Left.** `deny.toml` keeps its MPL allowance (freedesktop-desktop-entry and others still
  arrive under it). `Cargo.lock` was edited by hand to drop the crate; the first cargo run may
  prune its now-unused dependencies.

## dock-minimise-rect

- **What changed.** The dock publishes, per output, the resting rectangle of each app's icon
  (`rest_rects::app_rects`, from the frame at no magnification, whole logical px in the dock
  surface's coordinates; the Trash has none) into `Services::dock_rects` (`DockRects`, a watch, written
  only on change). The daemon's `ShellSeat` (Quire arm only) reads it with the arm's `DesktopView`
  and sends `SetMinimiseTarget` for each window whose app has an icon on the window's output,
  `ClearMinimiseTarget` for each that had one and has none (`MinimiseTargets::of` / `since`, pure,
  table-tested in `sill-model`). `ShellSeat::command` is the smallest command method. Every shell
  surface registration clears the "sent" record and sends all targets again, so a target the
  compositor refused with `NoSurface(Dock)` before the dock was registered arrives after it.
- **Matching is by app name.** The dock groups windows from the environment connection
  (the windows model's keys), the arm names them by identifier, and the two key spaces do not meet.
  A window therefore takes the rectangle of the icon filed under its `WindowView::app`: the icon's
  desktop-entry id and the raw app id of each of the icon's windows. If Quire's app key differs
  from both (a proven name unlike the entry id), that app's windows get no target.
- **Only a live casement run proves.** That the rectangle is in the coordinates
  `quire_shell_v1.set_minimise_target` expects (surface-local logical px, top-left origin) and
  that the genie lands on the icon rather than offset by the pill's gutter or the autohide slide
  (the rectangle ignores the slide); that a target sent before the dock's `SetShellRole` is
  refused and the resend after registration fixes it; that a dock restarted or an output hot-plugged
  re-sends; that a window on an output with no dock falls back to the compositor's own default.
- **Not done.** The `minimise to application` setting (one tile per app is the only mode); a
  stale entry stays in `DockRects` when an output goes (its windows have no output then, so no
  target is sent); the `DockEffect::SetMinimizeRects` placeholder in `io.rs` stays a no-op.

## p2-ch: the screen-sharing picker

- **Served.** `org.quire.Shell1.PickCaptureSource(app_label, kinds, multiple, cursor_modes) ->
  (sources, cursor)` at `/org/quire/Shell1`, frozen by transom's `chooser.rs`. Pure logic in
  `sill_model::capture_pick` (parse the call, `plan` the tabs and rows, `toggle` the picks, `reply`
  the answer, `step` one request at a time, `windows_of` the desktop's view); the service in
  `sill_services::capture_pick`; the sheet in `sill_overlays::capture_sheet`; tables in
  `sill-model` (`capture_pick/tests`), the stage in `capture_sheet/tests.rs`, a private-bus test in
  `sill-services/tests/it/capture_pick_bus.rs` and a lint case in `sill/tests/it/lint.rs`.
- **Name ownership.** The object is exported on the connection the intents service already holds as
  `org.quire.Shell` (the `seat`), so the interface is served from the name transom's proxy
  already calls and no second name is claimed. Without a seat nothing is served and transom's call
  fails with an unknown-name error ("no picker"). Only the owner of
  `org.freedesktop.impl.portal.desktop.quire` may call (a forged caller is `AccessDenied`).
- **Candidates.** Screens: the displays service's outputs, quoted by connector name (`DP-1`).
  Windows: the Quire arm's `DesktopView` pushed by the daemon on every `ShellMsg::Desktop`
  (documents that are not minimised and not `CaptureHidden`), quoted by the identifier casement
  publishes. **No fallback on the other arms**: the wlr toplevels carry no ext
  identifier (`desktop_wl::identifier` would make one up, `toplevel-N`, which casement would not
  know), so the Windows tab is empty there rather than wrong. Reading the ext identifier on
  the windows thread would fix it.
- **UI.** A centred quire `Sheet` over an Overlay layer (keyboard exclusive) on the banners'
  output, like the confirmation sheet. Screens/Windows `SegmentedControl` only when both kinds
  are accepted; a `List` of `Row`s (monitor/window glyph, title, detail, check); a pointer
  `SegmentedControl` only when more than one mode is offered; Cancel and Share. Keys: arrows move
  (and pick, when one source is allowed), Space picks the row under the keys, Return shares,
  Escape cancels. Share is disabled until something is picked, so there is no arming delay.
- **Deviations / not done.** Thumbnails (plain glyphs and names for v1). The caller leaving the
  bus takes the sheet down (watched by name owner), but a caller that merely stops waiting
  while connected leaves the sheet up until the person answers. The pointer words ("Hidden",
  "Shown", "By the App") are first drafts. The capture sheet is phase A like the other sheets
  (a layer surface, not unforgeable).


## winwire: a window's menu (the green light)

- **The route.** casement sends `quire_shell_v1.window_menu(toplevel, x, y, enabled)` when the pointer rests on a
  window's green light; `desktop-client` folds it into `ShellEvent::WindowMenu`; `sill-services` `note_of` makes
  `ShellNote::WindowMenu { window, at, enabled }`; `daemon::compositor::apply` opens it on
  `Links::window_menu` (`WindowMenuLink`); each output's `sill_overlays::WindowMenu` surface (Overlay, whole
  output, empty input region, draws nothing) opens quire's `Menu` as a context popup (`context_menu_popup`,
  `PopupConfig` of the desktop menu) when the point is on its output. A pick is written back to the link,
  `window_menu::spawn_picks` sends `DesktopCommand::PickWindowMenu(window, entry)` to the arm, and the Quire arm sends
  `window_menu_pick`. A dismissal sends nothing.
- **Dimmed, not hidden.** quire's context menu leaves unavailable entries out unless asked; `MenuModel::unavailable()`
  (default `Hide`) lets this menu ask for `Dim`, so the menu is always the same set of entries.
- **Submenus are quire's own** ("Move & Resize", "Full Screen Tile"); no section headers were needed.
- **Open, only a real run proves:** the popup is opened by a hover, not by a press on the shell's own surface, so it
  has no input serial. If casement refuses or immediately dismisses the grab, give the popup `PopupGrab::Ungrabbed`
  and close it on the next pointer press outside (the compositor knows the press; add a `window_menu_close` event).
  The menu opens `Full Screen Tile` entries dimmed until casement has a Split View command.

## splitwire: the Split View picker and divider

- **The route.** casement offers the other half of a Split View with `quire_shell_v1.split_choose(first, side)` and one
  `split_candidate(toplevel)` per candidate, and ends it with `split_choice_end(first)`; `desktop-client` folds them
  into `Shell::split_choice` and `ShellEvent::{SplitChoose, SplitCandidate, SplitChoiceEnd}`; `sill-services` `note_of`
  makes `ShellNote::{SplitChoose, SplitCandidate, SplitChoiceEnd}` (one note per event: the wire has no "list complete"
  event, so the picker fills as the candidates arrive); `daemon::compositor::apply` hands them to
  `sill::split_picker::apply`, which writes `SplitPickerLink` (`begin`, `add`, `end`) with the first window's output and
  each candidate's title from the latest `DesktopView`. Each output's `sill_overlays::SplitPicker` (Overlay, whole output,
  keyboard Exclusive, input everywhere, no blur) maps while an offer shows on it. A tile or Return writes
  `SplitAnswer::Picked`, Escape or a press on the backdrop `Cancelled`; `split_picker::spawn_answers` sends
  `DesktopCommand::PickSplit` / `CancelSplit`, and the Quire arm sends `split_pick` / `split_cancel`.
- **No window previews.** sill has no window previews (Mission Control and the switcher are the compositor's or app
  tiles), so each card is the window's app icon with its title under the selected tile: quire's `AppSwitcher` with the
  window identifier as the tile key. A thumbnail needs a per-window capture (casement has `ext-image-copy-capture`
  for toplevels); when the shell asks for those, the tile's `IconSource` is where the picture goes.
- **Shared with the switcher.** The icon source (`TileIcons`) and the tile sizes (`icon_side`, `metrics_of`) moved
  from the switcher to `sill_ui_kit::app_icons` so both surfaces draw the same tiles from `switcher.*`.
- **An offer with no output named** (the desktop did not say which output the first window is on) shows on every output.
- **The divider is not built.** The wire is complete (`DesktopCommand::DivideSplit(window, thousandths)` ->
  `split_divide`), but sill has no handle. A handle would be a second overlay surface whose input region is one 8 px
  element at the strip between the halves (`DIVIDER_WIDTH` in casement's `rust_scene/split.rs`), dragged with the
  pointer held (Wayland keeps the grab on the surface while a button is down, so the drag can leave the region); its
  share is `(pointer x - 4) / (output width - 8)` in thousandths. What stops it: (1) casement has no hit test or pointer
  grab on the strip and the full-screen windows are drawn above Top layers, so whether an Overlay surface gets pointer
  events over a full-screen window's pair is only proven by a real run; (2) `DesktopView` carries neither which
  windows are a pair nor the share, so the handle would take the strip's place from the two windows' geometry; (3) the
  share is not reported back, so a restarted shell would draw the handle at the middle until the next drag.
- **Open, only a real run proves.** Left of Screen in the green light's menu with two windows: the dimmed picker appears
  on the free half with the other window as a tile, a click on it makes both halves; Escape and a click on the dimmed
  first half both leave the first window alone, full screen. Whether Escape reaches the picker (keyboard Exclusive over a
  full-screen window), and whether a press on `.sill-split-scrim` arrives as a pointer-down on the surface.
  `dev/accept/split-picker.sh` (the recipe's acceptance script) is not written: it needs a running session.

## spacesbar

Mission Control's Spaces bar (lane `spacesbar`, written without a build: nothing here has been compiled or run).

- **What landed.** `ShellNote::Overview(Moved{mode,state,..})` now does something: `sill::spaces_bar::apply` turns it into
  `BarShow` (`sill_model::desktop::bar_show`: up for Mission Control Entering, Open or Tracking, down for Leaving, Closed and for
  App Expose and Show Desktop, which have no bar) on `SpacesBarLink`. `daemon/events.rs` writes each `DesktopView` to the same
  link. Each output's `sill_overlays::SpacesBar` (Top, anchored top/left/right, 112 px high = `STRIP_PX`, reserves nothing, no keyboard,
  input everywhere, spawned before the menu bar so the bar draws over its top) maps while the bar is up and unmaps when the leave
  has settled. It draws quire's `WorkspacePills` (a pill per Space of the output, the one on show `Selected`) centred, and a
  toolbar `Button` with the plus glyph at the right edge; the strip is padded by `bar.height_px` so the pills sit under the menu bar.
- **Names.** `bar_spaces`: desktops are "Desktop N" counted per output in order (as the Mac counts them, full-screen Spaces do
  not take a number); full-screen, Split View and agent Spaces are named by the apps with a window on them (app index name, else
  the app key), joined with " & "; with no known window, "Full Screen", "Split View" or "Agent". The wire carries no Space name, so
  a renamed Space cannot show its name.
- **Click.** A primary press on a pill sends `SwitchSpace(id)` then `Overview(MissionControl, Off)` (the Mac leaves Mission Control
  on a Space click); the plus sends `CreateSpace(output name)` (the existing path, `DesktopCommand::CreateSpace`) and stays in
  Mission Control. Right-click on a pill does nothing (the Space editor does not exist).
- **Departures.**
  1. *Names only, no thumbnails.* Sonoma expands the collapsed bar to Space thumbnails when the pointer reaches the top. The wire has
     no Space thumbnail and sill has no capture of a Space, so the bar stays at the collapsed state.
  2. *Motion is 120 ms, not about 350 ms.* quire has no top-edge slide and its sheet pair is the nearest known animation: `sheet-in`
     / `sheet-out` (`--t-quick`, `--e-out` / `--e-exit`) with `--sheet-dy` set to a lift of `--s-48` off the top (sill may not define
     keyframes: the lint rejects unknown animation names). The windows settle in about 350 ms, so the bar finishes first.
  3. *Every output.* The overview note names no output, so the bar shows on all outputs; the Spaces on each are its own.
  4. *Drag and drop of windows onto a Space is not done.* There is no drag from the compositor's window tiles to this surface
     (the tiles are the compositor's, not ours); `DesktopCommand::ToSpace` exists for when casement reports a drop on the strip.
  5. *Space reorder, remove and the Space menu are not done* (`MoveSpace`, `RemoveSpace` exist as commands; nothing in the bar sends them).
  6. *`Tracking` shows the bar at once,* at full strength, not scaled by `progress`; the progress field is ignored.
- **Interface asks.**
  - quire: `Anim::DropIn` / `Exit::LiftOut` (a top-edge slide-down and lift, with the overview's duration token, about 350 ms) for
    `use_presence`; today the bar borrows the sheet pair.
  - quire: a `SpacesBar` component (pills with a thumbnail slot that expands on hover, the Space add plus, drop target highlight)
    once the wire has thumbnails; `WorkspacePills` is the menu bar's segmented group, near enough for a collapsed bar.
  - casement: say which output Mission Control is open on (a per-output `overview_state`), report a Space thumbnail, and report a
    window dropped on the strip.
- **Open, only a real run proves.** Whether the strip lies where the compositor leaves it (the Top layer, anchored at y 0, with
  the menu bar drawn over its top; if the compositor places the strip below the menu bar's reserve the padding doubles); whether
  a press on a pill reaches the surface while the compositor holds the pointer for Mission Control; the pill row's centring and the
  plus alignment under Blitz's flex.

## mc2

Lane `mc2`, written without a build: nothing here has been compiled or run, so the gate decides. Adopts quire's `SpacesBar`
and casement's `quire_shell_v1` version 2 (casement-protocols `ea171a1`).

- **What landed.**
  1. *quire's bar.* `sill-overlays/src/spaces_bar/` now hangs `SpacesBar { label, spaces, shown, on_pick, on_add, on_hidden,
     expansion, common }` under the menu bar's padding. WorkspacePills, the own Add Button, `style.css`, `SPACES_BAR_CSS` and the
     `sheet-in`/`sheet-out` presence are gone (the lint case takes `css: ""`). A Space id is the `BarSpaceKey` string
     (`SpaceId::as_str`) and parsed back on a press (`SpaceId::parse`; an unparsable key sends nothing).
  2. *Wire.* The arm's client already binds `quire_shell_v1` at `1..=2` and every `wl_output` (desktop-client `bind_all`,
     `bind_outputs`); sill changed nothing there. New notes `ShellNote::{OverviewOutput, DropTarget, SpacePicture}`, new commands
     `DesktopCommand::{CaptureSpace, SetSpaceTargets}`. sill matches `ShellEvent`/`DesktopCommand` exhaustively nowhere else than
     `quire_wire_of` (arms added) and `desktop_wl::wire_of` (wlr refuses both); `note_of` has a catch-all.
  3. *Per output.* `BarOutputs::bar_on(overall, output)` (sill-model `bar_outputs.rs`): up only when the overview as a whole is
     Mission Control and the output is reported `Shown`; with no report at all every output follows the whole (desktop-client's
     `Overview::shown_on` rule, kept in the model because sill carries notes, not the client's state).
  4. *Pictures.* `QuireArm::execute` keeps the capture ticket of a `CaptureSpace`; `QuireArm::pictures(take)` (called by the
     shell thread after each sync with `DesktopClient::take_capture`, which the `Desktop` trait does not have) turns a finished
     frame into PNG bytes (`pictures.rs`, `Frame::to_rgba8`, `image`) and a `ShellNote::SpacePicture`, or `Failed`. The note
     reaches `SpacesBarLink::set_picture`; the tile gets `SpaceTile::thumbnail(ImageSource::png(..))`.
  5. *When to ask* (`thumbs_wanted`, pure): per laid-out bar, the Space its output shows, once when the bar comes up; again only
     after the last answer arrived and the Space or its windows (identifier, state, geometry) changed. A held request is the
     compositor's damage gate, but a new session's first frame completes at once, so asking after every frame would spin; a
     failed answer is not retried until something changes; a bar that goes forgets its ask.
  6. *Targets* (`tiles_of`, `target_commands`, pure): the surface measures its bar element (`use_rect`) and publishes a `BarLayout`
     (cards in the desktop's logical pixels: the bar's box plus the output's origin, and the card size at the output's scale) on
     the link's feed; `spaces_bar::spawn_outbox` follows the feed, sends `SetSpaceTargets` for each output whose cards changed (an
     empty set clears: sent as soon as the bar goes down) and the `CaptureSpace` requests.
  7. *Drop highlight.* `BarDrops::state_of` gives `DropState::Target` to the tile named by `DropTarget` (`SpaceTile::in_drag`).
- **Surface height and input (record asked for).** The layer surface is `bar.height_px + BAR_EXPANDED_PX` (150) tall
  (`layer::surface_px`), more than the compositor's 112 px reserve (`STRIP_PX`, kept: casement still keeps 112 free for the collapsed
  bar). The expanded bar therefore lies over the top (about 62 px with a 24 px menu bar) of the windows' layout, as the Mac's does.
  Input is `RegionSpec::Element { id: "spaces-bar" }`, the quire bar's own element (its `Common::id`), so the transparent rest of
  the surface passes the pointer to the compositor's window tiles; the menu-bar padding above the bar is not input either (the menu
  bar's own surface covers it). Ask for casement: reserve at least the bar's real height if the windows should not be overlapped.
- **Departures.**
  1. *Always expanded.* The bar is given `expansion: Some(Expanded)`: while the compositor holds the pointer for a window drag the
     bar gets no pointer events, so a hover-driven expansion could never happen during the one gesture the targets exist for; and
     the targets are the cards, which exist only expanded. The collapsed row of names is not reachable.
  2. *Card rectangles are arithmetic.* `SpacesBar` reports no per-tile rectangle or mounted handle, so `tiles_of` centres a row of
     168 x ... cards in the bar's box using quire's `spaces_bar.css` numbers (160 x 100 card, `--s-4` tile padding, `--s-16` gap,
     `--s-8` top padding). A harness test compares the first card with the drawn `.ds-spaces-bar-thumb`, which catches drift when
     quire changes. Interface ask to quire: `SpaceTile::common` (or `on_layout(Vec<(BarSpaceKey, Rect)>)`) so the host measures the
     cards.
  3. *Only the Space on show is pictured* (casement: "first version: only the Space the output shows now"). A tile keeps the last
     picture it got in the session (the feed holds them per output and Space; they go stale), else the quiet blank card.
  4. *No wallpaper under the picture.* The picture has none (windows over transparency) and the card is quire's blank colour. The
     wallpaper images exist (`WallpaperService`) but drawing one under quire's `<img>` needs a layer in quire's card or compositing
     in the daemon; not done. Ask to quire: a `SpaceTile` backdrop, or draw the card's background from an `ImageSource`.
  5. *Scale from the output.* The picture is asked at the output's scale (`OutputInfo::scale`), not the surface's own
     `preferred_scale`; they are equal unless fractional scale differs per surface.
  6. *Base64 per recompute.* `ImageSource::png` re-encodes each picture when the tile list is recomputed (a memo over the feed and
     the app index), which includes every feed change. Cheap at 320 x 200; a cache keyed by the picture's `seq` is the fix if it shows.
  7. *`Accepts` is never set.* Only the compositor's `drop_target` is known; the outline for "could take the drop" needs the
     compositor to say a drag has begun.
- **Tests.** sill-model `bar_tests.rs` (per-output show, card layout and scale, target commands, ask/refresh decisions, a bar that
  goes, a failed answer, drop highlight, the outbox, space ids); sill-ui-kit link tests (the new parts, repeats wake no one);
  sill-services `desktop_quire.rs` (the two notes, `SetSpaceTargets`, a picture asked for and returned as a PNG note);
  sill-overlays `spaces_bar.rs` (the existing five moved to quire's selectors, plus per-output, picture and drop highlight, the
  published cards against the drawn thumbnail); layer tests (height, input element); `sill` `spaces_bar.rs` (the notes reach the link).
- **Check at the gate.** Everything compiles for the first time: the `ds::prelude` names (`BarSpaceKey`, `ImageSource`, `Rect`, `Common`,
  `BarExpansion`), `Memo<BarShow>` in `use_map_when_up`/`use_publish_layout`, the `fn(&mut SpacesBarFeed) -> &mut T` closures in
  `SpacesBarLink::edit`, `Option::is_none_or`, the doc tests in `sill::spaces_bar`. The harness cases: that `use_rect` yields a
  rect in the harness and that it is settled after 400 ms; the first-card equality test is the one most likely to need the
  constants or the wait changed. Lint: the `spaces_bar` case now has `css: ""`.
- **Open, only a real run proves.** That `get_space_capture_source` returns a picture in time (ms) and with the right orientation
  (the frame's transform is not applied); that the `drop_target` event arrives while the pointer is held by the compositor's drag;
  that the cards sit where the compositor's targets think they do on a scaled output (rects are logical, origin from
  `OutputInfo::logical`); that an input region following the bar element leaves the window tiles below reachable; that
  nothing spins when the bar sits still; the lower 62 px of the bar over the window tiles looks right.

## mc4

Lane `mc4`, written without a build: nothing here has been compiled or run, so the gate decides. Adopts casement's `quire_shell_v1`
version 3 (casement-protocols `0f7afcb`, casement `3eaf1f7`) and completes the Spaces bar of `mc2`.

- **What landed.**
  1. *Version 3.* sill pins no version: desktop-client's `bind_all` binds `quire_shell_v1` at `1..=3` already, so nothing changed
     there. New note `ShellNote::DropTargetNew { output, over }`, new command `DesktopCommand::SetNewSpaceTarget { output, rect:
     Option<Rect> }` (`None` removes). Matched in `quire_wire_of`, `desktop_wl::command` (wlr refuses), `note_of`, `compositor::apply`.
  2. *A picture of every Space.* `thumbs_wanted` keeps one ask per output and Space (`ThumbAsks` keyed by both) and asks for every
     Space `bar_space_ids` lists on a laid-out output, shown or not. The refresh rule is the mc2 one per Space: again only after the
     last answer arrived and that Space's windows (identifier, state, geometry) changed. Switching Space changes no window, so the
     compositor's kept sessions are not asked again. A Space that leaves an output forgets its ask. The arm already tracked captures by
     id (`QuireArm::pending`), so several in flight needed nothing there. The mc2 departure "only the shown Space" is gone, and so is
     its test (replaced by the every-Space ask test).
  3. *Wallpaper under each picture.* quire's `SpaceTile` has no backdrop slot, so sill composes it with no quire change: the bar's
     wrapper div carries `--sill-bar-wallpaper: url("file://...")` (the same prepared image the wallpaper surface shows, light or dark
     as `resolve` says, through `wallpaper::url_for`, now shared by the wallpaper view and the bar) and `data-backdrop=wallpaper`;
     `spaces_bar/style.css` (`SPACES_BAR_CSS`, an `AppStyle`) paints it as the background of quire's `.ds-spaces-bar-thumb`, under the
     transparent picture. The lint case takes the sheet again. Interface ask to quire (cleaner): `SpaceTile::backdrop(ImageSource)`.
  4. *The plus.* `add_of` gives its rectangle from the bar's box, as `tiles_of` gives the cards: at the right edge, `--s-20` in,
     `--s-8` down, a square of `--ctl-h-m` (28 px). `BarLayout::add` carries it; `target_commands` sends `SetSpaceTargets` then
     `SetNewSpaceTarget` per output, each only when it changed, and `SetNewSpaceTarget { rect: None }` once the bar goes. The
     harness compares the published rectangle with the drawn button.
  5. *The plus lit.* `BarDrops` now holds the Space tile and the plus per output (`with_new`, `new_state_of`); a Space under the window
     puts the plus out. `ShellNote::DropTargetNew` -> `SpacesBarLink::set_drop_new`.
- **Departures.**
  1. *The plus is lit by sill's sheet, not quire's drop state.* quire's `SpacesBar` has no drop state for its plus (the `Button` is
     built inside it), so the wrapper says `data-add-drop=target` and `style.css` gives the button the wash and ring a lit Space gets
     (`--accent-soft`, `--accent`). Interface ask to quire: `SpacesBar::add_drop: DropState` (and `SpaceTile::backdrop`, and a
     measured rectangle for the plus, as for the cards).
  2. *The plus rectangle is arithmetic*, like the cards (FINDINGS `mc2` departure 2): a change to quire's bar padding or the medium
     control height needs `BAR_SIDE`, `ADD_PX` in `bar_layout.rs` changed with it. It is the button itself, 28 px: small to hit.
  3. *`Accepts` is still never set.* The protocol gives no event when a drag begins: `drop_target` and `drop_target_new` fire only on
     entering or leaving a rectangle (and at the end), so the shell cannot know a drag is in progress before the pointer reaches a
     tile. Ask for casement: an event at drag start and end (or the dragged window) would let the tiles that take it show the outline.
  4. *The wallpaper may not line up with the windows.* The picture is the output's size squeezed into 160 x 100 by `object-fit:
     cover`, the wallpaper by `background-size: cover`; they agree only when the output is 16:10. The file URL is read by the
     renderer's net layer as the wallpaper surface's `img` is; a CSS background through a custom property is what is unproven.
  5. *Drag between outputs and the created Space* are the compositor's: sill sends each output's targets and lights what the events
     name, nothing more.
- **Tests.** sill-model `bar_tests.rs` (the plus's place, targets then the plus's target sent, moved alone, cleared with the bar; an ask
  per Space, kept across a switch, forgotten when the Space leaves; the plus lit and put out by a tile; the outbox); `tests.rs` (the
  new command is Quire-only); sill-services `desktop_quire.rs` (`SetNewSpaceTarget` set and removed, `DropTargetNew` note); sill-ui-kit
  link test; `sill` `spaces_bar.rs` (the note reaches the link); sill-overlays `spaces_bar.rs` (the plus where it is drawn, a picture
  on every tile, the plus lit, the wallpaper backdrop for the right output only) and `io.rs` (`css_url`).
- **Check at the gate.** First compile: `ds::prelude::{DropState, Scheme, SpaceLook, resolve}` in `io.rs`; `Memo<Option<String>>` read
  as `(bar.wallpaper)()`; `"data-backdrop": Option<&str>` and `"data-add-drop": Option<&'static str>` attributes in `rsx!`;
  `AppStyle` inside `SpacesStrip`; the lint of `style.css` under `Profile::Strict` (selectors that name quire's classes,
  `var(--sill-bar-wallpaper)` as a `background-image`). The overlay harness cases: whether `html()` shows the inline custom property
  and `data-add-drop`; the plus-rectangle equality test (`.ds-spaces-bar-add .ds-button`) is the one most likely to need the
  constants changed.
- **Open, only a real run proves.** That every Space (hidden ones too) comes back with a picture and nothing spins; that the wallpaper
  shows under the pictures; that a drop on the plus makes a Desktop and the new card appears with its picture; that dragging between
  outputs lights the right bar; that the plus lights and puts its light out.

### mc4, part 2: quire's `SpacesBar` API (lane `sill-mc4`, written without a build)

Switches the Spaces bar onto quire `c22bbd9e` (`add_drop`, `on_layout`, `SpaceTile::backdrop`; quire FINDINGS `q-barasks`) and deletes
the workarounds above.

- **Resolved.** Departures 1 and 2 and the wallpaper's custom property: `spaces_bar/style.css`, `SPACES_BAR_CSS`, the wrapper's
  `data-add-drop` / `data-backdrop` / `--sill-bar-wallpaper`, `css_url`, and the arithmetic (`tiles_of`, `add_of`, `BAR_SIDE`, `ADD_PX`,
  the tile padding and gap) are gone. The plus is lit by `SpacesBar::add_drop` (`BarDrops::new_state_of`); each tile takes
  `.backdrop(ImageSource(file URL))`, the URL `wallpaper::url_for` gives (the string the wallpaper surface's `img` already loads);
  `io.rs` keeps the last `ds::prelude::BarLayout` quire reported (imported as `DrawnBar`, because `sill_model::desktop::BarLayout` is the
  published shape and keeps its name), and an effect publishes it as the link's `BarLayout` while the bar is up: the cards (`card`, so a
  collapsed bar publishes none), the plus, each moved by the surface's origin on the output, `thumb` still from `thumb_size`. The
  published shape, `target_commands` and the daemon are unchanged. The lint case is `css: ""` again.
- **Still open.** Departure 3 (`Accepts` is never set: no drag-start event from the compositor). Departure 4 (the wallpaper and the
  windows agree only on a 16:10 output) stands; the backdrop is now an `img` with `object-fit: cover`, so the file URL needs no
  background-image proof. The plus rectangle is quire's `span.ds-spaces-bar-add` (the button's box), not a smaller button.
- **Tests.** sill-model: the `tiles_of` / `add_of` tests are deleted (target commands, asks, drops kept). `io.rs`: the origin shift and
  the collapsed card. sill-overlays: published rects equal the drawn `.ds-spaces-bar-thumb` and `.ds-spaces-bar-add`; the lit plus is
  `.ds-spaces-bar-add[data-drop=target]`; `img.ds-spaces-bar-backdrop` appears on both tiles for the right output only.
- **Check at the gate.** `use_signal` / `use_hook` in `use_publish_layout`; `DrawnBar: PartialEq` (it is); `rect` equality to the whole
  pixel between quire's phase-published rects and the harness's `rect` (rounding may need a one-pixel tolerance); the lit-tile test
  assumes `data-drop` is only on the plus then; whether the surface's `logical` origin type is `i32`.

## wave-2026-10-15/dragstart

The Spaces bar shows its accepting state during a Mission Control window drag. `ShellNote::WindowDragBegin` / `WindowDragEnd`
(from `ShellEvent::WindowDragBegin` / `WindowDragEnd`, quire_shell_v1 version 4) go through `spaces_bar::apply_note` to
`SpacesBarLink::set_dragging`; `BarDrops` keeps the outputs with a drag under way and answers `DropState::Accepts` for every
Space tile and the plus there, `DropState::Target` still wins for the tile under the window. No quire change: `SpacesBar`
already draws `DropState::Accepts` through `SpaceTile::in_drag` and `add_drop`. Needs casement-protocols `08bcc62`; against a
compositor older than version 4 nothing is sent and the bar behaves as before.

## wave-6/colour-picker: `PickColor` and the eyedropper

- **Served.** `org.quire.Shell1.PickColor(app_label: s) -> (color: a(ddd))` on the same object as
  `PickCaptureSource` (`sill_services::capture_pick`), with the same caller check (only the owner of
  `org.freedesktop.impl.portal.desktop.quire`; anyone else is `AccessDenied`). The array is empty on
  cancel, else one `(red, green, blue)` in 0..1 sRGB (`sill_model::capture_pick::ColorReply::wire`,
  an 8-bit pixel over 255). One question at a time of either kind: a second call while a colour or a
  sources question is open is `LimitsExceeded`; the caller leaving the bus clears it
  (`CaptureEvent::CallerGone`).
- **No eyedropper is drawn yet, so every call is answered with an empty array at once.** The state
  says so (`CapturePicker::eyedropper` is `Absent` until a surface sends
  `CaptureCommand::Eyedropper(Present)`); nothing is asked of the person and nothing opens. What is
  missing for the real thing, exactly:
  1. *A picture of the whole output.* `desktop-client` can already capture an output as shown
     (`CaptureRequest::new(output)` through `DesktopCommand::CaptureOutput`, ext-image-copy-capture
     with shm), but sill only asks for a Space picture. It needs a `DesktopCommand::CaptureOutput`
     (`sill-model`), its arm in `desktop_wl/quire/command.rs`, `QuireArm::pending` to hold an output
     capture beside a Space capture, and a `ShellNote::OutputPicture` that carries the frame's RGBA
     bytes (`picture_of` PNG-encodes at the Space thumbnail size, so it cannot be reused as it is).
  2. *The surface.* An Overlay layer surface per output, keyboard `Exclusive`, shown over the frozen
     picture (taken before the surface maps, so the pointer and the surface are not in it): a
     magnifier loupe following the pointer (about 9 by 9 source pixels, a grid, the centre pixel
     boxed, its hex value under it), a click sends `CaptureCommand::ColorAnswer(id, Picked(Rgb8))`
     from the pixel under the pointer (logical position times the output's scale), Escape sends
     `Cancel`. It sends `Eyedropper(Present)` when it starts.
  3. *Gamut.* The pixel is the output's sRGB as casement composes it; on a wide-gamut panel with the
     colour pass the value read is the sRGB picture, which is what an app asked for.

