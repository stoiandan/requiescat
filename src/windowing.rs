use std::sync::OnceLock;

use iced::window;

static APPLICATION_ICON: OnceLock<Option<window::Icon>> = OnceLock::new();
const APPLICATION_ICON_BYTES: &[u8] = include_bytes!("../packaging/icons/requiescat.png");

pub fn application_icon() -> Option<window::Icon> {
    APPLICATION_ICON.get_or_init(load_application_icon).clone()
}

fn load_application_icon() -> Option<window::Icon> {
    let icon = image::load_from_memory_with_format(
        APPLICATION_ICON_BYTES,
        image::ImageFormat::Png,
    )
    .ok()?
    .into_rgba8();
    let width = icon.width();
    let height = icon.height();

    window::icon::from_rgba(icon.into_raw(), width, height).ok()
}
