use iced::Element;
use iced::widget::{Button, row};

#[derive(Debug, Clone, Copy)]
pub enum ToolbarAction {
    Draw,
    Grab,
}


pub(super) struct ToolBar {
    pub selected_action: ToolbarAction
}

impl Default for ToolBar {
    fn default() -> Self {
        Self {
            selected_action: ToolbarAction::Draw
        }
    }
}


impl ToolBar {
    pub fn view(&self) -> Element<'_, ToolbarAction> {
        row![
            Button::new("Draw").on_press(ToolbarAction::Draw),
            Button::new("Grab").on_press(ToolbarAction::Grab)
        ]
        .into()
    }

    pub fn update(&mut self, message: ToolbarAction) {
        self.selected_action = message;
    }
}
