# Manual checks

Checks that need a person at the machine. They are assumed to pass until someone runs them;
each has the command to start it and what to look for. Tick the box and note the date when done.
Add new ones at the end of their section.

## On a KWin session (test compositor, real devices)

KWin is not a target, but a KWin session at the machine is the quickest place to try sill's
surfaces with real devices: its own bar and dock over the desktop, BlueZ, MPRIS, the clipboard
(`ext_data_control_manager_v1`) and real pointer and keyboard. KWin lacks the COSMIC protocols
(toplevels, workspaces, output management, session lock), so the services that need them stay
at their defaults there; anything about those, or about displays and scale, is in the COSMIC
sections below. The monitor is the BenQ MA270U (27", 3840 x 2160).

- [ ] **CJK input in the launcher, a short real-keyboard spot check.** Automated by shell-host's
  `SILL_DIR=~/sill ~/shell-host/scripts/dev/accept-ime.sh` (fcitx5 Chewing on a private headless
  KWin; the field reads 你好). By hand, once: `bash dev/manual/ime-live.sh start`, click into
  the search field, switch fcitx5 to Chewing, type `su3cl3` and press **Return** to commit
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
- [ ] **Launcher with a Mac keyboard through Toshy** (real typing, Down, Space, Ctrl+Y, → twice,
  Tab, Escape three times, Ctrl+R, Shift+Tab to Emoji and Enter on a clipboard row already pass
  under fake input through `dev/accept/launcher.sh --real-input`; this is the physical keys and
  the look). Open the launcher with `sill launcher open` (Cmd+Space itself needs a COSMIC
  login) and type part of a document's name: ⌘Y shows the preview, Space after an arrow toggles
  it and never types a space, → at the end of the text shows it and a second → puts the focus
  ring on its first button, Tab walks the buttons, Escape leaves the pane, hides it, then closes.
  ⌘R on a document opens the file manager with it selected. Hold Down through a long list (Tab
  to Documents): the selected row stays in view. In the emoji grid (Shift+Tab twice with
  nothing typed) all four arrows move and ⌘Y shows the emoji's preview; Enter then Cmd+V pastes
  the emoji in another app. By eye: the focus ring on the pane's buttons in the dark scheme, and
  the chord hint on the selected row only. If ⌘ chords act like a terminal's, Toshy still thinks
  the terminal has focus: note it.
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

## Visual review

- [ ] **Icon styles in dark mode**: Colour, Muted and Monochrome in the dock
  (`dev/out/dock-icons-*.png` are light-mode only). A third-party raster in Muted or Monochrome
  goes through quire's dark tone band: check a black logo (Chrome's ring, a terminal) reads on
  its plate in the dark, and that nothing looks washed out.
- [ ] **The 24 bar and the control center by eye**: the bar at 24 with 22 items, 30-wide status
  slots with no gap, the control center's 22 capsule with its white knob, the 10/10 grid.
  Compare side by side with the reference screenshots.
- [ ] **Widgets by eye**: `sill widgets toggle-desktop`, then check the analog world-clock dials
  (set `widgets.world_clocks`, add World Clock to `widgets.desktop_widgets`) and the battery
  rings with real Bluetooth devices connected. The center's month is Medium (today beside the
  compact month, filling its tile); check it reads right, and whether the desktop's Small
  Batteries with two or three devices (a 2 x 2 grid, empty places as bare tracks, no numbers)
  reads as intended.
- [ ] **Screenshot thumbnail drag onto a real app**: show a thumbnail (take a screenshot, or
  `sill debug screenshot <a copy of a PNG>`), drag it onto a file manager window: the file
  copies there, the card leaves, the original stays; onto a terminal it inserts the path.

## Needs a real COSMIC login (not the nested compositor)

- [ ] **Cmd+Space through Toshy** opens the launcher (the `Launcher` system action override).
- [ ] **Super shortcuts** reach the shell (the nested compositor never gets them).
- [ ] **A compositor shortcut runs `sill launcher toggle`**: bind Ctrl+Alt+Space to
  `Spawn("sill launcher toggle")` (or use Cmd+Space above) and press it: the launcher opens, and a
  second press closes it. cosmic-comp must resolve `sill` on its own `PATH` and the spawned client
  must reach the daemon's socket; no injected key reaches a compositor's shortcut matcher.
- [ ] **Scale policy on first sight**: a display seen for the first time gets 2x (BenQ MA270U),
  and a scale set by hand afterwards is never changed back. The desktop at 2x reads
  comfortably; text and icons are sharp.
- [ ] **Brightness keys on the external monitor** (DDC/CI, with `dist/cosmic/.../system_actions`
  installed): the keys (or `sill display brightness up|down`) move the BenQ MA270U's backlight
  within about a quarter second, sill's OSD shows the level, no stuck value after fast repeats.
- [ ] **The Display capsule on the BenQ MA270U** (the pointer path passes headlessly at scale 2
  in `dev/accept/control-center.sh --real-input`): open the control center from the bar: within 2 s the
  Display capsule shows the monitor's level (sill's log says "sill: ddc: 1 display(s) answer
  DDC/CI"); drag it: the backlight follows during the drag, not only at the release; click the
  track: it jumps there. Change the brightness with the monitor's own buttons with the control
  center open: the capsule follows within 2 s. Put it back where it was afterwards.
- [ ] **Screenshot thumbnail and notification banners from the keyboard**: bind a shortcut to
  `sill focus-latest` in COSMIC Settings > Keyboard > Custom Shortcuts (or
  `Spawn("sill focus-latest")` in the `custom` file), keep typing in an app, take a screenshot
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
- [ ] **Cmd+Tab** (with `dist/cosmic` system_actions installed): a quick Cmd+Tab tap switches to
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

## Needs a sill login (`dist/sill-session`)

Install first (as root; nothing installs these for you): `install -m 755 dist/sill-session
/usr/local/bin/sill-session`, `install -m 644 dist/sill.desktop
/usr/share/wayland-sessions/sill.desktop`, `install -m 644 dist/sill-session.target
/usr/lib/systemd/user/sill-session.target`, `install -m 644 dist/pam.d/sill /etc/pam.d/sill`
(without it sill uses the `login` service), and sill itself on `PATH` (`cargo install --path
crates/sill` or a copy in `/usr/local/bin`). Copy `dist/cosmic/.../system_actions` to
`~/.config/cosmic/com.system76.CosmicSettings.Shortcuts/v1/` so the power key and
Super+Shift+Esc open sill's power menu. Then pick "COSMIC (sill)" at the login screen.

- [ ] **The session comes up**: bar, dock, wallpaper; `systemctl --user is-active
  graphical-session.target` says `active`; `sill polkit status` says `registered`.
- [ ] **The power key** opens sill's power menu and does not power off (`systemd-inhibit --list`
  shows `sill-session` holding `handle-power-key`).
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
  the session script restarts sill, and sill re-locks at start so you can unlock normally.

### Idle and the lock grace

Set a short screen-off time in COSMIC Settings (Power) or
`~/.config/cosmic/com.system76.CosmicIdle/v1/screen_off_time` (milliseconds, e.g. `60000`).
`dev/accept/idle.sh` covers the rules on the nested compositor with made-up wakes, since the
nested cosmic-comp counts no synthetic input as activity.

- [ ] `sill idle status` says `{"kind":"own"}` (`pgrep cosmic-idle` finds nothing). After that
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
- [ ] On battery, with a short suspend time (`idle.times = "sill"` and
  `idle.suspend_battery_min = 1`, or COSMIC's Power page): the machine suspends; on resume the
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

`cargo test -p sill-services --test keycap_socket` covers the socket against a fake daemon; these
need the real daemon on a machine with a Mac keyboard (or a PC keyboard with the Mac layout) and
root for the unit. Install keycap first (`install.sh` in the keycap repo, with its unit, config
folder and polkit rule `dist/keycap.polkit.rules`), then log in to a sill session.

- [ ] `sill keycap status` says `connected` with your keyboard listed and `grab` `active`.
- [ ] Focus a terminal (foot, Alacritty, COSMIC Terminal): Cmd+C copies in it, and Ctrl+C still
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

## Owned by another repository

- [ ] shell-host spike item f at a fractional output (1.5): run on the nested compositor at
  `cosmic@1.5`; the KWin run at 2.0 left `screen_row_equals_buffer_row` false.
