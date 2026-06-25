use iced::widget::{button, column, container, opaque, row, stack, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Theme, Vector};

use crate::theme;

pub struct ConfirmationDialog<Message> {
    title: String,
    body: String,
    cancel_label: String,
    confirm_label: String,
    on_cancel: Message,
    on_confirm: Message,
}

impl<Message> ConfirmationDialog<Message> {
    pub fn new(
        title: impl Into<String>,
        body: impl Into<String>,
        cancel_label: impl Into<String>,
        confirm_label: impl Into<String>,
        on_cancel: Message,
        on_confirm: Message,
    ) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            cancel_label: cancel_label.into(),
            confirm_label: confirm_label.into(),
            on_cancel,
            on_confirm,
        }
    }
}

impl<Message: Clone + 'static> ConfirmationDialog<Message> {
    pub fn overlay<'a>(self, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
        let dialog = container(
            column![
                column![
                    text(self.title).size(20).color(theme::TEXT_PRIMARY),
                    text(self.body)
                        .size(13)
                        .line_height(1.35)
                        .color(theme::TEXT_MUTED)
                ]
                .spacing(8),
                row![
                    button(text(self.cancel_label).size(13))
                        .padding([9, 16])
                        .style(secondary_button_style)
                        .on_press(self.on_cancel),
                    button(text(self.confirm_label).size(13))
                        .padding([9, 16])
                        .style(danger_button_style)
                        .on_press(self.on_confirm)
                ]
                .spacing(10)
                .align_y(Alignment::Center)
            ]
            .spacing(22),
        )
        .width(Length::Fill)
        .max_width(400)
        .padding(24)
        .style(|_| container::Style {
            background: Some(Background::Color(theme::SURFACE_ALT)),
            border: Border {
                color: theme::BORDER,
                width: 1.0,
                radius: 12.0.into(),
            },
            shadow: Shadow {
                color: theme::SHADOW,
                offset: Vector::new(0.0, 8.0),
                blur_radius: 24.0,
            },
            ..Default::default()
        });

        let overlay = container(dialog)
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .padding(24)
            .style(|_| container::Style {
                background: Some(Background::Color(theme::OVERLAY_BACKDROP)),
                ..Default::default()
            });

        stack![content.into(), opaque(overlay)].into()
    }
}

fn secondary_button_style(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => theme::SURFACE_HOVER,
            button::Status::Pressed => theme::SURFACE_ALT,
            button::Status::Disabled => theme::SURFACE,
            button::Status::Active => theme::SURFACE_RAISED,
        })),
        text_color: theme::TEXT_PRIMARY,
        border: Border {
            color: theme::BORDER,
            width: 1.0,
            radius: 9.0.into(),
        },
        ..Default::default()
    }
}

fn danger_button_style(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => theme::DANGER_HOVER,
            button::Status::Pressed => theme::DANGER_PRESSED,
            button::Status::Disabled => theme::DANGER_DISABLED,
            button::Status::Active => theme::DANGER,
        })),
        text_color: Color::WHITE,
        border: Border {
            radius: 9.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
