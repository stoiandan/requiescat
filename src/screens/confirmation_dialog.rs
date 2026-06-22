use iced::widget::{button, column, container, opaque, row, stack, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Theme, Vector};

const BACKDROP: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.45,
};
const PANEL: Color = Color::from_rgb(0.055, 0.17, 0.18);
const PANEL_RAISED: Color = Color::from_rgb(0.075, 0.225, 0.235);
const DANGER: Color = Color::from_rgb(0.82, 0.22, 0.20);
const DANGER_HOVER: Color = Color::from_rgb(0.95, 0.31, 0.28);
const TEXT_PRIMARY: Color = Color::from_rgb(0.94, 0.98, 0.97);
const TEXT_MUTED: Color = Color::from_rgb(0.65, 0.77, 0.75);
const BORDER_COLOR: Color = Color::from_rgb(0.12, 0.36, 0.37);

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
                    text(self.title).size(20).color(TEXT_PRIMARY),
                    text(self.body).size(13).line_height(1.35).color(TEXT_MUTED)
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
            background: Some(Background::Color(PANEL)),
            border: Border {
                color: BORDER_COLOR,
                width: 1.0,
                radius: 12.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba8(0, 0, 0, 0.35),
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
                background: Some(Background::Color(BACKDROP)),
                ..Default::default()
            });

        stack![content.into(), opaque(overlay)].into()
    }
}

fn secondary_button_style(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => PANEL_RAISED,
            button::Status::Pressed => Color::from_rgb(0.06, 0.19, 0.20),
            button::Status::Disabled => Color::from_rgb(0.045, 0.14, 0.145),
            button::Status::Active => Color::from_rgb(0.06, 0.20, 0.205),
        })),
        text_color: TEXT_PRIMARY,
        border: Border {
            color: BORDER_COLOR,
            width: 1.0,
            radius: 9.0.into(),
        },
        ..Default::default()
    }
}

fn danger_button_style(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => DANGER_HOVER,
            button::Status::Pressed => Color::from_rgb(0.64, 0.16, 0.15),
            button::Status::Disabled => Color::from_rgb(0.28, 0.14, 0.14),
            button::Status::Active => DANGER,
        })),
        text_color: Color::WHITE,
        border: Border {
            radius: 9.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
