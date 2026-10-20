//! Which logind session this process belongs to, found one way for every service that needs it
//! (the session's power actions, the locker, idle's hint): by pid (sill started from the session
//! script), else by the `XDG_SESSION_ID` the process edge read, else the user's display session
//! (sill as a systemd user service, which logind places in no session).

use logind_zbus::manager::ManagerProxy;
use logind_zbus::user::UserProxy;
use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

/// The three questions the lookup asks of logind, in the order it asks them.
pub trait SessionLookup {
    /// `GetSessionByPID` for this process.
    fn by_pid(&self) -> impl Future<Output = Option<OwnedObjectPath>> + Send;
    /// `GetSession(id)`.
    fn by_id(&self, id: &str) -> impl Future<Output = Option<OwnedObjectPath>> + Send;
    /// The session of this user's `Display`.
    fn display(&self) -> impl Future<Output = Option<OwnedObjectPath>> + Send;
}

/// The session path: by pid, else by `hint` (the `XDG_SESSION_ID` value), else the display.
pub async fn find_session(
    lookup: &impl SessionLookup,
    hint: Option<&str>,
) -> Option<OwnedObjectPath> {
    if let Some(path) = lookup.by_pid().await {
        return Some(path);
    }
    if let Some(id) = hint
        && let Some(path) = lookup.by_id(id).await
    {
        return Some(path);
    }
    lookup.display().await
}

/// logind's manager on a connection: the real [`SessionLookup`].
#[derive(Debug)]
pub struct ManagerLookup<'a, 'm> {
    pub connection: &'a Connection,
    pub manager: &'a ManagerProxy<'m>,
}

impl SessionLookup for ManagerLookup<'_, '_> {
    async fn by_pid(&self) -> Option<OwnedObjectPath> {
        self.manager
            .get_session_by_PID(std::process::id())
            .await
            .ok()
    }

    async fn by_id(&self, id: &str) -> Option<OwnedObjectPath> {
        self.manager.get_session(id).await.ok()
    }

    async fn display(&self) -> Option<OwnedObjectPath> {
        let uid = rustix::process::getuid().as_raw();
        let user_path = self.manager.get_user(uid).await.ok()?;
        let user = UserProxy::builder(self.connection)
            .path(user_path)
            .ok()?
            .build()
            .await
            .ok()?;
        let display = user.display().await.ok()?;
        self.manager.get_session(display.id()).await.ok()
    }
}

/// This process's session on `connection`, by [`find_session`].
pub async fn session_path(
    connection: &Connection,
    manager: &ManagerProxy<'_>,
    hint: Option<&str>,
) -> Option<OwnedObjectPath> {
    find_session(
        &ManagerLookup {
            connection,
            manager,
        },
        hint,
    )
    .await
}
