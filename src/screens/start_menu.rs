use std::path::{Path, PathBuf};

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::localization::{Localizer, MessageId};
use crate::persistence::CemeteryFile;

const BACKGROUND: Color = Color::from_rgb(0.035, 0.105, 0.11);
const PANEL: Color = Color::from_rgb(0.055, 0.17, 0.18);
const PANEL_RAISED: Color = Color::from_rgb(0.075, 0.225, 0.235);
const ACCENT: Color = Color::from_rgb(0.22, 0.76, 0.70);
const ACCENT_DARK: Color = Color::from_rgb(0.08, 0.43, 0.42);
const TEXT_PRIMARY: Color = Color::from_rgb(0.94, 0.98, 0.97);
const TEXT_MUTED: Color = Color::from_rgb(0.65, 0.77, 0.75);
const BORDER_COLOR: Color = Color::from_rgb(0.12, 0.36, 0.37);

#[derive(Debug, Clone)]
pub enum Message {
    ShowCemeteries,
    Back,
    OpenCemetery(PathBuf),
    ShowCreateCemetery,
    CemeteryNameChanged(String),
    SubmitCreateCemetery,
    ImportCemetery,
    ExportSelected,
}

pub fn view<'a>(
    localizer: &'a Localizer,
    cemeteries: &'a [CemeteryFile],
    selected: Option<&Path>,
    show_cemeteries: bool,
    show_create_cemetery: bool,
    new_cemetery_name: &'a str,
    status: Option<String>,
) -> Element<'a, Message> {
    if show_create_cemetery {
        return create_cemetery_form(localizer, new_cemetery_name, status);
    }

    if show_cemeteries {
        return cemetery_list(localizer, cemeteries, selected, status);
    }

    let library_summary = match cemeteries.len() {
        0 => localizer.text(MessageId::LibraryEmpty),
        count => localizer.count(MessageId::LibraryCount, count),
    };

    let brand = container(
        column![
            row![
                container(text("R").size(18).color(BACKGROUND))
                    .width(42)
                    .height(42)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .style(|_| container::Style {
                        background: Some(Background::Color(ACCENT)),
                        border: Border {
                            radius: 10.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                text("Requiescat").size(17).color(TEXT_PRIMARY)
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            column![
                text(localizer.text(MessageId::BrandTagline))
                    .size(24)
                    .line_height(1.15)
                    .color(TEXT_PRIMARY),
                text(localizer.text(MessageId::BrandDescription))
                    .size(13)
                    .line_height(1.4)
                    .color(TEXT_MUTED)
            ]
            .spacing(10),
            text(library_summary).size(12).color(TEXT_MUTED)
        ]
        .spacing(24),
    )
    .width(Length::Fixed(270.0))
    .height(Length::Fill)
    .padding(28)
    .style(|_| container::Style {
        background: Some(Background::Color(Color::from_rgb(0.04, 0.14, 0.145))),
        ..Default::default()
    });

    let (heading, supporting_text) = if cemeteries.is_empty() {
        (
            localizer.text(MessageId::SetupLibrary),
            localizer.text(MessageId::SetupLibraryDescription),
        )
    } else {
        (
            localizer.text(MessageId::WelcomeBack),
            localizer.text(MessageId::WelcomeBackDescription),
        )
    };

    let mut action_buttons = column![].spacing(10);

    if cemeteries.is_empty() {
        action_buttons = action_buttons
            .push(menu_button(
                localizer.text(MessageId::CreateNewCemetery),
                Message::ShowCreateCemetery,
                true,
            ))
            .push(menu_button(
                localizer.text(MessageId::ImportCemetery),
                Message::ImportCemetery,
                false,
            ));
    } else {
        action_buttons = action_buttons
            .push(menu_button(
                localizer.text(MessageId::OpenCemetery),
                Message::ShowCemeteries,
                true,
            ))
            .push(menu_button(
                localizer.text(MessageId::CreateNewCemetery),
                Message::ShowCreateCemetery,
                false,
            ))
            .push(menu_button(
                localizer.text(MessageId::ImportCemetery),
                Message::ImportCemetery,
                false,
            ));

        if let Some(selected) = selected {
            let export_label = selected
                .file_stem()
                .and_then(|name| name.to_str())
                .map(|name| localizer.value(MessageId::ExportNamedCemetery, "name", name))
                .unwrap_or_else(|| localizer.text(MessageId::ExportCemetery));

            action_buttons =
                action_buttons.push(menu_button(export_label, Message::ExportSelected, false));
        }
    }

    let actions = container(
        column![
            column![
                text(heading).size(24).color(TEXT_PRIMARY),
                text(supporting_text).size(13).color(TEXT_MUTED)
            ]
            .spacing(6),
            action_buttons,
            status_view(status)
        ]
        .spacing(24),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(28);

    let panel = container(row![brand, actions])
        .width(Length::Fill)
        .height(Length::Fill)
        .max_height(330)
        .style(|_| panel_style(16.0));

    screen(panel)
}

fn create_cemetery_form<'a>(
    localizer: &'a Localizer,
    name: &'a str,
    status: Option<String>,
) -> Element<'a, Message> {
    let create = button(
        text(localizer.text(MessageId::CreateCemetery))
            .size(14)
            .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([11, 16])
    .style(primary_button_style);
    let create = if name.trim().is_empty() {
        create
    } else {
        create.on_press(Message::SubmitCreateCemetery)
    };

    let content = column![
        column![
            text(localizer.text(MessageId::CreateNewCemetery))
                .size(24)
                .color(TEXT_PRIMARY),
            text(localizer.text(MessageId::CreateCemeteryDescription))
                .size(13)
                .color(TEXT_MUTED)
        ]
        .spacing(6),
        text_input(&localizer.text(MessageId::CemeteryName), name)
            .on_input(Message::CemeteryNameChanged)
            .on_submit(Message::SubmitCreateCemetery)
            .padding(11),
        column![
            create,
            menu_button(localizer.text(MessageId::BackToMenu), Message::Back, false)
        ]
        .spacing(10),
        status_view(status)
    ]
    .spacing(20);

    let panel = container(content)
        .width(Length::Fill)
        .max_width(440)
        .padding(28)
        .style(|_| panel_style(16.0));

    screen(panel)
}

fn cemetery_list<'a>(
    localizer: &'a Localizer,
    cemeteries: &'a [CemeteryFile],
    selected: Option<&Path>,
    status: Option<String>,
) -> Element<'a, Message> {
    let header = row![
        button(text("<").size(18))
            .on_press(Message::Back)
            .width(36)
            .height(36)
            .padding(0)
            .style(quiet_button_style),
        column![
            text(localizer.text(MessageId::CemeteryLibrary))
                .size(22)
                .color(TEXT_PRIMARY),
            text(localizer.text(MessageId::ChooseCemetery))
                .size(13)
                .color(TEXT_MUTED)
        ]
        .spacing(4)
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let mut entries = column![].spacing(10).width(Length::Fill);

    if cemeteries.is_empty() {
        entries = entries.push(
            container(
                column![
                    text(localizer.text(MessageId::NoCemeteries))
                        .size(18)
                        .color(TEXT_PRIMARY),
                    text(localizer.text(MessageId::NoCemeteriesDescription))
                        .size(13)
                        .color(TEXT_MUTED),
                    button(text(localizer.text(MessageId::ImportCemetery)))
                        .on_press(Message::ImportCemetery)
                        .padding([10, 16])
                        .style(primary_button_style)
                ]
                .spacing(12)
                .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .padding(32)
            .style(|_| container::Style {
                background: Some(Background::Color(PANEL)),
                border: Border {
                    color: BORDER_COLOR,
                    width: 1.0,
                    radius: 14.0.into(),
                },
                ..Default::default()
            }),
        );
    } else {
        for cemetery in cemeteries {
            let is_selected = selected == Some(cemetery.path());
            let detail = cemetery
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
                .unwrap_or_else(|| localizer.text(MessageId::SqliteCemetery));

            entries = entries.push(
                container(
                    row![
                        column![
                            text(cemetery.name()).size(16).color(TEXT_PRIMARY),
                            text(detail).size(12).color(TEXT_MUTED)
                        ]
                        .spacing(3)
                        .width(Length::Fill),
                        button(text(localizer.text(MessageId::Open)))
                            .on_press(Message::OpenCemetery(cemetery.path().to_owned()))
                            .padding([8, 14])
                            .style(primary_button_style)
                    ]
                    .align_y(Alignment::Center)
                    .spacing(12),
                )
                .padding([11, 14])
                .style(move |_| container::Style {
                    background: Some(Background::Color(if is_selected {
                        Color::from_rgb(0.065, 0.20, 0.205)
                    } else {
                        PANEL
                    })),
                    border: Border {
                        color: if is_selected {
                            ACCENT_DARK
                        } else {
                            BORDER_COLOR
                        },
                        width: 1.0,
                        radius: 9.0.into(),
                    },
                    ..Default::default()
                }),
            );
        }
    }

    let content = column![
        header,
        scrollable(entries).height(Length::Fill).width(Length::Fill),
        status_view(status)
    ]
    .spacing(20)
    .width(Length::Fill);

    let panel = container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .max_height(420)
        .padding(24)
        .style(|_| panel_style(14.0));

    screen(panel)
}

fn menu_button<'a>(
    label: impl text::IntoFragment<'a>,
    message: Message,
    primary: bool,
) -> button::Button<'a, Message> {
    button(text(label).size(14).align_x(Alignment::Center))
        .width(Length::Fill)
        .padding([11, 16])
        .style(move |theme, status| {
            if primary {
                primary_button_style(theme, status)
            } else {
                secondary_button_style(theme, status)
            }
        })
        .on_press(message)
}

fn status_view<'a>(status: Option<String>) -> Element<'a, Message> {
    match status {
        Some(status) => container(text(status).size(12).color(TEXT_MUTED))
            .width(Length::Fill)
            .padding([9, 12])
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgb(0.04, 0.14, 0.145))),
                border: Border {
                    color: BORDER_COLOR,
                    width: 1.0,
                    radius: 9.0.into(),
                },
                ..Default::default()
            })
            .into(),
        None => container(text("")).height(0).into(),
    }
}

fn screen<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(
        container(content)
            .width(Length::Fill)
            .max_width(760)
            .center_x(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(24)
    .center(Length::Fill)
    .style(|_| container::Style {
        background: Some(Background::Color(BACKGROUND)),
        ..Default::default()
    })
    .into()
}

fn panel_style(radius: f32) -> container::Style {
    container::Style {
        background: Some(Background::Color(PANEL)),
        border: Border {
            color: BORDER_COLOR,
            width: 1.0,
            radius: radius.into(),
        },
        ..Default::default()
    }
}

fn primary_button_style(_: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => Color::from_rgb(0.30, 0.84, 0.77),
        button::Status::Pressed => Color::from_rgb(0.17, 0.64, 0.59),
        button::Status::Disabled => Color::from_rgb(0.12, 0.26, 0.26),
        button::Status::Active => ACCENT,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: if status == button::Status::Disabled {
            Color::from_rgb(0.40, 0.51, 0.50)
        } else {
            BACKGROUND
        },
        border: Border {
            radius: 10.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn secondary_button_style(_: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => PANEL_RAISED,
        button::Status::Pressed => Color::from_rgb(0.06, 0.19, 0.20),
        button::Status::Disabled => Color::from_rgb(0.045, 0.14, 0.145),
        button::Status::Active => Color::from_rgb(0.06, 0.20, 0.205),
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: if status == button::Status::Disabled {
            Color::from_rgb(0.37, 0.48, 0.47)
        } else {
            TEXT_PRIMARY
        },
        border: Border {
            color: if status == button::Status::Hovered {
                ACCENT_DARK
            } else {
                BORDER_COLOR
            },
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

fn quiet_button_style(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(if status == button::Status::Hovered {
            PANEL_RAISED
        } else {
            Color::TRANSPARENT
        })),
        text_color: TEXT_PRIMARY,
        border: Border {
            color: BORDER_COLOR,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}
