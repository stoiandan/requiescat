use iced::widget::{Space, button, column, container, row, text, tooltip};
use iced::{Background, Border, Color, Element, Length, Shadow, Vector};

use crate::localization::{Localizer, MessageId};
use crate::models::{DelimiterType, GraveColor};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tool {
    Select,
    Draw,
    StampGrave,
    DrawDelimiter,
    Grab,
    Erase,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolbarAction {
    SelectTool(Tool),
    ToggleGrid,
    ToggleColorPicker,
    ToggleDelimiterTypePicker,
    SelectGraveColor(GraveColor),
    SelectDelimiterType(DelimiterType),
}

pub(super) struct Toolbar {
    selected_tool: Tool,
    show_grid: bool,
    selected_grave_color: GraveColor,
    selected_delimiter_type: DelimiterType,
    show_color_picker: bool,
    show_delimiter_type_picker: bool,
}

impl Default for Toolbar {
    fn default() -> Self {
        Self {
            selected_tool: Tool::Select,
            show_grid: true,
            selected_grave_color: GraveColor::default(),
            selected_delimiter_type: DelimiterType::default(),
            show_color_picker: false,
            show_delimiter_type_picker: false,
        }
    }
}

impl Toolbar {
    pub fn view<'a>(&'a self, localizer: &'a Localizer) -> Element<'a, ToolbarAction> {
        let tools = row![
            tool_button(
                "ⓘ",
                ToolbarAction::SelectTool(Tool::Select),
                self.selected_tool == Tool::Select,
                localizer.text(MessageId::ToolSelect),
            ),
            tool_button(
                "🖌",
                ToolbarAction::SelectTool(Tool::Draw),
                self.selected_tool == Tool::Draw,
                localizer.text(MessageId::ToolDraw),
            ),
            tool_button(
                "▯",
                ToolbarAction::SelectTool(Tool::StampGrave),
                self.selected_tool == Tool::StampGrave,
                localizer.text(MessageId::ToolStampGrave),
            ),
            delimiter_tool_button(
                self.selected_tool == Tool::DrawDelimiter,
                self.selected_delimiter_type,
                localizer.text(MessageId::ToolDelimiter),
            ),
            tool_button(
                "✋",
                ToolbarAction::SelectTool(Tool::Grab),
                self.selected_tool == Tool::Grab,
                localizer.text(MessageId::ToolGrab),
            ),
            tool_button(
                "#",
                ToolbarAction::ToggleGrid,
                self.show_grid,
                localizer.text(MessageId::ToolGrid),
            ),
            tool_button(
                "❌",
                ToolbarAction::SelectTool(Tool::Erase),
                self.selected_tool == Tool::Erase,
                localizer.text(MessageId::ToolErase),
            ),
            color_picker_button(
                self.selected_grave_color,
                localizer.text(MessageId::ToolGraveColor),
            )
        ]
        .spacing(8);

        let content = if self.show_delimiter_type_picker {
            column![
                container(delimiter_type_palette(
                    self.selected_delimiter_type,
                    localizer.text(MessageId::ToolDelimiterWall),
                    localizer.text(MessageId::ToolDelimiterRoad),
                ))
                .width(Length::Fill)
                .align_x(iced::Alignment::End),
                tools
            ]
            .spacing(8)
            .align_x(iced::Alignment::Center)
        } else if self.show_color_picker {
            column![
                container(color_palette(
                    self.selected_grave_color,
                    localizer.text(MessageId::ToolColorSwatch),
                ))
                .width(Length::Fill)
                .align_x(iced::Alignment::End),
                tools
            ]
            .spacing(8)
            .align_x(iced::Alignment::Center)
        } else {
            column![tools]
        };

        let panel = container(content)
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
            ToolbarAction::SelectTool(tool) => {
                self.selected_tool = tool;
                self.show_color_picker = false;
                self.show_delimiter_type_picker = false;
            }
            ToolbarAction::ToggleColorPicker => {
                self.show_color_picker = !self.show_color_picker;
                self.show_delimiter_type_picker = false;
            }
            ToolbarAction::ToggleDelimiterTypePicker => {
                self.selected_tool = Tool::DrawDelimiter;
                self.show_delimiter_type_picker = !self.show_delimiter_type_picker;
                self.show_color_picker = false;
            }
            ToolbarAction::SelectGraveColor(color) => {
                self.selected_grave_color = color;
                self.show_color_picker = false;
            }
            ToolbarAction::SelectDelimiterType(delimiter_type) => {
                self.selected_tool = Tool::DrawDelimiter;
                self.selected_delimiter_type = delimiter_type;
                self.show_delimiter_type_picker = false;
            }
        }
    }

    pub fn selected_tool(&self) -> Tool {
        self.selected_tool
    }

    pub fn show_grid(&self) -> bool {
        self.show_grid
    }

    pub fn selected_grave_color(&self) -> GraveColor {
        self.selected_grave_color
    }

    pub fn selected_delimiter_type(&self) -> DelimiterType {
        self.selected_delimiter_type
    }
}

fn tool_button(
    icon: &'static str,
    action: ToolbarAction,
    selected: bool,
    label: String,
) -> Element<'static, ToolbarAction> {
    tooltip_button(
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
        .style(move |_, status| teal_button_style(status, selected, 3.0)),
        label,
    )
}

fn delimiter_tool_button(
    selected: bool,
    delimiter_type: DelimiterType,
    label: String,
) -> Element<'static, ToolbarAction> {
    let icon = match delimiter_type {
        DelimiterType::Wall => "▧",
        DelimiterType::Road => "⋯",
    };

    tool_button(
        icon,
        ToolbarAction::ToggleDelimiterTypePicker,
        selected,
        label,
    )
}

fn delimiter_type_palette(
    selected: DelimiterType,
    wall_label: String,
    road_label: String,
) -> Element<'static, ToolbarAction> {
    let controls = row![
        delimiter_type_option(
            "▧",
            DelimiterType::Wall,
            selected == DelimiterType::Wall,
            wall_label,
        ),
        delimiter_type_option(
            "⋯",
            DelimiterType::Road,
            selected == DelimiterType::Road,
            road_label,
        )
    ]
    .spacing(6);

    container(controls)
        .padding([6, 8])
        .style(|_| picker_panel_style())
        .into()
}

fn delimiter_type_option(
    icon: &'static str,
    delimiter_type: DelimiterType,
    selected: bool,
    label: String,
) -> Element<'static, ToolbarAction> {
    tooltip_button(
        button(
            text(icon)
                .size(20)
                .align_x(iced::Alignment::Center)
                .align_y(iced::Alignment::Center),
        )
        .width(44)
        .height(34)
        .padding(0)
        .on_press(ToolbarAction::SelectDelimiterType(delimiter_type))
        .style(move |_, status| teal_button_style(status, selected, 2.0)),
        label,
    )
}

fn color_palette(selected: GraveColor, label: String) -> Element<'static, ToolbarAction> {
    let mut colors = row![].spacing(6);

    for color in GraveColor::PALETTE {
        colors = colors.push(color_swatch(color, selected == color, label.clone()));
    }

    container(colors)
        .padding([6, 8])
        .style(|_| picker_panel_style())
        .into()
}

fn color_picker_button(color: GraveColor, label: String) -> Element<'static, ToolbarAction> {
    tooltip_button(
        button(Space::new().width(Length::Fill).height(Length::Fill))
            .width(44)
            .height(44)
            .padding(0)
            .on_press(ToolbarAction::ToggleColorPicker)
            .style(move |_, status| color_button_style(status, color, false)),
        label,
    )
}

fn color_swatch(
    color: GraveColor,
    selected: bool,
    label: String,
) -> Element<'static, ToolbarAction> {
    tooltip_button(
        button(text(""))
            .width(34)
            .height(34)
            .padding(0)
            .on_press(ToolbarAction::SelectGraveColor(color))
            .style(move |_, status| color_button_style(status, color, selected)),
        label,
    )
}

fn tooltip_button(
    button: button::Button<'static, ToolbarAction>,
    label: String,
) -> Element<'static, ToolbarAction> {
    tooltip(
        button,
        container(text(label).size(12))
            .padding([6, 8])
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgb8(9, 31, 35))),
                text_color: Some(Color::WHITE),
                border: Border {
                    color: Color::from_rgb8(72, 164, 166),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                shadow: Shadow {
                    color: Color::from_rgba8(0, 0, 0, 0.35),
                    offset: Vector::new(0.0, 2.0),
                    blur_radius: 4.0,
                },
                ..Default::default()
            }),
        tooltip::Position::Top,
    )
    .gap(8)
    .into()
}

fn color_button_style(status: button::Status, color: GraveColor, selected: bool) -> button::Style {
    let pressed = status == button::Status::Pressed;
    let hovered = status == button::Status::Hovered;

    button::Style {
        background: Some(Background::Color(color.to_iced())),
        text_color: Color::WHITE,
        border: Border {
            color: if selected || hovered || pressed {
                Color::from_rgb8(231, 255, 250)
            } else {
                Color::from_rgb8(17, 78, 81)
            },
            width: if selected { 3.0 } else { 1.0 },
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
}

fn picker_panel_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(12, 43, 47))),
        border: Border {
            color: Color::from_rgb8(72, 164, 166),
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.35),
            offset: Vector::new(0.0, -2.0),
            blur_radius: 5.0,
        },
        ..Default::default()
    }
}

fn teal_button_style(
    status: button::Status,
    selected: bool,
    resting_shadow_y: f32,
) -> button::Style {
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
                Vector::new(0.0, resting_shadow_y)
            },
            blur_radius: if pressed { 1.0 } else { 2.0 },
        },
        ..Default::default()
    }
}
