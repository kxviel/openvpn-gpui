use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

pub(crate) struct Assets;
impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        let bytes: &'static [u8] = match path {
            "shield.svg" => include_bytes!("../../assets/shield.svg"),
            "power.svg" => include_bytes!("../../assets/power.svg"),
            "plus.svg" => include_bytes!("../../assets/plus.svg"),
            "chevron.svg" => include_bytes!("../../assets/chevron.svg"),
            "file.svg" => include_bytes!("../../assets/file.svg"),
            "close.svg" => include_bytes!("../../assets/close.svg"),
            "back.svg" => include_bytes!("../../assets/back.svg"),
            "menu.svg" => include_bytes!("../../assets/menu.svg"),
            "trash.svg" => include_bytes!("../../assets/trash.svg"),
            "lock.svg" => include_bytes!("../../assets/lock.svg"),
            "settings.svg" => include_bytes!("../../assets/settings.svg"),
            _ => return Ok(None),
        };
        Ok(Some(Cow::Borrowed(bytes)))
    }
    fn list(&self, _: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(vec![])
    }
}
