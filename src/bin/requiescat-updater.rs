#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use iced::stream;
use iced::widget::{button, column, container, progress_bar, text};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Size, Subscription, Task, window,
};
use requiescat::updater::{self, LauncherProgress};
use requiescat::windowing;

fn main() -> iced::Result {
    iced::application(
        UpdaterWindow::boot,
        UpdaterWindow::update,
        UpdaterWindow::view,
    )
    .title(UpdaterWindow::title)
    .subscription(UpdaterWindow::subscription)
    .window(window::Settings {
        icon: windowing::application_icon(),
        size: Size::new(360.0, 150.0),
        resizable: false,
        ..Default::default()
    })
    .centered()
    .run()
}

#[derive(Debug, Clone)]
enum Message {
    Progress(LauncherProgress),
    Finished(Result<(), String>),
    Close,
}

#[derive(Debug, Clone)]
enum Status {
    Checking,
    UpToDate,
    Downloading { version: String },
    Installing,
    Launching,
    Failed(String),
}

struct UpdaterWindow {
    status: Status,
    finished: bool,
}

impl UpdaterWindow {
    fn boot() -> (Self, Task<Message>) {
        (
            Self {
                status: Status::Checking,
                finished: false,
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        "Requiescat Updater".to_owned()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Progress(progress) => {
                self.status = match progress {
                    LauncherProgress::CheckingForUpdates => Status::Checking,
                    LauncherProgress::UpToDate => Status::UpToDate,
                    LauncherProgress::Downloading { version } => Status::Downloading { version },
                    LauncherProgress::Installing => Status::Installing,
                    LauncherProgress::CheckFailed { error } => Status::Failed(error),
                    LauncherProgress::LaunchingApplication => Status::Launching,
                };
            }
            Message::Finished(Ok(())) => {
                self.finished = true;
                return iced::exit();
            }
            Message::Finished(Err(error)) => {
                self.finished = true;
                self.status = Status::Failed(error);
            }
            Message::Close => return iced::exit(),
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let (title, detail, progress) = match &self.status {
            Status::Checking => (
                "Checking for updates",
                "Looking for the latest Requiescat release.",
                0.15,
            ),
            Status::UpToDate => ("Requiescat is up to date", "Opening the application.", 0.35),
            Status::Downloading { version } => ("Downloading update", version.as_str(), 0.55),
            Status::Installing => (
                "Installing update",
                "Preparing the updated application.",
                0.8,
            ),
            Status::Launching => ("Opening Requiescat", "Starting the application.", 1.0),
            Status::Failed(error) => ("Update failed", error.as_str(), 1.0),
        };

        let mut content = column![
            text(title)
                .size(18)
                .color(Color::from_rgb(0.94, 0.98, 0.97)),
            text(detail)
                .size(12)
                .color(Color::from_rgb(0.65, 0.77, 0.75)),
            container(progress_bar(0.0..=1.0, progress)).width(Length::Fill),
        ]
        .spacing(12)
        .align_x(Alignment::Start);

        if matches!(self.status, Status::Failed(_)) {
            content = content.push(
                button(text("Close").size(12))
                    .on_press(Message::Close)
                    .padding([7, 14]),
            );
        }

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(22)
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgb(0.035, 0.105, 0.11))),
                border: Border {
                    radius: 10.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.finished {
            Subscription::none()
        } else {
            Subscription::run(update_worker)
        }
    }
}

fn update_worker() -> impl iced::futures::Stream<Item = Message> {
    stream::channel(16, async |mut output| {
        let result = updater::run_launcher_mode_with_progress(|progress| {
            let _ = output.try_send(Message::Progress(progress));
        });

        let _ = output.try_send(Message::Finished(result.map_err(|error| error.to_string())));
    })
}
