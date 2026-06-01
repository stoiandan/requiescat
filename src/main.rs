mod models;
mod mouse;
mod screens;

use iced;

fn main() -> iced::Result {
    iced::application(
        screens::MapEditor::default,
        screens::MapEditor::update,
        screens::MapEditor::view,
    )
    .run()
}
