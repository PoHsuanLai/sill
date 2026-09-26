# Manual checks

Checks that need a person at the machine. They are assumed to pass until someone runs them;
each has the command to start it and what to look for. Tick the box and note the date when done.
Add new ones at the end of their section.

## On the KDE session (unlocked, at the machine)

- [ ] **CJK input in the launcher** (fcitx5 Chewing over text-input-v3; KWin has no virtual
  keyboard, so nothing can type for you). Run
  `bash dev/manual/ime-live.sh start`, click into the search field, switch fcitx5 to Chewing,
  type `su3cl3` and commit: 你好 should appear in the field. Then `bash dev/manual/ime-live.sh capture`
  (writes the capture path) and `bash dev/manual/ime-live.sh stop`. Also try a preedit that is
  deleted before commit (Backspace mid-word): no stray characters left behind.
- [ ] **Scale 2 look** on the MA270U: the desktop at 2x (the new default for 27" 4K) reads
  comfortably; text and icons sharp.
- [ ] **Brightness keys on the external monitor** (DDC/CI): after installing
  `dist/cosmic/.../system_actions` in a COSMIC session, or with `sill display brightness up|down`
  bound to keys: the MA270U's backlight follows within about a quarter second; no stuck value
  after fast repeats.
- [ ] **Monitor speakers follow the volume**: switch the default sink to the MA270U's HDMI
  audio, press volume up/down: the monitor's own volume moves too (its range is 0-50), and mute
  mutes it. Switch back to Bluetooth afterwards.

- [ ] **Power menu with the real keyboard** (real arrows, Escape and a scrim click already pass on a headless KWin through `dev/accept-real-input.sh`; this check is for a real COSMIC session) (the nested compositor never gave its Exclusive
  layer keyboard focus, shell G30): on a real COSMIC session, `sill power-menu open`, then
  arrows move the highlight, Esc closes. Do NOT press Enter on Restart/Shut Down unless you mean
  it; the dry run (`SILL_POWER_MENU_DRY_RUN=<file>` on a debug build) records instead.
- [ ] **Control center with the real keyboard** (real click and Escape already pass on a headless KWin; Tab/arrows between tiles still need a person): open it from the bar item (left of the clock);
  Tab/arrows move between tiles, Enter/Space toggle, Right on a chevron opens Wi-Fi/Bluetooth,
  Escape goes back then closes (a debug-opened popup gets no keyboard on the nested compositor).
- [ ] **Banners with a real mouse** (hover, the close button, a click, a 90 px and a 50 px drag,
  already pass under fake input on a headless KWin through
  `dev/accept-notifications-real-input.sh`; this is the feel with a real mouse and touchpad):
  `sill debug notify "Hi" --body "..." --action default=Open` on a debug build, in a session
  where sill holds `org.freedesktop.Notifications`. The card follows a drag 1:1 to the right and
  damped to the left, springs back under 80 px and flies out past it or on a flick; after a click
  on one card, the next card under the pointer should still pause and show its close button
  (FINDINGS G101 says it does not until the pointer leaves the column and comes back).
- [ ] **Notification center keys with a real keyboard** (real Escape, Down and Delete already
  pass on a headless KWin): open it from the clock; Up/Down move the ring, Return opens a
  group's "N more" or invokes a lone card's default action, Backspace (the Mac keyboard's
  Delete) clears the focused row, Escape closes.
- [ ] **The capsule level look on the light scheme** (quire Q105: the fill reads faintly on its
  track) when picking the level look from quire's sheets.

- [ ] **Drag icon follows the pointer** (shell-host F60): on a real session, run shell-host's
  `examples/demo_drag.rs --icon=root` and drag; the icon should follow the pointer (the headless
  KWin draws no cursor, so it could not be seen there). Then drag the screenshot thumbnail out
  onto a folder once sill wires `start_drag`.

## Visual review

- [ ] **Shipped icons, app table**: the notes row (COSMIC Edit, GNOME Text Editor, Kate) and the
  photos row (Loupe, Eye of GNOME, Gwenview, Shotwell) in
  `crates/sill-surfaces/src/app_icons/shipped.rs` `DEFAULT_TABLE`; add or remove apps.
- [ ] **Icon styles in dark mode**: Colour, Muted and Monochrome in the dock
  (`dev/out/dock-icons-*.png` are light-mode only). The plate tint under Monochrome lands with
  quire's IconView plate-tint change.

- [ ] **Widgets by eye**: `sill widgets toggle-desktop`, then check the analog world-clock dials
  (set `widgets.world_clocks`, add World Clock to `widgets.desktop_widgets`) and the battery rings
  with real Bluetooth devices connected (no live picture exists; render tests only).
- [ ] **Screenshot thumbnail drag**: take a screenshot, drag the thumbnail onto a Files window or
  a terminal: the file copies (the card leaves, the file stays); only Cancelled was run live.

## Needs a real COSMIC login (not the nested compositor)

- [ ] **Cmd+Space through Toshy** opens the launcher (the `Launcher` system action override).
- [ ] **Super shortcuts** reach the shell (the nested compositor never gets them; KDE eats them).
- [ ] **Scale policy on first sight**: a display seen for the first time gets 2x (MA270U), and a
  scale set by hand afterwards is never changed back.

## Needs a sill login (`dist/sill-session`, M7)

Install first (as root; nothing installs these for you): `install -m 755 dist/sill-session
/usr/local/bin/sill-session`, `install -m 644 dist/sill.desktop
/usr/share/wayland-sessions/sill.desktop`, `install -m 644 dist/sill-session.target
/usr/lib/systemd/user/sill-session.target`, and sill itself on `PATH` (`cargo install --path
crates/sill` or a copy in `/usr/local/bin`). Copy `dist/cosmic/.../system_actions` to
`~/.config/cosmic/com.system76.CosmicSettings.Shortcuts/v1/` so the power key and Super+Shift+Esc
open sill's power menu. Nothing needs to be installed from the distribution on this machine:
cosmic-greeter (the locker), cosmic-idle and KDE's polkit agent are already here. Optional:
`sudo dnf install swaylock` for a second locker to compare (`session.locker = "swaylock"`).
Then pick "COSMIC (sill)" at the login screen.

- [ ] **The session comes up**: bar, dock, wallpaper; `sill polkit-agent print` names
  `/usr/libexec/kf6/polkit-kde-authentication-agent-1`; `systemctl --user is-active
  graphical-session.target` says `active`.
- [ ] **Lock**: Super+Escape, the launcher's Lock row, and `loginctl lock-session` each show
  COSMIC's lock screen once (never two); the password unlocks it and the desktop is back as it
  was. `journalctl --user -t sill` (or sill's stderr) shows no locker errors.
- [ ] **Lock before sleep**: `systemctl suspend`, wake: the lock screen is already up when the
  screen comes back (no flash of the desktop). While the machine is awake, `systemd-inhibit
  --list` shows `sill ... lock before sleep ... delay`.
- [ ] **Idle lock**: set a short screen-off time in COSMIC Settings (Power) or
  `~/.config/cosmic/com.system76.CosmicIdle/v1/screen_off_time` (milliseconds, e.g. `60000`);
  after that long idle the screen fades out and is locked when it comes back.
- [ ] **A polkit prompt**: `pkexec true` shows KDE's password dialog; the right password returns
  0, Cancel returns 126.
- [ ] **The power key** opens sill's power menu and does not power off (the handle-power-key
  inhibitor is held: `systemd-inhibit --list` shows `sill-session`).
- [ ] **Log out** from the power menu ends the session back to the login screen (nothing left
  running: `loginctl list-sessions`).

## sill's own lock screen (M11), on a real COSMIC session

Set `session.locker = "sill"` and install `dist/pam.d/sill` as `/etc/pam.d/sill` (without it
sill uses the `login` service). `dev/accept-lock.sh` covers the made-up verdicts on the nested
compositor; these need a person and a real password.

- [ ] With several outputs: the password field is on the output under the pointer, every other
  output shows the clock only, and keyboard focus lands in the field. Clicking another output
  moves the field there, and typing goes to it. (The field starts on the output the compositor
  gives the keyboard, which cosmic-comp takes from the pointer; confirm that here.)
- [ ] **A real password**: a wrong one spins, shakes and empties the field (about 2 s:
  `pam_unix`'s delay); the right one fades the lock screen out and the desktop is back as it was.
  Nothing about the password appears in `journalctl --user -t sill` or sill's stderr.
- [ ] **Caps Lock**: turn it on before locking; the mark shows in the field from the first
  frame, and goes when Caps Lock is turned off.
- [ ] **Six wrong passwords** (five free tries): the field closes and says "Try again at HH:MM",
  one minute on; a new lock in that minute keeps the wait.
- [ ] **Lock before sleep** and **logind's Lock** (`loginctl lock-session`) show sill's own lock
  screen once, never two lockers.
- [ ] At a fractional scale (1.5x): the lock surfaces are crisp and fill every output.
- [ ] Kill the shell while locked (`pkill -f "sill run"` from a TTY): the session stays locked,
  the session script restarts sill, and sill re-locks at start so you can unlock normally.

## sill's own polkit agent (M11), on a sill login

`dev/accept-polkit.sh` covers the prompt on the nested compositor against a stand-in Authority
and a stand-in helper; these need polkitd, polkit's own helper and a real password.

- [ ] With `session.polkit_agent = "sill"` (and no other agent started for the session),
  `sill polkit status` says `registered`.
- [ ] `pkexec true` shows sill's prompt naming the action and you; the right password returns
  0 and the prompt hides.
- [ ] A wrong password shakes the field and empties it; three wrong tries end the request
  (`pkexec` returns 127, "Not authorized").
- [ ] Cancel, Escape and a click on the scrim each end `pkexec true` with 126.
- [ ] Caps Lock on shows the caps mark in the field.
- [ ] Two requests at once (`pkexec true & pkexec true`): the second shows once the first ends.

- [ ] **Cmd+Tab on a real login** (with dist/cosmic system_actions installed): a quick Cmd+Tab
  tap switches to the previous app with no panel; holding shows the panel after about 150 ms;
  Tab moves; releasing Cmd switches; Q quits the selected app; H hides it; Esc cancels.

- [ ] **Clipboard history on a real login** (M9 lane b): copy text in two apps, then open the
  launcher and type a word of it: both show as Clipboard rows, and Enter puts one back (paste it
  somewhere). Copy a password from KeePassXC (or another manager that marks copies): it never
  shows. `sill clipboard status` says `memory`, `ext`. Turning `launcher.clipboard_history`
  off empties it at once.

- [ ] **Launcher v2 with a Mac keyboard through Toshy** (M9 lane e; real typing, Down, Space,
  Ctrl+Y, → twice, Tab, Escape three times, Ctrl+R, Shift+Tab to Emoji and Enter on a clipboard
  row already pass under fake input on a headless KWin through
  `dev/accept-launcher-v2-real-input.sh`; this is the physical keys and the look). Open the
  launcher with Cmd+Space and type part of a document's name: ⌘Y shows the preview, Space after
  an arrow toggles it and never types a space, → at the end of the text shows it and a second →
  puts the focus ring on its first button, Tab walks the buttons, Escape leaves the pane, hides
  it, then closes. ⌘R on a document opens the file manager with it selected. Hold Down through
  a long list (Tab to Documents): the selected row should stay in view (FINDINGS Q340 says it
  does not yet). In the emoji grid (Shift+Tab twice with nothing typed) all four arrows move
  (fixed, F760) and ⌘Y shows the emoji's preview, and Enter then Cmd+V pastes the emoji in
  another app. Also by eye: the focus ring on the pane's buttons in the dark scheme, and `↵`
  following the selected row (F761).
- [ ] **A clipboard row pasted into a real app** (M9 lane e): on KWin, real Enter on a clipboard
  row was proven with `wl-paste` only; on a real login, Enter on an older clipboard row, then
  Ctrl+V in a text editor and in a terminal, pastes that entry.

## Owned by the other session (quire, shell-host)

- [ ] shell-host G100 and G101 (sill FINDINGS "M6 lane d"): `dev/accept-notifications-real-input.sh
  repro` passes both steps once shell-host moves the document's pointer on `wl_pointer.enter`
  and clears its hover when a surface is unmapped.
- [ ] shell-host spike item f at a fractional output (1.5): run on the nested compositor at
  `cosmic@1.5`; the KWin run at 2.0 left `screen_row_equals_buffer_row` false (shell-host F45).
- [ ] D1 G302 (sill FINDINGS "D1 status glyphs"): on the real session, join a saved Wi-Fi network
  from the control center and watch `sill debug` / the bar. Does `Network.link` ever read
  `Connecting` (NetworkManager's `PrimaryConnection` activating), or does the join only show
  in `ActivatingConnection`? If it only shows there, the network service should read it. Once
  Q390 lands, check that the bar's Wi-Fi glyph searches during the join and fills on joining.
