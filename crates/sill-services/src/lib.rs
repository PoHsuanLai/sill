//! Effectful services behind watch channels: power, network, volume, tray, apps, COSMIC Wayland state.

use cosmic_client_toolkit as _;
use cosmic_dbus_networkmanager as _;
use fend_core as _;
use freedesktop_desktop_entry as _;
use freedesktop_icons as _;
use image as _;
use libpulse_binding as _;
use logind_zbus as _;
use notify as _;
use serde as _;
use system_tray as _;
use thiserror as _;
use tokio as _;
use upower_dbus as _;
use zbus as _;
