use bdgm::game::ValidatedGame;
use iced::{
    Alignment, Color, Element,
    Length::Fill,
    Padding,
    widget::{Column, ProgressBar, button, column, container, row, text},
};

use crate::{
    core::{AppError, LaunchHandle, State},
    gui::view::widgets::centered_button_text,
};

use super::{
    message::Message,
    state::{AppState, Status},
};

#[cfg(windows)]
pub(super) mod raw_disc;

mod widgets;

pub(super) fn view(state: &AppState) -> Element<'_, Message> {
    if state.disc_picker_discs.is_some() {
        return disc_picker(state);
    }

    match &state.status {
        Status::Idle(_) | Status::Error(_) => {
            let content = main_view(state);
            let content: Element<'_, Message> = match &state.status {
                Status::Error(error) => content.push(error_view(error)).into(),
                _ => content.into(),
            };

            centered(content)
        }
        Status::Starting(state_change) => centered(starting_view(state_change)),
        Status::Playing(game, handle) => centered(playing_view(game, handle)),
    }
}

pub(super) fn main_view(state: &AppState) -> Column<'_, Message> {
    let buttons = row![
        button(centered_button_text("Open Disc"))
            .width(150)
            .height(50)
            .on_press(Message::OpenDiscClicked),
        button(centered_button_text("Open Image"))
            .width(150)
            .height(50)
            .on_press(Message::OpenImageClicked),
    ]
    .spacing(10);

    let mut content = column![buttons].spacing(10).align_x(Alignment::Center);

    if let Status::Idle(Some((game, exit_code))) = &state.status {
        content = content.push(text(exit_message(game, exit_code)).size(14));
    }

    content
}

pub(super) fn disc_picker(state: &AppState) -> Element<'_, Message> {
    #[cfg(not(windows))]
    let _ = state;

    let content: iced::widget::Column<'_, Message> = column![];
    let mut content = content.spacing(10).align_x(Alignment::Center);

    if cfg!(windows) {
        #[cfg(windows)]
        if let Some(discs) = &state.disc_picker_discs {
            for disc in discs {
                content = content.push(
                    button(centered_button_text(format!("Drive {disc}:")))
                        .width(150)
                        .height(50)
                        .on_press(Message::RunRawDisc(*disc)),
                );
            }
        }
    } else {
        content = content.push(text("Reading raw discs is only supported on Windows."));
    }

    content = content.push(
        button(centered_button_text("Cancel"))
            .width(150)
            .height(50)
            .on_press(Message::DiscPickerCancel),
    );

    centered(content)
}

pub(super) fn starting_view(state: &State) -> Element<'_, Message> {
    let mut component = column![text(state.to_string()).size(16)]
        .spacing(4)
        .align_x(Alignment::Center);
    if let State::Extracting(Some((current, total))) = state {
        let fraction = if *total > 0 {
            (*current as f64 / *total as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        component = component.push(ProgressBar::new(0.0..=1.0, fraction as f32).length(350));
    }

    component.into()
}

pub(super) fn playing_view<'a>(
    game: &'a ValidatedGame,
    handle: &'a LaunchHandle,
) -> Element<'a, Message> {
    let mut content = column![
        text(format!(
            "Playing {} version {}...",
            game.name(),
            game.version()
        ))
        .size(22),
        text(format!("ID: {}", game.id())).size(14),
    ]
    .spacing(4)
    .align_x(Alignment::Center);

    let mut buttons = row![
        button(centered_button_text("Stop"))
            .width(150)
            .height(50)
            .on_press(Message::StopGameClicked)
    ]
    .spacing(10)
    .padding(Padding::new(0.0).top(10));

    if let LaunchHandle::HtmlServer(server) = handle {
        content = content.push(text(format!("Serving at: {}", server.get_url())).size(14));
        buttons = buttons.push(
            button(centered_button_text("Open"))
                .width(150)
                .height(50)
                .on_press(Message::OpenHTMLGameClicked),
        );
    }

    content = content.push(buttons);
    container(content).padding(8).into()
}

pub(super) fn error_view(error: &AppError) -> Element<'_, Message> {
    text(error.to_string())
        .size(14)
        .color(Color::from_rgb(1.0, 0.2, 0.2))
        .into()
}

fn exit_message(game: &ValidatedGame, exit_code: &Option<i32>) -> String {
    match exit_code {
        Some(code) if *code != 0 => format!("{} exited with code {code}.", game.name()),
        _ => format!("{} exited.", game.name()),
    }
}

fn centered<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content).center_x(Fill).center_y(Fill).into()
}
