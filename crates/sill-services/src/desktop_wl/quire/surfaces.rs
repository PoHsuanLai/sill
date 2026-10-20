//! Which of the shell's surfaces plays which role to the compositor.

use casement_protocols::vocab::ShellRole;
use desktop_client::SurfaceKey;

/// One registered surface: its role, the output it is on and the client's key for it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    role: ShellRole,
    output: String,
    key: SurfaceKey,
}

/// The surfaces registered with the wire client, by role and output. A minimise target is
/// local to the dock's surface, so a command finds the dock on the window's output.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShellSurfaces {
    rows: Vec<Row>,
}

impl ShellSurfaces {
    /// Record `key` as the surface of `role` on `output`, replacing the one registered before
    /// (a hidden surface that is shown again has a new `wl_surface`).
    pub fn add(&mut self, role: ShellRole, output: impl Into<String>, key: SurfaceKey) {
        let output = output.into();
        self.rows
            .retain(|row| !(row.role == role && row.output == output));
        self.rows.push(Row { role, output, key });
    }

    /// The dock's surface on `output`, else any dock's.
    pub fn dock_for(&self, output: Option<&str>) -> Option<SurfaceKey> {
        let docks = || self.rows.iter().filter(|row| row.role == ShellRole::Dock);
        docks()
            .find(|row| Some(row.output.as_str()) == output)
            .or_else(|| docks().next())
            .map(|row| row.key)
    }
}

/// The role the compositor knows the shell surface named `name` (`Surface::NAME`) by; `None`
/// for a surface it has no role for (a sheet, a prompt, a catcher).
pub fn role_of_surface(name: &str) -> Option<ShellRole> {
    match name {
        "bar" => Some(ShellRole::Bar),
        "dock" => Some(ShellRole::Dock),
        "launcher" => Some(ShellRole::Launcher),
        "switcher" => Some(ShellRole::Switcher),
        "banners" | "notification-center" => Some(ShellRole::Notifications),
        "osd" => Some(ShellRole::Osd),
        "wallpaper" => Some(ShellRole::Wallpaper),
        _ => None,
    }
}
