# Manual checks

Checks that need a person at the machine. They are assumed to pass until someone runs them;
each has the command to start it and what to look for. Tick the box and note the date when done.
Add new ones at the end of their section.

## On a KWin session (test compositor, real devices)

KWin is not a target, but a KWin session at the machine is the quickest place to try sill's
surfaces with real devices: its own bar and dock over the desktop, BlueZ, MPRIS, the clipboard
(`ext_data_control_manager_v1`) and real pointer and keyboard. KWin lacks casement's protocols
(toplevels, workspaces, output management, session lock), so the services that need them stay
at their defaults there; anything about those, or about displays and scale, is in the casement
sections below. The monitor is the BenQ MA270U (27", 3840 x 2160).

- [ ] **CJK input in the launcher, a short real-keyboard spot check.** Automated by shell-host's
  `SILL_DIR=~/sill ~/shell-host/scripts/dev/accept-ime.sh` (fcitx5 McBopomofo on a private headless
  KWin; the field reads 你好). By hand, once: `bash dev/manual/ime-live.sh start`, click into
  the search field, switch fcitx5 to McBopomofo, type `su3cl3` and press **Return** to commit
  (Space opens the candidates): 你好 appears once, not doubled. Then `bash dev/manual/ime-live.sh
  capture` and `bash dev/manual/ime-live.sh stop`. Only one copy runs at a time.
- [x] **Monitor speakers follow the volume**: switch the default sink to the BenQ MA270U's HDMI
  audio, press volume up/down: the monitor's own volume moves too (its range is 0-50), and mute
  mutes it. Switch back afterwards. (Passed 2026-09-28.)
- [ ] **Control center with the real keyboard** (a real click, Tab, Shift+Tab, Space and Escape
  pass under fake input on a headless KWin with an app window focused first:
  `dev/accept/bar.sh --real-input`): with a window focused, open it from the bar item
  (left of the clock); Tab shows the focus ring and moves it tile by tile, Shift+Tab back,
  Enter/Space toggle, Right on a chevron opens Wi-Fi/Bluetooth, Escape goes back then closes,
  and afterwards typing reaches the window again.
- [ ] **Switching bar menus by click and by hover** (pass under fake input on a headless KWin,
  same script): click Wi-Fi, then Sound: Sound's menu stays open and Wi-Fi's is gone; with one
  open, sliding along the bar switches menus; a second click on the open item closes it; with
  the notification center open, a click on a bar item closes it and opens the menu.
- [ ] **Control center details on real services** (the harness covers each on fakes):
  - Connect a paired Bluetooth device from its pane (disconnect one you can spare first, not the
    keyboard): its glyph breathes, its disc seals, and its battery ring sweeps in with the
    percentage counting.
  - Play something in a real MPRIS player (Firefox, Spotify), open the center a minute in: the
    position bar starts where the track is (not where it was when the player last reported),
    steps once a second, holds when paused; the play/pause glyph grows the next action in.
  - This desktop has no keyboard backlight (`/sys/class/leds` lists no `*::kbd_backlight`): the
    Keyboard Brightness module must not show under Display.
- [ ] **The Bluetooth tile** says "On" (no device name) with the keyboard and headphones
  connected.
- [ ] **Bluetooth asks before it goes off.** With the Keychron connected over Bluetooth, open
  the control center and press the Bluetooth tile: "Turn Bluetooth off?" covers the panel and
  the keyboard keeps working. Press Escape (the keyboard stays connected), then press the tile
  again and click Cancel, then click beside the alert: each leaves it on. The pane's switch (the
  chevron, then the switch) asks the same. Only "Turn Off" (or Return) turns it off; turning it
  back on asks nothing. Automated on a fake BlueZ by `dev/accept/control-center.sh --real-input bluetooth`;
  by hand, the real radio and the look.
- [ ] **Banners with a real mouse** (hover, the close button, a click, a 90 px and a 50 px drag
  already pass under fake input on a headless KWin through
  `dev/accept/notifications.sh --real-input`; this is the feel with a real mouse and touchpad):
  `sill debug notify "Hi" --body "..." --action default=Open` on a debug build. It posts straight
  to sill's banners without D-Bus, so sill need not hold `org.freedesktop.Notifications`
  (`notifications.server = "passive"` leaves the name to the session's own daemon). The card
  follows a drag 1:1 to the right and damped to the left, springs back under 80 px and flies out
  past it or on a flick; after a click on one card, the next card under the pointer still pauses
  and shows its close button.
- [ ] **No grey block behind a moving banner** (at scale 2 on the 4K output, over a busy
  wallpaper): drag a banner right and hold, then let it fly; click one away. Nothing grey stands
  where the card was, during the motion or after it. `dev/accept/notifications.sh --real-input
  ghost` with `KWIN_SCALE=2` covers it headlessly.
- [ ] **Notification center rises and folds.** Each opening, the groups rise in turn under the
  panel's slide; "N more" enters the added cards and "Show less" folds them back; a new
  notification arriving while open enters without replaying the rest. With notifications from
  two apps, click the top group's Clear: its header and cards fold one after another, then the
  next group heals up. Clear All folds everything, and "No Notifications" shows once the rows
  have gone; Escape then closes the center. Automated by
  `dev/accept/notifications.sh --real-input exit`; by hand, the feel, side by side with macOS
  Sequoia's Notification Center.
- [ ] **Notification center keys with a real keyboard** (real Escape, Down and Delete already
  pass on a headless KWin): open it from the clock; Up/Down move the ring, Return opens a
  group's "N more" or invokes a lone card's default action, Backspace (the Mac keyboard's
  Delete) clears the focused row, Escape closes.
- [ ] **The capsule level look on the light scheme** (the fill reads faintly on its track) when
  picking the level look from quire's sheets.
- [ ] **Drag icon follows the real pointer**: run shell-host's `examples/demo_drag.rs
  --icon=root` and drag from its square: the icon follows the pointer the whole way (the
  headless KWin draws no cursor). The screenshot thumbnail's own drag is automated up to a drop
  with no target (`dev/accept/screenshot.sh --real-input`).
- [ ] **Launcher with a Mac keyboard through keycap** (real typing, Down, Space, Ctrl+Y, → twice,
  Tab, Escape three times, Ctrl+R, Shift+Tab to Emoji and Enter on a clipboard row already pass
  under fake input through `dev/accept/launcher.sh --real-input`; this is the physical keys and
  the look). Open the launcher with `sill launcher open` (Cmd+Space itself needs a sill (casement)
  login) and type part of a document's name: ⌘Y shows the preview, Space after an arrow toggles
  it and never types a space, → at the end of the text shows it and a second → puts the focus
  ring on its first button, Tab walks the buttons, Escape leaves the pane, hides it, then closes.
  ⌘R on a document opens the file manager with it selected. Hold Down through a long list (Tab
  to Documents): the selected row stays in view. In the emoji grid (Shift+Tab twice with
  nothing typed) all four arrows move and ⌘Y shows the emoji's preview; Enter then Cmd+V pastes
  the emoji in another app. By eye: the focus ring on the pane's buttons in the dark scheme, and
  the chord hint on the selected row only. If ⌘ chords act like a terminal's, keycap still thinks
  the terminal has focus (`keycap status` names the profile): note it.
- [ ] **A file from the launcher in the viewer** (needs `anyview` installed with its
  `org.quire.Anyview1.service` activation file, or already running). Type part of a document's
  name with other documents listed: the pane shows the viewer's own picture of it (a picture, a
  page, lines, a table, an archive's listing, a font's specimen); a picture and a PDF page must
  show on casement, where the pane's picture is a `TextureLayer` that the headless harness cannot
  draw (a blank plate there is a finding). Enter, or ⌘O, opens the viewer on that file, and ← and →
  there walk the rows the launcher listed, in its order. On an audio file the pane's "Play"
  starts it and the launcher closes. A file the viewer opened yesterday is found by its name
  (with "Page N" after the folder if it was left on one). With no viewer installed, Enter still
  opens the file in its default app.
- [ ] **Clipboard history in real apps**: copy text in two apps, then open the launcher and type
  a word of it: both show as Clipboard rows, and Enter puts one back (paste it somewhere; also
  an older row with Ctrl+V in a text editor and a terminal). Copy a password from KeePassXC (or
  another manager that marks copies): it never shows. `sill clipboard status` says `memory`,
  `ext`. Turning `launcher.clipboard_history` off empties it at once.
- [ ] **Dragging a desktop widget with a real pointer**: with `sill widgets toggle-desktop` on,
  press the Calendar and drag it: it lifts under the pointer and follows it, and a quiet slot
  guide shows the cell it will land in; held over the Batteries, they step aside and go back
  when you move on; released, the Calendar springs into its cell. Quit and restart sill: the
  place is kept. Click the month arrows and Now Playing's buttons without moving: they work; a
  drag starting on a button presses nothing.
- [ ] **Edit Widgets with a real pointer**: open the notification center and press "Edit
  Widgets" at its foot: the center closes and the gallery sheet rises at the bottom, one preview
  per widget. Pick World Clock, then "Add to Desktop": the button settles to a check and
  "Added", `widgets.desktop` turns on if it was off, and the clock rises into the first free
  place from the top right. Remove Up Next from the Notification Center list: "Removed Up Next
  from the Notification Center." and, on the desktop, a removed card plays its exit. Add Up Next
  to the center twice: the second is refused, its line shaken once. Escape, Done and a click
  outside the sheet each close it. Automated at scale 2 by `dev/accept/widgets.sh --real-input`;
  by hand only the feel is left.
- [ ] **The desktop's context menu with a real mouse**: with `sill widgets toggle-desktop` on,
  press the secondary button on the bare wallpaper: a menu opens with its corner at the pointer
  (and flips left or up near the right and bottom edges): Change Wallpaper..., Edit Widgets...,
  Show View Options. Release without moving: nothing is picked. Change Wallpaper... opens
  Settings on its Wallpaper page (the desktop entry `org.quire.Settings.Wallpaper`; the
  entry is absent from the menu on a machine without it). Edit Widgets... rises the sheet on the
  monitor you pressed on, not the others. Rest on Show View Options: its submenu opens below the
  menu (G365) with the widget size and spacing in force checked; Large grows the widgets at
  once, and the choice survives a restart. A secondary press while the menu is open closes it
  (on the real compositor the press ends the menu's grab) and a second press opens a new one;
  it must never end the session (a second popup made while one holds the grab is a protocol
  error). With two monitors, each keeps its own widget list: add World Clock on one, remove
  the Batteries on the other, and `sill widgets toggle-desktop` off and on. Automated at scale
  1 on two outputs by `dev/accept/widgets.sh --outputs`; by hand only the feel is left.
- [ ] **`sill widgets edit` on the focused monitor**: with a window active on the second
  monitor, run `sill widgets edit`: the sheet rises on that monitor (the
  active window's output, as the daemon knows it).

## Visual review

- [ ] **Icon styles in dark mode**: Colour, Muted and Monochrome in the dock
  (`dev/out/dock-icons-*.png` are light-mode only). A third-party raster in Muted or Monochrome
  goes through quire's dark tone band: check a black logo (Chrome's ring, a terminal) reads on
  its plate in the dark, and that nothing looks washed out.
- [ ] **The 24 bar and the control center by eye**: the bar at 24 with 22 items, 30-wide status
  slots with no gap, the control center's 22 capsule with its white knob, the 10/10 grid.
  Compare side by side with the reference screenshots.
- [ ] **Widgets by eye**: `sill widgets toggle-desktop`, then check the analog world-clock dials
  (set `widgets.world_clocks`, add World Clock in Edit Widgets) and the battery
  rings with real Bluetooth devices connected. The center's month is Medium (today beside the
  compact month, filling its tile); check it reads right, and whether the desktop's Small
  Batteries with two or three devices (a 2 x 2 grid, empty places as bare tracks, no numbers)
  reads as intended. A city on another date than yours says it under its name ("Sat 3 Oct"),
  and "Today" otherwise; check the two lines fit under each dial.
- [ ] **Screenshot thumbnail drag onto a real app**: show a thumbnail (take a screenshot, or
  `sill debug screenshot <a copy of a PNG>`), drag it onto a file manager window: the file
  copies there, the card leaves, the original stays; onto a terminal it inserts the path.

## Needs a real sill (casement) login (not the headless compositor)

- [ ] **Cmd+Space through keycap** opens the launcher (keycap writes Super+Space from the sill layer
  `~/.config/keycap/keycap.toml`; Alt+Space means the layer is missing and only `/etc/keycap/keycap.toml`, the
  Plasma overlay, applies).
- [ ] **Super shortcuts** reach the shell (the headless compositor never gets them).
- [ ] **A compositor shortcut runs `sill launcher toggle`**: bind Ctrl+Alt+Space to run
  `sill launcher toggle` (or use Cmd+Space above) and press it: the launcher opens, and a
  second press closes it. The compositor must resolve `sill` on its own `PATH` and the spawned
  client must reach the daemon's socket; no injected key reaches a compositor's shortcut matcher.
- [ ] **Scale policy on first sight**: a display seen for the first time gets 2x (BenQ MA270U),
  and a scale set by hand afterwards is never changed back. The desktop at 2x reads
  comfortably; text and icons are sharp.
- [ ] **Brightness keys on the external monitor** (DDC/CI, with the brightness shortcuts bound to
  `sill display brightness up|down`): the keys (or `sill display brightness up|down`) move the BenQ MA270U's backlight
  within about a quarter second, sill's OSD shows the level, no stuck value after fast repeats.
- [ ] **The Display capsule on the BenQ MA270U** (the pointer path passes headlessly at scale 2
  in `dev/accept/control-center.sh --real-input`): open the control center from the bar: within 2 s the
  Display capsule shows the monitor's level (sill's log says "sill: ddc: 1 display(s) answer
  DDC/CI"); drag it: the backlight follows during the drag, not only at the release; click the
  track: it jumps there. Change the brightness with the monitor's own buttons with the control
  center open: the capsule follows within 2 s. Put it back where it was afterwards.
- [ ] **Screenshot thumbnail and notification banners from the keyboard**: bind a shortcut to
  run `sill focus-latest`, keep typing in an app, take a screenshot
  (`sill screenshot area`): the thumbnail slides in and typing still goes to the app. Press the
  shortcut: the thumbnail takes the keyboard (its focus ring shows on the picture, the hold stops
  running out). Enter or Space opens the file, Delete or Backspace trashes it, Escape slides it
  away and keeps the file; Tab moves to its Delete button and back. After any of them typing
  reaches the app again. Then post two notifications from different apps
  (`notify-send -A default=Open -A reply=Reply mail one; notify-send chat two`): both slide in and
  typing still goes to the app. The shortcut puts the ring on the newest (a thumbnail that
  appeared after them takes it instead); its hold stops running out while the other still runs
  out. Down and Up move the ring between the two and stop at the ends, Enter runs the default
  action, Tab walks to its buttons (Enter presses one), Escape, Delete or Backspace closes it
  and the ring moves on to the other; closing the last gives typing back to the app. With none
  showing the shortcut does nothing.
- [ ] **Power menu with the real keyboard** (real Tab, Escape and a click outside the alert already pass
  on a headless KWin through `dev/accept/power-menu.sh --real-input`): `sill power-menu open`, then Tab
  moves the ring between the buttons, Esc closes. Do NOT press Return or Space on Restart/Shut Down unless you mean it; the
  dry run (`sill run --session-dry-run <file>` on a debug build) records instead.
- [ ] **Cmd+Tab** (with the window-switch shortcut bound to `sill switcher next|prev`): a quick Cmd+Tab tap switches to
  the previous app with no panel; holding shows the panel after about 150 ms; Tab moves;
  releasing Cmd switches; Q quits the selected app; H hides it; Esc cancels.
- [ ] **Control center details on real services, the rest**:
  - Join a saved Wi-Fi network from the Wi-Fi pane: after about 0.4 s a spinner replaces its
    lock, then its disc turns accent with one seal; a wrong password shakes the row once and
    says "Couldn’t connect". The bar's Wi-Fi glyph searches during the join and fills on
    joining, whether or not another connection is up; with a wired connection up, the join
    shows only in the control center's Wi-Fi list.
  - On a laptop with a keyboard backlight: the Keyboard Brightness module shows under Display,
    follows the laptop's own backlight keys, and a drag sets the nearest step.

## Needs a sill (casement) login

Install with `dist/install.sh` (`--dry-run` first; it builds and installs the binary, the PAM
service, the intents manifest, the skills and the settings schema, and never touches Plasma or
Toshy), then the session from the desktop repo. `docs/first-login.md` is the checklist for the
first real login; the boxes below are what it proves. Then pick "sill (casement)" at the login
screen.

- [ ] **GPU choice**: the journal line `shell-host: adapter <name> ...` names the GPU the
  compositor renders on, and `sill` is not at 100% CPU.
- [ ] **Key repeat** is 400 ms / 10 per second at once (hold a key).
- [ ] **Secrets**: `secret-tool search xdg:schema org.freedesktop.Secret.Generic` finds the
  Secret Service without a wallet prompt; Chrome and Electron apps stay signed in.
- [ ] **Workspace gestures**: with two workspaces, `sill workspace next` and `prev` move between
  them; at the ends nothing happens; the Magic Mouse two-finger swipes do the same (palmrest).
  `sill workspace overview` fails with "unsupported" (no overview yet).
- [ ] **The session comes up**: bar, dock, wallpaper; `systemctl --user is-active
  graphical-session.target` says `active`; `sill polkit status` says `registered`.
- [ ] **The power key** opens sill's power menu and does not power off (`systemd-inhibit --list`
  shows a `handle-power-key` block held for sill).
- [ ] **Log out** from the power menu ends the session back to the login screen (nothing left
  running: `loginctl list-sessions`).

### The lock screen

- [ ] **Lock**: Super+Escape, ⌃⌘Q, the launcher's Lock row and `loginctl lock-session` each show
  sill's lock screen once; the password unlocks it and the desktop is back as it was.
- [ ] **Lock before sleep**: `systemctl suspend`, wake: the lock screen is already up when the
  screen comes back (no flash of the desktop). While awake, `systemd-inhibit --list` shows
  `sill ... lock before sleep ... delay`.
- [ ] With several outputs: the password field is on the output under the pointer, every other
  output shows the clock only, and keyboard focus lands in the field. Clicking another output
  moves the field there, and typing goes to it.
- [ ] **A real password**: a wrong one spins, shakes and empties the field (about 2 s:
  `pam_unix`'s delay); the right one fades the lock screen out. Nothing about the password
  appears in `journalctl --user -t sill` or sill's stderr.
- [ ] **Caps Lock**: turn it on before locking; the mark shows in the field from the first
  frame, and goes when Caps Lock is turned off.
- [ ] **Six wrong passwords, no lockout**: every wrong one spins, shakes and empties the field;
  the field never closes, and the right password on the seventh try unlocks at once.
- [ ] At a fractional scale (1.5x): the lock surfaces are crisp and fill every output.
- [ ] Kill the shell while locked (`pkill -f "sill run"` from a TTY): the session stays locked,
  sill is started again, and sill re-locks at start so you can unlock normally.

### Idle and the lock grace

Set a short screen-off time in sill's idle settings (e.g. 60 s).
`dev/accept/idle.sh` covers the rules on casement with made-up wakes, since the
headless compositor counts no synthetic input as activity.

- [ ] `sill idle status` says `{"kind":"own"}`. After that
  long idle: `dimmed` 5 s before the screen-off time, every screen fading to half over about
  2 s (a key or the mouse during it brings them back at once, and a click during it lands on the
  window under it), then `displays_off`, and `sill lock status` says cause `idle_screen_off`.
- [ ] The screens are really off (dark, not black-but-lit) while the lock is granted, and nothing
  of the desktop shows when they come back. The lock screen, left alone, goes dark again after
  60 s.
- [ ] Let the screen sleep, wake it within 5 s (a key or the mouse): no password. After more
  than 5 s: the password is asked.
- [ ] Lock by hand (⌃⌘Q, Super+Escape, `loginctl lock-session`, the launcher's Lock row), wake at
  once: the password is asked.
- [ ] Play a video in a browser (Firefox or Chromium): the screen never dims or goes off while
  it plays (`sill idle status` shows `"inhibition":"held"` for a ScreenSaver inhibit), and does
  once it is paused.
- [ ] On battery, with a short suspend time (`idle.suspend_battery_min = 1`): the machine suspends; on resume the
  password is asked, even at once.
- [ ] Close the lid or suspend, resume at once: the password is asked.
- [ ] With `session.lock_grace_s = 0`, sleep and wake at once: the password is asked.

### The polkit agent

`dev/accept/polkit.sh` covers the prompt on the nested compositor against a stand-in Authority
and a stand-in helper; these need polkitd, polkit's own helper and a real password.

- [ ] `pkexec true` shows sill's prompt naming the action and you; the right password returns
  0 and the prompt hides.
- [ ] A wrong password shakes the field and empties it; three wrong tries end the request
  (`pkexec` returns 127, "Not authorized").
- [ ] Cancel, Escape and a click on the scrim each end `pkexec true` with 126.
- [ ] Caps Lock on shows the caps mark in the field.
- [ ] Two requests at once (`pkexec true & pkexec true`): the second shows once the first ends.

### keycap (the Mac keyboard layout)

`cargo test -p sill-services --test it keycap_socket` covers the socket against a fake daemon; these
need the real daemon on a machine with a Mac keyboard (or a PC keyboard with the Mac layout) and
root for the unit. Install keycap first (`install.sh` in the keycap repo, with its unit, config
folder and polkit rule `dist/keycap.polkit.rules`), run sill's `dist/install.sh` (it copies the
sill layer `dist/keycap/keycap.toml` to `~/.config/keycap/keycap.toml` when missing), then log in to a sill session.

- [ ] `sill keycap status` says `connected` with your keyboard listed and `grab` `active`.
- [ ] Cmd+Space opens the launcher and again closes it (Super+Space, not Alt+Space), and Cmd+Shift+4 starts
  `sill screenshot area`.
- [ ] Focus a terminal (foot, Alacritty): Cmd+C copies in it, and Ctrl+C still
  interrupts a running program. Focus a browser: Cmd+C copies there too. Moving between the two
  changes nothing visible and loses no keystroke.
- [ ] Add an override in `~/.config/sill/settings.toml` (`[[keyboard.overrides]]` with `app = "<the
  desktop-entry id of an app that is open>"` and `profile = "<a profile in keycap.toml>"`): within
  a second, that app's windows switch to the profile (a key the profile remaps behaves
  differently there) and other apps do not. Change the `profile`, then delete the row: each
  switches again. A row with `profile = "Bad Name"` is dropped and a `sill: settings.toml:` line
  in the daemon's log says so; the other rows still apply.
- [ ] Edit `~/.config/keycap/keycap.toml` (a rule on a key you can tell): the change applies
  within a second, no restart.
- [ ] `sudo pkill -KILL keycap` once: keys keep working, **no notification** appears (systemd
  brings it back in 200 ms; `sill keycap status` is `connected` again).
- [ ] Make it give up: run `for i in 1 2 3 4 5 6; do sudo pkill -KILL keycap; sleep 0.4; done`.
  After about 3 s a "Mac keyboard layout stopped" notification appears, the keys work as on a
  standard keyboard, and `sill keycap status` says `failed`. `sill keycap retry` (no password
  asked) brings it back: the notification goes and the layout works again.
- [ ] `sudo systemctl stop keycap`: nothing is posted (stopped on purpose); `sudo systemctl start
  keycap` and the layout is back.
- [ ] `sudo systemctl mask --now keycap`: nothing is posted and `sill keycap status` says `not_installed`
  or `unknown`; `sudo systemctl unmask keycap && sudo systemctl start keycap` restores it.
- [ ] Start another remapper that grabs the same keyboard (`keyd`, or `kmonad`): an "Another keyboard
  program is running" notification names the keyboard; stopping the other program clears it
  within about 30 s, and keycap grabs the keyboard.
- [ ] Press both Shifts and Esc: a "Mac keyboard layout paused" notification appears and the
  keys work as on a standard keyboard. `sill keycap resume` brings the layout back and the
  notification goes.
- [ ] Log out and in again: the login screen already has the layout, and after login the terminal
  and the browser behave as in the second item.

## The focused app's menus in the bar (a sill login, a quire app that exports its menu)

- [ ] **detent's Edit menu shows in the bar**: focus detent (built with the menu export on); the bar
  reads detent's name in bold, then File, Edit, View, Window and Help. Press Edit: Undo and Redo list
  with their Cmd keys; hover across File, a tray item and Sound switches the one popup with no gap;
  pick a row and detent runs it. A window that is not a quire app shows only its name's menu (Hide,
  Quit, its windows).

## The confirmation sheet and `quire-do` on a real sill (casement) login

Needs docket's `intentd` running (`systemctl --user start intentd` from docket's `dist/`) and
`quire-do` on the PATH; `dist/install.sh` has put `org.quire.Shell.toml` where intentd reads it.
`dev/accept/confirm.sh` runs the whole chain on a private headless KWin; these are the parts that
need a person, a real compositor or real eyes.

- [ ] **The sheet's look**: `quire-do shell dock.pin --app firefox.desktop` in a terminal. A
  narrow sheet appears centred on the main display: "Keep an app in the dock?", "The terminal wants
  to do this in Shell.", the app as a fact, the reasons, "Allow from the terminal: this action,
  until logout", Don't Allow and Allow. The buttons are greyed for about half a second; Return in
  that half second does nothing; afterwards Return allows. Compare with
  `dev/out/confirm.png` from the acceptance run. Dark and light.
- [ ] **Hold to confirm**: `quire-do shell lock`. The Allow button reads "Hold to Allow"; a click does
  nothing; holding it with the mouse for a second locks the screen (the label says
  "Keep holding" meanwhile); letting go early abandons it. From the keyboard, Tab to it and hold
  Return or Space for a second; a tap does nothing. The keyboard starts on Don't Allow.
- [ ] **Escape and the scrim**: Escape denies at once, also in the first half second; the terminal
  exits 4.
- [ ] **A second question waits**: run two `quire-do shell dock.pin` calls back to back: the second
  shows "1 more waiting" under the first, and appears (arming again) after the first is answered.
- [ ] **Control center, Terminal**: after "Allow from the terminal" a Terminal module appears at
  the foot of the control center listing the action with Revoke. Revoke it: the row goes and the
  next `quire-do` call asks again. (The module is absent while nothing is granted.)
- [ ] **A paused terminal**: three refused calls in a row (`quire-do memory forget memory.fact:x`,
  exit 3) pause it; the next call exits 7. Intentd does not yet announce the pause
  (`BreakerTripped` is declared and not emitted), so the Terminal module shows nothing until a grant
  exists; `sill intents resume` is the way back today.
- [ ] **The keyboard still reaches the window behind**: after answering, typing reaches the
  terminal again (the sheet's `Exclusive` keyboard is released when it leaves).
- [ ] **Phase A is forgeable**: a key typed by uinput answers the sheet. Known; Phase B closes it.

## The companion (a sill session with intentd, companiond, cuad and memoryd installed)

The service, the machines and the launcher's mode are tested on stand-ins and fakes; these need
the real daemons or a real keyboard.

- [ ] **Double tap, route B** (a lone Cmd tap mapped to the unique chord that runs `sill companion
  tap`; Toshy did this and keycap does not emit that chord yet, so this waits for a sender): tap Cmd twice within `companion.double_tap_ms` with the launcher closed: the launcher opens as a
  prompt ("Ask the companion", no rows). Cmd-click and Cmd-drag in a terminal and a browser are
  unharmed (spike T). One tap does nothing; two taps a second apart do nothing.
- [ ] **Double tap with the launcher open in search**: type `mail` in the launcher, tap Cmd twice: the
  same panel (no remount, no flash) becomes the prompt; the field is empty and the rows are the
  companion's. Escape puts `mail` back in the search with its rows. Tap Cmd twice again from a fresh
  prompt: it closes.
- [ ] **A summon over an app that serves the companion** (mailo with docket-ds): with mailo focused,
  tap Cmd twice: mailo's own search field becomes the prompt and the launcher stays shut; with a
  window that is not a quire app, the launcher opens instead, within a blink of `app_wait`.
- [ ] **Ask from the launcher** (flows (a) and (b), real companiond): ask "what is on my calendar
  today": the answer's card shows in the pane as it streams, the field empties, the selection moves to
  the answer; Escape returns to the Ask row; Command-Enter on a card with an "Open" button opens the
  app and closes the panel. Needs quire's `AnswerCard` fill; until then the pane's card is an empty
  placeholder and only the row's words and buttons show.
- [ ] **Command-period** in the prompt: stops the answer being made; on a run's row (Tab to Runs) it
  stops the run. `sill companion stop-all` pauses every run that is going and leaves paused and ended
  ones alone.
- [ ] **A run's row** (a real cua run): the row follows the run's phase; "Take Over" and "Hand Back"
  change the state; "Watch" does nothing from sill (the compositor's) and must not close the panel.
- [ ] **Undo**: after a companion action, the Companion filter lists it under recent activity with
  "Undo"; Enter undoes it and the row reads "Undone".
- [ ] **Memory filter**: Tab to Memory: the newest events read "You archived ...", newest first, each
  row's app below.

## Keys as shared actions (keycap's keymap)

- [ ] **Launcher keys on a live session**: with a window focused whose keycap profile is `default`
  (Cmd written as Ctrl), open the launcher and press Cmd+K, Cmd+R, Cmd+Y, Cmd+B and Cmd+Backspace
  on rows that carry them. The launcher resolves Super (Command) through the keymap only, so each
  must work. While a sill surface holds the keyboard sill tells keycap `keymap-aware` (and the
  window's profile again when it lets go): check in keycap's log that the
  profile flips on open and back on close, and that Cmd+Tab still switches apps while the launcher is up.
- [ ] **Dock Cmd/Alt+click**: Cmd+click on a running tile raises it with the other windows hidden as
  before; Alt+click hides the current one. The dock reads Command as the keymap's primary modifier,
  which is Super on our desktop; if keycap rewrites Cmd to Ctrl for the focused window the click
  reads as plain.
- [ ] **temor** (profile `keymap-aware`): Cmd+H reaches temor as Super+H (hide), not Ctrl+H.
- [ ] **Companion answer text**: a handle that holds mail or web text shows between curly quotes; one
  the person typed shows plain; against an older intentd every handle is quoted.
- [ ] **Screen-sharing picker**: from an app that shares (a browser's "share screen"), the sheet
  appears centred on the main display with Screens and Windows when both are allowed; arrows move,
  Space picks, Return shares, Escape cancels; Share stays dim until something is picked; a second
  request while one is open is refused; the window list matches the open windows (needs casement's
  identifiers).
- [ ] **Mac level sliders** (quire design/29 section 16): in the control center the Display, Sound and
  Keyboard Brightness sliders are a 22 high well under a round white knob that holds the glyph
  (sun, speaker with waves, keys); a click on the well jumps the knob, a drag moves it, the arrows
  step a sixteenth (Shift a sixty-fourth); disabled keeps the knob, dimmed. The volume and
  brightness OSD shows sixteen squares under the title (the card is still 296 x 80; check the squares
  are not cramped or the card too tall).

## Owned by another repository

- [ ] shell-host spike item f at a fractional output (1.5): run on casement at
  1.5x; the KWin run at 2.0 left `screen_row_equals_buffer_row` false.

## casement (tty session), first-login findings 2026-10-15

- [ ] **VT switch**: in the casement session press Ctrl+Alt+F3. The screen goes to VT 3 (a login
  prompt). Switch back with Ctrl+Alt+F1 or F2 (whichever VT holds the session), and the desktop
  returns drawn and responsive. Try it on the Keychron both with Fn held (F-row as F-keys) and in
  its media-key mode, where plain Ctrl+Alt+F3 only works if the keyboard sends F3. The chord must not
  reach the focused app.
- [ ] **Multi-GPU rendering**: with the NVIDIA card driving the monitors and the AMD iGPU present,
  windows appear on every monitor and the log has no "Failed to create EGLImage" or BAD_MATCH lines.
  Drag a video or a game window and confirm it is smooth. If `gpu.render` is set to the iGPU's PCI
  address, windows are expected to be blank on the NVIDIA monitors until the copy path lands (known gap,
  FINDINGS wave-2026-10-15/session).
- [ ] **Scroll direction**: two-finger scroll on the touchpad and the wheel on a mouse (and a
  trackball if there is one) both move the content with the fingers or wheel the way the Mac does
  (as on macOS with natural scrolling on). Turn "Natural scrolling"
  off for Mouse in Settings, replug the mouse (settings apply when a device appears), and the wheel
  reverses while the touchpad does not.

## Mission Control's Spaces bar (lanes spacesbar, mc2 and mc4, not built or run when written)

- [ ] **Spaces bar**: in the casement session open Mission Control (the key, the hot corner or the gesture). A row of the output's Spaces
  drops in across the top, under the menu bar, over the spread windows: each Space is a 160 x 100 card with its name under it
  ("Desktop 1", "Desktop 2", and the apps' names for a full-screen Space), the current Space ringed in the accent, and a plus at the
  right edge. Every card, the hidden Spaces' too, has a picture of its Space's windows over the
  output's wallpaper (the wallpaper is the one on the desktop, light or dark as the desktop is); a Space with no windows shows the bare
  wallpaper. Switching Space with Mission Control open keeps the pictures. The expanded bar is taller than the 112 px the windows leave free, so its lower part lies over the top of the window
  tiles; a click on a window tile below the bar still reaches it, and a click on the bar's own box does not fall through. Closing Mission
  Control lifts it away and it is gone (no stuck strip, `sill debug stats` shows the surface unmapped). A click on a card switches
  to that Space and leaves Mission Control; the plus adds a Desktop and stays in Mission Control. App Expose and Show Desktop show no
  bar. With two monitors the bar is on the monitors Mission Control is up on, each with its own Spaces.
- [ ] **Drag a window onto a Space**: in Mission Control drag a window tile up onto a card. The card under the dragged window lights
  (accent wash, ringed card) and the window moves to that Space on release. The lit card is the one the window is over, not a
  neighbour (the card rectangles are worked out from the bar's box and quire's card size: if they are off, FINDINGS `mc2`).
- [ ] **Drag a window onto the plus**: in Mission Control drag a window tile up onto the plus at the bar's right edge. The plus lights
  (accent wash and ring) while the window is over it, and no card is lit at the same time; releasing makes a new Desktop on that
  output, moves the window to it and stays in Mission Control, where the new card appears with its picture. Moving the window off the
  plus puts the light out. The plus is a 28 px square, small to hit: if it is off, FINDINGS `mc4`. Dragging between two monitors works
  and each monitor's bar lights its own targets.
- [ ] **Wallpaper under pictures**: with a wallpaper set, each card shows it behind the windows, filling the card; with a different
  wallpaper per monitor each bar shows its own monitor's. Windows in the picture line up with the wallpaper (a small offset on
  an output whose shape is not 16:10 is expected, FINDINGS `mc4`).
- [ ] **Picture refresh**: with Mission Control open, close a window on the current Space: the card's picture follows within a
  moment and the shell does not use a core while the bar sits still.

## The colour picker's eyedropper (casement session, an app that asks for a colour)

- [ ] **Pick a colour**: in an app that asks for a colour from the screen (a colour well's eyedropper, a portal `PickColor`
  caller), the screen holds still and a round magnifier follows the pointer, the pixels around it enlarged with the one under
  the pointer ringed in the middle. A click gives the app that pixel's colour (compare with a screenshot of the same spot) and
  the magnifier goes. The magnifier is round, its pixels are hard-edged squares, and it fades in when it first appears.
- [ ] **Cancel**: Escape, or a click with the other button, gives the app no colour and the screen is live again.
- [ ] **Two monitors**: each monitor holds still under its own picture and the magnifier follows the pointer onto either one;
  a pick on the second monitor reads that monitor's pixel.
- [ ] **A fractional scale** (150 %): the pixel picked is the one the middle of the magnifier shows, not a neighbour.
- [ ] **No cursor over the magnifier**: the arrow is hidden while the magnifier is up (FINDINGS `wave-6/colour-picker`, if it is
  drawn over the middle cell shell-host does not map `cursor: none`).
- [ ] **Real hardware**: casement's tty backend has no capture read-back yet, so on a real screen the app is answered with no
  colour at once and nothing is drawn (FINDINGS `wave-6/colour-picker`, known limits).
