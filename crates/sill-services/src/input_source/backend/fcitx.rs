//! fcitx5's controller over the session bus (no crate for four methods).

use sill_model::input_source::{InputRefusal, next_of};
use zbus::proxy;

pub(super) const NAME: &str = "org.fcitx.Fcitx5";

#[proxy(
    interface = "org.fcitx.Fcitx.Controller1",
    default_service = "org.fcitx.Fcitx5",
    default_path = "/controller"
)]
trait Controller {
    fn current_input_method(&self) -> zbus::Result<String>;
    fn current_input_method_group(&self) -> zbus::Result<String>;

    /// The group's default layout and its input methods as (unique name, language).
    fn input_method_group_info(
        &self,
        group: &str,
    ) -> zbus::Result<(String, Vec<(String, String)>)>;

    #[zbus(name = "SetCurrentIM")]
    fn set_current_im(&self, name: &str) -> zbus::Result<()>;
}

/// Switch fcitx5 to the next input method of its current group; the name it was told.
pub(super) async fn switch(connection: &zbus::Connection) -> Result<String, InputRefusal> {
    let failed = |error: zbus::Error| InputRefusal::Failed(error.to_string());
    let proxy = ControllerProxy::new(connection).await.map_err(failed)?;
    let current = proxy.current_input_method().await.map_err(failed)?;
    let group = proxy.current_input_method_group().await.map_err(failed)?;
    let (_layout, entries) = proxy
        .input_method_group_info(&group)
        .await
        .map_err(failed)?;
    let names: Vec<String> = entries.into_iter().map(|(name, _language)| name).collect();
    let next = next_of(&current, &names)
        .ok_or(InputRefusal::NothingToSwitchTo)?
        .to_owned();
    proxy.set_current_im(&next).await.map_err(failed)?;
    Ok(next)
}
