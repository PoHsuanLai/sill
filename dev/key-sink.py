#!/usr/bin/env python3
"""A plain GTK 4 window that stands in for the app a person is typing into: it takes the
keyboard focus when clicked (as a real app window does on KDE) and prints every key it receives,
one `key <name>` line each, and `focus in` / `focus out` as its window gains and loses the
keyboard. dev/accept-menus-keyboard-real-input.sh runs it on the headless KWin so a shell surface
opened from the bar must take the keyboard away from a focused window, as on the user's session.
"""
import sys
import time

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Gdk", "4.0")
from gi.repository import Gdk, GLib, Gtk  # noqa: E402


def say(line):
    print(f"{time.monotonic():.3f} {line}", flush=True)


def main():
    Gtk.init()
    loop = GLib.MainLoop()
    window = Gtk.Window(title="sill key sink")
    window.set_default_size(800, 500)
    keys = Gtk.EventControllerKey()

    def pressed(_controller, keyval, _code, _state):
        say(f"key {Gdk.keyval_name(keyval)}")
        return True

    keys.connect("key-pressed", pressed)
    window.add_controller(keys)
    window.connect("notify::is-active",
                   lambda w, _p: say("focus in" if w.is_active() else "focus out"))
    window.connect("close-request", lambda _w: (loop.quit(), False)[1])
    window.present()
    say("ready")
    loop.run()


if __name__ == "__main__":
    sys.exit(main())
