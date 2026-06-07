use iced::widget::{button, container, row, text};
use iced::{Background, Border, Color, Element, Length, Shadow, Vector};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tool {
    Select,
    Draw,
    StampGrave,
    Grab,
    Erase,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolbarAction {
    SelectTool(Tool),
    ToggleGrid,
}

pub(super) struct Toolbar {
    selected_tool: Tool,
    show_grid: bool,
}

impl Default for Toolbar {
    fn default() -> Self {
        Self {
            selected_tool: Tool::Select,
            show_grid: true,
        }
    }
}

impl Toolbar {
    pub fn view(&self) -> Element<'_, ToolbarAction> {
        let tools = row![
            tool_button(
                "ⓘ",
                ToolbarAction::SelectTool(Tool::Select),
                self.selected_tool == Tool::Select,
            ),
            tool_button(
                "🖌",
                ToolbarAction::SelectTool(Tool::Draw),
                self.selected_tool == Tool::Draw,
            ),
            tool_button(
                "▯",
                ToolbarAction::SelectTool(Tool::StampGrave),
                self.selected_tool == Tool::StampGrave,
            ),
            tool_button(
                "✋",
                ToolbarAction::SelectTool(Tool::Grab),
                self.selected_tool == Tool::Grab,
            ),
            tool_button("#", ToolbarAction::ToggleGrid, self.show_grid),
            tool_button(
                "❌",
                ToolbarAction::SelectTool(Tool::Erase),
                self.selected_tool == Tool::Erase
            )
        ]
        .spacing(8);

        let panel = container(tools)
            .padding([10, 14])
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgb8(20, 64, 68))),
                border: Border {
                    color: Color::from_rgb8(42, 139, 143),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                shadow: Shadow {
                    color: Color::from_rgba8(0, 0, 0, 0.35),
                    offset: Vector::new(0.0, -2.0),
                    blur_radius: 4.0,
                },
                ..Default::default()
            });

        container(panel)
            .width(Length::Fill)
            .padding([8, 12])
            .center_x(Length::Fill)
            .into()
    }

    pub fn update(&mut self, message: ToolbarAction) {
        match message {
            ToolbarAction::ToggleGrid => self.show_grid = !self.show_grid,
            ToolbarAction::SelectTool(tool) => self.selected_tool = tool,
        }
    }

    pub fn selected_tool(&self) -> Tool {
        self.selected_tool
    }

    pub fn show_grid(&self) -> bool {
        self.show_grid
    }
}

fn tool_button(
    icon: &'static str,
    action: ToolbarAction,
    selected: bool,
) -> button::Button<'static, ToolbarAction> {
    button(
        text(icon)
            .size(22)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center),
    )
    .width(44)
    .height(44)
    .padding(0)
    .on_press(action)
    .style(move |_, status| {
        let pressed = status == button::Status::Pressed;
        let hovered = status == button::Status::Hovered;

        button::Style {
            background: Some(Background::Color(if selected || pressed {
                Color::from_rgb8(24, 117, 120)
            } else if hovered {
                Color::from_rgb8(52, 151, 153)
            } else {
                Color::from_rgb8(38, 126, 129)
            })),
            text_color: Color::WHITE,
            border: Border {
                color: if selected {
                    Color::from_rgb8(151, 255, 244)
                } else {
                    Color::from_rgb8(17, 78, 81)
                },
                width: if selected { 2.0 } else { 1.0 },
                radius: 4.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba8(0, 0, 0, 0.45),
                offset: if pressed {
                    Vector::new(0.0, 1.0)
                } else {
                    Vector::new(0.0, 3.0)
                },
                blur_radius: if pressed { 1.0 } else { 2.0 },
            },
            ..Default::default()
        }
    })
}
