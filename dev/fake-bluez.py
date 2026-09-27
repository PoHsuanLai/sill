#!/usr/bin/env python3
"""A made-up BlueZ for dev/accept-cc-real-input.sh (FINDINGS F1072): `org.bluez` on the private
bus named by argv[1], never the system bus, with one adapter (hci0, powered) and one paired,
connected keyboard. It answers what sill's Bluetooth service asks (the object manager at `/`,
`Adapter1.Powered` read and set, `Device1`'s fields, Connect and Disconnect) and emits the
property changes BlueZ would. Every `Powered` set is appended to argv[2] ("powered true" or
"powered false"), which is how the script reads what the control center did.

Usage: fake-bluez.py <bus address> <log file>
"""

import sys

import dbus
import dbus.bus
import dbus.mainloop.glib
import dbus.service
from gi.repository import GLib

PROPS = "org.freedesktop.DBus.Properties"
MANAGER = "org.freedesktop.DBus.ObjectManager"
ADAPTER = "org.bluez.Adapter1"
DEVICE = "org.bluez.Device1"
HCI = "/org/bluez/hci0"
KEYBOARD = HCI + "/dev_AA_BB_CC_DD_EE_01"


class Thing(dbus.service.Object):
    """An object with properties on one interface."""

    def __init__(self, bus, path, iface, props, log):
        super().__init__(bus, path)
        self.path, self.iface, self.props, self.log = path, iface, props, log

    @dbus.service.method(PROPS, in_signature="ss", out_signature="v")
    def Get(self, iface, name):
        return self.props[name]

    @dbus.service.method(PROPS, in_signature="s", out_signature="a{sv}")
    def GetAll(self, iface):
        return self.props if iface == self.iface else {}

    @dbus.service.method(PROPS, in_signature="ssv", out_signature="")
    def Set(self, iface, name, value):
        self.change(name, value)

    @dbus.service.signal(PROPS, signature="sa{sv}as")
    def PropertiesChanged(self, iface, changed, invalidated):
        pass

    def change(self, name, value):
        self.props[name] = value
        self.PropertiesChanged(self.iface, {name: value}, [])


class Adapter(Thing):
    def __init__(self, bus, log, keyboard):
        props = {
            "Address": dbus.String("00:11:22:33:44:55"),
            "Name": dbus.String("fake-hci0"),
            "Alias": dbus.String("fake-hci0"),
            "Powered": dbus.Boolean(True),
        }
        super().__init__(bus, HCI, ADAPTER, props, log)
        self.keyboard = keyboard

    def change(self, name, value):
        super().change(name, value)
        if name == "Powered":
            with open(self.log, "a") as out:
                out.write(f"powered {'true' if value else 'false'}\n")
            # BlueZ drops every connection with the radio, and makes none back by itself.
            if not value:
                self.keyboard.change("Connected", dbus.Boolean(False))


class Device(Thing):
    def __init__(self, bus, log):
        props = {
            "Address": dbus.String("AA:BB:CC:DD:EE:01"),
            "Alias": dbus.String("Fake Keyboard"),
            "Icon": dbus.String("input-keyboard"),
            "Paired": dbus.Boolean(True),
            "Connected": dbus.Boolean(True),
            "Adapter": dbus.ObjectPath(HCI),
        }
        super().__init__(bus, KEYBOARD, DEVICE, props, log)

    @dbus.service.method(DEVICE, in_signature="", out_signature="")
    def Connect(self):
        self.change("Connected", dbus.Boolean(True))

    @dbus.service.method(DEVICE, in_signature="", out_signature="")
    def Disconnect(self):
        self.change("Connected", dbus.Boolean(False))


class Root(dbus.service.Object):
    def __init__(self, bus, things):
        super().__init__(bus, "/")
        self.things = things

    @dbus.service.method(MANAGER, in_signature="", out_signature="a{oa{sa{sv}}}")
    def GetManagedObjects(self):
        return {
            dbus.ObjectPath(t.path): {t.iface: dbus.Dictionary(t.props, signature="sv")}
            for t in self.things
        }

    @dbus.service.signal(MANAGER, signature="oa{sa{sv}}")
    def InterfacesAdded(self, path, interfaces):
        pass

    @dbus.service.signal(MANAGER, signature="oas")
    def InterfacesRemoved(self, path, interfaces):
        pass


def main():
    address, log = sys.argv[1], sys.argv[2]
    if "system_bus_socket" in address or not address.startswith("unix:"):
        sys.exit(f"fake-bluez: refusing {address}: not a private bus")
    dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
    bus = dbus.bus.BusConnection(address)
    keyboard = Device(bus, log)
    adapter = Adapter(bus, log, keyboard)
    Root(bus, [adapter, keyboard])
    name = dbus.service.BusName("org.bluez", bus, do_not_queue=True)
    print(f"fake-bluez: org.bluez on {address}", flush=True)
    GLib.MainLoop().run()
    del name


if __name__ == "__main__":
    main()
