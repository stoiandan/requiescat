use iced::widget::{Space, button, column, container, row, svg, text, tooltip};
use iced::{Background, Border, Color, Element, Length, Shadow, Vector};

use crate::localization::{Localizer, MessageId};
use crate::models::{DelimiterType, GraveColor};
use crate::theme;

#[derive(Debug, Clone, Copy)]
enum ToolbarIcon {
    Select,
    Draw,
    StampGrave,
    DelimiterWall,
    DelimiterRoad,
    Grab,
    Grid,
    Erase,
}

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
                ToolbarIcon::Select,
                ToolbarAction::SelectTool(Tool::Select),
                self.selected_tool == Tool::Select,
                localizer.text(MessageId::ToolSelect),
            ),
            tool_button(
                ToolbarIcon::Draw,
                ToolbarAction::SelectTool(Tool::Draw),
                self.selected_tool == Tool::Draw,
                localizer.text(MessageId::ToolDraw),
            ),
            tool_button(
                ToolbarIcon::StampGrave,
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
                ToolbarIcon::Grab,
                ToolbarAction::SelectTool(Tool::Grab),
                self.selected_tool == Tool::Grab,
                localizer.text(MessageId::ToolGrab),
            ),
            tool_button(
                ToolbarIcon::Grid,
                ToolbarAction::ToggleGrid,
                self.show_grid,
                localizer.text(MessageId::ToolGrid),
            ),
            tool_button(
                ToolbarIcon::Erase,
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
                background: Some(Background::Color(theme::SURFACE_ALT)),
                border: Border {
                    color: theme::BORDER,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                shadow: Shadow {
                    color: theme::SHADOW,
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
    icon: ToolbarIcon,
    action: ToolbarAction,
    selected: bool,
    label: String,
) -> Element<'static, ToolbarAction> {
    tooltip_button(
        button(toolbar_icon(icon))
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
        DelimiterType::Wall => ToolbarIcon::DelimiterWall,
        DelimiterType::Road => ToolbarIcon::DelimiterRoad,
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
            ToolbarIcon::DelimiterWall,
            DelimiterType::Wall,
            selected == DelimiterType::Wall,
            wall_label,
        ),
        delimiter_type_option(
            ToolbarIcon::DelimiterRoad,
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
    icon: ToolbarIcon,
    delimiter_type: DelimiterType,
    selected: bool,
    label: String,
) -> Element<'static, ToolbarAction> {
    tooltip_button(
        button(toolbar_icon(icon))
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

fn toolbar_icon(icon: ToolbarIcon) -> Element<'static, ToolbarAction> {
    let bytes: &'static [u8] = match icon {
        ToolbarIcon::Select => include_bytes!("../../../assets/toolbar/select.svg"),
        ToolbarIcon::Draw => include_bytes!("../../../assets/toolbar/draw.svg"),
        ToolbarIcon::StampGrave => include_bytes!("../../../assets/toolbar/stamp-grave.svg"),
        ToolbarIcon::DelimiterWall => include_bytes!("../../../assets/toolbar/delimiter-wall.svg"),
        ToolbarIcon::DelimiterRoad => include_bytes!("../../../assets/toolbar/delimiter-road.svg"),
        ToolbarIcon::Grab => include_bytes!("../../../assets/toolbar/grab.svg"),
        ToolbarIcon::Grid => include_bytes!("../../../assets/toolbar/grid.svg"),
        ToolbarIcon::Erase => include_bytes!("../../../assets/toolbar/erase.svg"),
    };

    svg(svg::Handle::from_memory(bytes))
        .width(24)
        .height(24)
        .style(|_, _| svg::Style {
            color: Some(theme::TEXT_PRIMARY),
        })
        .into()
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
                background: Some(Background::Color(theme::SURFACE)),
                text_color: Some(theme::TEXT_PRIMARY),
                border: Border {
                    color: theme::BORDER,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                shadow: Shadow {
                    color: theme::SHADOW,
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
                theme::BORDER_BRIGHT
            } else {
                theme::BORDER
            },
            width: if selected { 3.0 } else { 1.0 },
            radius: 4.0.into(),
        },
        shadow: Shadow {
            color: theme::HEAVY_SHADOW,
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
        background: Some(Background::Color(theme::SURFACE)),
        border: Border {
            color: theme::BORDER,
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow {
            color: theme::SHADOW,
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
            theme::ACCENT_ACTIVE
        } else if hovered {
            theme::ACCENT_HOVER_DARK
        } else {
            theme::ACCENT_REST
        })),
        text_color: theme::TEXT_PRIMARY,
        border: Border {
            color: if selected {
                theme::BORDER_BRIGHT
            } else {
                theme::BORDER
            },
            width: if selected { 2.0 } else { 1.0 },
            radius: 4.0.into(),
        },
        shadow: Shadow {
            color: theme::HEAVY_SHADOW,
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
