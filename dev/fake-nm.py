#!/usr/bin/env python3
"""A made-up NetworkManager for dev/accept/control-center.sh: `org.freedesktop.NetworkManager`
on the private bus named by argv[1], never the system bus, with one Wi-Fi device (wlan0), three
access points (one active and connected), one saved connection and a primary wireless
connection. It answers what sill's Network and Wi-Fi services ask: the manager's
`PrimaryConnection`, `ActivatingConnection`, `Connectivity`, `WirelessEnabled`,
`WirelessHardwareEnabled` and `GetAllDevices`; the device's `DeviceType`, `State`,
`AccessPoints`, `ActiveAccessPoint`, `LastScan`, `RequestScan` and `Disconnect`; each access
point's fields; the saved connection's `GetSettings`; the active connection's `State`, `Type`
and `Devices`. It emits the property changes NetworkManager would.

Every request that would change a real radio or connection is only logged, to argv[2] (one line
each: "scan", "radio on", "radio off", "disconnect", "activate"), which is how a script reads what
the control center did. Nothing here touches a real network.

Usage: fake-nm.py <bus address> <log file>
"""

import sys
import time

import dbus
import dbus.bus
import dbus.mainloop.glib
import dbus.service
from gi.repository import GLib

PROPS = "org.freedesktop.DBus.Properties"
NM = "org.freedesktop.NetworkManager"
NM_PATH = "/org/freedesktop/NetworkManager"
SETTINGS = NM + ".Settings"
SETTINGS_PATH = NM_PATH + "/Settings"
SETTINGS_CONN = NM + ".Settings.Connection"
DEVICE = NM + ".Device"
WIRELESS = NM + ".Device.Wireless"
ACCESS_POINT = NM + ".AccessPoint"
ACTIVE = NM + ".Connection.Active"
DEVICE_PATH = NM_PATH + "/Devices/1"
ACTIVE_PATH = NM_PATH + "/ActiveConnection/1"
CONN_PATH = SETTINGS_PATH + "/1"
AP_PATHS = [NM_PATH + f"/AccessPoint/{n}" for n in (1, 2, 3)]

DEVICE_TYPE_WIFI = 2
DEVICE_STATE_ACTIVATED = 100
CONNECTIVITY_FULL = 4
ACTIVE_STATE_ACTIVATED = 2
AP_FLAGS_PRIVACY = 1
AP_SEC_PSK = 0x100  # WpaFlags / RsnFlags: key management PSK

# (ssid, strength, secured)
ACCESS_POINTS = [("Fake Home", 78, True), ("Fake Cafe", 54, False), ("Fake Neighbour", 31, True)]


def ssid_bytes(text):
    return dbus.Array([dbus.Byte(b) for b in text.encode()], signature="y")


class Obj(dbus.service.Object):
    """An object whose properties live on one or more interfaces (`self.ifaces`)."""

    def __init__(self, bus, path, ifaces, log):
        super().__init__(bus, path)
        self.path, self.ifaces, self.log = path, ifaces, log

    def note(self, line):
        with open(self.log, "a") as out:
            out.write(line + "\n")

    @dbus.service.method(PROPS, in_signature="ss", out_signature="v")
    def Get(self, iface, name):
        return self.ifaces[iface][name]

    @dbus.service.method(PROPS, in_signature="s", out_signature="a{sv}")
    def GetAll(self, iface):
        return dbus.Dictionary(self.ifaces.get(iface, {}), signature="sv")

    @dbus.service.method(PROPS, in_signature="ssv", out_signature="")
    def Set(self, iface, name, value):
        self.on_set(iface, name, value)

    @dbus.service.signal(PROPS, signature="sa{sv}as")
    def PropertiesChanged(self, iface, changed, invalidated):
        pass

    def on_set(self, iface, name, value):
        self.change(iface, name, value)

    def change(self, iface, name, value):
        self.ifaces[iface][name] = value
        self.PropertiesChanged(iface, {name: value}, [])


class Manager(Obj):
    def __init__(self, bus, log):
        props = {
            "PrimaryConnection": dbus.ObjectPath(ACTIVE_PATH),
            "ActivatingConnection": dbus.ObjectPath("/"),
            "Connectivity": dbus.UInt32(CONNECTIVITY_FULL),
            "WirelessEnabled": dbus.Boolean(True),
            "WirelessHardwareEnabled": dbus.Boolean(True),
            "State": dbus.UInt32(70),
            "Version": dbus.String("1.99.0-fake"),
        }
        super().__init__(bus, NM_PATH, {NM: props}, log)

    def on_set(self, iface, name, value):
        if name == "WirelessEnabled":
            self.note(f"radio {'on' if value else 'off'}")
        self.change(iface, name, value)

    @dbus.service.method(NM, in_signature="", out_signature="ao")
    def GetAllDevices(self):
        return [dbus.ObjectPath(DEVICE_PATH)]

    @dbus.service.method(NM, in_signature="ooo", out_signature="o")
    def ActivateConnection(self, connection, device, specific):
        self.note("activate")
        return dbus.ObjectPath(ACTIVE_PATH)


class Settings(Obj):
    def __init__(self, bus, log):
        super().__init__(bus, SETTINGS_PATH, {SETTINGS: {
            "Connections": dbus.Array([dbus.ObjectPath(CONN_PATH)], signature="o"),
        }}, log)

    @dbus.service.signal(SETTINGS, signature="o")
    def NewConnection(self, path):
        pass

    @dbus.service.signal(SETTINGS, signature="o")
    def ConnectionRemoved(self, path):
        pass


class SavedConnection(Obj):
    def __init__(self, bus, log):
        super().__init__(bus, CONN_PATH, {SETTINGS_CONN: {
            "Unsaved": dbus.Boolean(False),
        }}, log)

    @dbus.service.method(SETTINGS_CONN, in_signature="", out_signature="a{sa{sv}}")
    def GetSettings(self):
        return {
            "connection": {
                "id": dbus.String("Fake Home"),
                "type": dbus.String("802-11-wireless"),
            },
            "802-11-wireless": {"ssid": ssid_bytes("Fake Home")},
        }


class AccessPoint(Obj):
    def __init__(self, bus, path, log, ssid, strength, secured):
        props = {
            "Ssid": ssid_bytes(ssid),
            "Strength": dbus.Byte(strength),
            "Flags": dbus.UInt32(AP_FLAGS_PRIVACY if secured else 0),
            "WpaFlags": dbus.UInt32(0),
            "RsnFlags": dbus.UInt32(AP_SEC_PSK if secured else 0),
            "HwAddress": dbus.String("02:00:00:00:00:%02x" % strength),
            "Frequency": dbus.UInt32(5180),
            "MaxBitrate": dbus.UInt32(300000),
        }
        super().__init__(bus, path, {ACCESS_POINT: props}, log)


class Device(Obj):
    def __init__(self, bus, log):
        device = {
            "Interface": dbus.String("wlan0"),
            "DeviceType": dbus.UInt32(DEVICE_TYPE_WIFI),
            "State": dbus.UInt32(DEVICE_STATE_ACTIVATED),
            "Managed": dbus.Boolean(True),
            "ActiveConnection": dbus.ObjectPath(ACTIVE_PATH),
        }
        wireless = {
            "AccessPoints": dbus.Array([dbus.ObjectPath(p) for p in AP_PATHS], signature="o"),
            "ActiveAccessPoint": dbus.ObjectPath(AP_PATHS[0]),
            "LastScan": dbus.Int64(int(time.monotonic() * 1000)),
            "HwAddress": dbus.String("02:00:00:00:00:01"),
        }
        super().__init__(bus, DEVICE_PATH, {DEVICE: device, WIRELESS: wireless}, log)

    @dbus.service.method(WIRELESS, in_signature="a{sv}", out_signature="")
    def RequestScan(self, options):
        self.note("scan")
        GLib.timeout_add(300, self.scanned)

    def scanned(self):
        self.change(WIRELESS, "LastScan", dbus.Int64(int(time.monotonic() * 1000)))
        return False

    @dbus.service.method(DEVICE, in_signature="", out_signature="")
    def Disconnect(self):
        self.note("disconnect")

    @dbus.service.signal(WIRELESS, signature="o")
    def AccessPointAdded(self, path):
        pass

    @dbus.service.signal(WIRELESS, signature="o")
    def AccessPointRemoved(self, path):
        pass


class ActiveConnection(Obj):
    def __init__(self, bus, log):
        super().__init__(bus, ACTIVE_PATH, {ACTIVE: {
            "Id": dbus.String("Fake Home"),
            "Type": dbus.String("802-11-wireless"),
            "State": dbus.UInt32(ACTIVE_STATE_ACTIVATED),
            "Devices": dbus.Array([dbus.ObjectPath(DEVICE_PATH)], signature="o"),
            "Connection": dbus.ObjectPath(CONN_PATH),
            "Default": dbus.Boolean(True),
        }}, log)


def main():
    address, log = sys.argv[1], sys.argv[2]
    if "system_bus_socket" in address or not address.startswith("unix:"):
        sys.exit(f"fake-nm: refusing {address}: not a private bus")
    dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
    bus = dbus.bus.BusConnection(address)
    keep = [Manager(bus, log), Settings(bus, log), SavedConnection(bus, log), Device(bus, log),
            ActiveConnection(bus, log)]
    keep += [AccessPoint(bus, path, log, *spec) for path, spec in zip(AP_PATHS, ACCESS_POINTS)]
    name = dbus.service.BusName(NM, bus, do_not_queue=True)
    print(f"fake-nm: {NM} on {address}", flush=True)
    GLib.MainLoop().run()
    del name, keep


if __name__ == "__main__":
    main()
