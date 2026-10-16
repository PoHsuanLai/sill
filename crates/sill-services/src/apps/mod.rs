//! The application index: desktop entries from every XDG applications directory, their icons
//! resolved once, rescanned on change, through freedesktop-desktop-entry and freedesktop-icons; the
//! theme from GTK's settings, else hicolor;
//! `notify` on the applications directories, 500 ms.

mod backend;

use sill_model::apps::{AppIndex, AppsCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

pub use backend::{
    IconRoots, IconTheme, applications_dirs, icon_theme, parse_entry, pixmap_dirs, resolve_icon,
    scan,
};

/// The desktop entries and their icons as a [`Service`]: scanned once and published, then every
/// applications directory watched and rescanned after half a second of quiet.
#[derive(Debug)]
pub enum AppsService {}

impl Service for AppsService {
    const NAME: &'static str = "apps";
    type State = AppIndex;
    type Command = AppsCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let (config, data) = (env.dirs.config.clone(), env.dirs.data.clone());
        let roots = env.dirs.icon_file_roots();
        async move {
            backend::run(io.state, io.commands, config, data, roots).await;
            Ok(())
        }
    }
}
